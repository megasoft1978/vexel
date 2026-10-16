#include "shape.h"
#include "shape_render.h"
#include "shape_render_internal.h"
#include "shape_render_preflight.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define CANVAS_W 1056U
#define CANVAS_H 1000U
#define OLD_BUDGET 200000000ULL
/* Measured on 1056x1000 (budget 382,452,224): 1,000 teeth need 365,760,771 work
 * units (over the old 200M, inside the new budget); 1,100 teeth exceed it. */
#define TEETH_INSIDE 1000U
#define TEETH_OVER 1100U

static int failures;

static void check(bool condition, const char *message)
{
    if (condition) return;
    (void)fprintf(stderr, "FAIL: %s\n", message);
    failures++;
}

/* A gradient-filled zigzag whose every edge spans all rows, so the active-edge
 * count, and with it the raster work, grows linearly with `teeth`. */
static char *make_source(unsigned teeth)
{
    size_t capacity = 512U + (size_t)teeth * 24U;
    char *text = malloc(capacity);
    if (text == NULL) return NULL;
    size_t used = (size_t)snprintf(text, capacity,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"%u\" height=\"%u\" "
        "viewBox=\"0 0 %u %u\"><defs><linearGradient id=\"g\">"
        "<stop offset=\"0\" stop-color=\"#f00\"/><stop offset=\"1\" "
        "stop-color=\"#00f\"/></linearGradient></defs>"
        "<path fill=\"url(#g)\" d=\"M0 0", CANVAS_W, CANVAS_H, CANVAS_W, CANVAS_H);
    for (unsigned tooth = 0U; tooth < teeth && used + 24U < capacity; tooth++)
        used += (size_t)snprintf(text + used, capacity - used, " L%.3f %u",
            (double)(tooth + 1U) * (double)CANVAS_W / (double)teeth,
            tooth % 2U == 0U ? CANVAS_H : 0U);
    (void)snprintf(text + used, capacity - used, " Z\"/></svg>");
    return text;
}

static svg_shape_error render(unsigned teeth, uint64_t *work_units,
                                       size_t *offset)
{
    char *source = make_source(teeth);
    svg_shape_document document = {0};
    svg_shape_diagnostic diagnostic = {0};
    if (source == NULL) return SVG_SHAPE_ERROR_OOM;
    svg_shape_error category = svg_shape_decode(
        (base_bytes){(const uint8_t *)source, strlen(source)}, &document,
        &diagnostic);
    free(source);
    if (category != SVG_SHAPE_ERROR_NONE) return category;
    raster_tile tile = {0};
    raster_shape_work_stats stats = {0};
    bool rendered = raster_shape_render_with_stats(&document, &tile,
                                                   &diagnostic, &stats);
    if (work_units != NULL) *work_units = stats.total_work;
    if (offset != NULL) *offset = diagnostic.offset;
    if (rendered) raster_tile_destroy(&tile);
    svg_shape_document_destroy(&document);
    return rendered ? SVG_SHAPE_ERROR_NONE : diagnostic.category;
}

int main(void)
{
    size_t canvas_budget = raster_preflight_work_budget(CANVAS_W, CANVAS_H);
    check(raster_preflight_work_budget(1U, 1U) == OLD_BUDGET,
          "small canvas keeps the 200M budget");
    check(raster_preflight_work_budget(1024U, 1024U) == OLD_BUDGET,
          "1024x1024 keeps the 200M budget");
    check(canvas_budget == 382452224ULL, "1056x1000 gets 200M + 7424 * 24576");
    check(raster_preflight_work_budget(1600U, 1000U) == 13751796224ULL,
          "1600x1000 gets 13,751,796,224");
    check(raster_preflight_work_budget(8192U, 8192U) == 16000000000ULL &&
          raster_preflight_work_budget(UINT32_MAX, UINT32_MAX) ==
          16000000000ULL, "budget is capped at 16e9");

    uint64_t inside_work = 0U, over_work = 0U;
    size_t offset = 1U;
    svg_shape_error inside = render(TEETH_INSIDE, &inside_work, &offset);
    check(inside == SVG_SHAPE_ERROR_NONE, "document inside the new budget renders");
    check(inside_work > OLD_BUDGET && inside_work <= canvas_budget,
          "inside document needs more than the old fixed budget and fits the new one");
    offset = 1U;
    svg_shape_error over = render(TEETH_OVER, &over_work, &offset);
    check(over == SVG_SHAPE_ERROR_LIMIT && offset == 0U,
          "document over the new budget fails with LIMIT at offset 0");
    (void)printf("inside work=%llu budget=%zu over=%d\n",
                 (unsigned long long)inside_work, canvas_budget, (int)over);
    return failures == 0 ? 0 : 1;
}
