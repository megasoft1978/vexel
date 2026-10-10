#include "shape_internal.h"

#include "../reference/reference.h"

#include <math.h>
#include <string.h>

#define SVG_IMAGE_MAX_ENCODED_BYTES (32U * 1024U * 1024U)
#define SVG_IMAGE_MAX_URI_BYTES 44739266U

typedef struct image_geometry_context {
    base_bytes source;
    svg_xml_token *tokens;
    size_t token_count;
    size_t element;
} image_geometry_context;

static bool span_is(base_bytes source, svg_xml_span span, const char *literal)
{
    size_t length = 0U;
    while (literal[length] != '\0') length++;
    return span.offset <= source.len && span.length == length &&
           length <= source.len - span.offset &&
           memcmp(source.ptr + span.offset, literal, length) == 0;
}

static bool attribute(base_bytes source, const svg_xml_token *tokens,
                      size_t count, size_t element, const char *local,
                      const char *prefix, svg_xml_span *out)
{
    for (size_t i = element + 1U;
         i < count && tokens[i].kind == SVG_XML_TOKEN_ATTRIBUTE; ++i) {
        if (!span_is(source, tokens[i].local_name, local)) continue;
        if (prefix == NULL ? tokens[i].prefix.length == 0U :
            span_is(source, tokens[i].prefix, prefix)) {
            *out = tokens[i].value;
            return true;
        }
    }
    return false;
}

static int base64_value(uint8_t byte)
{
    if (byte >= 'A' && byte <= 'Z') return (int)(byte - 'A');
    if (byte >= 'a' && byte <= 'z') return (int)(byte - 'a') + 26;
    if (byte >= '0' && byte <= '9') return (int)(byte - '0') + 52;
    if (byte == '+') return 62;
    if (byte == '/') return 63;
    return -1;
}

static bool decode_base64(svg_shape_builder *builder, base_bytes source,
                          svg_xml_span payload, uint8_t **out,
                          size_t *out_length)
{
    *out = NULL;
    *out_length = 0U;
    if (payload.length == 0U || payload.length > SVG_IMAGE_MAX_URI_BYTES ||
        payload.length % 4U != 0U || payload.offset > source.len ||
        payload.length > source.len - payload.offset) {
        svg_shape_fail(builder->diag, SVG_SHAPE_ERROR_LIMIT, payload.offset);
        return false;
    }
    size_t padding = 0U;
    if (source.ptr[payload.offset + payload.length - 1U] == '=') padding++;
    if (payload.length > 1U &&
        source.ptr[payload.offset + payload.length - 2U] == '=') padding++;
    size_t groups = payload.length / 4U;
    if (groups > SIZE_MAX / 3U) {
        svg_shape_fail(builder->diag, SVG_SHAPE_ERROR_LIMIT, payload.offset);
        return false;
    }
    size_t decoded_length = groups * 3U - padding;
    if (decoded_length == 0U || decoded_length > SVG_IMAGE_MAX_ENCODED_BYTES) {
        svg_shape_fail(builder->diag, SVG_SHAPE_ERROR_LIMIT, payload.offset);
        return false;
    }
    uint8_t *decoded = svg_shape_malloc(decoded_length);
    if (decoded == NULL) {
        svg_shape_fail(builder->diag, SVG_SHAPE_ERROR_OOM, payload.offset);
        return false;
    }
    size_t output = 0U;
    for (size_t group = 0U; group < groups; ++group) {
        size_t at = payload.offset + group * 4U;
        int a = base64_value(source.ptr[at]);
        int b = base64_value(source.ptr[at + 1U]);
        bool final = group + 1U == groups;
        int c = source.ptr[at + 2U] == '=' ? 0 :
                base64_value(source.ptr[at + 2U]);
        int d = source.ptr[at + 3U] == '=' ? 0 :
                base64_value(source.ptr[at + 3U]);
        bool legal_padding = final &&
            ((source.ptr[at + 2U] == '=' && source.ptr[at + 3U] == '=') ||
             (source.ptr[at + 2U] != '=' && source.ptr[at + 3U] == '='));
        if (a < 0 || b < 0 || c < 0 || d < 0 ||
            ((source.ptr[at + 2U] == '=' || source.ptr[at + 3U] == '=') &&
             !legal_padding) ||
            (source.ptr[at + 2U] == '=' && (b & 15) != 0) ||
            (source.ptr[at + 3U] == '=' && source.ptr[at + 2U] != '=' &&
             (c & 3) != 0)) {
            svg_shape_free(decoded);
            svg_shape_fail(builder->diag, SVG_SHAPE_ERROR_UNSUPPORTED,
                           at);
            return false;
        }
        uint32_t bits = ((uint32_t)a << 18U) | ((uint32_t)b << 12U) |
                        ((uint32_t)c << 6U) | (uint32_t)d;
        decoded[output++] = (uint8_t)(bits >> 16U);
        if (source.ptr[at + 2U] != '=')
            decoded[output++] = (uint8_t)(bits >> 8U);
        if (source.ptr[at + 3U] != '=')
            decoded[output++] = (uint8_t)bits;
    }
    if (output != decoded_length) {
        svg_shape_free(decoded);
        svg_shape_fail(builder->diag, SVG_SHAPE_ERROR_UNSUPPORTED,
                       payload.offset);
        return false;
    }
    *out = decoded;
    *out_length = decoded_length;
    return true;
}

static bool data_payload(base_bytes source, svg_xml_span uri,
                         svg_xml_span *payload)
{
    if (uri.offset > source.len || uri.length > source.len - uri.offset)
        return false;
    size_t comma = uri.offset;
    size_t end = uri.offset + uri.length;
    while (comma < end && source.ptr[comma] != ',') comma++;
    if (comma == end || comma + 1U == end) return false;
    size_t header_length = comma - uri.offset;
    static const char png[] = "data:image/png;base64";
    static const char jpeg[] = "data:image/jpeg;base64";
    if (!((header_length == sizeof(png) - 1U &&
           memcmp(source.ptr + uri.offset, png, sizeof(png) - 1U) == 0) ||
          (header_length == sizeof(jpeg) - 1U &&
           memcmp(source.ptr + uri.offset, jpeg, sizeof(jpeg) - 1U) == 0)))
        return false;
    *payload = (svg_xml_span){comma + 1U, end - comma - 1U};
    return true;
}

static bool image_geometry(svg_shape_builder *builder, void *opaque)
{
    image_geometry_context *context = opaque;
    base_bytes source = context->source;
    svg_xml_token *tokens = context->tokens;
    size_t element = context->element;
    const char *names[] = {"x", "y", "width", "height"};
    double values[4] = {0.0, 0.0, 0.0, 0.0};
    bool has_width = false, has_height = false;
    for (size_t i = 0U; i < 4U; ++i) {
        svg_xml_span span;
        if (!attribute(source, tokens, context->token_count, element,
                       names[i], NULL, &span)) continue;
        if (i == 2U) has_width = true;
        if (i == 3U) has_height = true;
        double reference = i == 0U || i == 2U ?
            builder->viewport_width : builder->viewport_height;
        if (!svg_shape_read_length(source, span, reference, &values[i])) {
            svg_shape_fail(builder->diag, SVG_SHAPE_ERROR_NUMBER, span.offset);
            return false;
        }
    }
    if (!has_width || !has_height || values[2] < 0.0 || values[3] < 0.0) {
        svg_shape_fail(builder->diag, SVG_SHAPE_ERROR_NUMBER,
                       tokens[element].name.offset);
        return false;
    }
    svg_shape_image image = {
        .x = values[0], .y = values[1],
        .width = values[2], .height = values[3],
        .draw_x = values[0], .draw_y = values[1],
        .draw_width = values[2], .draw_height = values[3]
    };
    geom_point points[4] = {
        {values[0], values[1]},
        {values[0] + values[2], values[1]},
        {values[0] + values[2], values[1] + values[3]},
        {values[0], values[1] + values[3]}
    };
    for (size_t i = 0U; i < 4U; ++i) {
        if (!svg_shape_point_bounded(points[i])) {
            svg_shape_fail(builder->diag, SVG_SHAPE_ERROR_NUMBER,
                           tokens[element].name.offset);
            return false;
        }
        if (!svg_shape_builder_add_edge(builder, points[i],
                                         points[(i + 1U) % 4U])) return false;
    }

    if (values[2] == 0.0 || values[3] == 0.0) {
        builder->doc->images[builder->shape_index] = image;
        builder->doc->paints[builder->shape_index].kind =
            SVG_SHAPE_PAINT_IMAGE;
        return true;
    }
    svg_xml_span href = {0};
    bool plain = attribute(source, tokens, context->token_count, element,
                           "href", NULL, &href);
    bool xlink = attribute(source, tokens, context->token_count, element,
                           "href", "xlink", &href);
    if (plain == xlink || (xlink && !svg_reference_xlink_is_bound(
            source, tokens, context->token_count)) ||
        !data_payload(source, href, &href)) {
        svg_shape_fail(builder->diag, SVG_SHAPE_ERROR_UNSUPPORTED,
                       plain || xlink ? href.offset : tokens[element].name.offset);
        return false;
    }
    if (builder->media == NULL || builder->media->decode_image == NULL ||
        builder->media->release_image == NULL) {
        svg_shape_fail(builder->diag, SVG_SHAPE_ERROR_UNSUPPORTED,
                       href.offset);
        return false;
    }
    uint8_t *encoded = NULL;
    size_t encoded_length = 0U;
    if (!decode_base64(builder, source, href, &encoded, &encoded_length))
        return false;
    svg_shape_decoded_image decoded = {0};
    svg_shape_media_status media_status = builder->media->decode_image(
        builder->media->context, (base_bytes){encoded, encoded_length},
        &decoded);
    svg_shape_free(encoded);
    if (media_status != SVG_SHAPE_MEDIA_OK) {
        svg_shape_error error = media_status == SVG_SHAPE_MEDIA_LIMIT ?
            SVG_SHAPE_ERROR_LIMIT : media_status == SVG_SHAPE_MEDIA_OOM ?
            SVG_SHAPE_ERROR_OOM : SVG_SHAPE_ERROR_UNSUPPORTED;
        svg_shape_fail(builder->diag, error, href.offset);
        return false;
    }
    size_t minimum_row;
    size_t required_bytes;
    if (decoded.rgba == NULL || decoded.width == 0U || decoded.height == 0U ||
        decoded.width > 16384U || decoded.height > 16384U ||
        (size_t)decoded.width > SIZE_MAX / 4U ||
        (minimum_row = (size_t)decoded.width * 4U,
         decoded.row_stride < minimum_row) ||
        (size_t)decoded.height > SIZE_MAX / decoded.row_stride ||
        (required_bytes = (size_t)decoded.height * decoded.row_stride,
         required_bytes > decoded.byte_count) ||
        (size_t)decoded.width > SIZE_MAX / (size_t)decoded.height) {
        builder->media->release_image(builder->media->context, &decoded);
        svg_shape_fail(builder->diag, SVG_SHAPE_ERROR_UNSUPPORTED,
                       href.offset);
        return false;
    }
    size_t pixels = (size_t)decoded.width * (size_t)decoded.height;
    if (pixels > SVG_SHAPE_MEDIA_MAX_IMAGE_PIXELS -
                 builder->decoded_image_pixels) {
        builder->media->release_image(builder->media->context, &decoded);
        svg_shape_fail(builder->diag, SVG_SHAPE_ERROR_LIMIT, href.offset);
        return false;
    }
    builder->decoded_image_pixels += pixels;
    image.rgba = decoded.rgba;
    image.byte_count = decoded.byte_count;
    image.row_stride = decoded.row_stride;
    image.pixel_width = decoded.width;
    image.pixel_height = decoded.height;
    image.owner = decoded.owner;

    svg_shape_aspect aspect = {.align_x = 0.5, .align_y = 0.5, .set = true};
    svg_xml_span aspect_span;
    if (attribute(source, tokens, context->token_count, element,
                  "preserveAspectRatio", NULL, &aspect_span) &&
        !svg_shape_parse_preserve_aspect(source, aspect_span, &aspect)) {
        decoded.owner = image.owner;
        builder->media->release_image(builder->media->context, &decoded);
        svg_shape_fail(builder->diag, SVG_SHAPE_ERROR_UNSUPPORTED,
                       aspect_span.offset);
        return false;
    }
    if (!aspect.none) {
        double sx = image.width / (double)image.pixel_width;
        double sy = image.height / (double)image.pixel_height;
        double scale = aspect.slice ? fmax(sx, sy) : fmin(sx, sy);
        image.draw_width = (double)image.pixel_width * scale;
        image.draw_height = (double)image.pixel_height * scale;
        image.draw_x = image.x +
            (image.width - image.draw_width) * aspect.align_x;
        image.draw_y = image.y +
            (image.height - image.draw_height) * aspect.align_y;
    }
    builder->doc->images[builder->shape_index] = image;
    builder->doc->paints[builder->shape_index].kind = SVG_SHAPE_PAINT_IMAGE;
    return true;
}

bool svg_shape_builder_add_image_element(
    svg_shape_builder *builder, base_bytes source, svg_xml_token *tokens,
    size_t token_count, size_t element, const svg_css_paint *style,
    geom_affine transform)
{
    image_geometry_context context = {source, tokens, token_count, element};
    if (!svg_shape_builder_add_custom_element(
            builder, source, tokens, token_count, element, style, transform,
            image_geometry, &context)) return false;
    size_t shape = builder->shape_index;
    size_t first = builder->doc->shape_edge_start[shape];
    size_t count = builder->doc->shape_edge_count[shape];
    for (size_t i = first; i < first + count; ++i)
        builder->doc->colors[i * 4U + 3U] = 0U;
    return true;
}
