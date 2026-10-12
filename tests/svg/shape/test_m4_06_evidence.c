#include "test_m4_06_evidence.h"
#include "shape.h"
#include "shape_render.h"

#include <CommonCrypto/CommonDigest.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define USE_BIG_BYTES (64U * 1024U - 1U)
#define JOB_LIMIT_BYTES (1024U * 1024U)

/* Appends a defs target "<g id=ID><!--a...a--></g>" of exactly total bytes. */
static char *append_target(char *cursor, const char *id, size_t total)
{
    int head = snprintf(cursor, 64U, "<g id=\"%s\"><!--", id);
    if (head < 0 || total < (size_t)head + 3U + 4U) return NULL;
    size_t payload = total - (size_t)head - 3U - 4U;
    memset(cursor + head, 'a', payload);
    cursor += (size_t)head + payload;
    memcpy(cursor, "--></g>", 7U);
    return cursor + 7U;
}

/* 16 uses of a 65,535-byte target plus one use of a tiny target of tiny_bytes. */
static bool decode_job_total(size_t tiny_bytes, const char *tiny_id,
                             svg_shape_error expected)
{
    static const char prefix[] = "<svg width=\"1\" height=\"1\"><defs>";
    static const char suffix[] = "</defs>";
    size_t capacity = sizeof(prefix) + USE_BIG_BYTES + tiny_bytes +
                      sizeof(suffix) + 17U * 24U + 16U;
    char *source = malloc(capacity);
    if (source == NULL) return false;
    char *cursor = source;
    memcpy(cursor, prefix, sizeof(prefix) - 1U);
    cursor += sizeof(prefix) - 1U;
    char *start = cursor;
    cursor = append_target(cursor, "big", USE_BIG_BYTES);
    if (cursor == NULL || (size_t)(cursor - start) != USE_BIG_BYTES) { free(source); return false; }
    start = cursor;
    cursor += (size_t)snprintf(cursor, 64U, "<g id=\"%s\"/>", tiny_id);
    if ((size_t)(cursor - start) != tiny_bytes) { free(source); return false; }
    memcpy(cursor, suffix, sizeof(suffix) - 1U);
    cursor += sizeof(suffix) - 1U;
    for (unsigned i = 0U; i < 16U; ++i) {
        memcpy(cursor, "<use href=\"#big\"/>", 18U);
        cursor += 18U;
    }
    cursor += (size_t)snprintf(cursor, 64U, "<use href=\"#%s\"/></svg>", tiny_id);
    size_t length = (size_t)(cursor - source);
    unsigned char before[CC_SHA256_DIGEST_LENGTH], after[CC_SHA256_DIGEST_LENGTH];
    svg_shape_document document = {0};
    svg_shape_diagnostic diagnostic = {0};
    bool ok = CC_SHA256(source, (CC_LONG)length, before) != NULL &&
              16U * USE_BIG_BYTES + tiny_bytes ==
                  JOB_LIMIT_BYTES + (expected == SVG_SHAPE_ERROR_NONE ? 0U : 1U);
    svg_shape_error error = ok ? svg_shape_decode(
        (base_bytes){(const uint8_t *)source, length}, &document, &diagnostic)
        : SVG_SHAPE_ERROR_XML;
    ok = ok && error == expected &&
         (expected != SVG_SHAPE_ERROR_LIMIT || diagnostic.category == SVG_SHAPE_ERROR_LIMIT) &&
         CC_SHA256(source, (CC_LONG)length, after) != NULL &&
         memcmp(before, after, sizeof(before)) == 0;
    if (!ok) fprintf(stderr, "job reference total tiny=%zu: expected %d got %d\n",
                     tiny_bytes, (int)expected, (int)error);
    svg_shape_document_destroy(&document);
    free(source);
    return ok;
}

/* Long %.17g viewBox values must fit the generated userSpaceOnUse wrapper. */
static bool clip_wrapper_extreme_check(void)
{
    static const char source[] =
        "<svg width=\"4096\" height=\"4096\" "
        "viewBox=\"-1234.1234567891 -1234.1234567891 "
        "4096.1234567891 4096.1234567891\">"
        "<defs><clipPath id=\"c\" clipPathUnits=\"userSpaceOnUse\"><rect width=\"1\" height=\"1\"/></clipPath></defs>"
        "<rect width=\"1\" height=\"1\" clip-path=\"url(#c)\"/></svg>";
    unsigned char before[CC_SHA256_DIGEST_LENGTH], after[CC_SHA256_DIGEST_LENGTH];
    svg_shape_document document = {0};
    svg_shape_diagnostic diagnostic = {0};
    if (CC_SHA256(source, (CC_LONG)(sizeof(source) - 1U), before) == NULL) return false;
    svg_shape_error error = svg_shape_decode(
        (base_bytes){(const uint8_t *)source, sizeof(source) - 1U}, &document, &diagnostic);
    bool ok = error == SVG_SHAPE_ERROR_NONE &&
              CC_SHA256(source, (CC_LONG)(sizeof(source) - 1U), after) != NULL &&
              memcmp(before, after, sizeof(before)) == 0;
    if (!ok) fprintf(stderr, "clip wrapper extreme: error %d\n", (int)error);
    svg_shape_document_destroy(&document);
    return ok;
}

static bool render_rgba(const char *source, raster_tile *tile, svg_shape_document *doc)
{
    svg_shape_diagnostic diagnostic = {0};
    if (svg_shape_decode((base_bytes){(const uint8_t *)source, strlen(source)}, doc,
                         &diagnostic) != SVG_SHAPE_ERROR_NONE) return false;
    if (!raster_shape_render(doc, tile, &diagnostic)) {
        svg_shape_document_destroy(doc);
        return false;
    }
    return true;
}

static bool pixel_is(const raster_tile *tile, uint32_t x, uint32_t y,
                     unsigned r, unsigned g, unsigned b, unsigned a)
{
    const uint8_t *p = tile->rgba + ((size_t)y * tile->width + x) * 4U;
    bool ok = p[0] == r && p[1] == g && p[2] == b && p[3] == a;
    if (!ok) fprintf(stderr, "pixel (%u,%u): expected %u,%u,%u,%u got %u,%u,%u,%u\n",
                     x, y, r, g, b, a, p[0], p[1], p[2], p[3]);
    return ok;
}

/* Pixel-center t=(x+0.5)/16, channel = round(255*(1-t)) / round(255*t). */
static bool exact_gradient_check(void)
{
    static const char linear[] =
        "<svg width=\"16\" height=\"16\"><defs><linearGradient id=\"g\">"
        "<stop offset=\"0\" stop-color=\"#ff0000\"/>"
        "<stop offset=\"1\" stop-color=\"#0000ff\"/></linearGradient></defs>"
        "<rect width=\"16\" height=\"16\" fill=\"url(#g)\"/></svg>";
    raster_tile tile = {0};
    svg_shape_document doc = {0};
    if (!render_rgba(linear, &tile, &doc)) return false;
    /* x=0: t=1/32 -> 247.03/7.97; x=8: t=17/32 -> 119.53/135.47; x=15: t=31/32. */
    bool ok = pixel_is(&tile, 0U, 8U, 247U, 0U, 8U, 255U) &&
              pixel_is(&tile, 8U, 8U, 120U, 0U, 135U, 255U) &&
              pixel_is(&tile, 15U, 8U, 8U, 0U, 247U, 255U);
    raster_tile_destroy(&tile);
    svg_shape_document_destroy(&doc);
    if (!ok) return false;
    /* Blue stop-opacity 0 -> 1 over opaque red; x=8: a=17/32 -> R 119.53, B 135.47. */
    static const char alpha[] =
        "<svg width=\"16\" height=\"16\"><defs><linearGradient id=\"g\">"
        "<stop offset=\"0\" stop-color=\"#0000ff\" stop-opacity=\"0\"/>"
        "<stop offset=\"1\" stop-color=\"#0000ff\" stop-opacity=\"1\"/></linearGradient></defs>"
        "<rect width=\"16\" height=\"16\" fill=\"#ff0000\"/>"
        "<rect width=\"16\" height=\"16\" fill=\"url(#g)\"/></svg>";
    if (!render_rgba(alpha, &tile, &doc)) return false;
    ok = pixel_is(&tile, 8U, 8U, 120U, 0U, 135U, 255U);
    raster_tile_destroy(&tile);
    svg_shape_document_destroy(&doc);
    return ok;
}

bool svg_shape_m4_06_evidence_tests(void)
{
    if (!decode_job_total(16U, "tiny12", SVG_SHAPE_ERROR_NONE)) return false;
    if (!decode_job_total(17U, "tiny123", SVG_SHAPE_ERROR_LIMIT)) return false;
    if (!clip_wrapper_extreme_check()) return false;
    return exact_gradient_check();
}
