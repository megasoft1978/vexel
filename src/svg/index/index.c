#include "index.h"
#include "xml_reader.h"
#include "platform/platform_task.h"

#include <stdlib.h>
#include <stdatomic.h>
#include <string.h>

typedef struct open_element { size_t record; size_t name_offset; size_t name_length; } open_element;
typedef struct validation_payload {
    svg_xml_diagnostic diagnostic;
    svg_xml_token *tokens;
    size_t token_count;
    atomic_size_t bytes_read;
    atomic_size_t error_offset;
    atomic_bool failed;
} validation_payload;

struct svg_index_session {
    base_bytes source;
    svg_index_element *elements;
    size_t element_count;
    size_t element_capacity;
    size_t attribute_count;
    size_t root_open_length;
    size_t preview_record;
    size_t depth;
    size_t root_count;
    uint64_t generation;
    svg_xml_reader *reader;
    bool canceled;
    bool complete;
    bool preview;
    bool preview_candidate;
    bool preview_root_supported;
    svg_xml_diagnostic diagnostic;
    svg_xml_diagnostic reader_diagnostic;
    validation_payload validation;
    svg_xml_token *tokens;
    size_t token_count;
    platform_task_queue *validation_queue;
    bool validation_complete;
    bool source_scanned;
    open_element stack[SVG_INDEX_MAX_DEPTH];
};

#define SVG_INDEX_MAX_COMPONENT_BYTES (SVG_INDEX_MAX_STORAGE_BYTES / 2U)
static bool task_is_canceled(void *context)
{
    const platform_task_cancel_token *token = context;
    return atomic_load_explicit(&token->canceled, memory_order_acquire);
}

static void set_error(svg_index_session *s, svg_xml_error code, size_t at)
{
    if (s->diagnostic.category == SVG_XML_ERROR_NONE) {
        s->diagnostic.category = code;
        s->diagnostic.offset = at;
    }
}

static bool validate_document(void *opaque, const platform_task_cancel_token *token,
                              int64_t *out_value)
{
    svg_index_session *s = opaque;
    svg_xml_token_stream *stream = NULL;
    svg_xml_stream_status status = svg_xml_token_stream_start(s->source,
        SVG_INDEX_MAX_COMPONENT_BYTES, task_is_canceled, (void *)token, &stream);
    while (status == SVG_XML_STREAM_OK &&
           !atomic_load_explicit(&token->canceled, memory_order_acquire)) {
        status = svg_xml_token_stream_step(stream, SVG_INDEX_MAX_STEP_BYTES);
        svg_xml_result partial = svg_xml_token_stream_result(stream);
        if (partial.diagnostic.category != SVG_XML_ERROR_NONE) {
            atomic_store_explicit(&s->validation.error_offset,
                partial.diagnostic.offset, memory_order_relaxed);
            atomic_store_explicit(&s->validation.failed, true, memory_order_release);
        }
        atomic_store_explicit(&s->validation.bytes_read,
            svg_xml_token_stream_bytes_read(stream), memory_order_release);
    }
    if (atomic_load_explicit(&token->canceled, memory_order_acquire) ||
        status == SVG_XML_STREAM_CANCELED) {
        svg_xml_token_stream_destroy(stream);
        return false;
    }
    svg_xml_result result = svg_xml_token_stream_result(stream);
    s->validation.diagnostic = result.diagnostic;
    s->validation.token_count = result.token_count;
    if (status == SVG_XML_STREAM_COMPLETE &&
        !svg_xml_token_stream_take(stream, &s->validation.tokens,
                                   &s->validation.token_count)) {
        s->validation.diagnostic = (svg_xml_diagnostic){SVG_XML_ERROR_OUTPUT_EXHAUSTED, 0U};
        status = SVG_XML_STREAM_FAILED;
    }
    *out_value = (int64_t)s->validation.diagnostic.category;
    svg_xml_token_stream_destroy(stream);
    return status == SVG_XML_STREAM_COMPLETE &&
           s->validation.diagnostic.category == SVG_XML_ERROR_NONE;
}

static void harvest_validation(svg_index_session *s)
{
    if (s->validation_complete) return;
    platform_task_result result;
    if (platform_task_poll(s->validation_queue, &result)) {
        s->validation_complete = true;
        s->tokens = s->validation.tokens;
        s->token_count = s->validation.token_count;
        s->validation.tokens = NULL;
        if (s->validation.diagnostic.category != SVG_XML_ERROR_NONE)
            s->diagnostic = s->validation.diagnostic;
        else if (!result.succeeded)
            set_error(s, SVG_XML_ERROR_OUTPUT_EXHAUSTED, 0U);
        else if (s->reader_diagnostic.category != SVG_XML_ERROR_NONE)
            s->diagnostic = s->reader_diagnostic;
    }
}

static bool equal_name(const svg_index_session *s, size_t at, size_t n, const char *text)
{
    size_t length = strlen(text);
    return n == length && memcmp(s->source.ptr + at, text, n) == 0;
}

static bool preview_root_is_svg(const svg_index_session *s)
{
    if (s->element_count == 0U) return false;
    const svg_index_element *root = &s->elements[0];
    return root->depth == 0U && root->name_length == 3U &&
           memcmp(s->source.ptr + root->name_offset, "svg", 3U) == 0;
}

static bool xml_space(uint8_t byte)
{
    return byte == ' ' || byte == '\t' || byte == '\r' || byte == '\n';
}

static bool attr_is(const svg_index_session *s, size_t start, size_t length,
                    const char *name)
{
    size_t name_length = strlen(name);
    return length == name_length && memcmp(s->source.ptr + start, name, length) == 0;
}

static bool preview_attributes_supported(const svg_index_session *s,
                                         const svg_xml_reader_tag *tag,
                                         svg_index_support shape)
{
    size_t end = tag->source.offset + tag->source.length;
    size_t at = tag->name.offset + tag->name.length;
    bool width = false, height = false, data = false;
    while (at < end) {
        while (at < end && xml_space(s->source.ptr[at])) ++at;
        if (at >= end || s->source.ptr[at] == '>' || s->source.ptr[at] == '/') break;
        size_t name = at;
        while (at < end && !xml_space(s->source.ptr[at]) &&
               s->source.ptr[at] != '=' && s->source.ptr[at] != '/' &&
               s->source.ptr[at] != '>') ++at;
        size_t name_length = at - name;
        if (name_length == 0U || memchr(s->source.ptr + name, ':', name_length) != NULL)
            return false;
        while (at < end && xml_space(s->source.ptr[at])) ++at;
        if (at >= end || s->source.ptr[at++] != '=') return false;
        while (at < end && xml_space(s->source.ptr[at])) ++at;
        if (at >= end || (s->source.ptr[at] != '\'' && s->source.ptr[at] != '"')) return false;
        uint8_t quote = s->source.ptr[at++];
        size_t value = at;
        while (at < end && s->source.ptr[at] != quote) ++at;
        if (at >= end) return false;
        size_t value_length = at - value;
        bool accepted = false;
        if (shape == SVG_INDEX_UNSUPPORTED) {
            accepted = attr_is(s, name, name_length, "xmlns") ||
                attr_is(s, name, name_length, "version") ||
                attr_is(s, name, name_length, "width") ||
                attr_is(s, name, name_length, "height") ||
                attr_is(s, name, name_length, "viewBox");
            if (attr_is(s, name, name_length, "width")) width = true;
            if (attr_is(s, name, name_length, "height")) height = true;
            if (attr_is(s, name, name_length, "xmlns") &&
                (value_length != 26U || memcmp(s->source.ptr + value,
                    "http://www.w3.org/2000/svg", 26U) != 0)) return false;
        } else if (shape == SVG_INDEX_SUPPORTED_RECT) {
            accepted = attr_is(s, name, name_length, "fill") ||
                attr_is(s, name, name_length, "fill-rule") ||
                attr_is(s, name, name_length, "x") || attr_is(s, name, name_length, "y") ||
                attr_is(s, name, name_length, "width") || attr_is(s, name, name_length, "height");
            if (attr_is(s, name, name_length, "width")) width = true;
            if (attr_is(s, name, name_length, "height")) height = true;
        } else {
            accepted = attr_is(s, name, name_length, "fill") ||
                attr_is(s, name, name_length, "fill-rule") || attr_is(s, name, name_length, "d");
            if (attr_is(s, name, name_length, "d")) data = true;
        }
        if (!accepted) return false;
        at++;
    }
    return shape == SVG_INDEX_UNSUPPORTED ? width && height :
        shape == SVG_INDEX_SUPPORTED_RECT ? width && height : data;
}

static bool reserve_element(svg_index_session *s)
{
    if (s->element_count >= SVG_INDEX_MAX_ELEMENTS) return false;
    if (s->element_count < s->element_capacity) return true;
    size_t cap = s->element_capacity == 0 ? 256U : s->element_capacity * 2U;
    if (cap > SVG_INDEX_MAX_ELEMENTS) cap = SVG_INDEX_MAX_ELEMENTS;
    if (cap > SVG_INDEX_MAX_COMPONENT_BYTES / sizeof(*s->elements)) return false;
    void *next = realloc(s->elements, cap * sizeof(*s->elements));
    if (next == NULL) return false;
    s->elements = next;
    s->element_capacity = cap;
    return true;
}

static void finish_open_tag(svg_index_session *s, const svg_xml_reader_tag *tag)
{
    bool closing = tag->kind == SVG_XML_TOKEN_END_ELEMENT;
    bool self_closing = tag->kind == SVG_XML_TOKEN_EMPTY_ELEMENT;
    if (tag->attribute_count > SVG_INDEX_MAX_ATTRIBUTES - s->attribute_count) {
        set_error(s, SVG_XML_ERROR_ATTRIBUTE_LIMIT, tag->source.offset);
        return;
    }
    s->attribute_count += tag->attribute_count;
    if (closing) {
        if (s->depth == 0) { set_error(s, SVG_XML_ERROR_MISMATCHED_TAG, tag->source.offset); return; }
        open_element top = s->stack[s->depth - 1U];
        if (top.name_length != tag->name.length ||
            memcmp(s->source.ptr + top.name_offset, s->source.ptr + tag->name.offset,
                   tag->name.length) != 0) {
            set_error(s, SVG_XML_ERROR_MISMATCHED_TAG, tag->source.offset); return;
        }
        --s->depth;
        s->elements[top.record].source_length = tag->source.offset + tag->source.length -
                                                s->elements[top.record].source_offset;
        s->elements[top.record].closed = true;
        if (!s->preview_candidate && !s->preview && s->depth == 1U && s->preview_root_supported &&
            s->elements[top.record].support != SVG_INDEX_UNSUPPORTED)
            { s->preview_candidate = true; s->preview_record = top.record; }
        return;
    }
    if (tag->name.length == 0U || tag->name.length == 4096U) {
        set_error(s, SVG_XML_ERROR_INVALID_NAME, tag->name.offset);
        return;
    }
    if (s->depth >= SVG_INDEX_MAX_DEPTH) { set_error(s, SVG_XML_ERROR_DEPTH_LIMIT, tag->source.offset); return; }
    if (s->depth == 0 && ++s->root_count > 1U) {
        set_error(s, SVG_XML_ERROR_MULTIPLE_ROOTS, tag->source.offset); return;
    }
    if (!reserve_element(s)) { set_error(s, SVG_XML_ERROR_NODE_LIMIT, tag->source.offset); return; }
    size_t i = s->element_count++;
    svg_index_element *e = &s->elements[i];
    e->parent = s->depth == 0 ? SIZE_MAX : s->stack[s->depth - 1U].record;
    e->source_offset = tag->source.offset;
    e->source_length = tag->source.length;
    e->name_offset = tag->name.offset;
    e->name_length = tag->name.length;
    e->depth = s->depth;
    e->support = equal_name(s, e->name_offset, e->name_length, "rect") ? SVG_INDEX_SUPPORTED_RECT :
                 equal_name(s, e->name_offset, e->name_length, "path") ? SVG_INDEX_SUPPORTED_PATH : SVG_INDEX_UNSUPPORTED;
    e->closed = self_closing;
    if (s->depth == 0U) {
        s->root_open_length = tag->source.length;
        s->preview_root_supported = preview_root_is_svg(s) &&
            preview_attributes_supported(s, tag, SVG_INDEX_UNSUPPORTED);
    }
    if (s->depth == 1U && e->support != SVG_INDEX_UNSUPPORTED &&
        !preview_attributes_supported(s, tag, e->support))
        e->support = SVG_INDEX_UNSUPPORTED;
    if (!s->preview_candidate && !s->preview && s->depth == 1U && s->preview_root_supported && self_closing &&
        e->support != SVG_INDEX_UNSUPPORTED) {
        s->preview_candidate = true; s->preview_record = i;
    }
    if (!self_closing) {
        s->stack[s->depth].record = i;
        s->stack[s->depth].name_offset = tag->name.offset;
        s->stack[s->depth].name_length = tag->name.length;
        ++s->depth;
    }
}

static bool index_reader_tag(void *context, const svg_xml_reader_tag *tag)
{
    finish_open_tag(context, tag);
    return true;
}

static void publish_validated_preview(svg_index_session *s)
{
    if (!s->preview_candidate || s->preview) return;
    const svg_index_element *shape = &s->elements[s->preview_record];
    size_t span_end = shape->source_offset + shape->source_length;
    bool failed = atomic_load_explicit(&s->validation.failed, memory_order_acquire);
    size_t validated_end = failed ?
        atomic_load_explicit(&s->validation.error_offset, memory_order_relaxed) :
        atomic_load_explicit(&s->validation.bytes_read, memory_order_acquire);
    if (validated_end >= span_end)
        s->preview = true;
}

svg_index_status svg_index_start(base_bytes source, uint64_t generation,
                                 svg_index_session **out_session)
{
    if (out_session == NULL) return SVG_INDEX_INVALID;
    *out_session = NULL;
    if (source.len > SVG_INDEX_MAX_SOURCE_BYTES) return SVG_INDEX_LIMIT;
    if (!base_bytes_valid(source)) return SVG_INDEX_INVALID;
    svg_index_session *s = calloc(1, sizeof(*s));
    if (s == NULL) return SVG_INDEX_OOM;
    s->source = source;
    atomic_init(&s->validation.bytes_read, 0U);
    atomic_init(&s->validation.error_offset, 0U);
    atomic_init(&s->validation.failed, false);
    s->generation = generation;
    svg_xml_reader_status reader_status = svg_xml_reader_start(source, &s->reader);
    if (reader_status != SVG_XML_READER_OK) {
        free(s);
        return reader_status == SVG_XML_READER_LIMIT ? SVG_INDEX_LIMIT : SVG_INDEX_OOM;
    }
    platform_task_config config = {1U, 1U, NULL, NULL, NULL};
    if (platform_task_queue_create(&config, &s->validation_queue) != PLATFORM_TASK_OK ||
        platform_task_submit(s->validation_queue, 1U, 1U, validate_document, s) != PLATFORM_TASK_OK) {
        if (s->validation_queue != NULL) {
            platform_task_shutdown(s->validation_queue);
            platform_task_destroy(s->validation_queue);
        }
        svg_xml_reader_destroy(s->reader);
        free(s);
        return SVG_INDEX_OOM;
    }
    *out_session = s;
    return SVG_INDEX_OK;
}

svg_index_status svg_index_step(svg_index_session *s, size_t byte_budget)
{
    if (s == NULL || byte_budget == 0) return SVG_INDEX_INVALID;
    if (s->canceled) return SVG_INDEX_CANCELED;
    if (s->complete) return SVG_INDEX_COMPLETE;
    harvest_validation(s);
    publish_validated_preview(s);
    if (s->diagnostic.category != SVG_XML_ERROR_NONE)
        return s->validation_complete ? SVG_INDEX_MALFORMED : SVG_INDEX_OK;
    if (s->preview_candidate && !s->preview) return SVG_INDEX_OK;
    svg_xml_reader_status reader_status = svg_xml_reader_step(s->reader, byte_budget,
                                                               index_reader_tag, s);
    svg_xml_reader_progress reader_progress = svg_xml_reader_get_progress(s->reader);
    if (reader_progress.diagnostic.category != SVG_XML_ERROR_NONE)
        s->reader_diagnostic = reader_progress.diagnostic;
    if (reader_status == SVG_XML_READER_MALFORMED) {
        harvest_validation(s);
        publish_validated_preview(s);
        if (!s->validation_complete) return SVG_INDEX_OK;
        if (s->diagnostic.category == SVG_XML_ERROR_NONE)
            s->diagnostic = s->reader_diagnostic;
        return SVG_INDEX_MALFORMED;
    }
    if (reader_status == SVG_XML_READER_COMPLETE) {
        if (s->depth != 0U || s->root_count != 1U)
            set_error(s, SVG_XML_ERROR_INCOMPLETE_DOCUMENT, s->source.len);
        else s->source_scanned = true;
    }
    harvest_validation(s);
    publish_validated_preview(s);
    if (s->diagnostic.category != SVG_XML_ERROR_NONE)
        return s->validation_complete ? SVG_INDEX_MALFORMED : SVG_INDEX_OK;
    if (s->source_scanned && s->validation_complete) s->complete = true;
    return s->diagnostic.category == SVG_XML_ERROR_NONE ?
           (s->complete ? SVG_INDEX_COMPLETE : SVG_INDEX_OK) : SVG_INDEX_MALFORMED;
}

svg_index_progress svg_index_get_progress(const svg_index_session *s)
{
    svg_index_progress p = {0};
    if (s == NULL) return p;
    p.bytes_read = svg_xml_reader_get_progress(s->reader).bytes_read;
    p.source_bytes = s->source.len;
    p.element_count = s->element_count; p.attribute_count = s->attribute_count;
    p.token_count = s->token_count;
    p.generation = s->generation; p.diagnostic = s->diagnostic;
    p.complete = s->complete; p.canceled = s->canceled; p.first_preview_ready = s->preview;
    return p;
}

svg_index_status svg_index_advance_generation(svg_index_session *s, uint64_t generation)
{
    if (s == NULL || generation <= s->generation) return SVG_INDEX_INVALID;
    s->generation = generation; s->preview = false; s->preview_candidate = false;
    return SVG_INDEX_OK;
}

bool svg_index_preview_generation_is_current(const svg_index_session *s,
                                             uint64_t generation)
{
    return s != NULL && !s->canceled && s->preview && generation == s->generation;
}

void svg_index_cancel(svg_index_session *s)
{
    if (s == NULL) return;
    s->canceled = true;
    (void)platform_task_cancel(s->validation_queue, 1U);
}

bool svg_index_element_at(const svg_index_session *s, size_t index, svg_index_element *out)
{
    if (s == NULL || out == NULL || index >= s->element_count) return false;
    *out = s->elements[index]; return true;
}

bool svg_index_token_at(const svg_index_session *s, size_t index, svg_xml_token *out)
{
    if (s == NULL || out == NULL || !s->complete || index >= s->token_count) return false;
    *out = s->tokens[index];
    return true;
}

bool svg_index_make_preview_source(const svg_index_session *s, base_mut_bytes *out)
{
    if (out == NULL) return false;
    *out = (base_mut_bytes){NULL, 0U};
    if (s == NULL || !s->preview || s->root_open_length == 0U ||
        s->preview_record >= s->element_count) return false;
    const svg_index_element *root = &s->elements[0];
    const svg_index_element *shape = &s->elements[s->preview_record];
    if (root->name_length != 3U || memcmp(s->source.ptr + root->name_offset, "svg", 3U) != 0 ||
        shape->source_length == 0U) return false;
    size_t total = 0U;
    if (!base_size_add(s->root_open_length, shape->source_length, &total) ||
        !base_size_add(total, 6U, &total) || total > 16U * 1024U * 1024U) return false;
    uint8_t *memory = malloc(total);
    if (memory == NULL) return false;
    memcpy(memory, s->source.ptr + root->source_offset, s->root_open_length);
    memcpy(memory + s->root_open_length, s->source.ptr + shape->source_offset,
           shape->source_length);
    memcpy(memory + total - 6U, "</svg>", 6U);
    *out = (base_mut_bytes){memory, total};
    return true;
}

void svg_index_destroy(svg_index_session *s)
{
    if (s == NULL) return;
    platform_task_shutdown(s->validation_queue);
    platform_task_destroy(s->validation_queue);
    svg_xml_reader_destroy(s->reader);
    free(s->tokens); free(s->validation.tokens); free(s->elements); free(s);
}
