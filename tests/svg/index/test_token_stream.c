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

int main(void)
{
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
