#include "viewer_timeline.h"

#include <math.h>
#include <stdio.h>
#include <string.h>

#define CHECK(expression) do { \
    if (!(expression)) { \
        (void)fprintf(stderr, "%s:%d: check failed: %s\n", \
            __FILE__, __LINE__, #expression); \
        return false; \
    } \
} while (0)

static ui_semantics_snapshot semantics_scratch;

static bool decode(const char *source, svg_shape_document *document)
{
    svg_shape_diagnostic diagnostic = {0};
    return svg_shape_decode((base_bytes){(const uint8_t *)source, strlen(source)},
        document, &diagnostic) == SVG_SHAPE_ERROR_NONE;
}

static bool test_tracks_labels_and_duration(void)
{
    static const char source[] =
        "<svg width='48' height='16'><rect id='dot' x='1' y='4' width='8' height='8' fill='red'>"
        "<animate attributeName='x' values='1;39' dur='2s'/></rect></svg>";
    svg_shape_document document = {0};
    CHECK(decode(source, &document));
    viewer_timeline timeline;
    viewer_timeline_init(&timeline);
    viewer_timeline_reset(&timeline, &document);
    char label[128] = {0};
    size_t label_length = 0U;
    bool okay = viewer_timeline_track_count(&timeline) == 1U &&
        fabs(viewer_timeline_duration(&timeline) - 2.0) < 1.0e-9 &&
        viewer_timeline_track_label(&timeline, 0U, label, sizeof(label), &label_length) &&
        label_length == strlen("rect#dot / x") && strcmp(label, "rect#dot / x") == 0;
    viewer_timeline_destroy(&timeline);
    svg_shape_document_destroy(&document);
    CHECK(okay);
    return true;
}

static bool test_seek_play_pause_loop_and_keys(void)
{
    static const char source[] =
        "<svg width='16' height='8'><rect x='1' y='1' width='4' height='4' fill='red'>"
        "<animate attributeName='x' from='1' to='9' dur='2s'/></rect></svg>";
    svg_shape_document document = {0};
    CHECK(decode(source, &document));
    viewer_timeline timeline;
    viewer_timeline_init(&timeline);
    viewer_timeline_reset(&timeline, &document);

    CHECK(viewer_timeline_seek(&timeline, 0.5));
    CHECK(fabs(timeline.time_seconds - 0.5) < 1.0e-9);
    CHECK(viewer_timeline_handle_key(&timeline, 49U, " ", 1U, 10.0));
    CHECK(timeline.playing);
    CHECK(viewer_timeline_advance(&timeline, 10.5));
    CHECK(fabs(timeline.time_seconds - 1.0) < 1.0e-9);
    CHECK(viewer_timeline_handle_key(&timeline, 0U, "[", 1U, 10.5));
    CHECK(!timeline.playing && fabs(timeline.time_seconds - (1.0 - 1.0 / 30.0)) < 1.0e-9);
    CHECK(viewer_timeline_handle_key(&timeline, 0U, "]", 1U, 10.5));
    CHECK(fabs(timeline.time_seconds - 1.0) < 1.0e-9);
    CHECK(viewer_timeline_set_loop(&timeline, true));
    CHECK(viewer_timeline_toggle_play(&timeline, 11.0));
    CHECK(viewer_timeline_advance(&timeline, 13.5));
    CHECK(timeline.playing && fabs(timeline.time_seconds - 1.5) < 1.0e-9);
    CHECK(viewer_timeline_seek(&timeline, 10.0));
    CHECK(!timeline.playing && fabs(timeline.time_seconds - 2.0) < 1.0e-9);

    viewer_timeline_destroy(&timeline);
    svg_shape_document_destroy(&document);
    return true;
}

static bool test_layout_pointer_scrub_and_rows(void)
{
    static const char source[] =
        "<svg width='32' height='12'><rect id='one' x='1' y='1' width='4' height='4' fill='red'>"
        "<animate attributeName='x' from='1' to='9' dur='2s'/></rect>"
        "<rect id='two' x='4' y='5' width='4' height='4' fill='blue'>"
        "<animate attributeName='y' from='5' to='9' dur='4s'/></rect></svg>";
    svg_shape_document document = {0};
    CHECK(decode(source, &document));
    viewer_timeline timeline;
    viewer_timeline_init(&timeline);
    viewer_timeline_reset(&timeline, &document);
    viewer_timeline_layout layout;
    CHECK(viewer_timeline_layout_compute(&timeline, 900U, 720U, 2.0, &layout));
    CHECK(layout.track_count == 2U && layout.visible_track_count == 2U);
    CHECK(layout.panel.y + layout.panel.height <= 720U);
    CHECK(layout.scrubber.width > 400U);
    double track_inset = ceil(8.0 * layout.backing_scale);
    double track_width = layout.scrubber.width - ceil(64.0 * layout.backing_scale);
    double scrub_x = (double)layout.scrubber.x + track_inset + track_width / 2.0;
    double scrub_y = (double)layout.scrubber.y + (double)layout.scrubber.height / 2.0;
    CHECK(viewer_timeline_pointer(&timeline, &layout, VIEWER_TIMELINE_POINTER_DOWN,
        scrub_x, scrub_y, 0.0, 1.0));
    CHECK(timeline.dragging_scrubber);
    CHECK(fabs(timeline.time_seconds - 2.0) < 0.02);
    CHECK(viewer_timeline_pointer(&timeline, &layout, VIEWER_TIMELINE_POINTER_MOVE,
        (double)layout.scrubber.x + track_inset + track_width, scrub_y, 0.0, 1.1));
    CHECK(fabs(timeline.time_seconds - viewer_timeline_duration(&timeline)) < 1.0e-9);
    CHECK(viewer_timeline_pointer(&timeline, &layout, VIEWER_TIMELINE_POINTER_UP,
        0.0, 0.0, 0.0, 1.2));
    CHECK(!timeline.dragging_scrubber);
    CHECK(viewer_timeline_pointer(&timeline, &layout, VIEWER_TIMELINE_POINTER_DOWN,
        (double)layout.track_rows[0].x + 2.0,
        (double)layout.track_rows[0].y + 2.0, 0.0, 1.3));
    CHECK(timeline.selected_track == 0U && fabs(timeline.time_seconds) < 1.0e-9);

    viewer_timeline_destroy(&timeline);
    svg_shape_document_destroy(&document);
    return true;
}

static bool test_frame_preview_and_bounds(void)
{
    static const char source[] =
        "<svg width='16' height='8'><rect x='1' y='1' width='4' height='4' fill='red'>"
        "<animate attributeName='x' from='1' to='9' dur='2s'/></rect></svg>";
    svg_shape_document document = {0};
    CHECK(decode(source, &document));
    viewer_timeline timeline;
    viewer_timeline_init(&timeline);
    viewer_timeline_reset(&timeline, &document);
    const raster_tile *start = viewer_timeline_prepare_frame(&timeline);
    CHECK(start != NULL && start->width == 16U && start->height == 8U);
    const raster_tile *cached = viewer_timeline_prepare_frame(&timeline);
    CHECK(cached == start);
    CHECK(start->rgba[(size_t)2U * start->width * 4U + 2U * 4U + 3U] == 255U);
    CHECK(viewer_timeline_seek(&timeline, 1.0));
    const raster_tile *middle = viewer_timeline_prepare_frame(&timeline);
    CHECK(middle != NULL && middle->width == 16U && middle->height == 8U);
    CHECK(middle->rgba[(size_t)2U * middle->width * 4U + 6U * 4U + 3U] == 255U);
    CHECK(middle->rgba[(size_t)2U * middle->width * 4U + 2U * 4U + 3U] == 0U);
    CHECK(!viewer_timeline_seek(&timeline, NAN));
    CHECK(!viewer_timeline_seek(&timeline, INFINITY));
    CHECK(fabs(timeline.time_seconds - 1.0) < 1.0e-9);

    viewer_timeline_destroy(&timeline);
    svg_shape_document_destroy(&document);
    return true;
}

static bool snapshot_has_label(const ui_semantics_snapshot *snapshot,
    ui_role role, const char *expected)
{
    size_t expected_length = strlen(expected);
    for (size_t index = 0U; index < snapshot->count; ++index) {
        const ui_semantics_node *node = &snapshot->nodes[index];
        const char *label = NULL;
        size_t length = 0U;
        if (node->role == role && ui_semantics_text(snapshot, node, false,
                &label, &length) && length == expected_length &&
            memcmp(label, expected, length) == 0) return true;
    }
    return false;
}

static bool test_accessibility_labels(void)
{
    static const char source[] =
        "<svg width='16' height='8'><rect id='dot' x='1' y='1' width='4' height='4' fill='red'>"
        "<animate attributeName='x' from='1' to='9' dur='2s'/></rect></svg>";
    svg_shape_document document = {0};
    CHECK(decode(source, &document));
    viewer_timeline timeline;
    viewer_timeline_init(&timeline);
    viewer_timeline_reset(&timeline, &document);
    viewer_timeline_layout layout;
    CHECK(viewer_timeline_layout_compute(&timeline, 900U, 720U, 2.0, &layout));
    CHECK(viewer_timeline_accessibility_snapshot(&timeline, &layout,
        UI_THEME_DARK, false, &semantics_scratch));
    CHECK(snapshot_has_label(&semantics_scratch, UI_ROLE_BUTTON, "Play animation"));
    CHECK(snapshot_has_label(&semantics_scratch, UI_ROLE_SLIDER, "Timeline position"));
    CHECK(snapshot_has_label(&semantics_scratch, UI_ROLE_CHECKBOX, "Loop animation"));
    CHECK(snapshot_has_label(&semantics_scratch, UI_ROLE_ROW, "rect#dot / x"));
    viewer_timeline_destroy(&timeline);
    svg_shape_document_destroy(&document);
    return true;
}

int main(void)
{
    if (!test_tracks_labels_and_duration() || !test_seek_play_pause_loop_and_keys() ||
        !test_layout_pointer_scrub_and_rows() || !test_frame_preview_and_bounds() ||
        !test_accessibility_labels())
        return 1;
    (void)puts("viewer timeline: passed");
    return 0;
}
