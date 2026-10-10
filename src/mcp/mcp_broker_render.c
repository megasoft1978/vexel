#define _DARWIN_C_SOURCE
#define _POSIX_C_SOURCE 200809L

#include "mcp_broker_internal.h"
#include "svg/shape/media.h"

#include "raster/shape_render.h"
#include "raster/tile.h"
#include "shape.h"

#include <errno.h>
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

svg_shape_media_providers svg_app_media_providers(void);

#define MCP_RENDER_MAX_AXIS 512U
#define MCP_RENDER_PNG_LIMIT (2U * 1024U * 1024U)

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
    size_t scale_token = mcp_json_object_get(request, args, "scale");
    if (scale_token != MCP_JSON_NO_TOKEN &&
        (!parse_double_token(request, scale_token, &scale) || scale < 0.01 || scale > 16.0))
        return mcp_broker_set_error(out, "invalid_scale", "render scale must be between 0.01 and 16");
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
    if (!raster_shape_render(&shape, &tile, &diagnostic)) {
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
