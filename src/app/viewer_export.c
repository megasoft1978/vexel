#include "viewer_internal.h"

#include "platform/macos/platform_clipboard.h"
#include "platform/macos/platform_dialog.h"
#include "platform/macos/platform_png.h"
#include "platform/platform_atomic_file.h"
#include "platform/platform_atomic_new_file.h"
#include "raster/shape_render.h"

#include <errno.h>
#include <math.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>

#define VIEWER_EXPORT_SOURCE_LIMIT (64U * 1024U * 1024U)

svg_shape_media_providers svg_app_media_providers(void);

static void export_error(viewer_app *app, const char *message)
{
    if (app != NULL && message != NULL) {
        viewer_editor_set_status(app->editor, message);
        (void)platform_dialog_show_error("PNG export failed",
            sizeof("PNG export failed") - 1U, message, strlen(message));
    }
}

static uint8_t maximum_export_scale(const svg_shape_document *document)
{
    if (document == NULL || document->width == 0U || document->height == 0U) return 1U;
    uint32_t axis = document->width > document->height ?
        document->width : document->height;
    uint64_t pixels = (uint64_t)document->width * document->height;
    if (axis <= 1024U && pixels <= PLATFORM_PNG_MAX_PIXELS / 16U) return 4U;
    if (axis <= 2048U && pixels <= PLATFORM_PNG_MAX_PIXELS / 4U) return 2U;
    return 1U;
}

static bool save_png_file(base_bytes path, base_bytes bytes)
{
    if (!base_bytes_valid(path) || path.len == 0U || !base_bytes_valid(bytes))
        return false;
    char path_text[PLATFORM_DIALOG_PATH_CAP];
    if (path.len >= sizeof(path_text) || memchr(path.ptr, '\0', path.len) != NULL)
        return false;
    memcpy(path_text, path.ptr, path.len);
    path_text[path.len] = '\0';
    struct stat info;
    if (lstat(path_text, &info) == 0) {
        platform_atomic_file_result replaced = platform_atomic_file_replace(
            path, bytes, NULL);
        return replaced.error == PLATFORM_ATOMIC_FILE_OK;
    }
    if (errno != ENOENT) return false;
    platform_atomic_new_file_result created = platform_atomic_file_create_new(
        path, bytes, NULL);
    return created.error == PLATFORM_ATOMIC_NEW_FILE_OK;
}

static bool crop_selection(const raster_tile *source, svg_shape_rect bounds,
    uint8_t scale, uint8_t **out_pixels, uint32_t *out_width,
    uint32_t *out_height, size_t *out_stride)
{
    if (source == NULL || source->rgba == NULL || out_pixels == NULL ||
        out_width == NULL || out_height == NULL || out_stride == NULL ||
        !isfinite(bounds.x) || !isfinite(bounds.y) ||
        !isfinite(bounds.width) || !isfinite(bounds.height) ||
        bounds.width < 0.0 || bounds.height < 0.0) return false;
    double x0d = floor(bounds.x * scale);
    double y0d = floor(bounds.y * scale);
    double x1d = ceil((bounds.x + bounds.width) * scale);
    double y1d = ceil((bounds.y + bounds.height) * scale);
    x0d = fmax(0.0, fmin(x0d, source->width));
    y0d = fmax(0.0, fmin(y0d, source->height));
    x1d = fmax(0.0, fmin(x1d, source->width));
    y1d = fmax(0.0, fmin(y1d, source->height));
    if (!(x1d > x0d)) x1d = fmin((double)source->width, x0d + 1.0);
    if (!(y1d > y0d)) y1d = fmin((double)source->height, y0d + 1.0);
    if (!(x1d > x0d) || !(y1d > y0d)) return false;
    uint32_t x0 = (uint32_t)x0d, y0 = (uint32_t)y0d;
    uint32_t width = (uint32_t)(x1d - x0d), height = (uint32_t)(y1d - y0d);
    if ((size_t)width > SIZE_MAX / 4U) return false;
    size_t stride = (size_t)width * 4U;
    if ((size_t)height > SIZE_MAX / stride || stride * height > PLATFORM_PNG_MAX_BYTES)
        return false;
    uint8_t *pixels = malloc(stride * (size_t)height);
    if (pixels == NULL) return false;
    for (uint32_t row = 0U; row < height; ++row) {
        size_t source_offset = ((size_t)(y0 + row) * source->width + x0) * 4U;
        memcpy(pixels + (size_t)row * stride, source->rgba + source_offset, stride);
    }
    *out_pixels = pixels;
    *out_width = width;
    *out_height = height;
    *out_stride = stride;
    return true;
}

bool viewer_export_png(viewer_app *app, bool copy_to_clipboard)
{
    if (app == NULL || app->editor == NULL || !app->document_visible) return false;
    size_t source_length = viewer_editor_byte_length(app->editor);
    if (source_length == 0U || source_length > VIEWER_EXPORT_SOURCE_LIMIT) {
        export_error(app, "The current SVG exceeds the 64 MiB export limit");
        return false;
    }
    uint8_t *source = malloc(source_length);
    if (source == NULL) { export_error(app, "SVG export allocation failed"); return false; }
    size_t materialized = 0U;
    if (!viewer_editor_materialize(app->editor, source, source_length,
            &materialized) || materialized != source_length) {
        free(source);
        export_error(app, "Could not read the current SVG source");
        return false;
    }
    svg_shape_document document = {0};
    svg_shape_diagnostic diagnostic = {0};
    svg_shape_media_providers providers = svg_app_media_providers();
    svg_shape_error decoded = svg_shape_decode_with_media(
        (base_bytes){source, source_length}, &providers, &document, &diagnostic);
    if (decoded != SVG_SHAPE_ERROR_NONE) {
        free(source);
        export_error(app, "Could not render the current SVG source");
        return false;
    }
    uint64_t document_pixels = (uint64_t)document.width * document.height;
    if (document.width == 0U || document.height == 0U ||
        document_pixels > PLATFORM_PNG_MAX_PIXELS) {
        svg_shape_document_destroy(&document);
        free(source);
        export_error(app, "The current SVG exceeds the 16 million PNG pixel limit");
        return false;
    }

    char path[PLATFORM_DIALOG_PATH_CAP] = {0};
    size_t path_length = 0U;
    uint8_t scale = 1U;
    svg_shape_rect selection_bounds = {0};
    bool has_selection = viewer_editor_get_selection_bounds(app->editor,
        &document, &selection_bounds);
    bool selection_only = copy_to_clipboard && has_selection;
    if (!copy_to_clipboard) {
        platform_dialog_status chosen = platform_dialog_save_png(
            app->display_name, strlen(app->display_name),
            maximum_export_scale(&document), path, sizeof(path), &path_length,
            &scale, &selection_only);
        if (chosen != PLATFORM_DIALOG_ACCEPT) {
            svg_shape_document_destroy(&document);
            free(source);
            if (chosen == PLATFORM_DIALOG_ERROR)
                export_error(app, "Could not open the PNG export panel");
            return chosen == PLATFORM_DIALOG_CANCEL;
        }
    }
    if (selection_only && !has_selection) {
        svg_shape_document_destroy(&document);
        free(source);
        export_error(app, "Select a rendered element before exporting the selection");
        return false;
    }
    if (document.width == 0U || document.height == 0U ||
        (uint32_t)scale > 4096U / document.width ||
        (uint32_t)scale > 4096U / document.height) {
        svg_shape_document_destroy(&document);
        free(source);
        export_error(app, "The selected PNG scale exceeds the 4096 pixel limit");
        return false;
    }
    svg_shape_document render_document = document;
    render_document.width *= scale;
    render_document.height *= scale;
    raster_tile rendered = {0};
    if (!raster_shape_render(&render_document, &rendered, &diagnostic)) {
        svg_shape_document_destroy(&document);
        free(source);
        export_error(app, "The SVG renderer could not create the PNG pixels");
        return false;
    }
    const uint8_t *pixels = rendered.rgba;
    uint8_t *cropped = NULL;
    uint32_t output_width = rendered.width, output_height = rendered.height;
    size_t output_stride = (size_t)rendered.width * 4U;
    if (selection_only && !crop_selection(&rendered, selection_bounds, scale,
            &cropped, &output_width, &output_height, &output_stride)) {
        raster_tile_destroy(&rendered);
        svg_shape_document_destroy(&document);
        free(source);
        export_error(app, "The selected element is outside the PNG export bounds");
        return false;
    }
    if (selection_only) pixels = cropped;
    size_t output_bytes = output_stride * (size_t)output_height;
    platform_png_blob png = {0};
    bool encoded = platform_png_encode_rgba8(pixels, output_bytes, output_stride,
        output_width, output_height, &png);
    free(cropped);
    raster_tile_destroy(&rendered);
    svg_shape_document_destroy(&document);
    free(source);
    if (!encoded) {
        export_error(app, "ImageIO could not encode the PNG within its size limit");
        return false;
    }
    bool saved;
    if (copy_to_clipboard) {
        saved = platform_clipboard_write_png(png.bytes, png.length);
        if (saved) viewer_editor_set_status(app->editor, "Selection copied as PNG");
    } else {
        saved = save_png_file((base_bytes){(const uint8_t *)path, path_length},
            (base_bytes){png.bytes, png.length});
        if (saved) viewer_editor_set_status(app->editor, "PNG exported");
    }
    platform_png_blob_destroy(&png);
    if (!saved) export_error(app, copy_to_clipboard ?
        "Could not write the PNG to the system clipboard" :
        "Could not save the PNG file");
    return saved;
}
