#include "project_drawing_asset.h"

#include "xml_tokenizer.h"

#include <stdlib.h>
#include <string.h>

#define DRAWING_ASSET_MAX_IDS 65536U
#define DRAWING_ASSET_MAX_ID_BYTES 256U
#define DRAWING_ASSET_MAX_REFERENCE_BYTES 4096U

typedef struct drawing_id {
    base_bytes value;
    size_t offset;
} drawing_id;

typedef struct asset_scan {
    base_bytes source;
    drawing_id *ids;
    size_t id_capacity;
    size_t id_count;
    size_t pool_capacity;
    size_t pool_used;
    uint8_t *pool;
    unsigned pass;
    base_bytes element_name;
    base_bytes element_prefix;
    svg_shape_error error;
    size_t error_offset;
} asset_scan;

static bool span_is(base_bytes source, svg_xml_span span, const char *text)
{
    size_t length = strlen(text);
    return span.offset <= source.len && span.length == length &&
        length <= source.len - span.offset &&
        memcmp(source.ptr + span.offset, text, length) == 0;
}

static bool token_name_is(base_bytes source, base_bytes name, const char *text)
{
    return name.len == strlen(text) && name.len <= source.len &&
        name.ptr != NULL && memcmp(name.ptr, text, name.len) == 0;
}

static bool token_span(base_bytes source, svg_xml_span span, base_bytes *out)
{
    if (out == NULL || span.offset > source.len ||
        span.length > source.len - span.offset) return false;
    *out = (base_bytes){source.ptr + span.offset, span.length};
    return true;
}

static int bytes_order(base_bytes a, base_bytes b)
{
    size_t common = a.len < b.len ? a.len : b.len;
    int order = common == 0U ? 0 : memcmp(a.ptr, b.ptr, common);
    if (order != 0) return order;
    return a.len < b.len ? -1 : a.len > b.len ? 1 : 0;
}

static int id_compare(const void *left, const void *right)
{
    const drawing_id *a = left;
    const drawing_id *b = right;
    int order = bytes_order(a->value, b->value);
    if (order != 0) return order;
    return a->offset < b->offset ? -1 : a->offset > b->offset ? 1 : 0;
}

static bool id_exists(const asset_scan *scan, base_bytes id)
{
    size_t low = 0U, high = scan->id_count;
    while (low < high) {
        size_t middle = low + (high - low) / 2U;
        int order = bytes_order(scan->ids[middle].value, id);
        if (order == 0) return true;
        if (order < 0) low = middle + 1U;
        else high = middle;
    }
    return false;
}

static bool fail(asset_scan *scan, svg_shape_error error, size_t offset)
{
    scan->error = error;
    scan->error_offset = offset;
    return false;
}

static bool scan_css_urls(asset_scan *scan, base_bytes value, size_t offset)
{
    uint8_t string_quote = 0U;
    for (size_t i = 0U; i < value.len; ++i) {
        if (string_quote != 0U) {
            if (value.ptr[i] == '\\' && i + 1U < value.len) i++;
            else if (value.ptr[i] == string_quote) string_quote = 0U;
            continue;
        }
        if (value.ptr[i] == '\'' || value.ptr[i] == '"') {
            string_quote = value.ptr[i];
            continue;
        }
        if (i + 1U < value.len && value.ptr[i] == '/' &&
            value.ptr[i + 1U] == '*') {
            i += 2U;
            while (i + 1U < value.len &&
                   !(value.ptr[i] == '*' && value.ptr[i + 1U] == '/')) i++;
            if (i + 1U >= value.len)
                return fail(scan, SVG_SHAPE_ERROR_UNSUPPORTED, offset + i);
            i++;
            continue;
        }
        if (value.ptr[i] == '\\') {
            return fail(scan, SVG_SHAPE_ERROR_UNSUPPORTED, offset + i);
        }
        if (i + 7U <= value.len &&
            (value.ptr[i] == '@' &&
             (value.ptr[i + 1U] == 'i' || value.ptr[i + 1U] == 'I') &&
             (value.ptr[i + 2U] == 'm' || value.ptr[i + 2U] == 'M') &&
             (value.ptr[i + 3U] == 'p' || value.ptr[i + 3U] == 'P') &&
             (value.ptr[i + 4U] == 'o' || value.ptr[i + 4U] == 'O') &&
             (value.ptr[i + 5U] == 'r' || value.ptr[i + 5U] == 'R') &&
             (value.ptr[i + 6U] == 't' || value.ptr[i + 6U] == 'T')))
            return fail(scan, SVG_SHAPE_ERROR_UNSUPPORTED, offset + i);
        if (i + 4U > value.len ||
            (value.ptr[i] != 'u' && value.ptr[i] != 'U') ||
            (value.ptr[i + 1U] != 'r' && value.ptr[i + 1U] != 'R') ||
            (value.ptr[i + 2U] != 'l' && value.ptr[i + 2U] != 'L')) continue;
        size_t cursor = i + 3U;
        while (cursor < value.len && (value.ptr[cursor] == ' ' ||
            value.ptr[cursor] == '\t' || value.ptr[cursor] == '\r' ||
            value.ptr[cursor] == '\n')) cursor++;
        if (cursor >= value.len || value.ptr[cursor++] != '(') continue;
        while (cursor < value.len && (value.ptr[cursor] == ' ' ||
            value.ptr[cursor] == '\t' || value.ptr[cursor] == '\r' ||
            value.ptr[cursor] == '\n')) cursor++;
        uint8_t quote = 0U;
        if (cursor < value.len && (value.ptr[cursor] == '\'' ||
                                   value.ptr[cursor] == '"'))
            quote = value.ptr[cursor++];
        while (cursor < value.len && (value.ptr[cursor] == ' ' ||
            value.ptr[cursor] == '\t' || value.ptr[cursor] == '\r' ||
            value.ptr[cursor] == '\n')) cursor++;
        size_t start = cursor;
        while (cursor < value.len && value.ptr[cursor] != ')' &&
            (quote == 0U || value.ptr[cursor] != quote)) cursor++;
        size_t close_paren = cursor + (quote == 0U ? 0U : 1U);
        while (close_paren < value.len && (value.ptr[close_paren] == ' ' ||
            value.ptr[close_paren] == '\t' || value.ptr[close_paren] == '\r' ||
            value.ptr[close_paren] == '\n')) close_paren++;
        if (cursor >= value.len || close_paren >= value.len ||
            value.ptr[close_paren] != ')')
            return fail(scan, SVG_SHAPE_ERROR_UNSUPPORTED, offset + start);
        size_t end = cursor;
        for (size_t escaped = start; escaped < end; ++escaped)
            if (value.ptr[escaped] == '\\')
                return fail(scan, SVG_SHAPE_ERROR_UNSUPPORTED,
                            offset + escaped);
        while (end > start && (value.ptr[end - 1U] == ' ' ||
            value.ptr[end - 1U] == '\t' || value.ptr[end - 1U] == '\r' ||
            value.ptr[end - 1U] == '\n')) end--;
        if (end <= start || value.ptr[start] != '#')
            return fail(scan, SVG_SHAPE_ERROR_UNSUPPORTED, offset + start);
        if (!id_exists(scan, (base_bytes){value.ptr + start + 1U,
                                         end - start - 1U}))
            return fail(scan, SVG_SHAPE_ERROR_UNSUPPORTED, offset + start);
        i = cursor;
    }
    return true;
}

static bool check_href(asset_scan *scan, const svg_xml_token *token)
{
    uint8_t decoded[DRAWING_ASSET_MAX_REFERENCE_BYTES];
    size_t length = 0U, decode_error = 0U;
    if (token->value.length > sizeof(decoded) ||
        !svg_xml_decode_attribute_value(scan->source, token->value,
            decoded, sizeof(decoded), &length, &decode_error))
        return fail(scan, SVG_SHAPE_ERROR_LIMIT,
            token->value.offset + decode_error);
    if (length >= 2U && decoded[0] == '#') {
        if (!id_exists(scan, (base_bytes){decoded + 1U, length - 1U}))
            return fail(scan, SVG_SHAPE_ERROR_UNSUPPORTED,
                token->value.offset);
        return true;
    }
    bool anchor = token_name_is(scan->source, scan->element_name, "a") &&
        scan->element_prefix.len == 0U;
    static const char dangerous_scheme[] = "javascript:";
    size_t scheme_at = 0U, matched = 0U;
    while (scheme_at < length && decoded[scheme_at] <= 0x20U) scheme_at++;
    for (; scheme_at < length && matched < sizeof(dangerous_scheme) - 1U;
         ++scheme_at) {
        uint8_t byte = decoded[scheme_at];
        if (byte == '\t' || byte == '\n' || byte == '\r') continue;
        uint8_t expected = (uint8_t)dangerous_scheme[matched];
        if (byte >= 'A' && byte <= 'Z') byte = (uint8_t)(byte - 'A' + 'a');
        if (byte != expected) break;
        matched++;
    }
    bool javascript = matched == sizeof(dangerous_scheme) - 1U;
    if (anchor && !javascript) return true;
    bool image = token_name_is(scan->source, scan->element_name, "image") &&
        scan->element_prefix.len == 0U;
    bool data_uri = image && length >= 5U &&
        (decoded[0] == 'd' || decoded[0] == 'D') &&
        (decoded[1] == 'a' || decoded[1] == 'A') &&
        (decoded[2] == 't' || decoded[2] == 'T') &&
        (decoded[3] == 'a' || decoded[3] == 'A') && decoded[4] == ':';
    return data_uri ? true : fail(scan, SVG_SHAPE_ERROR_UNSUPPORTED,
                                  token->value.offset);
}

static bool visit_asset(void *context, const svg_xml_token *token)
{
    asset_scan *scan = context;
    if (token->kind == SVG_XML_TOKEN_DOCTYPE ||
        token->kind == SVG_XML_TOKEN_PROCESSING_INSTRUCTION)
        return fail(scan, SVG_SHAPE_ERROR_UNSUPPORTED, token->source.offset);
    if (token->kind == SVG_XML_TOKEN_START_ELEMENT ||
        token->kind == SVG_XML_TOKEN_EMPTY_ELEMENT) {
        if (span_is(scan->source, token->local_name, "script") ||
            span_is(scan->source, token->local_name, "foreignObject"))
            return fail(scan, SVG_SHAPE_ERROR_UNSUPPORTED, token->name.offset);
        if (!token_span(scan->source, token->local_name, &scan->element_name) ||
            !token_span(scan->source, token->prefix, &scan->element_prefix))
            return fail(scan, SVG_SHAPE_ERROR_XML, token->name.offset);
        return true;
    }
    if (token->kind == SVG_XML_TOKEN_ATTRIBUTE) {
        if (token->local_name.length >= 2U &&
            token->local_name.offset <= scan->source.len &&
            token->local_name.length <= scan->source.len - token->local_name.offset &&
            (scan->source.ptr[token->local_name.offset] == 'o' ||
             scan->source.ptr[token->local_name.offset] == 'O') &&
            (scan->source.ptr[token->local_name.offset + 1U] == 'n' ||
             scan->source.ptr[token->local_name.offset + 1U] == 'N'))
            return fail(scan, SVG_SHAPE_ERROR_UNSUPPORTED, token->name.offset);
        if (span_is(scan->source, token->local_name, "id") &&
            token->prefix.length == 0U) {
            if (scan->pass < 2U) {
                if (scan->id_count >= DRAWING_ASSET_MAX_IDS)
                    return fail(scan, SVG_SHAPE_ERROR_LIMIT, token->value.offset);
                if (scan->pass == 0U) {
                    scan->id_count++;
                } else {
                    if (scan->id_count >= scan->id_capacity ||
                        scan->pool_used > scan->pool_capacity ||
                        DRAWING_ASSET_MAX_ID_BYTES >
                            scan->pool_capacity - scan->pool_used)
                        return fail(scan, SVG_SHAPE_ERROR_LIMIT, token->value.offset);
                    uint8_t *destination = scan->pool + scan->pool_used;
                    size_t decoded = 0U, error_offset = 0U;
                    if (!svg_xml_decode_attribute_value(scan->source, token->value,
                        destination, DRAWING_ASSET_MAX_ID_BYTES, &decoded,
                        &error_offset))
                        return fail(scan, SVG_SHAPE_ERROR_XML,
                            token->value.offset + error_offset);
                    if (decoded == 0U)
                        return fail(scan, SVG_SHAPE_ERROR_LIMIT, token->value.offset);
                    scan->ids[scan->id_count] = (drawing_id){
                        {destination, decoded}, token->value.offset};
                    scan->id_count++;
                }
                if (scan->pass == 1U)
                    scan->pool_used += DRAWING_ASSET_MAX_ID_BYTES;
            }
        }
        bool href = span_is(scan->source, token->local_name, "href") &&
            (token->prefix.length == 0U ||
             span_is(scan->source, token->prefix, "xlink"));
        if (href && scan->pass == 2U && !check_href(scan, token)) return false;
        bool style_value = span_is(scan->source, token->local_name, "style") ||
            span_is(scan->source, token->local_name, "fill") ||
            span_is(scan->source, token->local_name, "stroke") ||
            span_is(scan->source, token->local_name, "clip-path") ||
            span_is(scan->source, token->local_name, "filter") ||
            span_is(scan->source, token->local_name, "mask") ||
            span_is(scan->source, token->local_name, "marker") ||
            span_is(scan->source, token->local_name, "marker-start") ||
            span_is(scan->source, token->local_name, "marker-mid") ||
            span_is(scan->source, token->local_name, "marker-end");
        if (style_value && scan->pass == 2U) {
            uint8_t decoded[DRAWING_ASSET_MAX_REFERENCE_BYTES];
            size_t length = 0U, error_offset = 0U;
            if (token->value.length > sizeof(decoded) ||
                !svg_xml_decode_attribute_value(scan->source, token->value,
                    decoded, sizeof(decoded), &length, &error_offset))
                return fail(scan, SVG_SHAPE_ERROR_LIMIT,
                    token->value.offset + error_offset);
            if (!scan_css_urls(scan, (base_bytes){decoded, length},
                               token->value.offset)) return false;
        }
        return true;
    }
    if ((token->kind == SVG_XML_TOKEN_TEXT ||
         token->kind == SVG_XML_TOKEN_CDATA) && scan->pass == 2U &&
        token_name_is(scan->source, scan->element_name, "style")) {
        if (token->value.length > DRAWING_ASSET_MAX_REFERENCE_BYTES)
            return fail(scan, SVG_SHAPE_ERROR_LIMIT, token->value.offset);
        uint8_t decoded[DRAWING_ASSET_MAX_REFERENCE_BYTES];
        size_t length = 0U, error_offset = 0U;
        base_bytes css_text;
        if (token->kind == SVG_XML_TOKEN_CDATA) {
            if (token->value.offset > scan->source.len ||
                token->value.length > scan->source.len - token->value.offset)
                return fail(scan, SVG_SHAPE_ERROR_XML, token->value.offset);
            css_text = (base_bytes){scan->source.ptr + token->value.offset,
                                    token->value.length};
        } else {
            if (!svg_xml_decode_attribute_value(scan->source, token->value,
                decoded, sizeof(decoded), &length, &error_offset))
                return fail(scan, SVG_SHAPE_ERROR_XML,
                    token->value.offset + error_offset);
            css_text = (base_bytes){decoded, length};
        }
        if (!scan_css_urls(scan, css_text, token->value.offset)) return false;
    }
    return true;
}

static svg_xml_stream_status scan_svg(asset_scan *scan,
    svg_xml_cancel_check_fn canceled, void *cancel_context,
    svg_xml_result *out_result)
{
    svg_xml_token_stream *stream = NULL;
    svg_xml_stream_status status = svg_xml_token_stream_start_visitor(
        scan->source, visit_asset, scan, canceled, cancel_context, &stream);
    if (status != SVG_XML_STREAM_OK) return status;
    do {
        status = svg_xml_token_stream_step(stream, 64U * 1024U);
    } while (status == SVG_XML_STREAM_OK);
    if (out_result != NULL) *out_result = svg_xml_token_stream_result(stream);
    svg_xml_token_stream_destroy(stream);
    return status;
}

project_drawing_asset_result project_drawing_asset_validate(base_bytes source,
    svg_xml_cancel_check_fn canceled, void *cancel_context, bool *out_canceled)
{
    if (out_canceled != NULL) *out_canceled = false;
    if (!base_bytes_valid(source) || source.len == 0U ||
        source.len > PROJECT_BUNDLE_MAX_ASSET_BYTES)
        return (project_drawing_asset_result){SVG_SHAPE_ERROR_INVALID_ARGUMENT, 0U};
    asset_scan count = {.source = source};
    svg_xml_result xml = {0};
    svg_xml_stream_status status = scan_svg(&count, canceled, cancel_context, &xml);
    if (status != SVG_XML_STREAM_COMPLETE) {
        bool is_canceled = status == SVG_XML_STREAM_CANCELED &&
            count.error == SVG_SHAPE_ERROR_NONE;
        if (out_canceled != NULL) *out_canceled = is_canceled;
        return (project_drawing_asset_result){
            is_canceled ? SVG_SHAPE_ERROR_LIMIT :
                count.error != SVG_SHAPE_ERROR_NONE ? count.error : SVG_SHAPE_ERROR_XML,
            count.error != SVG_SHAPE_ERROR_NONE ? count.error_offset :
                xml.diagnostic.offset};
    }
    size_t pool_capacity = 0U;
    if (count.id_count > SIZE_MAX / sizeof(*count.ids) ||
        !base_size_mul(count.id_count, DRAWING_ASSET_MAX_ID_BYTES,
                       &pool_capacity))
        return (project_drawing_asset_result){SVG_SHAPE_ERROR_LIMIT, 0U};
    drawing_id *ids = count.id_count == 0U ? NULL :
        calloc(count.id_count, sizeof(*ids));
    uint8_t *pool = pool_capacity == 0U ? NULL : malloc(pool_capacity);
    if ((count.id_count != 0U && ids == NULL) ||
        (pool_capacity != 0U && pool == NULL)) {
        free(ids);
        free(pool);
        return (project_drawing_asset_result){SVG_SHAPE_ERROR_OOM, 0U};
    }
    asset_scan fill = {.source = source, .ids = ids, .pass = 1U,
        .id_capacity = count.id_count, .pool_capacity = pool_capacity,
        .pool = pool};
    status = scan_svg(&fill, canceled, cancel_context, &xml);
    if (status != SVG_XML_STREAM_COMPLETE || fill.error != SVG_SHAPE_ERROR_NONE ||
        fill.id_count != count.id_count || fill.pool_used != pool_capacity) {
        bool is_canceled = status == SVG_XML_STREAM_CANCELED &&
            fill.error == SVG_SHAPE_ERROR_NONE;
        if (out_canceled != NULL) *out_canceled = is_canceled;
        project_drawing_asset_result result = {
            is_canceled ? SVG_SHAPE_ERROR_LIMIT :
                fill.error != SVG_SHAPE_ERROR_NONE ? fill.error : SVG_SHAPE_ERROR_XML,
            fill.error != SVG_SHAPE_ERROR_NONE ? fill.error_offset : xml.diagnostic.offset};
        free(ids);
        free(pool);
        return result;
    }
    if (fill.id_count > 1U) qsort(ids, fill.id_count, sizeof(*ids), id_compare);
    for (size_t i = 1U; i < fill.id_count; ++i) {
        if (bytes_order(ids[i - 1U].value, ids[i].value) == 0) {
            size_t offset = ids[i].offset;
            free(ids);
            free(pool);
            return (project_drawing_asset_result){SVG_SHAPE_ERROR_UNSUPPORTED, offset};
        }
    }
    fill.pass = 2U;
    fill.element_name = (base_bytes){0};
    fill.element_prefix = (base_bytes){0};
    status = scan_svg(&fill, canceled, cancel_context, &xml);
    if (status != SVG_XML_STREAM_COMPLETE || fill.error != SVG_SHAPE_ERROR_NONE) {
        bool is_canceled = status == SVG_XML_STREAM_CANCELED &&
            fill.error == SVG_SHAPE_ERROR_NONE;
        if (out_canceled != NULL) *out_canceled = is_canceled;
        project_drawing_asset_result result = {
            is_canceled ? SVG_SHAPE_ERROR_LIMIT :
                fill.error != SVG_SHAPE_ERROR_NONE ? fill.error : SVG_SHAPE_ERROR_XML,
            fill.error != SVG_SHAPE_ERROR_NONE ? fill.error_offset : xml.diagnostic.offset};
        free(ids);
        free(pool);
        return result;
    }
    if (canceled != NULL && canceled(cancel_context)) {
        if (out_canceled != NULL) *out_canceled = true;
        free(ids);
        free(pool);
        return (project_drawing_asset_result){SVG_SHAPE_ERROR_LIMIT, 0U};
    }
    svg_shape_document document = {0};
    svg_shape_diagnostic diagnostic = {SVG_SHAPE_ERROR_NONE, 0U};
    bool decode_canceled = false;
    svg_shape_error error = svg_shape_decode_cancellable(source, canceled,
        cancel_context, &decode_canceled, &document, &diagnostic);
    if (decode_canceled && out_canceled != NULL) *out_canceled = true;
    svg_shape_document_destroy(&document);
    free(ids);
    free(pool);
    return (project_drawing_asset_result){error, diagnostic.offset};
}
