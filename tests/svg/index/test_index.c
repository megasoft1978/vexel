#define _POSIX_C_SOURCE 200809L

#include "index.h"
#include "index_progress.h"

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <time.h>

static int fail(const char *message)
{
    (void)fprintf(stderr, "svg index: %s\n", message);
    return 1;
}

static uint64_t wall_time_ns(void)
{
    struct timespec value = {0};
    (void)timespec_get(&value, TIME_UTC);
    return (uint64_t)value.tv_sec * 1000000000U + (uint64_t)value.tv_nsec;
}

static bool cancel_at_percent(const uint8_t *source, size_t length, unsigned target)
{
    svg_index_session *session = NULL;
    if (svg_index_start((base_bytes){source, length}, target, &session) != SVG_INDEX_OK) return false;
    svg_index_progress p = svg_index_get_progress(session);
    ui_index_progress view = {0};
    while (p.bytes_read < length && p.bytes_read * 100U / length < target) {
        uint64_t started = wall_time_ns();
        if (svg_index_step(session, SVG_INDEX_MAX_STEP_BYTES) != SVG_INDEX_OK) {
            svg_index_destroy(session);
            return false;
        }
        uint64_t elapsed = wall_time_ns() - started;
        if (elapsed >= 100000000U) { svg_index_destroy(session); return false; }
        p = svg_index_get_progress(session);
        if (!ui_index_progress_update(&view, &p)) {
            svg_index_destroy(session);
            return false;
        }
    }
    bool reached = p.bytes_read * 100U / length >= target && !p.complete;
    svg_index_cancel(session);
    reached = reached && svg_index_step(session, SVG_INDEX_MAX_STEP_BYTES) == SVG_INDEX_CANCELED;
    p = svg_index_get_progress(session);
    reached = reached && p.canceled;
    uint64_t destroy_start = wall_time_ns();
    svg_index_destroy(session);
    reached = reached && wall_time_ns() - destroy_start < 100000000U;
    return reached;
}

static bool same_token(svg_xml_token a, svg_xml_token b)
{
    return a.kind == b.kind && a.depth == b.depth &&
           a.source.offset == b.source.offset && a.source.length == b.source.length &&
           a.name.offset == b.name.offset && a.name.length == b.name.length &&
           a.value.offset == b.value.offset && a.value.length == b.value.length &&
           a.prefix.offset == b.prefix.offset && a.prefix.length == b.prefix.length &&
           a.local_name.offset == b.local_name.offset && a.local_name.length == b.local_name.length;
}

static bool equivalent_file(const char *path)
{
    FILE *file = fopen(path, "rb");
    if (file == NULL || fseek(file, 0L, SEEK_END) != 0) {
        if (file != NULL) (void)fclose(file);
        return false;
    }
    long end = ftell(file);
    if (end <= 0L || (unsigned long)end > 16UL * 1024UL * 1024UL ||
        fseek(file, 0L, SEEK_SET) != 0) {
        (void)fclose(file);
        return false;
    }
    size_t length = (size_t)end;
    uint8_t *source = malloc(length);
    svg_xml_token *reference = calloc(length, sizeof(*reference));
    if (source == NULL || reference == NULL || fread(source, 1U, length, file) != length) {
        free(source); free(reference); (void)fclose(file); return false;
    }
    (void)fclose(file);
    svg_xml_result expected = svg_xml_tokenize((base_bytes){source, length},
        svg_xml_default_options(), reference, length);
    svg_xml_token_stream *stream = NULL;
    bool okay = expected.diagnostic.category == SVG_XML_ERROR_NONE &&
        svg_xml_token_stream_start((base_bytes){source, length},
            SVG_INDEX_MAX_STORAGE_BYTES / 2U, NULL, NULL, &stream) == SVG_XML_STREAM_OK;
    svg_xml_stream_status status = SVG_XML_STREAM_OK;
    size_t steps = 0U;
    bool emitted_before_complete = false;
    while (okay && status == SVG_XML_STREAM_OK && steps < 10000U) {
        size_t before = svg_xml_token_stream_bytes_read(stream);
        status = svg_xml_token_stream_step(stream, 7U);
        size_t after = svg_xml_token_stream_bytes_read(stream);
        okay = after >= before && after - before <= 7U;
        ++steps;
        svg_xml_result progress = svg_xml_token_stream_result(stream);
        emitted_before_complete = emitted_before_complete ||
            (progress.token_count > 0U && status == SVG_XML_STREAM_OK);
    }
    svg_xml_result incremental = svg_xml_token_stream_result(stream);
    svg_xml_token *actual_tokens = NULL;
    size_t actual_count = 0U;
    okay = okay && status == SVG_XML_STREAM_COMPLETE && steps > 1U && emitted_before_complete &&
        incremental.token_count == expected.token_count &&
        svg_xml_token_stream_take(stream, &actual_tokens, &actual_count) &&
        actual_count == expected.token_count;
    for (size_t i = 0U; okay && i < expected.token_count; ++i) {
        okay = same_token(actual_tokens[i], reference[i]);
    }
    svg_xml_token_stream_destroy(stream);
    free(actual_tokens);
    free(reference); free(source);
    return okay;
}

static bool equivalent_diagnostic(const uint8_t *source, size_t length,
                                  svg_xml_diagnostic *expected_out,
                                  svg_xml_diagnostic *actual_out)
{
    svg_xml_token *tokens = calloc(length, sizeof(*tokens));
    if (tokens == NULL) return false;
    svg_xml_result expected = svg_xml_tokenize((base_bytes){source, length},
        svg_xml_default_options(), tokens, length);
    free(tokens);
    svg_index_session *session = NULL;
    if (svg_index_start((base_bytes){source, length}, 1U, &session) != SVG_INDEX_OK) return false;
    svg_index_status status = SVG_INDEX_OK;
    while (status == SVG_INDEX_OK) status = svg_index_step(session, 3U);
    svg_xml_diagnostic actual = svg_index_get_progress(session).diagnostic;
    if (expected_out != NULL) *expected_out = expected.diagnostic;
    if (actual_out != NULL) *actual_out = actual;
    bool okay = status == SVG_INDEX_MALFORMED &&
        actual.category == expected.diagnostic.category &&
        actual.offset == expected.diagnostic.offset;
    svg_index_destroy(session);
    return okay;
}

static bool check_diagnostic_equivalence(void)
{
    static const char *const malformed[] = {
        "<svg><rect/></bad>",
        "<svg><rect width=1/></svg>",
        "<svg><!-- -- --></svg>",
        "<!DOCTYPE svg><svg/>",
        "<svg><rect x='1' x='2'/></svg>",
        "<svg><rect"
    };
    (void)printf("{\"status\":\"passed\",\"diagnostic_exact_offsets\":[");
    for (size_t i = 0U; i < sizeof(malformed) / sizeof(malformed[0]); ++i) {
        size_t length = strlen(malformed[i]);
        svg_xml_diagnostic expected = {0}, actual = {0};
        if (!equivalent_diagnostic((const uint8_t *)malformed[i], length,
                                   &expected, &actual)) return false;
        (void)printf("%s{\"case\":%zu,\"category\":\"%s\",\"offset\":%zu}",
                     i == 0U ? "" : ",", i, svg_xml_error_name(expected.category), expected.offset);
    }
    (void)puts("]}");
    return true;
}

static bool check_frozen_fixture_equivalence(int argc, char **argv)
{
    if (argc < 3 || strcmp(argv[1], "--equivalence") != 0) return false;
    for (int i = 2; i < argc; ++i) {
        if (!equivalent_file(argv[i])) {
            (void)fprintf(stderr, "token equivalence failed: %s\n", argv[i]);
            return false;
        }
    }
    (void)printf("{\"status\":\"passed\",\"fixture_count\":%d,\"comparison\":\"resumable tokenizer stream vs one-shot token kinds and spans\"}\n",
                 argc - 2);
    return true;
}

int main(int argc, char **argv)
{
    if (argc > 1) return check_frozen_fixture_equivalence(argc, argv) ? 0 : 1;
    if (!check_diagnostic_equivalence()) return fail("incremental diagnostic category or offset differs from one-shot tokenizer");
    static const uint8_t source[] =
        "<svg width='2' height='2'><rect width='2' height='2'/><custom/><path d='M0 0'/></svg>";
    svg_index_session *session = NULL;
    if (svg_index_start((base_bytes){source, sizeof(source) - 1U}, 7U, &session) != SVG_INDEX_OK)
        return fail("start failed");
    svg_index_progress p = svg_index_get_progress(session);
    if (p.bytes_read != 0 || p.generation != 7U) return fail("initial progress mismatch");
    svg_index_status step = SVG_INDEX_OK;
    while (step == SVG_INDEX_OK) step = svg_index_step(session, 1U);
    p = svg_index_get_progress(session);
    if (step != SVG_INDEX_COMPLETE || !p.complete || !p.first_preview_ready)
        return fail("session did not complete with preview");
    if (p.element_count != 4U || p.attribute_count != 5U) return fail("count mismatch");
    svg_xml_token *reference = calloc(sizeof(source), sizeof(*reference));
    if (reference == NULL) return fail("reference token allocation failed");
    svg_xml_result one_shot = svg_xml_tokenize((base_bytes){source, sizeof(source) - 1U},
        svg_xml_default_options(), reference, sizeof(source));
    bool equivalent = one_shot.diagnostic.category == SVG_XML_ERROR_NONE &&
                      one_shot.token_count == p.token_count;
    for (size_t i = 0U; equivalent && i < p.token_count; ++i) {
        svg_xml_token token;
        equivalent = svg_index_token_at(session, i, &token) && same_token(token, reference[i]);
    }
    free(reference);
    if (!equivalent) return fail("one-shot token stream mismatch");
    ui_index_progress view = {0};
    if (!ui_index_progress_update(&view, &p) || view.percent != 100U ||
        view.preview_visible || view.loading ||
        !ui_index_progress_publish_preview(&view, 7U) || !view.preview_visible ||
        ui_index_progress_publish_preview(&view, 6U))
        return fail("progress UI tile-publication state mismatch");
    svg_index_progress stale = p;
    stale.generation = 6U;
    if (ui_index_progress_update(&view, &stale)) return fail("stale progress was accepted");
    svg_index_element e;
    size_t rect_offset = sizeof("<svg width='2' height='2'>") - 1U;
    size_t rect_length = sizeof("<rect width='2' height='2'/>") - 1U;
    if (!svg_index_element_at(session, 1U, &e) || e.support != SVG_INDEX_SUPPORTED_RECT ||
        e.depth != 1U || e.parent != 0U || !e.closed || e.source_offset != rect_offset ||
        e.source_length != rect_length ||
        memcmp(source + e.source_offset, "<rect width='2' height='2'/>", rect_length) != 0)
        return fail("rect record mismatch");
    if (!svg_index_element_at(session, 2U, &e) || e.support != SVG_INDEX_UNSUPPORTED)
        return fail("unsupported record missing");
    base_mut_bytes preview_source = {0};
    static const char expected_preview[] =
        "<svg width='2' height='2'><rect width='2' height='2'/></svg>";
    if (!svg_index_make_preview_source(session, &preview_source) ||
        preview_source.len != sizeof(expected_preview) - 1U || memcmp(preview_source.ptr,
        expected_preview, sizeof(expected_preview) - 1U) != 0) return fail("preview source mismatch");
    free(preview_source.ptr);
    if (!svg_index_preview_generation_is_current(session, 7U))
        return fail("current preview generation rejected");
    if (svg_index_advance_generation(session, 8U) != SVG_INDEX_OK ||
        svg_index_get_progress(session).first_preview_ready ||
        svg_index_preview_generation_is_current(session, 7U))
        return fail("generation invalidation failed");
    svg_index_destroy(session);

    static const uint8_t container_spans[] =
        "<svg width='2' height='2'><g><rect width='1' height='1'/></g></svg>";
    if (svg_index_start((base_bytes){container_spans, sizeof(container_spans) - 1U},
                        1U, &session) != SVG_INDEX_OK)
        return fail("container span session start failed");
    step = SVG_INDEX_OK;
    while (step == SVG_INDEX_OK) step = svg_index_step(session, SVG_INDEX_MAX_STEP_BYTES);
    svg_index_element root_record = {0}, group_record = {0}, child_record = {0};
    size_t group_offset = sizeof("<svg width='2' height='2'>") - 1U;
    size_t child_length = sizeof("<rect width='1' height='1'/>") - 1U;
    bool spans_retained = step == SVG_INDEX_COMPLETE &&
        svg_index_element_at(session, 0U, &root_record) &&
        root_record.parent == SIZE_MAX && root_record.source_offset == 0U &&
        root_record.source_length == sizeof(container_spans) - 1U && root_record.closed &&
        svg_index_element_at(session, 1U, &group_record) && group_record.parent == 0U &&
        group_record.source_offset == group_offset &&
        group_record.source_length == sizeof("<g><rect width='1' height='1'/></g>") - 1U &&
        group_record.closed && svg_index_element_at(session, 2U, &child_record) &&
        child_record.parent == 1U && child_record.source_offset == group_offset + sizeof("<g>") - 1U &&
        child_record.source_length == child_length && child_record.closed;
    svg_index_destroy(session);
    if (!spans_retained) return fail("root or non-empty parent source span was not retained");

    static const uint8_t malformed_prefix[] =
        "<svg width='2' height='2'>bad & raw<rect width='2' height='2'/></svg>";
    if (svg_index_start((base_bytes){malformed_prefix, sizeof(malformed_prefix) - 1U},
                        2U, &session) != SVG_INDEX_OK)
        return fail("malformed-prefix session start failed");
    step = SVG_INDEX_OK;
    bool malformed_prefix_previewed = false;
    while (step == SVG_INDEX_OK) {
        step = svg_index_step(session, 1U);
        p = svg_index_get_progress(session);
        malformed_prefix_previewed = malformed_prefix_previewed || p.first_preview_ready ||
            svg_index_preview_generation_is_current(session, 2U);
    }
    bool malformed_prefix_rejected = step == SVG_INDEX_MALFORMED &&
        !malformed_prefix_previewed &&
        svg_index_get_progress(session).diagnostic.category == SVG_XML_ERROR_MALFORMED_SYNTAX &&
        svg_index_get_progress(session).diagnostic.offset ==
            (size_t)(strstr((const char *)malformed_prefix, "&") -
                     (const char *)malformed_prefix);
    svg_index_destroy(session);
    if (!malformed_prefix_rejected)
        return fail("preview was published before malformed prefix validation");

    static const uint8_t rich[] = "<?xml version='1.0' encoding=\"UTF-8\"?>\r\n<!--c--><?p data?><a:root xmlns:a='urn:x' a:k=\"v&amp;x\">x\r\ny<![CDATA[z]]><a:child/></a:root>";
    if (svg_index_start((base_bytes){rich, sizeof(rich) - 1U}, 3U, &session) != SVG_INDEX_OK)
        return fail("rich XML session start failed");
    step = SVG_INDEX_OK;
    while (step == SVG_INDEX_OK) step = svg_index_step(session, 3U);
    p = svg_index_get_progress(session);
    size_t rich_capacity = sizeof(rich);
    svg_xml_token *rich_reference = calloc(rich_capacity, sizeof(*rich_reference));
    if (rich_reference == NULL) { svg_index_destroy(session); return fail("rich token allocation failed"); }
    one_shot = svg_xml_tokenize((base_bytes){rich, sizeof(rich) - 1U},
        svg_xml_default_options(), rich_reference, rich_capacity);
    equivalent = step == SVG_INDEX_COMPLETE && one_shot.diagnostic.category == SVG_XML_ERROR_NONE &&
                 p.token_count == one_shot.token_count;
    for (size_t i = 0U; equivalent && i < p.token_count; ++i) {
        svg_xml_token token;
        equivalent = svg_index_token_at(session, i, &token) && same_token(token, rich_reference[i]);
    }
    free(rich_reference);
    svg_index_destroy(session);
    if (!equivalent) return fail("chunk-boundary XML token stream mismatch");

    static const uint8_t first_preview[] =
        "<svg width='2' height='2'><rect width='1' height='1'></rect>"
        "<path d='M0 0'></path></svg>";
    if (svg_index_start((base_bytes){first_preview, sizeof(first_preview) - 1U}, 1U, &session) != SVG_INDEX_OK)
        return fail("first preview session start failed");
    step = SVG_INDEX_OK;
    while (step == SVG_INDEX_OK) step = svg_index_step(session, SVG_INDEX_MAX_STEP_BYTES);
    base_mut_bytes first_preview_source = {0};
    static const char expected_first_preview[] =
        "<svg width='2' height='2'><rect width='1' height='1'></rect></svg>";
    bool preserved_first = step == SVG_INDEX_COMPLETE &&
        svg_index_make_preview_source(session, &first_preview_source) &&
        first_preview_source.len == sizeof(expected_first_preview) - 1U &&
        memcmp(first_preview_source.ptr, expected_first_preview,
               sizeof(expected_first_preview) - 1U) == 0;
    free(first_preview_source.ptr);
    svg_index_destroy(session);
    if (!preserved_first) return fail("later shape replaced the first supported preview");

    static const uint8_t malformed[] =
        "<svg width='2' height='2'><rect width='1' height='1'/></broken>";
    if (svg_index_start((base_bytes){malformed, sizeof(malformed) - 1U}, 1U, &session) != SVG_INDEX_OK)
        return fail("malformed session start failed");
    step = SVG_INDEX_OK;
    while (step == SVG_INDEX_OK) step = svg_index_step(session, SVG_INDEX_MAX_STEP_BYTES);
    p = svg_index_get_progress(session);
    bool bad_tail = step == SVG_INDEX_MALFORMED && !p.complete && p.first_preview_ready &&
                    p.diagnostic.category == SVG_XML_ERROR_MISMATCHED_TAG &&
                    p.diagnostic.offset == (size_t)(strstr((const char *)malformed,
                        "</broken>") - (const char *)malformed);
    svg_xml_token *malformed_tokens = calloc(sizeof(malformed), sizeof(*malformed_tokens));
    if (malformed_tokens == NULL) { svg_index_destroy(session); return fail("diagnostic reference allocation failed"); }
    svg_xml_result malformed_reference = svg_xml_tokenize((base_bytes){malformed, sizeof(malformed) - 1U},
        svg_xml_default_options(), malformed_tokens, sizeof(malformed));
    free(malformed_tokens);
    bad_tail = bad_tail && malformed_reference.diagnostic.category == p.diagnostic.category &&
               malformed_reference.diagnostic.offset == p.diagnostic.offset;
    svg_index_destroy(session);
    if (!bad_tail) return fail("malformed tail was not diagnosed after preview");

    size_t long_length = 1024U * 1024U;
    uint8_t *long_source = malloc(long_length);
    if (long_source == NULL) return fail("progress fixture allocation failed");
    static const char prefix[] = "<svg><rect width='1' height='1'/>";
    static const char suffix[] = "</svg>";
    memcpy(long_source, prefix, sizeof(prefix) - 1U);
    memset(long_source + sizeof(prefix) - 1U, ' ',
           long_length - (sizeof(prefix) - 1U) - (sizeof(suffix) - 1U));
    memcpy(long_source + long_length - (sizeof(suffix) - 1U), suffix, sizeof(suffix) - 1U);
    bool canceled = cancel_at_percent(long_source, long_length, 10U) &&
                    cancel_at_percent(long_source, long_length, 50U) &&
                    cancel_at_percent(long_source, long_length, 90U);
    free(long_source);
    if (!canceled) return fail("10/50/90 cancellation failed");

    size_t large_tag_length = 128U * 1024U;
    uint8_t *large_tag_source = malloc(large_tag_length);
    if (large_tag_source == NULL) return fail("large attribute fixture allocation failed");
    static const char large_tag_prefix[] = "<svg width='";
    static const char large_tag_suffix[] = "' height='1'><rect width='1' height='1'/></svg>";
    memcpy(large_tag_source, large_tag_prefix, sizeof(large_tag_prefix) - 1U);
    memset(large_tag_source + sizeof(large_tag_prefix) - 1U, '1',
        large_tag_length - (sizeof(large_tag_prefix) - 1U) - (sizeof(large_tag_suffix) - 1U));
    memcpy(large_tag_source + large_tag_length - (sizeof(large_tag_suffix) - 1U),
        large_tag_suffix, sizeof(large_tag_suffix) - 1U);
    if (svg_index_start((base_bytes){large_tag_source, large_tag_length}, 4U,
                        &session) != SVG_INDEX_OK) {
        free(large_tag_source);
        return fail("large attribute session start failed");
    }
    step = SVG_INDEX_OK;
    uint64_t max_large_tag_step = 0U;
    while (step == SVG_INDEX_OK) {
        uint64_t started = wall_time_ns();
        step = svg_index_step(session, SVG_INDEX_MAX_STEP_BYTES);
        uint64_t elapsed = wall_time_ns() - started;
        if (elapsed > max_large_tag_step) max_large_tag_step = elapsed;
    }
    p = svg_index_get_progress(session);
    bool large_tag_indexed = step == SVG_INDEX_COMPLETE && p.complete &&
        !p.first_preview_ready && max_large_tag_step < 100000000U;
    svg_index_destroy(session);
    free(large_tag_source);
    if (!large_tag_indexed)
        return fail("large start tag was not bounded and retained as preview-ineligible");

    size_t validation_length = 1024U * 1024U;
    uint8_t *validation_source = malloc(validation_length);
    if (validation_source == NULL) return fail("validation cleanup fixture allocation failed");
    static const char validation_prefix[] = "<svg/>";
    memset(validation_source, ' ', validation_length);
    memcpy(validation_source, validation_prefix, sizeof(validation_prefix) - 1U);
    if (svg_index_start((base_bytes){validation_source, validation_length}, 1U, &session) != SVG_INDEX_OK) {
        free(validation_source);
        return fail("validation cleanup session start failed");
    }
    const struct timespec settle = {0, 100000000L};
    (void)nanosleep(&settle, NULL);
    svg_index_destroy(session);
    free(validation_source);

    static const uint8_t bad_large[] = "<svg width=1 height=1><rect/></svg>";
    size_t bad_length = 70U * 1024U;
    uint8_t *bad_source = malloc(bad_length);
    if (bad_source == NULL) return fail("large diagnostic fixture allocation failed");
    memcpy(bad_source, bad_large, sizeof(bad_large) - 1U);
    memset(bad_source + sizeof(bad_large) - 1U, ' ',
           bad_length - (sizeof(bad_large) - 1U));
    if (svg_index_start((base_bytes){bad_source, bad_length}, 1U, &session) != SVG_INDEX_OK) {
        free(bad_source);
        return fail("large malformed session start failed");
    }
    step = SVG_INDEX_OK;
    while (step == SVG_INDEX_OK) step = svg_index_step(session, SVG_INDEX_MAX_STEP_BYTES);
    p = svg_index_get_progress(session);
    bool bad_attribute = step == SVG_INDEX_MALFORMED &&
        p.diagnostic.category == SVG_XML_ERROR_MALFORMED_SYNTAX && !p.complete;
    svg_xml_token *bad_reference_tokens = calloc(bad_length, sizeof(*bad_reference_tokens));
    if (bad_reference_tokens == NULL) {
        svg_index_destroy(session); free(bad_source);
        return fail("large diagnostic reference allocation failed");
    }
    svg_xml_result bad_reference = svg_xml_tokenize((base_bytes){bad_source, bad_length},
        svg_xml_default_options(), bad_reference_tokens, bad_length);
    free(bad_reference_tokens);
    bad_attribute = bad_attribute && bad_reference.diagnostic.category == p.diagnostic.category &&
                    bad_reference.diagnostic.offset == p.diagnostic.offset;
    svg_index_destroy(session);
    free(bad_source);
    if (!bad_attribute) return fail("large malformed source was accepted");

    static const uint8_t transformed_first[] =
        "<svg width='2' height='2'><rect width='1' height='1' transform='translate(1)'/>"
        "<rect width='1' height='1'/></svg>";
    if (svg_index_start((base_bytes){transformed_first, sizeof(transformed_first) - 1U},
                        1U, &session) != SVG_INDEX_OK)
        return fail("unsupported-feature preview session start failed");
    step = SVG_INDEX_OK;
    while (step == SVG_INDEX_OK) step = svg_index_step(session, SVG_INDEX_MAX_STEP_BYTES);
    svg_index_element unsupported_shape = {0}, supported_shape = {0};
    base_mut_bytes supported_preview = {0};
    static const char expected_supported_preview[] =
        "<svg width='2' height='2'><rect width='1' height='1'/></svg>";
    bool fallback = step == SVG_INDEX_COMPLETE &&
        svg_index_element_at(session, 1U, &unsupported_shape) &&
        unsupported_shape.support == SVG_INDEX_UNSUPPORTED &&
        svg_index_element_at(session, 2U, &supported_shape) &&
        supported_shape.support == SVG_INDEX_SUPPORTED_RECT &&
        svg_index_make_preview_source(session, &supported_preview) &&
        supported_preview.len == sizeof(expected_supported_preview) - 1U &&
        memcmp(supported_preview.ptr, expected_supported_preview,
               sizeof(expected_supported_preview) - 1U) == 0;
    free(supported_preview.ptr);
    svg_index_destroy(session);
    if (!fallback) return fail("unsupported transform blocked the next renderable shape");

    static const uint8_t non_svg[] = "<g><rect width='2'/></g>";
    if (svg_index_start((base_bytes){non_svg, sizeof(non_svg) - 1U}, 1U, &session) != SVG_INDEX_OK)
        return fail("non-SVG session start failed");
    step = SVG_INDEX_OK;
    while (step == SVG_INDEX_OK) step = svg_index_step(session, SVG_INDEX_MAX_STEP_BYTES);
    p = svg_index_get_progress(session);
    base_mut_bytes non_svg_preview = {0};
    bool non_svg_rejected = step == SVG_INDEX_COMPLETE && !p.first_preview_ready &&
        !svg_index_make_preview_source(session, &non_svg_preview);
    free(non_svg_preview.ptr);
    svg_index_destroy(session);
    if (!non_svg_rejected) return fail("non-SVG root advertised a preview");

    session = NULL;
    if (svg_index_start((base_bytes){NULL, SVG_INDEX_MAX_SOURCE_BYTES + 1U}, 1U, &session) != SVG_INDEX_LIMIT ||
        session != NULL) return fail("oversized source was not rejected before dereference");
    if (svg_index_start((base_bytes){source, sizeof(source) - 1U}, 1U, &session) != SVG_INDEX_OK)
        return fail("cancel session start failed");
    svg_index_cancel(session);
    if (svg_index_step(session, 64U) != SVG_INDEX_CANCELED) return fail("cancel failed");
    svg_index_destroy(session);
    (void)printf("{\"status\":\"passed\",\"checks\":[\"progressive first supported element\",\"preview precedes completion\",\"10 percent cancel\",\"50 percent cancel\",\"90 percent cancel\",\"stale generation\",\"unsupported element retained\",\"oversized source rejected before dereference\"],\"cancel_percentages\":[10,50,90]}\n");
    return 0;
}
