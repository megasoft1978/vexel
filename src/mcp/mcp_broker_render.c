#define _DARWIN_C_SOURCE
#define _POSIX_C_SOURCE 200809L

#include "mcp_broker_internal.h"
#include "platform_atomic_new_file.h"
#include "platform/media/platform_animation.h"
#include "svg/shape/media.h"

#include "raster/shape_render.h"
#include "raster/tile.h"
#include "shape.h"

#include <errno.h>
#include <fcntl.h>
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

svg_shape_media_providers svg_app_media_providers(void);

#define MCP_RENDER_MAX_AXIS 512U
#define MCP_RENDER_PNG_LIMIT (2U * 1024U * 1024U)
#define MCP_ANIMATION_TOTAL_FRAME_PIXELS 16000000ULL

static bool buffer_byte(mcp_json_buffer *out, uint8_t value)
{
    return mcp_json_buffer_append(out, (const char *)&value, 1U);
}

static bool buffer_be32(mcp_json_buffer *out, uint32_t value)
{
    uint8_t bytes[4] = {(uint8_t)(value >> 24U), (uint8_t)(value >> 16U),
                        (uint8_t)(value >> 8U), (uint8_t)value};
    return mcp_json_buffer_append(out, (const char *)bytes, sizeof(bytes));
}

static uint32_t crc32_bytes(const uint8_t *data, size_t length)
{
    uint32_t value = 0xffffffffU;
    for (size_t index = 0U; index < length; index++) {
        value ^= data[index];
        for (unsigned bit = 0U; bit < 8U; bit++)
            value = (value >> 1U) ^ (0xedb88320U & (uint32_t)-(int32_t)(value & 1U));
    }
    return ~value;
}

static uint32_t adler32_bytes(const uint8_t *data, size_t length)
{
    uint32_t a = 1U, b = 0U;
    for (size_t index = 0U; index < length; index++) {
        a = (a + data[index]) % 65521U;
        b = (b + a) % 65521U;
    }
    return (b << 16U) | a;
}

static bool append_png_chunk(mcp_json_buffer *png, const char tag[4],
                             const uint8_t *payload, size_t length)
{
    if (length > UINT32_MAX || png->length > png->limit ||
        png->limit - png->length < 12U || length > png->limit - png->length - 12U) return false;
    size_t crc_input_length = length + 4U;
    uint8_t *crc_input = malloc(crc_input_length);
    if (crc_input == NULL) return false;
    memcpy(crc_input, tag, 4U);
    if (length > 0U) memcpy(crc_input + 4U, payload, length);
    uint32_t crc = crc32_bytes(crc_input, crc_input_length);
    free(crc_input);
    return buffer_be32(png, (uint32_t)length) &&
        mcp_json_buffer_append(png, tag, 4U) &&
        mcp_json_buffer_append(png, (const char *)payload, length) &&
        buffer_be32(png, crc);
}

static bool encode_png(const raster_tile *tile, mcp_json_buffer *out)
{
    if (tile == NULL || out == NULL || tile->rgba == NULL || tile->width == 0U || tile->height == 0U ||
        tile->width > MCP_RENDER_MAX_AXIS || tile->height > MCP_RENDER_MAX_AXIS) return false;
    size_t row_bytes = (size_t)tile->width * 4U;
    if (row_bytes > SIZE_MAX - 1U || (row_bytes + 1U) > SIZE_MAX / tile->height) return false;
    size_t raw_length = (row_bytes + 1U) * tile->height;
    if (raw_length > 2U * 1024U * 1024U) return false;
    uint8_t *raw = malloc(raw_length);
    if (raw == NULL) return false;
    for (uint32_t y = 0U; y < tile->height; y++) {
        size_t row_offset = (size_t)y * (row_bytes + 1U);
        raw[row_offset] = 0U;
        memcpy(raw + row_offset + 1U, tile->rgba + (size_t)y * row_bytes, row_bytes);
    }
    mcp_json_buffer deflate = {0};
    if (!mcp_json_buffer_init(&deflate, raw_length + 32U, MCP_RENDER_PNG_LIMIT)) { free(raw); return false; }
    bool okay = buffer_byte(&deflate, 0x78U) && buffer_byte(&deflate, 0x01U);
    size_t cursor = 0U;
    while (okay && cursor < raw_length) {
        size_t amount = raw_length - cursor;
        if (amount > 65535U) amount = 65535U;
        uint16_t chunk_length = (uint16_t)amount;
        uint16_t complement = (uint16_t)~chunk_length;
        uint8_t header[5] = {
            (uint8_t)(cursor + amount == raw_length ? 1U : 0U),
            (uint8_t)chunk_length, (uint8_t)(chunk_length >> 8U),
            (uint8_t)complement, (uint8_t)(complement >> 8U)};
        okay = mcp_json_buffer_append(&deflate, (const char *)header, sizeof(header)) &&
            mcp_json_buffer_append(&deflate, (const char *)raw + cursor, amount);
        cursor += amount;
    }
    uint32_t adler = adler32_bytes(raw, raw_length);
    uint8_t adler_bytes[4] = {(uint8_t)(adler >> 24U), (uint8_t)(adler >> 16U),
                              (uint8_t)(adler >> 8U), (uint8_t)adler};
    if (okay) okay = mcp_json_buffer_append(&deflate, (const char *)adler_bytes, sizeof(adler_bytes));
    free(raw);
    if (!okay) { mcp_json_buffer_destroy(&deflate); return false; }

    static const uint8_t signature[8] = {137U, 80U, 78U, 71U, 13U, 10U, 26U, 10U};
    uint8_t ihdr[13] = {
        (uint8_t)(tile->width >> 24U), (uint8_t)(tile->width >> 16U),
        (uint8_t)(tile->width >> 8U), (uint8_t)tile->width,
        (uint8_t)(tile->height >> 24U), (uint8_t)(tile->height >> 16U),
        (uint8_t)(tile->height >> 8U), (uint8_t)tile->height,
        8U, 6U, 0U, 0U, 0U};
    okay = mcp_json_buffer_append(out, (const char *)signature, sizeof(signature)) &&
        append_png_chunk(out, "IHDR", ihdr, sizeof(ihdr)) &&
        append_png_chunk(out, "IDAT", (const uint8_t *)deflate.data, deflate.length) &&
        append_png_chunk(out, "IEND", NULL, 0U);
    mcp_json_buffer_destroy(&deflate);
    return okay;
}

static char *encode_base64(const uint8_t *bytes, size_t length, size_t *out_length)
{
    static const char table[] = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    if (bytes == NULL || out_length == NULL || length > (SIZE_MAX - 2U) / 4U * 3U) return NULL;
    size_t output_length = ((length + 2U) / 3U) * 4U;
    if (output_length > MCP_RENDER_PNG_LIMIT) return NULL;
    char *output = malloc(output_length + 1U);
    if (output == NULL) return NULL;
    size_t input = 0U, cursor = 0U;
    while (input < length) {
        uint32_t value = (uint32_t)bytes[input++] << 16U;
        bool has_second = input < length;
        if (has_second) value |= (uint32_t)bytes[input++] << 8U;
        bool has_third = input < length;
        if (has_third) value |= bytes[input++];
        output[cursor++] = table[(value >> 18U) & 0x3fU];
        output[cursor++] = table[(value >> 12U) & 0x3fU];
        output[cursor++] = has_second ? table[(value >> 6U) & 0x3fU] : '=';
        output[cursor++] = has_third ? table[value & 0x3fU] : '=';
    }
    output[cursor] = '\0';
    *out_length = cursor;
    return output;
}

static bool parse_double_token(const mcp_json_doc *doc, size_t token, double *out)
{
    if (doc == NULL || token >= doc->token_count || doc->tokens[token].type != MCP_JSON_NUMBER || out == NULL)
        return false;
    char text[64];
    size_t length = 0U;
    if (!mcp_json_token_copy(doc, token, text, sizeof(text) - 1U, &length)) return false;
    text[length] = '\0';
    errno = 0;
    char *end = NULL;
    double value = strtod(text, &end);
    if (errno != 0 || end != text + length || !isfinite(value)) return false;
    *out = value;
    return true;
}

static bool region_number(const mcp_json_doc *request, size_t region, const char *name,
                          double *out, bool *present)
{
    size_t token = mcp_json_object_get(request, region, name);
    *present = token != MCP_JSON_NO_TOKEN;
    return !*present || parse_double_token(request, token, out);
}

static bool build_region(const mcp_json_doc *request, size_t args, double scale,
                         svg_shape_document *shape)
{
    size_t region = mcp_json_object_get(request, args, "region");
    if (region == MCP_JSON_NO_TOKEN) {
        double scaled_width = ceil((double)shape->width * scale);
        double scaled_height = ceil((double)shape->height * scale);
        if (!(scaled_width >= 1.0 && scaled_height >= 1.0) ||
            scaled_width > MCP_RENDER_MAX_AXIS || scaled_height > MCP_RENDER_MAX_AXIS) return false;
        shape->width = (uint32_t)scaled_width;
        shape->height = (uint32_t)scaled_height;
        return true;
    }
    double x = 0.0, y = 0.0, width = 0.0, height = 0.0;
    bool has_x = false, has_y = false, has_width = false, has_height = false;
    if (!region_number(request, region, "x", &x, &has_x) ||
        !region_number(request, region, "y", &y, &has_y) ||
        !region_number(request, region, "width", &width, &has_width) ||
        !region_number(request, region, "height", &height, &has_height) ||
        !has_x || !has_y || !has_width || !has_height || !(width > 0.0) || !(height > 0.0) ||
        fabs(x) > 1000000.0 || fabs(y) > 1000000.0) return false;
    double pixel_width = ceil(width * scale), pixel_height = ceil(height * scale);
    if (!(pixel_width >= 1.0 && pixel_height >= 1.0) ||
        pixel_width > MCP_RENDER_MAX_AXIS || pixel_height > MCP_RENDER_MAX_AXIS) return false;
    shape->width = (uint32_t)pixel_width;
    shape->height = (uint32_t)pixel_height;
    shape->view_x = x;
    shape->view_y = y;
    shape->view_width = width;
    shape->view_height = height;
    return true;
}

bool mcp_broker_render_png(mcp_broker *broker, mcp_broker_document *document,
                           const mcp_json_doc *request, size_t args,
                           mcp_tool_response *out)
{
    if (broker == NULL || document == NULL || request == NULL || out == NULL) return false;
    double scale = 1.0;
    double time_seconds = 0.0;
    bool has_time = false;
    size_t scale_token = mcp_json_object_get(request, args, "scale");
    if (scale_token != MCP_JSON_NO_TOKEN &&
        (!parse_double_token(request, scale_token, &scale) || scale < 0.01 || scale > 16.0))
        return mcp_broker_set_error(out, "invalid_scale", "render scale must be between 0.01 and 16");
    size_t time_token = mcp_json_object_get(request, args, "time");
    has_time = time_token != MCP_JSON_NO_TOKEN;
    if (has_time && (!parse_double_token(request, time_token, &time_seconds) ||
                     time_seconds < 0.0 || time_seconds > 3600.0))
        return mcp_broker_set_error(out, "invalid_time", "render time must be between 0 and 3600 seconds");
    size_t source_length = document_edit_byte_length(document->edit);
    if (source_length == 0U || source_length > 64U * 1024U * 1024U)
        return mcp_broker_set_error(out, "render_source_limit", "document source is empty or exceeds the 64 MiB render limit");
    uint8_t *source = malloc(source_length);
    if (source == NULL || !document_edit_materialize(document->edit, source, source_length, &source_length)) {
        free(source);
        return mcp_broker_set_error(out, "source_unavailable", "document source could not be materialized");
    }
    svg_shape_document shape = {0};
    svg_shape_diagnostic diagnostic = {SVG_SHAPE_ERROR_NONE, 0U};
    svg_shape_media_providers providers = svg_app_media_providers();
    svg_shape_error decode_error = svg_shape_decode_with_media(
        (base_bytes){source, source_length}, &providers, &shape, &diagnostic);
    if (decode_error != SVG_SHAPE_ERROR_NONE) {
        free(source);
        return mcp_broker_set_error(out, "render_decode_failed", svg_shape_error_name(decode_error));
    }
    if (!build_region(request, args, scale, &shape)) {
        svg_shape_document_destroy(&shape);
        free(source);
        return mcp_broker_set_error(out, "invalid_region", "render region is malformed or exceeds the 512 pixel axis limit");
    }
    raster_tile tile = {0};
    bool rendered = has_time ?
        raster_shape_render_at(&shape, time_seconds, &tile, &diagnostic) :
        raster_shape_render(&shape, &tile, &diagnostic);
    if (!rendered) {
        const char *message = svg_shape_error_name(diagnostic.category);
        svg_shape_document_destroy(&shape);
        free(source);
        return mcp_broker_set_error(out, "render_failed", message == NULL ? "rasterization failed" : message);
    }
    svg_shape_document_destroy(&shape);
    free(source);
    mcp_json_buffer png = {0};
    if (!mcp_json_buffer_init(&png, 4096U, MCP_RENDER_PNG_LIMIT) || !encode_png(&tile, &png)) {
        mcp_json_buffer_destroy(&png);
        raster_tile_destroy(&tile);
        return mcp_broker_set_error(out, "png_encode_failed", "PNG output exceeded its bounded size");
    }
    uint32_t width = tile.width, height = tile.height;
    raster_tile_destroy(&tile);
    size_t base64_length = 0U;
    char *base64 = encode_base64((const uint8_t *)png.data, png.length, &base64_length);
    mcp_json_buffer_destroy(&png);
    if (base64 == NULL) return mcp_broker_set_error(out, "png_encode_failed", "PNG could not be encoded as MCP image content");
    free(document->last_render_base64);
    document->last_render_base64 = base64;
    document->last_render_length = base64_length;
    document->last_render_width = width;
    document->last_render_height = height;
    char dimensions[64];
    int dimensions_length = snprintf(dimensions, sizeof(dimensions), "%u,\"height\":%u}", width, height);
    return dimensions_length > 0 && (size_t)dimensions_length < sizeof(dimensions) &&
        mcp_json_buffer_literal(&out->structured_json, "{\"documentId\":" ) &&
        mcp_json_buffer_quoted(&out->structured_json, document->id, strlen(document->id)) &&
        mcp_json_buffer_literal(&out->structured_json, ",\"mimeType\":\"image/png\",\"width\":" ) &&
        mcp_json_buffer_append(&out->structured_json, dimensions, (size_t)dimensions_length) &&
        mcp_json_buffer_literal(&out->content_json, "[{\"type\":\"image\",\"data\":" ) &&
        mcp_json_buffer_quoted(&out->content_json, base64, base64_length) &&
        mcp_json_buffer_literal(&out->content_json, ",\"mimeType\":\"image/png\"}]" );
}

static bool write_all(int descriptor, const uint8_t *bytes, size_t length)
{
    size_t offset = 0U;
    while (offset < length) {
        ssize_t amount = write(descriptor, bytes + offset, length - offset);
        if (amount < 0 && errno == EINTR) continue;
        if (amount <= 0) return false;
        offset += (size_t)amount;
    }
    return true;
}

static platform_atomic_new_file_result publish_granted_gif(
    const char *requested_path, const char *canonical_path,
    const uint8_t *bytes, size_t length, bool *grant_valid)
{
    char rechecked_path[MCP_BROKER_PATH_CAP];
    *grant_valid = mcp_access_resolve_save_path(requested_path, rechecked_path) &&
        strcmp(rechecked_path, canonical_path) == 0;
    if (!*grant_valid) {
        platform_atomic_new_file_result result = {0};
        result.error = PLATFORM_ATOMIC_NEW_FILE_INVALID;
        return result;
    }
    return platform_atomic_file_create_new(
        (base_bytes){(const uint8_t *)canonical_path, strlen(canonical_path)},
        (base_bytes){bytes, length}, NULL);
}

static void clear_response_result(mcp_tool_response *out)
{
    out->structured_json.length = 0U;
    if (out->structured_json.data != NULL) out->structured_json.data[0] = '\0';
}

static bool prepare_animation_response(mcp_tool_response *out,
    const char *document_id, const char *format, const char *path,
    size_t frame_count, uint32_t width, uint32_t height,
    size_t *durable_offset)
{
    char numeric[64];
    int numeric_length = snprintf(numeric, sizeof(numeric),
        "%zu,\"width\":%u,\"height\":%u", frame_count, width, height);
    if (numeric_length <= 0 || (size_t)numeric_length >= sizeof(numeric))
        return false;
    if (!mcp_json_buffer_literal(&out->structured_json, "{\"documentId\":") ||
        !mcp_json_buffer_quoted(&out->structured_json, document_id,
            strlen(document_id)) ||
        !mcp_json_buffer_literal(&out->structured_json, ",\"format\":" ) ||
        !mcp_json_buffer_quoted(&out->structured_json, format, strlen(format)) ||
        !mcp_json_buffer_literal(&out->structured_json, ",\"path\":" ) ||
        !mcp_json_buffer_quoted(&out->structured_json, path, strlen(path)) ||
        !mcp_json_buffer_literal(&out->structured_json, ",\"frames\":" ) ||
        !mcp_json_buffer_append(&out->structured_json, numeric,
            (size_t)numeric_length)) return false;
    if (strcmp(format, "gif") == 0) {
        if (!mcp_json_buffer_literal(&out->structured_json,
                ",\"published\":true,\"durable\":")) return false;
        *durable_offset = out->structured_json.length;
        return mcp_json_buffer_literal(&out->structured_json, "false}");
    }
    return mcp_json_buffer_literal(&out->structured_json, "}");
}

static void set_response_durable(mcp_tool_response *out, size_t offset,
                                 bool durable)
{
    if (durable) memcpy(out->structured_json.data + offset, "true ", 5U);
}

static bool parse_export_fps(const mcp_json_doc *request, size_t args,
                             uint32_t *out_fps)
{
    size_t token = mcp_json_object_get(request, args, "fps");
    uint64_t fps = 0U;
    if (token == MCP_JSON_NO_TOKEN || !mcp_json_uint(request, token, &fps) ||
        fps == 0U || fps > 30U) return false;
    *out_fps = (uint32_t)fps;
    return true;
}

static bool parse_export_animation(const mcp_json_doc *request, size_t args,
                                   char path[MCP_BROKER_PATH_CAP], char format[16],
                                   uint32_t *fps, double *duration,
                                   size_t *frame_count)
{
    size_t path_length = 0U, format_length = 0U;
    if (!mcp_broker_string_arg(request, args, "path", path,
            MCP_BROKER_PATH_CAP, &path_length) || path_length == 0U ||
        !mcp_broker_string_arg(request, args, "format", format, 16U,
            &format_length) ||
        (strcmp(format, "gif") != 0 && strcmp(format, "frames") != 0) ||
        !parse_export_fps(request, args, fps)) return false;
    size_t duration_token = mcp_json_object_get(request, args, "duration");
    if (!parse_double_token(request, duration_token, duration) ||
        *duration <= 0.0 || *duration > 3600.0) return false;
    double requested_frames = ceil((double)*fps * *duration);
    if (!isfinite(requested_frames) || requested_frames < 1.0 ||
        requested_frames > PLATFORM_ANIMATION_MAX_FRAMES) return false;
    *frame_count = (size_t)requested_frames;
    return true;
}

static bool write_png_frame(int directory_fd, size_t frame_index,
                            const raster_tile *tile, char filename[32])
{
    char name[32];
    int name_length = snprintf(name, sizeof(name), "frame-%04zu.png", frame_index);
    if (name_length <= 0 || (size_t)name_length >= sizeof(name)) return false;
    mcp_json_buffer png = {0};
    if (!mcp_json_buffer_init(&png, 4096U, MCP_RENDER_PNG_LIMIT) ||
        !encode_png(tile, &png)) {
        mcp_json_buffer_destroy(&png);
        return false;
    }
    int descriptor = openat(directory_fd, name,
        O_WRONLY | O_CREAT | O_EXCL | O_NOFOLLOW, 0600);
    bool okay = descriptor >= 0 && write_all(descriptor,
        (const uint8_t *)png.data, png.length) && fsync(descriptor) == 0;
    if (descriptor >= 0 && close(descriptor) != 0) okay = false;
    if (!okay && descriptor >= 0) (void)unlinkat(directory_fd, name, 0);
    mcp_json_buffer_destroy(&png);
    if (okay) memcpy(filename, name, strlen(name) + 1U);
    return okay;
}

static bool has_vexel_extension(const char *path)
{
    if (path == NULL) return false;
    size_t length = strlen(path);
    if (length < 6U || path[length - 6U] != '.') return false;
    static const char extension[] = "vexel";
    for (size_t index = 0U; index < sizeof(extension) - 1U; index++) {
        char value = path[length - 5U + index];
        if (value >= 'A' && value <= 'Z') value = (char)(value - 'A' + 'a');
        if (value != extension[index]) return false;
    }
    return true;
}

static bool native_export(mcp_broker *broker, mcp_broker_document *document,
                          const char *requested_path, const char *format,
                          uint32_t fps, double duration, size_t frame_count,
                          mcp_tool_response *out)
{
    if (broker == NULL || document == NULL || document != broker->app_document ||
        !document->app_owned || !has_vexel_extension(document->path)) return false;
    if (broker->host.current_project_bundle == NULL ||
        broker->host.current_timeline_session == NULL) {
        (void)mcp_broker_set_error(out, "native_timeline_unavailable",
            "the active .vexel document has no native animation export route");
        return true;
    }
    project_timeline_session *session =
        broker->host.current_timeline_session(broker->host.context);
    const project_bundle_view *bundle =
        broker->host.current_project_bundle(broker->host.context);
    if (session == NULL || bundle == NULL ||
        bundle->version != PROJECT_BUNDLE_VERSION) {
        (void)mcp_broker_set_error(out, "native_timeline_unavailable",
            "the active .vexel document has no editable native-v1 animation");
        return true;
    }
    size_t source_length = document_edit_byte_length(document->edit);
    if (source_length == 0U || source_length > 16U * 1024U * 1024U) {
        (void)mcp_broker_set_error(out, "animation_source_limit",
            "native animation export is limited to 16 MiB scene sources");
        return true;
    }
    uint8_t *source = malloc(source_length);
    if (source == NULL || !document_edit_materialize(document->edit, source,
            source_length, &source_length)) {
        free(source);
        (void)mcp_broker_set_error(out, "source_unavailable",
            "the active project scene could not be materialized");
        return true;
    }
    svg_shape_document scene = {0};
    svg_shape_diagnostic diagnostic = {SVG_SHAPE_ERROR_NONE, 0U};
    svg_shape_media_providers providers = svg_app_media_providers();
    svg_shape_error decoded = svg_shape_decode_with_media(
        (base_bytes){source, source_length}, &providers, &scene, &diagnostic);
    if (decoded != SVG_SHAPE_ERROR_NONE) {
        free(source);
        (void)mcp_broker_set_error(out, "animation_decode_failed",
            svg_shape_error_name(decoded));
        return true;
    }
    project_timeline_view timeline = {0};
    project_timeline_result decoded_timeline =
        project_timeline_session_decode(session, &timeline);
    if (decoded_timeline.error != PROJECT_TIMELINE_OK || timeline.empty ||
        timeline.ticks_per_second == 0U || timeline.duration_ticks == 0U) {
        svg_shape_document_destroy(&scene);
        free(source);
        (void)mcp_broker_set_error(out, "native_timeline_invalid",
            "the active project timeline cannot be exported");
        return true;
    }
    double native_duration = (double)timeline.duration_ticks /
        (double)timeline.ticks_per_second;
    if (duration > native_duration) {
        svg_shape_document_destroy(&scene);
        free(source);
        (void)mcp_broker_set_error(out, "native_duration_exceeded",
            "requested export duration exceeds the native timeline duration");
        return true;
    }
    if (scene.width == 0U || scene.height == 0U) {
        svg_shape_document_destroy(&scene);
        free(source);
        (void)mcp_broker_set_error(out, "animation_dimensions",
            "native animation frame dimensions are invalid");
        return true;
    }
    double scale = fmin(1.0, fmin((double)MCP_RENDER_MAX_AXIS / scene.width,
                                  (double)MCP_RENDER_MAX_AXIS / scene.height));
    uint32_t width = (uint32_t)ceil((double)scene.width * scale);
    uint32_t height = (uint32_t)ceil((double)scene.height * scale);
    if (width == 0U || height == 0U || width > MCP_RENDER_MAX_AXIS ||
        height > MCP_RENDER_MAX_AXIS || frame_count == 0U ||
        frame_count > PLATFORM_ANIMATION_MAX_FRAMES ||
        (uint64_t)width * height > MCP_ANIMATION_TOTAL_FRAME_PIXELS / frame_count) {
        svg_shape_document_destroy(&scene);
        free(source);
        (void)mcp_broker_set_error(out, "animation_work_limit",
            "native export exceeds its frame, dimension, or pixel-work limit");
        return true;
    }
    scene.width = width;
    scene.height = height;

    char canonical_path[MCP_BROKER_PATH_CAP];
    int directory_fd = -1;
    if (strcmp(format, "frames") == 0) {
        if (!mcp_access_resolve_granted_directory(requested_path, canonical_path)) {
            svg_shape_document_destroy(&scene);
            free(source);
            (void)mcp_broker_set_error(out, "path_not_granted",
                "frame output directory is not granted");
            return true;
        }
        directory_fd = open(canonical_path, O_RDONLY | O_DIRECTORY | O_NOFOLLOW);
        if (directory_fd < 0) {
            svg_shape_document_destroy(&scene);
            free(source);
            (void)mcp_broker_set_error(out, "export_open_failed",
                "frame output directory could not be opened");
            return true;
        }
    } else if (!mcp_access_resolve_save_path(requested_path, canonical_path)) {
        svg_shape_document_destroy(&scene);
        free(source);
        (void)mcp_broker_set_error(out, "path_not_granted",
            "GIF output path is not inside a granted directory");
        return true;
    }

    platform_animation_frame gif_frames[PLATFORM_ANIMATION_MAX_FRAMES] = {0};
    char written_names[PLATFORM_ANIMATION_MAX_FRAMES][32] = {{0}};
    bool okay = true;
    for (size_t index = 0U; okay && index < frame_count; index++) {
        uint64_t numerator = (uint64_t)index * timeline.ticks_per_second;
        uint64_t sample_tick = numerator / fps;
        if (sample_tick >= timeline.duration_ticks || sample_tick > UINT32_MAX) {
            okay = false;
            break;
        }
        raster_tile tile = {0};
        project_animation_render_result rendered =
            project_timeline_session_render_frame(session, &scene, bundle,
                (uint32_t)sample_tick, &tile);
        okay = rendered.error == PROJECT_ANIMATION_RENDER_OK;
        if (okay && strcmp(format, "gif") == 0) {
            gif_frames[index] = (platform_animation_frame){
                .rgba = tile.rgba, .byte_count = tile.byte_count,
                .width = tile.width, .height = tile.height};
            tile.rgba = NULL;
            tile.byte_count = 0U;
        } else if (okay) {
            okay = write_png_frame(directory_fd, index, &tile,
                                   written_names[index]);
        }
        raster_tile_destroy(&tile);
    }
    svg_shape_document_destroy(&scene);
    free(source);
    if (directory_fd >= 0 && close(directory_fd) != 0) okay = false;

    char result_path[MCP_BROKER_PATH_CAP] = {0};
    platform_atomic_new_file_result publication = {0};
    bool grant_valid = true;
    bool response_encoding_failed = false;
    size_t durable_offset = 0U;
    if (okay && strcmp(format, "gif") == 0) {
        platform_animation_bytes gif = {0};
        okay = platform_animation_encode_gif(gif_frames, frame_count, fps,
            &gif) == PLATFORM_MEDIA_OK;
        if (okay) {
            if (!prepare_animation_response(out, document->id, format,
                    canonical_path, frame_count, width, height,
                    &durable_offset)) {
                response_encoding_failed = true;
                okay = false;
            } else {
                publication = publish_granted_gif(requested_path,
                    canonical_path, gif.bytes, gif.length, &grant_valid);
                okay = publication.published;
            }
        }
        platform_animation_bytes_destroy(&gif);
    } else if (okay) {
        memcpy(result_path, canonical_path, strlen(canonical_path) + 1U);
    }
    for (size_t index = 0U; index < frame_count; index++)
        free((void *)gif_frames[index].rgba);
    if (response_encoding_failed) {
        clear_response_result(out);
        (void)mcp_broker_set_error(out, "response_failed",
            "native animation response could not be encoded before publication");
        return true;
    }
    if (!okay) {
        clear_response_result(out);
        if (!grant_valid)
            return mcp_broker_set_error(out, "path_not_granted",
                "GIF output path is no longer inside a granted directory");
        if (strcmp(format, "frames") == 0) {
            int cleanup_fd = open(canonical_path,
                O_RDONLY | O_DIRECTORY | O_NOFOLLOW);
            if (cleanup_fd >= 0) {
                for (size_t index = 0U; index < frame_count; index++)
                    if (written_names[index][0] != '\0')
                        (void)unlinkat(cleanup_fd, written_names[index], 0);
                (void)close(cleanup_fd);
            }
        }
        (void)mcp_broker_set_error(out, "animation_export_failed",
            "native frame rendering or export failed within configured limits");
        return true;
    }
    if (strcmp(format, "gif") == 0) {
        set_response_durable(out, durable_offset,
            publication.error == PLATFORM_ATOMIC_NEW_FILE_OK);
        return true;
    }
    return prepare_animation_response(out, document->id, format, result_path,
        frame_count, width, height, NULL);
}

bool mcp_broker_export_animation(mcp_broker *broker,
                                 mcp_broker_document *document,
                                 const mcp_json_doc *request, size_t args,
                                 mcp_tool_response *out)
{
    if (document == NULL || request == NULL || out == NULL)
        return mcp_broker_set_error(out, "invalid_arguments", "animation export request is invalid");
    char path[MCP_BROKER_PATH_CAP], format[16];
    uint32_t fps = 0U;
    double duration = 0.0;
    size_t frame_count = 0U;
    if (!parse_export_animation(request, args, path, format, &fps, &duration,
                                &frame_count))
        return mcp_broker_set_error(out, "invalid_animation_export", "format, fps, duration, or frame count is outside its limit");
    if (native_export(broker, document, path, format, fps, duration,
            frame_count, out)) return true;

    char canonical_path[MCP_BROKER_PATH_CAP];
    int output_directory_fd = -1;
    if (strcmp(format, "frames") == 0) {
        if (!mcp_access_resolve_granted_directory(path, canonical_path))
            return mcp_broker_set_error(out, "path_not_granted", "frame output directory is not granted");
        output_directory_fd = open(canonical_path, O_RDONLY | O_DIRECTORY | O_NOFOLLOW);
        if (output_directory_fd < 0)
            return mcp_broker_set_error(out, "export_open_failed", "frame output directory could not be opened");
    } else if (!mcp_access_resolve_save_path(path, canonical_path)) {
        return mcp_broker_set_error(out, "path_not_granted", "GIF output path is not inside a granted directory");
    }

    size_t source_length = document_edit_byte_length(document->edit);
    if (source_length == 0U || source_length > 16U * 1024U * 1024U) {
        if (output_directory_fd >= 0) (void)close(output_directory_fd);
        return mcp_broker_set_error(out, "animation_source_limit", "document exceeds the 16 MiB animation export source limit");
    }
    uint8_t *source = malloc(source_length);
    if (source == NULL || !document_edit_materialize(document->edit, source,
            source_length, &source_length)) {
        free(source);
        if (output_directory_fd >= 0) (void)close(output_directory_fd);
        return mcp_broker_set_error(out, "source_unavailable", "document source could not be materialized");
    }
    svg_shape_document shape = {0};
    svg_shape_diagnostic diagnostic = {SVG_SHAPE_ERROR_NONE, 0U};
    svg_shape_media_providers providers = svg_app_media_providers();
    svg_shape_error decode_error = svg_shape_decode_with_media(
        (base_bytes){source, source_length}, &providers, &shape, &diagnostic);
    free(source);
    if (decode_error != SVG_SHAPE_ERROR_NONE) {
        if (output_directory_fd >= 0) (void)close(output_directory_fd);
        return mcp_broker_set_error(out, "animation_decode_failed", svg_shape_error_name(decode_error));
    }
    double scale = fmin(1.0, fmin((double)MCP_RENDER_MAX_AXIS / shape.width,
                                  (double)MCP_RENDER_MAX_AXIS / shape.height));
    uint32_t width = (uint32_t)ceil((double)shape.width * scale);
    uint32_t height = (uint32_t)ceil((double)shape.height * scale);
    if (width == 0U || height == 0U || width > MCP_RENDER_MAX_AXIS ||
        height > MCP_RENDER_MAX_AXIS) {
        svg_shape_document_destroy(&shape);
        if (output_directory_fd >= 0) (void)close(output_directory_fd);
        return mcp_broker_set_error(out, "animation_dimensions", "animation frame dimensions are invalid");
    }
    uint64_t frame_pixels = (uint64_t)width * height;
    if (frame_pixels == 0U || frame_pixels >
        MCP_ANIMATION_TOTAL_FRAME_PIXELS / frame_count) {
        svg_shape_document_destroy(&shape);
        if (output_directory_fd >= 0) (void)close(output_directory_fd);
        return mcp_broker_set_error(out, "animation_work_limit", "total animation export pixel work exceeds its 16 million pixel limit");
    }
    svg_shape_document frame_shape = shape;
    frame_shape.width = width;
    frame_shape.height = height;
    platform_animation_frame gif_frames[PLATFORM_ANIMATION_MAX_FRAMES] = {0};
    char written_names[PLATFORM_ANIMATION_MAX_FRAMES][32] = {{0}};
    bool okay = true;
    for (size_t index = 0U; okay && index < frame_count; index++) {
        raster_tile tile = {0};
        double time_seconds = (double)index / (double)fps;
        okay = raster_shape_render_at(&frame_shape, time_seconds, &tile, &diagnostic);
        if (okay && strcmp(format, "gif") == 0) {
            gif_frames[index] = (platform_animation_frame){
                .rgba = tile.rgba, .byte_count = tile.byte_count,
                .width = tile.width, .height = tile.height};
            tile.rgba = NULL;
            tile.byte_count = 0U;
        } else if (okay) {
            okay = write_png_frame(output_directory_fd, index, &tile,
                                   written_names[index]);
        }
        raster_tile_destroy(&tile);
    }
    svg_shape_document_destroy(&shape);
    if (output_directory_fd >= 0 && close(output_directory_fd) != 0) okay = false;

    char result_path[MCP_BROKER_PATH_CAP] = {0};
    platform_atomic_new_file_result publication = {0};
    bool grant_valid = true;
    bool response_encoding_failed = false;
    size_t durable_offset = 0U;
    if (okay && strcmp(format, "gif") == 0) {
        platform_animation_bytes gif = {0};
        platform_media_status status = platform_animation_encode_gif(
            gif_frames, frame_count, fps, &gif);
        if (status != PLATFORM_MEDIA_OK) okay = false;
        if (okay) {
            if (!prepare_animation_response(out, document->id, format,
                    canonical_path, frame_count, width, height,
                    &durable_offset)) {
                response_encoding_failed = true;
                okay = false;
            } else {
                publication = publish_granted_gif(path, canonical_path,
                    gif.bytes, gif.length, &grant_valid);
                okay = publication.published;
            }
        }
        platform_animation_bytes_destroy(&gif);
    } else if (okay) {
        memcpy(result_path, canonical_path, strlen(canonical_path) + 1U);
    }
    for (size_t index = 0U; index < frame_count; index++) free((void *)gif_frames[index].rgba);
    if (response_encoding_failed) {
        clear_response_result(out);
        return mcp_broker_set_error(out, "response_failed",
            "animation response could not be encoded before publication");
    }
    if (!okay) {
        clear_response_result(out);
        if (!grant_valid)
            return mcp_broker_set_error(out, "path_not_granted",
                "GIF output path is no longer inside a granted directory");
        if (strcmp(format, "frames") == 0) {
            int cleanup_fd = open(canonical_path, O_RDONLY | O_DIRECTORY | O_NOFOLLOW);
            if (cleanup_fd >= 0) {
                for (size_t index = 0U; index < frame_count; index++)
                    if (written_names[index][0] != '\0')
                        (void)unlinkat(cleanup_fd, written_names[index], 0);
                (void)close(cleanup_fd);
            }
        }
        return mcp_broker_set_error(out, "animation_export_failed", "frame rendering or export failed within the configured limits");
    }
    if (strcmp(format, "gif") == 0) {
        set_response_durable(out, durable_offset,
            publication.error == PLATFORM_ATOMIC_NEW_FILE_OK);
        return true;
    }
    return prepare_animation_response(out, document->id, format, result_path,
        frame_count, width, height, NULL);
}
