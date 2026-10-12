/* Focused tests for the decode-path source ceiling and token-storage budget
 * (WO-M4-10 / WO-GAP-M4-10). Tests (b) and the failure half of (d) need the
 * test-only budget entry, which exists only where SHAPE_MAX_TOKEN_BYTES does. */
#include "shape.h"
#include "shape_alloc.h"
#include "traverse_internal.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mman.h>

#define SIXTEEN_MIB (16U * 1024U * 1024U)

static int failures;

static void check(bool condition, const char *what)
{
    if (!condition) {
        fprintf(stderr, "FAIL: %s\n", what);
        failures++;
    } else {
        printf("ok: %s\n", what);
    }
}

/* A well-formed document: n rects followed by a comment padded to total_len. */
static char *build_document(size_t rects, size_t total_len, size_t *length)
{
    static const char head[] = "<svg width=\"8\" height=\"8\">";
    static const char rect[] =
        "<rect x=\"1\" y=\"1\" width=\"1\" height=\"1\" fill=\"red\"/>";
    static const char tail[] = "</svg>";
    static const char open[] = "<!--";
    static const char close[] = "-->";
    size_t fixed = sizeof(head) - 1U + rects * (sizeof(rect) - 1U) +
        sizeof(tail) - 1U + sizeof(open) - 1U + sizeof(close) - 1U;
    if (total_len < fixed) total_len = fixed;
    char *buffer = malloc(total_len + 1U);
    if (buffer == NULL) return NULL;
    size_t at = 0U;
    memcpy(buffer + at, head, sizeof(head) - 1U); at += sizeof(head) - 1U;
    for (size_t i = 0U; i < rects; ++i) {
        memcpy(buffer + at, rect, sizeof(rect) - 1U); at += sizeof(rect) - 1U;
    }
    memcpy(buffer + at, open, sizeof(open) - 1U); at += sizeof(open) - 1U;
    size_t pad = total_len - fixed;
    memset(buffer + at, 'x', pad); at += pad;
    memcpy(buffer + at, close, sizeof(close) - 1U); at += sizeof(close) - 1U;
    memcpy(buffer + at, tail, sizeof(tail) - 1U); at += sizeof(tail) - 1U;
    buffer[at] = '\0';
    *length = at;
    return buffer;
}

static void test_over_16_mib_decodes(void)
{
    size_t length = 0U;
    char *source = build_document(1000U, SIXTEEN_MIB + 65536U, &length);
    check(source != NULL && length > SIXTEEN_MIB, "(a) document exceeds 16 MiB");
    if (source == NULL) return;
    svg_shape_document document;
    svg_shape_diagnostic diagnostic = {0};
    svg_shape_error error = svg_shape_decode(
        (base_bytes){(const uint8_t *)source, length}, &document, &diagnostic);
    check(error == SVG_SHAPE_ERROR_NONE, "(a) over-16-MiB document decodes");
    if (error == SVG_SHAPE_ERROR_NONE) {
        check(document.shape_count == 1000U, "(a) all 1000 shapes preserved");
        svg_shape_document_destroy(&document);
    } else {
        fprintf(stderr, "  error=%s offset=%zu\n", svg_shape_error_name(error),
                diagnostic.offset);
    }
    free(source);
}

#ifdef SHAPE_MAX_TOKEN_BYTES
#define TINY_BUDGET 4096U

static void test_tiny_budget_is_limit(void)
{
    size_t length = 0U;
    char *source = build_document(200U, 0U, &length);
    check(source != NULL, "(b) source built");
    if (source == NULL) return;
    svg_shape_document document;
    svg_shape_diagnostic diagnostic = {0};
    svg_shape_error error = svg_shape_decode_source_with_token_budget(
        (base_bytes){(const uint8_t *)source, length}, TINY_BUDGET, &document,
        &diagnostic);
    check(error == SVG_SHAPE_ERROR_LIMIT, "(b) tiny budget fails with LIMIT");
    if (error != SVG_SHAPE_ERROR_LIMIT)
        fprintf(stderr, "  got %s\n", svg_shape_error_name(error));
    check(diagnostic.category == error, "(b) diagnostic category matches");
    check(diagnostic.offset > 0U && diagnostic.offset <= length,
          "(b) offset is nonzero and within the source");
    check(document.shape_count == 0U, "(b) failure leaves the document empty");
    free(source);
}

static void test_ceiling_enforced(void)
{
    size_t length = (size_t)SHAPE_MAX_SOURCE_BYTES + 1U;
    void *map = mmap(NULL, length, PROT_READ, MAP_PRIVATE | MAP_ANON, -1, 0);
    check(map != MAP_FAILED, "(c) anonymous mapping of 1 GiB + 1 created");
    if (map == MAP_FAILED) return;
    svg_shape_document document;
    svg_shape_diagnostic diagnostic = {0};
    svg_shape_error error = svg_shape_decode(
        (base_bytes){(const uint8_t *)map, length}, &document, &diagnostic);
    check(error == SVG_SHAPE_ERROR_LIMIT, "(c) 1 GiB + 1 source is LIMIT");
    munmap(map, length);
}

static void test_accounting_balances(void)
{
    size_t length = 0U;
    char *source = build_document(200U, 0U, &length);
    check(source != NULL, "(d) source built");
    if (source == NULL) return;
    base_bytes bytes = {(const uint8_t *)source, length};
    svg_shape_alloc_stats stats;
    svg_shape_document document;
    svg_shape_diagnostic diagnostic = {0};

    svg_shape_alloc_tracking_start();
    svg_shape_error error = svg_shape_decode(bytes, &document, &diagnostic);
    check(error == SVG_SHAPE_ERROR_NONE, "(d) success path decodes");
    if (error == SVG_SHAPE_ERROR_NONE) svg_shape_document_destroy(&document);
    svg_shape_alloc_tracking_stop(&stats);
    check(stats.allocation_count > 0U && !stats.overflow,
          "(d) hook saw allocations on success path");
    check(stats.live_bytes == 0U, "(d) live bytes zero after destroy (success)");

    svg_shape_alloc_tracking_start();
    error = svg_shape_decode_source_with_token_budget(
        bytes, TINY_BUDGET, &document, &diagnostic);
    check(error == SVG_SHAPE_ERROR_LIMIT, "(d) failure path is LIMIT");
    if (error == SVG_SHAPE_ERROR_NONE) svg_shape_document_destroy(&document);
    svg_shape_alloc_tracking_stop(&stats);
    check(stats.live_bytes == 0U, "(d) live bytes zero after failure");
    free(source);
}
#endif

int main(void)
{
    test_over_16_mib_decodes();
#ifdef SHAPE_MAX_TOKEN_BYTES
    test_tiny_budget_is_limit();
    test_ceiling_enforced();
    test_accounting_balances();
#else
    printf("note: token budget entry absent (pre-change tree); (b)-(d) skipped\n");
#endif
    printf("%s\n", failures == 0 ? "PASS" : "FAILED");
    return failures == 0 ? 0 : 1;
}
