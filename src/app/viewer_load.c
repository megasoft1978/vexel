#define _POSIX_C_SOURCE 200809L

#include "viewer_load.h"

#include "index.h"
#include "raster/shape_render.h"

#include <errno.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/types.h>

svg_shape_media_providers svg_app_media_providers(void);

#define VIEWER_LOAD_CHUNK_BYTES (1024U * 1024U)

static bool is_canceled(const platform_task_cancel_token *token)
{
    return token != NULL && atomic_load_explicit(&token->canceled, memory_order_acquire);
}

static bool xml_cancelled(void *context)
{
    return is_canceled(context);
}

static void set_load_error(viewer_loaded_document *document, const char *message)
{
    if (document == NULL || message == NULL) {
        return;
    }
    size_t length = 0U;
    while (length + 1U < sizeof(document->load_error) && message[length] != '\0') {
        length++;
    }
    memcpy(document->load_error, message, length);
    document->load_error[length] = '\0';
}

static bool read_bounded_file(const char *path, const platform_task_cancel_token *token,
                              viewer_loaded_document *document)
{
    FILE *file = fopen(path, "rb");
    if (file == NULL) {
        set_load_error(document, "cannot open file");
        return false;
    }
    bool okay = fseeko(file, 0, SEEK_END) == 0;
    off_t raw_size = okay ? ftello(file) : (off_t)-1;
    if (raw_size < 0 || (uintmax_t)raw_size > (uintmax_t)PROJECT_BUNDLE_MAX_BYTES ||
        fseeko(file, 0, SEEK_SET) != 0) {
        set_load_error(document, "file exceeds the 1 GiB document limit or cannot be sized");
        (void)fclose(file);
        return false;
    }
    size_t length = (size_t)raw_size;
    uint8_t *source = malloc(length == 0U ? 1U : length);
    if (source == NULL) {
        set_load_error(document, "source allocation failed");
        (void)fclose(file);
        return false;
    }
    size_t offset = 0U;
    while (offset < length) {
        if (is_canceled(token)) {
            free(source);
            (void)fclose(file);
            return false;
        }
        size_t amount = length - offset;
        if (amount > VIEWER_LOAD_CHUNK_BYTES) {
            amount = VIEWER_LOAD_CHUNK_BYTES;
        }
        size_t read = fread(source + offset, 1U, amount, file);
        if (read != amount) {
            free(source);
            set_load_error(document, "file read failed or changed while loading");
            (void)fclose(file);
            return false;
        }
        offset += read;
    }
    if (fclose(file) != 0) {
        free(source);
        set_load_error(document, "file close failed");
        return false;
    }
    document->source = source;
    document->source_owner = source;
    document->source_length = length;
    return true;
}

static viewer_load_status decode_document_source(
    const platform_task_cancel_token *token, viewer_loaded_document *out)
{
    if (is_canceled(token)) return VIEWER_LOAD_CANCELED;
    svg_shape_diagnostic diagnostic = {SVG_SHAPE_ERROR_NONE, 0U};
    svg_shape_media_providers providers = svg_app_media_providers();
    svg_shape_error error = svg_shape_decode_with_media(
        (base_bytes){out->source, out->source_length}, &providers,
        &out->shape, &diagnostic);
    if (error != SVG_SHAPE_ERROR_NONE) {
        out->shape_error = error;
        out->error_offset = diagnostic.offset;
        return VIEWER_LOAD_FAILED;
    }
    if (is_canceled(token)) return VIEWER_LOAD_CANCELED;
    if (!raster_shape_render(&out->shape, &out->tile, &diagnostic)) {
        out->shape_error = diagnostic.category == SVG_SHAPE_ERROR_NONE ?
            SVG_SHAPE_ERROR_INVALID_ARGUMENT : diagnostic.category;
        out->error_offset = diagnostic.offset;
        return VIEWER_LOAD_FAILED;
    }
    return is_canceled(token) ? VIEWER_LOAD_CANCELED : VIEWER_LOAD_OK;
}

viewer_load_status viewer_decode_owned_source(uint8_t *source, size_t source_length,
    const platform_task_cancel_token *token, viewer_loaded_document *out)
{
    if (out == NULL || (source == NULL && source_length != 0U)) return VIEWER_LOAD_FAILED;
    *out = (viewer_loaded_document){0};
    out->source = source;
    out->source_owner = source;
    out->source_length = source_length;
    if (source_length == 0U || source_length > (size_t)SVG_INDEX_MAX_SOURCE_BYTES) {
        set_load_error(out, "source is empty or exceeds the 1 GiB source limit");
        viewer_loaded_document_destroy(out);
        set_load_error(out, "source is empty or exceeds the 1 GiB source limit");
        return VIEWER_LOAD_FAILED;
    }
    viewer_load_status status = decode_document_source(token, out);
    if (status != VIEWER_LOAD_OK) {
        char error[sizeof(out->load_error)];
        memcpy(error, out->load_error, sizeof(error));
        svg_shape_error shape_error = out->shape_error;
        size_t error_offset = out->error_offset;
        viewer_loaded_document_destroy(out);
        memcpy(out->load_error, error, sizeof(out->load_error));
        out->shape_error = shape_error;
        out->error_offset = error_offset;
    }
    return status;
}

viewer_load_status viewer_load_document(const char *path,
                                        const platform_task_cancel_token *token,
                                        viewer_loaded_document *out)
{
    if (path == NULL || path[0] == '\0' || out == NULL) {
        return VIEWER_LOAD_FAILED;
    }
    *out = (viewer_loaded_document){0};
    if (is_canceled(token)) return VIEWER_LOAD_CANCELED;
    if (!read_bounded_file(path, token, out)) {
        if (is_canceled(token)) {
            return VIEWER_LOAD_CANCELED;
        }
        return VIEWER_LOAD_FAILED;
    }
    uint8_t *source = out->source_owner;
    size_t source_length = out->source_length;
    *out = (viewer_loaded_document){0};
    return viewer_decode_owned_project(source, source_length, token, out);
}

viewer_load_status viewer_decode_owned_project(uint8_t *source,
    size_t source_length, const platform_task_cancel_token *token,
    viewer_loaded_document *out)
{
    if (out == NULL || (source == NULL && source_length != 0U)) {
        free(source);
        return VIEWER_LOAD_FAILED;
    }
    *out = (viewer_loaded_document){0};
    if (is_canceled(token)) {
        free(source);
        return VIEWER_LOAD_CANCELED;
    }
    out->source = source;
    out->source_owner = source;
    out->source_length = source_length;
    if (project_bundle_has_magic((base_bytes){out->source_owner,
            out->source_length})) {
        if (out->source_length > PROJECT_BUNDLE_MAX_BYTES) {
            viewer_loaded_document_destroy(out);
            set_load_error(out, "native project exceeds the 1 GiB project limit");
            return VIEWER_LOAD_FAILED;
        }
        project_bundle_result decoded = project_bundle_decode(
            (base_bytes){out->source_owner, out->source_length}, &out->project);
        if (decoded.error != PROJECT_BUNDLE_OK) {
            viewer_loaded_document_destroy(out);
            set_load_error(out, "native project is corrupt or exceeds a format limit");
            return VIEWER_LOAD_FAILED;
        }
        out->native_project = true;
        if (!project_bundle_asset_overlay_init(&out->asset_overlay,
                &out->project)) {
            viewer_loaded_document_destroy(out);
            set_load_error(out, "native project assets could not be indexed");
            return VIEWER_LOAD_FAILED;
        }
        out->source = (uint8_t *)out->project.scene_svg.ptr;
        out->source_length = out->project.scene_svg.len;
        project_timeline_view timeline_view = {0};
        project_timeline_context_owner timeline_context = {0};
        project_timeline_integration_result timeline = project_timeline_validate_bundle_owned(
            &out->project, out->project.scene_svg, xml_cancelled, (void *)token,
            &timeline_view, &timeline_context);
        out->timeline_compatibility = timeline.compatibility;
        if (timeline.canceled || is_canceled(token)) {
            project_timeline_context_owner_destroy(&timeline_context);
            viewer_loaded_document_destroy(out);
            return VIEWER_LOAD_CANCELED;
        }
        if (timeline.validation.error != PROJECT_TIMELINE_OK) {
            char message[sizeof(out->load_error)];
            if (timeline.asset_name.len != 0U) {
                int precision = timeline.asset_name.len > 40U ? 40 :
                    (int)timeline.asset_name.len;
                (void)snprintf(message, sizeof(message),
                    "drawing asset %.*s invalid at byte %zu", precision,
                    (const char *)timeline.asset_name.ptr,
                    timeline.asset_validation.error_offset);
            } else {
                (void)snprintf(message, sizeof(message), "native timeline %s at byte %zu",
                    project_timeline_error_name(timeline.validation.error),
                    timeline.validation.error_offset);
            }
            project_timeline_error error = timeline.validation.error;
            size_t error_offset = timeline.validation.error_offset;
            svg_shape_error asset_error = timeline.asset_validation.error;
            project_timeline_context_owner_destroy(&timeline_context);
            viewer_loaded_document_destroy(out);
            set_load_error(out, message);
            out->timeline_error = error;
            out->shape_error = asset_error;
            out->error_offset = error_offset;
            return VIEWER_LOAD_FAILED;
        }
        if (timeline.compatibility == PROJECT_TIMELINE_COMPATIBILITY_NATIVE_V1) {
            project_timeline_edit_result session = project_timeline_session_create(
                out->project.timeline_json, &timeline_context,
                project_timeline_edit_default_limits(), &out->timeline_session);
            if (session.error != PROJECT_TIMELINE_OK) {
                project_timeline_context_owner_destroy(&timeline_context);
                viewer_loaded_document_destroy(out);
                set_load_error(out, "native timeline session allocation failed");
                out->timeline_error = session.error;
                out->error_offset = session.error_offset;
                return VIEWER_LOAD_FAILED;
            }
        }
        project_timeline_context_owner_destroy(&timeline_context);
    }
    viewer_load_status status = decode_document_source(token, out);
    if (status != VIEWER_LOAD_OK) {
        char error[sizeof(out->load_error)];
        memcpy(error, out->load_error, sizeof(error));
        svg_shape_error shape_error = out->shape_error;
        size_t error_offset = out->error_offset;
        viewer_loaded_document_destroy(out);
        memcpy(out->load_error, error, sizeof(out->load_error));
        out->shape_error = shape_error;
        out->error_offset = error_offset;
    }
    return status;
}

void viewer_loaded_document_destroy(viewer_loaded_document *document)
{
    if (document == NULL) {
        return;
    }
    raster_tile_destroy(&document->tile);
    project_timeline_session_destroy(document->timeline_session);
    document->timeline_session = NULL;
    project_bundle_asset_overlay_destroy(&document->asset_overlay);
    svg_shape_document_destroy(&document->shape);
    free(document->source_owner == NULL ? document->source : document->source_owner);
    if (document->project_owner != document->source_owner)
        free(document->project_owner);
    *document = (viewer_loaded_document){0};
}
