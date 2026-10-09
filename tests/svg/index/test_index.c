#include "index.h"
#include "index_progress.h"

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static int fail(const char *message)
{
    (void)fprintf(stderr, "svg index: %s\n", message);
    return 1;
}

static bool cancel_at_percent(const uint8_t *source, size_t length, unsigned target)
{
    svg_index_session *session = NULL;
    if (svg_index_start((base_bytes){source, length}, target, &session) != SVG_INDEX_OK) return false;
    svg_index_progress p = svg_index_get_progress(session);
    while (p.bytes_read < length && p.bytes_read * 100U / length < target) {
        if (svg_index_step(session, SVG_INDEX_MAX_STEP_BYTES) != SVG_INDEX_OK) {
            svg_index_destroy(session);
            return false;
        }
        p = svg_index_get_progress(session);
    }
    bool reached = p.bytes_read * 100U / length >= target && !p.complete;
    svg_index_cancel(session);
    reached = reached && svg_index_step(session, SVG_INDEX_MAX_STEP_BYTES) == SVG_INDEX_CANCELED;
    p = svg_index_get_progress(session);
    reached = reached && p.canceled;
    svg_index_destroy(session);
    return reached;
}

int main(void)
{
    static const uint8_t source[] = "<svg><rect width='2'/><custom/><path d='M0 0'/></svg>";
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
    if (p.element_count != 4U || p.attribute_count != 2U) return fail("count mismatch");
    ui_index_progress view = {0};
    if (!ui_index_progress_update(&view, &p) || view.percent != 100U ||
        !view.preview_visible || view.loading) return fail("progress UI state mismatch");
    svg_index_progress stale = p;
    stale.generation = 6U;
    if (ui_index_progress_update(&view, &stale)) return fail("stale progress was accepted");
    svg_index_element e;
    if (!svg_index_element_at(session, 1U, &e) || e.support != SVG_INDEX_SUPPORTED_RECT ||
        e.depth != 1U || e.parent != 0U || !e.closed) return fail("rect record mismatch");
    if (!svg_index_element_at(session, 2U, &e) || e.support != SVG_INDEX_UNSUPPORTED)
        return fail("unsupported record missing");
    base_mut_bytes preview_source = {0};
    if (!svg_index_make_preview_source(session, &preview_source) ||
        preview_source.len != 28U || memcmp(preview_source.ptr,
        "<svg><rect width='2'/></svg>", 28U) != 0) return fail("preview source mismatch");
    free(preview_source.ptr);
    if (svg_index_advance_generation(session, 8U) != SVG_INDEX_OK ||
        svg_index_get_progress(session).first_preview_ready) return fail("generation invalidation failed");
    svg_index_destroy(session);

    static const uint8_t malformed[] = "<svg><rect/></broken>";
    if (svg_index_start((base_bytes){malformed, sizeof(malformed) - 1U}, 1U, &session) != SVG_INDEX_OK)
        return fail("malformed session start failed");
    step = SVG_INDEX_OK;
    while (step == SVG_INDEX_OK) step = svg_index_step(session, SVG_INDEX_MAX_STEP_BYTES);
    p = svg_index_get_progress(session);
    bool bad_tail = step == SVG_INDEX_MALFORMED && !p.complete && p.first_preview_ready &&
                    p.diagnostic.category == SVG_XML_ERROR_MISMATCHED_TAG;
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

    session = NULL;
    if (svg_index_start((base_bytes){NULL, SVG_INDEX_MAX_SOURCE_BYTES + 1U}, 1U, &session) != SVG_INDEX_LIMIT ||
        session != NULL) return fail("oversized source was not rejected before dereference");
    if (svg_index_start((base_bytes){source, sizeof(source) - 1U}, 1U, &session) != SVG_INDEX_OK)
        return fail("cancel session start failed");
    svg_index_cancel(session);
    if (svg_index_step(session, 64U) != SVG_INDEX_CANCELED) return fail("cancel failed");
    svg_index_destroy(session);
    (void)printf("{\"status\":\"passed\",\"checks\":[\"progressive first supported element\",\"preview precedes completion\",\"10 percent cancel\",\"50 percent cancel\",\"90 percent cancel\",\"stale generation\",\"unsupported element retained\",\"oversized source rejected before dereference\"]}\n");
    return 0;
}
