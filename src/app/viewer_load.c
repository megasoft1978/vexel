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
    if (raw_size < 0 || (uintmax_t)raw_size > (uintmax_t)SVG_INDEX_MAX_SOURCE_BYTES ||
        fseeko(file, 0, SEEK_SET) != 0) {
        set_load_error(document, "file exceeds the 1 GiB source limit or cannot be sized");
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
    out->source_length = source_length;
    if (source_length == 0U || source_length > (size_t)SVG_INDEX_MAX_SOURCE_BYTES) {
        set_load_error(out, "source is empty or exceeds the 1 GiB source limit");
        return VIEWER_LOAD_FAILED;
    }
    return decode_document_source(token, out);
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
    return decode_document_source(token, out);
}

void viewer_loaded_document_destroy(viewer_loaded_document *document)
{
    if (document == NULL) {
        return;
    }
    raster_tile_destroy(&document->tile);
    svg_shape_document_destroy(&document->shape);
    free(document->source);
    *document = (viewer_loaded_document){0};
}
