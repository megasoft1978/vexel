#include "xml_tokenizer.h"

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define CHECK(condition) do { if (!(condition)) { \
    (void)fprintf(stderr, "check failed at %s:%d: %s\n", __FILE__, __LINE__, #condition); \
    abort(); \
} } while (0)

static bool token_equal(const svg_xml_token *a, const svg_xml_token *b)
{
    return a->kind == b->kind && a->depth == b->depth &&
        a->source.offset == b->source.offset && a->source.length == b->source.length &&
        a->name.offset == b->name.offset && a->name.length == b->name.length &&
        a->value.offset == b->value.offset && a->value.length == b->value.length;
}

typedef struct visitor_log {
    svg_xml_token *tokens;
    size_t count;
    size_t capacity;
    size_t stop_after;
} visitor_log;

typedef struct cancel_after_calls {
    size_t calls;
    size_t cancel_at;
} cancel_after_calls;

static bool collect_token(void *context, const svg_xml_token *token)
{
    visitor_log *log = context;
    if (log->tokens != NULL && log->count < log->capacity)
        log->tokens[log->count] = *token;
    ++log->count;
    return log->stop_after == 0U || log->count < log->stop_after;
}

static bool always_cancel(void *context)
{
    (void)context;
    return true;
}

static bool cancel_at_call(void *context)
{
    cancel_after_calls *cancel = context;
    ++cancel->calls;
    return cancel->calls >= cancel->cancel_at;
}

static void check_discard_visitor(void)
{
    static const uint8_t source[] =
        "<?xml version='1.0'?><svg width='2'><g id='a'/><text>x</text></svg>";
    svg_xml_token expected[16] = {{0}};
    svg_xml_result reference = svg_xml_tokenize(
        (base_bytes){source, sizeof(source) - 1U}, svg_xml_default_options(),
        expected, 16U);
    CHECK(reference.diagnostic.category == SVG_XML_ERROR_NONE);
    svg_xml_token actual[16] = {{0}};
    visitor_log log = {actual, 0U, 16U, 0U};
    svg_xml_token_stream *stream = NULL;
    CHECK(svg_xml_token_stream_start_visitor((base_bytes){source, sizeof(source) - 1U},
        collect_token, &log, NULL, NULL, &stream) == SVG_XML_STREAM_OK);
    size_t retained = svg_xml_token_stream_visitor_storage_bytes(stream);
    CHECK(retained >= sizeof(svg_xml_token) * 4097U);
    svg_xml_stream_status status = SVG_XML_STREAM_OK;
    while (status == SVG_XML_STREAM_OK)
        status = svg_xml_token_stream_step(stream, 3U);
    CHECK(status == SVG_XML_STREAM_COMPLETE);
    CHECK(log.count == reference.token_count);
    for (size_t i = 0U; i < log.count; ++i)
        CHECK(token_equal(&actual[i], &expected[i]));
    CHECK(actual[1].kind == SVG_XML_TOKEN_START_ELEMENT &&
          actual[1].source.length == sizeof("<svg width='2'>") - 1U);
    CHECK(actual[2].kind == SVG_XML_TOKEN_ATTRIBUTE);
    CHECK(svg_xml_token_stream_visitor_storage_bytes(stream) == retained);
    svg_xml_token *no_vector = (svg_xml_token *)(uintptr_t)1U;
    size_t no_count = SIZE_MAX;
    CHECK(!svg_xml_token_stream_take(stream, &no_vector, &no_count));
    CHECK(no_vector == NULL && no_count == 0U);
    svg_xml_token_stream_destroy(stream);

    static const uint8_t all_kinds[] =
        "<?xml version='1.0'?><!--c--><?p x?><!DOCTYPE svg [<!ENTITY e 'x'>]>"
        "<svg a='v'><![CDATA[t]]><g/></svg>";
    svg_xml_token all_expected[32] = {{0}}, all_actual[32] = {{0}};
    svg_xml_result all_reference = svg_xml_tokenize(
        (base_bytes){all_kinds, sizeof(all_kinds) - 1U}, svg_xml_default_options(),
        all_expected, 32U);
    CHECK(all_reference.diagnostic.category == SVG_XML_ERROR_NONE);
    for (size_t budget = 3U; budget <= 5U; ++budget) {
        visitor_log all_log = {all_actual, 0U, 32U, 0U};
        CHECK(svg_xml_token_stream_start_visitor((base_bytes){all_kinds,
            sizeof(all_kinds) - 1U}, collect_token, &all_log, NULL, NULL,
            &stream) == SVG_XML_STREAM_OK);
        status = SVG_XML_STREAM_OK;
        size_t all_steps = 0U;
        while (status == SVG_XML_STREAM_OK && all_steps++ < 10000U)
            status = svg_xml_token_stream_step(stream, budget);
        CHECK(status == SVG_XML_STREAM_COMPLETE &&
              all_log.count == all_reference.token_count);
        bool declaration = false, comment = false, pi = false, doctype = false;
        bool start = false, empty = false, end = false, attribute = false, cdata = false;
        for (size_t i = 0U; i < all_log.count; ++i) {
            CHECK(token_equal(&all_actual[i], &all_expected[i]));
            declaration |= all_actual[i].kind == SVG_XML_TOKEN_DECLARATION;
            comment |= all_actual[i].kind == SVG_XML_TOKEN_COMMENT;
            pi |= all_actual[i].kind == SVG_XML_TOKEN_PROCESSING_INSTRUCTION;
            doctype |= all_actual[i].kind == SVG_XML_TOKEN_DOCTYPE;
            start |= all_actual[i].kind == SVG_XML_TOKEN_START_ELEMENT;
            empty |= all_actual[i].kind == SVG_XML_TOKEN_EMPTY_ELEMENT;
            end |= all_actual[i].kind == SVG_XML_TOKEN_END_ELEMENT;
            attribute |= all_actual[i].kind == SVG_XML_TOKEN_ATTRIBUTE;
            cdata |= all_actual[i].kind == SVG_XML_TOKEN_CDATA;
        }
        CHECK(declaration && comment && pi && doctype && start && empty && end &&
              attribute && cdata);
        svg_xml_token_stream_destroy(stream);
    }

    static const uint8_t duplicate[] = "<svg a='1' a='2'/>";
    visitor_log duplicate_log = {NULL, 0U, 0U, 0U};
    CHECK(svg_xml_token_stream_start_visitor((base_bytes){duplicate,
        sizeof(duplicate) - 1U}, collect_token, &duplicate_log, NULL, NULL,
        &stream) == SVG_XML_STREAM_OK);
    status = SVG_XML_STREAM_OK;
    while (status == SVG_XML_STREAM_OK)
        status = svg_xml_token_stream_step(stream, 64U);
    CHECK(status == SVG_XML_STREAM_FAILED && duplicate_log.count == 0U);
    CHECK(svg_xml_token_stream_result(stream).diagnostic.category ==
          SVG_XML_ERROR_DUPLICATE_ATTRIBUTE);
    svg_xml_token_stream_destroy(stream);

    static const uint8_t bad_declaration[] =
        "<?xml version='1.0' encoding='UTF-16'?><svg/>";
    visitor_log bad_declaration_log = {NULL, 0U, 0U, 0U};
    CHECK(svg_xml_token_stream_start_visitor((base_bytes){bad_declaration,
        sizeof(bad_declaration) - 1U}, collect_token, &bad_declaration_log,
        NULL, NULL, &stream) == SVG_XML_STREAM_OK);
    status = SVG_XML_STREAM_OK;
    while (status == SVG_XML_STREAM_OK)
        status = svg_xml_token_stream_step(stream, 64U);
    CHECK(status == SVG_XML_STREAM_FAILED && bad_declaration_log.count == 0U);
    CHECK(svg_xml_token_stream_result(stream).diagnostic.category ==
          SVG_XML_ERROR_UNSUPPORTED_ENCODING);
    svg_xml_token_stream_destroy(stream);

    static const uint8_t cancel_declaration[] =
        "<?xml version='1.0' encoding='UTF-8' standalone='yes'?><svg/>";
    visitor_log cancel_declaration_log = {NULL, 0U, 0U, 0U};
    cancel_after_calls cancel_during_validation = {
        0U, sizeof(cancel_declaration) - 1U + 50U};
    CHECK(svg_xml_token_stream_start_visitor((base_bytes){cancel_declaration,
        sizeof(cancel_declaration) - 1U}, collect_token, &cancel_declaration_log,
        cancel_at_call, &cancel_during_validation,
        &stream) == SVG_XML_STREAM_OK);
    status = svg_xml_token_stream_step(stream, 64U * 1024U);
    CHECK(status == SVG_XML_STREAM_CANCELED && cancel_declaration_log.count == 0U);
    svg_xml_token_stream_destroy(stream);

    static const uint8_t one_tag[] = "<svg a='x'/>";
    visitor_log attribute_cancel = {NULL, 0U, 0U, 2U};
    CHECK(svg_xml_token_stream_start_visitor((base_bytes){one_tag,
        sizeof(one_tag) - 1U}, collect_token, &attribute_cancel, NULL, NULL,
        &stream) == SVG_XML_STREAM_OK);
    status = SVG_XML_STREAM_OK;
    while (status == SVG_XML_STREAM_OK)
        status = svg_xml_token_stream_step(stream, 64U);
    CHECK(status == SVG_XML_STREAM_CANCELED && attribute_cancel.count == 2U);
    svg_xml_token_stream_destroy(stream);

    visitor_log external_cancel = {NULL, 0U, 0U, 0U};
    CHECK(svg_xml_token_stream_start_visitor((base_bytes){one_tag,
        sizeof(one_tag) - 1U}, collect_token, &external_cancel, always_cancel,
        NULL, &stream) == SVG_XML_STREAM_OK);
    CHECK(svg_xml_token_stream_step(stream, 64U) == SVG_XML_STREAM_CANCELED);
    CHECK(external_cancel.count == 0U);
    svg_xml_token_stream_destroy(stream);

    size_t repetitions = 20000U;
    size_t length = 11U + repetitions * 4U;
    uint8_t *large = malloc(length);
    CHECK(large != NULL);
    size_t at = 0U;
    memcpy(large + at, "<svg>", 5U); at += 5U;
    for (size_t i = 0U; i < repetitions; ++i) {
        memcpy(large + at, "<g/>", 4U); at += 4U;
    }
    memcpy(large + at, "</svg>", 6U); at += 6U;
    CHECK(at == length);
    visitor_log total = {NULL, 0U, 0U, 0U};
    CHECK(svg_xml_token_stream_start_visitor((base_bytes){large, at}, collect_token,
        &total, NULL, NULL, &stream) == SVG_XML_STREAM_OK);
    size_t bounded = svg_xml_token_stream_visitor_storage_bytes(stream);
    status = SVG_XML_STREAM_OK;
    while (status == SVG_XML_STREAM_OK)
        status = svg_xml_token_stream_step(stream, 257U);
    CHECK(status == SVG_XML_STREAM_COMPLETE);
    CHECK(total.count == 1U + repetitions + 1U);
    CHECK(svg_xml_token_stream_visitor_storage_bytes(stream) == bounded);
    svg_xml_token_stream_destroy(stream);
    free(large);

    static const uint8_t malformed[] = "<svg><g/></oops>";
    visitor_log prior = {NULL, 0U, 0U, 0U};
    CHECK(svg_xml_token_stream_start_visitor((base_bytes){malformed,
        sizeof(malformed) - 1U}, collect_token, &prior, NULL, NULL,
        &stream) == SVG_XML_STREAM_OK);
    status = SVG_XML_STREAM_OK;
    while (status == SVG_XML_STREAM_OK)
        status = svg_xml_token_stream_step(stream, 4U);
    CHECK(status == SVG_XML_STREAM_FAILED && prior.count >= 2U);
    CHECK(svg_xml_token_stream_result(stream).diagnostic.category ==
          SVG_XML_ERROR_MISMATCHED_TAG);
    svg_xml_token_stream_destroy(stream);

    visitor_log canceled = {NULL, 0U, 0U, 2U};
    CHECK(svg_xml_token_stream_start_visitor((base_bytes){source, sizeof(source) - 1U},
        collect_token, &canceled, NULL, NULL, &stream) == SVG_XML_STREAM_OK);
    status = SVG_XML_STREAM_OK;
    while (status == SVG_XML_STREAM_OK)
        status = svg_xml_token_stream_step(stream, 64U);
    CHECK(status == SVG_XML_STREAM_CANCELED && canceled.count == 2U);
    svg_xml_token_stream_destroy(stream);
}

static void check_validation_and_parsing_share_budget(void)
{
    static const uint8_t source[] = "<svg/>";
    svg_xml_token_stream *stream = NULL;
    CHECK(svg_xml_token_stream_start((base_bytes){source, sizeof(source) - 1U},
        8U * sizeof(svg_xml_token), NULL, NULL, &stream) == SVG_XML_STREAM_OK);
    CHECK(svg_xml_token_stream_step(stream, sizeof(source) - 1U) == SVG_XML_STREAM_OK);
    CHECK(svg_xml_token_stream_result(stream).token_count == 0U);
    CHECK(svg_xml_token_stream_bytes_read(stream) == 0U);
    CHECK(svg_xml_token_stream_step(stream, sizeof(source) - 1U) == SVG_XML_STREAM_COMPLETE);
    svg_xml_token_stream_destroy(stream);
}

static void check_split_utf8_budget(void)
{
    static const uint8_t source[] = "<svg> \xc3\xa9</svg>";
    svg_xml_token reference[4] = {{0}};
    svg_xml_result expected = svg_xml_tokenize((base_bytes){source, sizeof(source) - 1U},
        svg_xml_default_options(), reference, 4U);
    CHECK(expected.diagnostic.category == SVG_XML_ERROR_NONE);
    svg_xml_token_stream *stream = NULL;
    CHECK(svg_xml_token_stream_start((base_bytes){source, sizeof(source) - 1U},
        8U * sizeof(svg_xml_token), NULL, NULL, &stream) == SVG_XML_STREAM_OK);
    svg_xml_stream_status status = SVG_XML_STREAM_OK;
    for (size_t step = 0U; status == SVG_XML_STREAM_OK && step < 40U; ++step) {
        size_t before = svg_xml_token_stream_bytes_read(stream);
        status = svg_xml_token_stream_step(stream, 7U);
        size_t after = svg_xml_token_stream_bytes_read(stream);
        CHECK(after >= before && after - before <= 7U);
    }
    CHECK(status == SVG_XML_STREAM_COMPLETE);
    svg_xml_token *actual = NULL;
    size_t count = 0U;
    CHECK(svg_xml_token_stream_take(stream, &actual, &count));
    CHECK(count == expected.token_count);
    for (size_t i = 0U; i < count; ++i) CHECK(token_equal(&actual[i], &reference[i]));
    free(actual);
    svg_xml_token_stream_destroy(stream);
}

static void check_doctype_stream(void)
{
    static const uint8_t source[] =
        "<?xml version=\"1.0\"?>\n<!DOCTYPE svg PUBLIC \"-//W3C//DTD SVG 1.1//EN\" \"x.dtd\" [\n"
        "<!-- ]> \">\" --><!ENTITY e \"a>b]\"><?pi ]> ?>]>\n<svg><g/></svg>";
    base_bytes input = {source, sizeof(source) - 1U};
    svg_xml_token reference[16] = {{0}};
    svg_xml_result expected = svg_xml_tokenize(input, svg_xml_default_options(), reference, 16U);
    CHECK(expected.diagnostic.category == SVG_XML_ERROR_NONE);
    bool saw_doctype = false;
    for (size_t i = 0U; i < expected.token_count; ++i)
        saw_doctype |= reference[i].kind == SVG_XML_TOKEN_DOCTYPE;
    CHECK(saw_doctype);
    for (size_t budget = 3U; budget <= 9U; ++budget) {
        svg_xml_token_stream *stream = NULL;
        CHECK(svg_xml_token_stream_start(input, 16U * sizeof(svg_xml_token), NULL, NULL,
                                         &stream) == SVG_XML_STREAM_OK);
        svg_xml_stream_status status = SVG_XML_STREAM_OK;
        for (size_t step = 0U; status == SVG_XML_STREAM_OK && step < 4000U; ++step)
            status = svg_xml_token_stream_step(stream, budget);
        CHECK(status == SVG_XML_STREAM_COMPLETE);
        svg_xml_token *actual = NULL;
        size_t count = 0U;
        CHECK(svg_xml_token_stream_take(stream, &actual, &count));
        CHECK(count == expected.token_count);
        for (size_t i = 0U; i < count; ++i) CHECK(token_equal(&actual[i], &reference[i]));
        free(actual);
        svg_xml_token_stream_destroy(stream);
    }
}

int main(void)
{
    check_discard_visitor();
    check_doctype_stream();
    check_validation_and_parsing_share_budget();
    check_split_utf8_budget();
    const size_t run = 128U * 1024U;
    static const char start[] = "<svg a=\"";
    static const char comment_start[] = "<!--";
    static const char comment_end[] = "-->";
    static const char cdata_start[] = "<![CDATA[";
    static const char cdata_end[] = "]]>";
    static const char pi_start[] = "<?p ";
    static const char pi_end[] = "?>";
    static const char end[] = "</svg";
    size_t length = sizeof(start) - 1U + run + 2U + sizeof(comment_start) - 1U + run + sizeof(comment_end) - 1U + run +
        sizeof(cdata_start) - 1U + run + sizeof(cdata_end) - 1U +
        sizeof(pi_start) - 1U + run + sizeof(pi_end) - 1U + sizeof(end) - 1U + run + 1U;
    uint8_t *source = malloc(length);
    svg_xml_token *reference = calloc(length, sizeof(*reference));
    CHECK(source != NULL && reference != NULL);
    size_t at = 0U;
#define APPEND_LITERAL(value) do { size_t n = sizeof(value) - 1U; memcpy(source + at, value, n); at += n; } while (0)
    APPEND_LITERAL(start);
    memset(source + at, 'a', run); at += run; source[at++] = '"'; source[at++] = '>';
    APPEND_LITERAL(comment_start);
    memset(source + at, 'c', run); at += run; APPEND_LITERAL(comment_end);
    memset(source + at, 't', run); at += run; APPEND_LITERAL(cdata_start);
    memset(source + at, 'd', run); at += run; APPEND_LITERAL(cdata_end);
    APPEND_LITERAL(pi_start); memset(source + at, 'p', run); at += run;
    APPEND_LITERAL(pi_end); APPEND_LITERAL(end);
    memset(source + at, ' ', run); at += run; source[at++] = '>';
#undef APPEND_LITERAL
    CHECK(at == length);
    base_bytes input = {source, length};
    svg_xml_result expected = svg_xml_tokenize(input, svg_xml_default_options(), reference, length);
    CHECK(expected.diagnostic.category == SVG_XML_ERROR_NONE);
    svg_xml_token_stream *stream = NULL;
    CHECK(svg_xml_token_stream_start(input, 16U * sizeof(svg_xml_token), NULL, NULL,
                                     &stream) == SVG_XML_STREAM_OK);
    svg_xml_stream_status status = SVG_XML_STREAM_OK;
    size_t steps = 0U;
    while (status == SVG_XML_STREAM_OK && steps < 2000U) {
        size_t before = svg_xml_token_stream_bytes_read(stream);
        status = svg_xml_token_stream_step(stream, 1024U);
        size_t after = svg_xml_token_stream_bytes_read(stream);
        CHECK(after >= before && after - before <= 1024U);
        ++steps;
    }
    CHECK(status == SVG_XML_STREAM_COMPLETE && steps > 2U);
    svg_xml_token *actual = NULL;
    size_t actual_count = 0U;
    CHECK(svg_xml_token_stream_take(stream, &actual, &actual_count));
    CHECK(actual_count == expected.token_count);
    for (size_t i = 0U; i < actual_count; ++i) CHECK(token_equal(&actual[i], &reference[i]));
    free(actual);
    free(reference);
    free(source);
    svg_xml_token_stream_destroy(stream);
    (void)puts("long tags and tokens preserved one-shot spans within 1 KiB steps");
    return EXIT_SUCCESS;
}
