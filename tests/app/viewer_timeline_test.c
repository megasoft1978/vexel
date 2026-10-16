#include "viewer_timeline.h"
#include "platform/macos/platform_png.h"
#include "platform/platform_text_ui.h"
#include "ui/ui_widgets.h"
#include "viewer_anim_edit.h"

#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define CHECK(expression) do { \
    if (!(expression)) { \
        (void)fprintf(stderr, "%s:%d: check failed: %s\n", __FILE__, __LINE__, #expression); \
        return false; \
    } \
} while (0)

static ui_semantics_snapshot semantics_scratch;

static bool decode(const char *source, svg_shape_document *document)
{
    svg_shape_diagnostic diagnostic = {0};
    svg_shape_error error = svg_shape_decode(
        (base_bytes){(const uint8_t *)source, strlen(source)}, document, &diagnostic);
    if (error != SVG_SHAPE_ERROR_NONE)
        (void)fprintf(stderr, "viewer timeline fixture decode failed: %d at %zu\n",
            (int)error, diagnostic.offset);
    return error == SVG_SHAPE_ERROR_NONE;
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
    CHECK(viewer_timeline_layout_compute(&timeline, 900U, 720U, 1.0, &layout));
    CHECK(layout.track_count == 2U && layout.visible_track_count == 2U);
    CHECK(layout.panel.y + layout.panel.height <= 720U);
    CHECK(layout.scrubber.width > 400U);
    double track_inset = 0.0;
    double track_width = layout.scrubber.width;
    double scrub_x = (double)layout.scrubber.x + track_width / 2.0;
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
    CHECK(timeline.preview_valid);
    CHECK(viewer_timeline_prepare_frame(&timeline) == middle);
    CHECK(!viewer_timeline_seek(&timeline, NAN));
    CHECK(!viewer_timeline_seek(&timeline, INFINITY));
    CHECK(fabs(timeline.time_seconds - 1.0) < 1.0e-9);
    document.width = RASTER_TILE_MAX_AXIS + 1U;
    CHECK(viewer_timeline_seek(&timeline, 1.5));
    CHECK(viewer_timeline_prepare_frame(&timeline) == NULL &&
        !timeline.preview_valid && timeline.preview_failed);
    CHECK(viewer_timeline_prepare_frame(&timeline) == NULL &&
        !timeline.preview_valid);

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

static bool snapshot_has_value_fragment(const ui_semantics_snapshot *snapshot,
    ui_role role, const char *fragment, bool require_disabled)
{
    size_t fragment_length = strlen(fragment);
    for (size_t index = 0U; index < snapshot->count; ++index) {
        const ui_semantics_node *node = &snapshot->nodes[index];
        if (node->role != role ||
            (require_disabled && (node->state & UI_STATE_DISABLED) == 0U)) continue;
        const char *value = NULL;
        size_t value_length = 0U;
        if (!ui_semantics_text(snapshot, node, true, &value, &value_length) ||
            value_length < fragment_length) continue;
        for (size_t offset = 0U; offset <= value_length - fragment_length; ++offset) {
            if (memcmp(value + offset, fragment, fragment_length) == 0) return true;
        }
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
    CHECK(viewer_timeline_layout_compute(&timeline, 1800U, 1440U, 2.0, &layout));
    CHECK(viewer_timeline_accessibility_snapshot(&timeline, &layout,
        UI_THEME_DARK, false, &semantics_scratch));
    CHECK(snapshot_has_label(&semantics_scratch, UI_ROLE_BUTTON, "Play animation"));
    CHECK(snapshot_has_label(&semantics_scratch, UI_ROLE_SLIDER, "Timeline position"));
    CHECK(snapshot_has_label(&semantics_scratch, UI_ROLE_BUTTON, "Preview mode"));
    CHECK(snapshot_has_label(&semantics_scratch, UI_ROLE_BUTTON, "Preview speed"));
    CHECK(snapshot_has_label(&semantics_scratch, UI_ROLE_BUTTON,
        "Fit timeline to work area"));
    CHECK(snapshot_has_label(&semantics_scratch, UI_ROLE_ROW, "rect#dot / x"));
    viewer_timeline_destroy(&timeline);
    svg_shape_document_destroy(&document);
    return true;
}

static bool test_indefinite_and_unsupported_accessibility(void)
{
    static const char source[] =
        "<svg width='16' height='8'><rect x='1' y='1' width='4' height='4'>"
        "<animate attributeName='x' from='1' to='9' dur='2s' repeatCount='indefinite'/>"
        "</rect></svg>";
    svg_shape_document document = {0};
    CHECK(decode(source, &document));
    viewer_timeline timeline;
    viewer_timeline_init(&timeline);
    viewer_timeline_reset(&timeline, &document);
    viewer_timeline_layout layout;
    CHECK(viewer_timeline_layout_compute(&timeline, 1280U, 800U, 1.0, &layout));
    CHECK(viewer_timeline_accessibility_snapshot(&timeline, &layout,
        UI_THEME_DARK, false, &semantics_scratch));
    CHECK(snapshot_has_value_fragment(&semantics_scratch, UI_ROLE_SLIDER,
        "indefinite source; preview 0.00 to 5.00s", false));

    CHECK(document.animation.track_count == 1U);
    svg_anim_track *track = &document.animation.tracks[0];
    track->supported = false;
    document.animation.diagnostics[0] = (svg_anim_diagnostic){
        SVG_ANIM_DIAG_UNSUPPORTED_TIMING, track->element_offset + 1U};
    document.animation.diagnostic_count = 1U;
    CHECK(viewer_timeline_accessibility_snapshot(&timeline, &layout,
        UI_THEME_DARK, false, &semantics_scratch));
    CHECK(snapshot_has_value_fragment(&semantics_scratch, UI_ROLE_ROW,
        "unsupported-timing", true));

    viewer_timeline_destroy(&timeline);
    svg_shape_document_destroy(&document);
    return true;
}

static bool append_text(char *buffer, size_t capacity, size_t *length,
                        const char *text)
{
    if (buffer == NULL || length == NULL || text == NULL || *length >= capacity)
        return false;
    size_t text_length = strlen(text);
    if (text_length >= capacity - *length) return false;
    memcpy(buffer + *length, text, text_length);
    *length += text_length;
    buffer[*length] = '\0';
    return true;
}

static bool test_docked_layout_controls_and_navigation(void)
{
    static const char source[] =
        "<svg width='48' height='16'><rect id='dot' x='1' y='4' width='8' height='8' fill='red'>"
        "<animate attributeName='x' values='1;39' dur='2s'/></rect></svg>";
    svg_shape_document document = {0};
    CHECK(decode(source, &document));
    viewer_timeline timeline;
    viewer_timeline_init(&timeline);
    viewer_timeline_reset(&timeline, &document);
    CHECK(timeline.dock_open);
    viewer_timeline_layout layout;
    CHECK(viewer_timeline_layout_compute(&timeline, 1280U, 800U, 1.0, &layout));
    CHECK(layout.panel.height == 220U && layout.ruler.width > 0U &&
        layout.track_gutter.x < layout.time_grid.x &&
        layout.time_grid.width > 500U && layout.visible_track_count == 1U);
    CHECK(viewer_timeline_layout_compute(&timeline, 2560U, 1600U, 2.0, &layout));
    CHECK(layout.panel.height == 440U && layout.backing_scale == 2.0);
    CHECK(viewer_timeline_layout_compute(&timeline, 3840U, 2400U, 3.0, &layout));
    CHECK(layout.panel.height == 660U &&
        viewer_timeline_key_hit_radius(layout.backing_scale) == 18U);
    CHECK(viewer_timeline_layout_compute(&timeline, 280U, 800U, 1.0, &layout));
    CHECK(layout.compact && layout.mode_button.width > 0U &&
        layout.speed_button.width > 0U && layout.fit_button.width > 0U &&
        layout.fit_button.x + layout.fit_button.width <= 280U);
    CHECK(viewer_timeline_pointer(&timeline, &layout,
        VIEWER_TIMELINE_POINTER_DOWN,
        (double)layout.resize_handle.x + 8.0,
        (double)layout.panel.y + 2.0, 0.0, 1.0));
    CHECK(viewer_timeline_pointer(&timeline, &layout,
        VIEWER_TIMELINE_POINTER_MOVE,
        (double)layout.panel.x + 8.0, (double)layout.panel.y - 18.0,
        0.0, 1.1));
    CHECK(timeline.dock_height_points > VIEWER_TIMELINE_DOCK_DEFAULT_POINTS);
    CHECK(viewer_timeline_pointer(&timeline, &layout,
        VIEWER_TIMELINE_POINTER_UP, 0.0, 0.0, 0.0, 1.2));
    CHECK(viewer_timeline_dock_resize(&timeline,
        (int32_t)VIEWER_TIMELINE_DOCK_DEFAULT_POINTS -
            (int32_t)timeline.dock_height_points));

    CHECK(viewer_timeline_dock_set_open(&timeline, false));
    CHECK(viewer_timeline_layout_compute(&timeline, 480U, 640U, 1.0, &layout));
    CHECK(layout.panel.height == 32U && layout.collapsed);
    CHECK(viewer_timeline_dock_set_open(&timeline, true));
    CHECK(viewer_timeline_dock_resize(&timeline, 200));
    CHECK(timeline.dock_height_points == 320U);
    CHECK(viewer_timeline_dock_resize(&timeline, -400));
    CHECK(timeline.dock_height_points == 184U);

    CHECK(viewer_timeline_layout_compute(&timeline, 1280U, 280U, 1.0, &layout));
    CHECK(layout.collapsed && layout.panel.height == VIEWER_TIMELINE_DOCK_COLLAPSED_POINTS);
    CHECK(viewer_timeline_accessibility_snapshot(&timeline, &layout,
        UI_THEME_DARK, false, &semantics_scratch));
    CHECK(snapshot_has_label(&semantics_scratch, UI_ROLE_BUTTON, "Close timeline"));
    CHECK(viewer_timeline_pointer(&timeline, &layout,
        VIEWER_TIMELINE_POINTER_DOWN,
        (double)layout.collapse_toggle.x + 2.0,
        (double)layout.collapse_toggle.y + 2.0, 0.0, 2.0));
    CHECK(!timeline.dock_open);
    CHECK(viewer_timeline_layout_compute(&timeline, 1280U, 800U, 1.0, &layout));
    CHECK(layout.collapsed);
    CHECK(viewer_timeline_pointer(&timeline, &layout,
        VIEWER_TIMELINE_POINTER_DOWN,
        (double)layout.collapse_toggle.x + 2.0,
        (double)layout.collapse_toggle.y + 2.0, 0.0, 2.1));
    CHECK(timeline.dock_open);

    CHECK(viewer_timeline_zoom_at(&timeline, 1.0, 2.0));
    CHECK(timeline.zoom_factor == 2.0 &&
        fabs(timeline.view_start_seconds - 0.5) < 1.0e-9);
    CHECK(viewer_timeline_pan(&timeline, 0.25));
    CHECK(fabs(timeline.view_start_seconds - 0.75) < 1.0e-9);
    CHECK(viewer_timeline_fit(&timeline));
    CHECK(timeline.zoom_factor == 1.0 && timeline.view_start_seconds == 0.0);
    CHECK(viewer_timeline_key_hit_radius(1.0) == 6U &&
        viewer_timeline_key_hit_radius(3.0) == 18U);

    viewer_timeline_destroy(&timeline);
    svg_shape_document_destroy(&document);
    return true;
}

static bool test_modes_speed_virtualization_and_selection_sync(void)
{
    char source[32768] = {0};
    size_t source_length = 0U;
    CHECK(append_text(source, sizeof(source), &source_length,
        "<svg width='64' height='16'>"));
    for (size_t index = 0U; index < SVG_ANIM_MAX_TRACKS; ++index) {
        char element[192];
        int amount = snprintf(element, sizeof(element),
            "<rect id='target-%03zu' x='0' y='0' width='1' height='1'>"
            "<animate attributeName='x' values='0;20' dur='2s'/></rect>", index);
        CHECK(amount > 0 && (size_t)amount < sizeof(element));
        CHECK(append_text(source, sizeof(source), &source_length, element));
    }
    CHECK(append_text(source, sizeof(source), &source_length, "</svg>"));
    svg_shape_document document = {0};
    CHECK(decode(source, &document));
    viewer_timeline timeline;
    viewer_timeline_init(&timeline);
    viewer_timeline_reset(&timeline, &document);
    CHECK(viewer_timeline_track_count(&timeline) == SVG_ANIM_MAX_TRACKS);
    viewer_timeline_layout layout;
    CHECK(viewer_timeline_layout_compute(&timeline, 1280U, 800U, 1.0, &layout));
    CHECK(layout.visible_track_count > 5U &&
        layout.visible_track_count < viewer_timeline_track_count(&timeline));

    CHECK(viewer_timeline_set_mode(&timeline, VIEWER_TIMELINE_MODE_PING_PONG));
    CHECK(viewer_timeline_set_speed(&timeline, 2.0));
    CHECK(!viewer_timeline_set_speed(&timeline, 3.0));
    CHECK(viewer_timeline_toggle_play(&timeline, 10.0));
    CHECK(viewer_timeline_advance(&timeline, 10.5));
    CHECK(fabs(timeline.time_seconds - 1.0) < 1.0e-9);
    CHECK(viewer_timeline_advance(&timeline, 11.0));
    CHECK(fabs(timeline.time_seconds - 2.0) < 1.0e-9 && timeline.playing);
    CHECK(viewer_timeline_advance(&timeline, 11.1));
    CHECK(fabs(timeline.time_seconds - 1.8) < 1.0e-9 && timeline.playing);
    CHECK(viewer_timeline_set_mode(&timeline, VIEWER_TIMELINE_MODE_ONE_SHOT));
    CHECK(viewer_timeline_seek(&timeline, 0.0));
    CHECK(viewer_timeline_toggle_play(&timeline, 12.0));
    CHECK(viewer_timeline_advance(&timeline, 13.1));
    CHECK(!timeline.playing && fabs(timeline.time_seconds - 2.0) < 1.0e-9);

    viewer_editor *editor = viewer_editor_create((base_bytes){
        (const uint8_t *)source, source_length}, UI_THEME_DARK, 1280U, 800U);
    CHECK(editor != NULL);
    CHECK(viewer_editor_set_timeline_state(editor, true, true, 220U));
    CHECK(viewer_timeline_layout_compute(&timeline, 1280U, 800U, 1.0, &layout));
    timeline.first_track = 73U;
    CHECK(viewer_timeline_layout_compute(&timeline, 1280U, 800U, 1.0, &layout));
    CHECK(layout.first_track == 73U && layout.visible_track_count > 0U);
    size_t expected_offset = document.animation.tracks[73U].target_offset;
    CHECK(viewer_timeline_process_event(&timeline, &(platform_event){
        .type = PLATFORM_EVENT_POINTER_DOWN,
        .logical_x = (int32_t)(layout.track_gutter.x + 8U),
        .logical_y = (int32_t)(layout.track_rows[0].y + 4U),
        .button = 0U}, editor, 1280U, 800U, 1.0));
    document_source_span selected = {0};
    CHECK(viewer_editor_selection_span(editor, &selected));
    CHECK(selected.offset == expected_offset);
    CHECK(timeline.selected_track == 73U);
    CHECK(viewer_editor_set_timeline_state(editor, true, false, 220U));
    viewer_timeline_sync_selection(&timeline, editor);
    CHECK(timeline.selected_track == 73U);

    viewer_editor_destroy(editor);
    viewer_timeline_destroy(&timeline);
    svg_shape_document_destroy(&document);
    return true;
}

static bool test_reduce_motion_keeps_explicit_preview(void)
{
    static const char source[] =
        "<svg width='16' height='8'><rect x='1' y='1' width='4' height='4' fill='red'>"
        "<animate attributeName='x' from='1' to='9' dur='2s'/></rect></svg>";
    svg_shape_document document = {0};
    CHECK(decode(source, &document));
    viewer_timeline timeline;
    viewer_timeline_init(&timeline);
    viewer_timeline_reset(&timeline, &document);
    viewer_timeline_set_reduced_motion(&timeline, true);
    CHECK(timeline.reduced_motion);
    CHECK(viewer_timeline_toggle_play(&timeline, 2.0));
    CHECK(viewer_timeline_advance(&timeline, 2.5));
    CHECK(timeline.playing && fabs(timeline.time_seconds - 0.5) < 1.0e-9);
    viewer_timeline_destroy(&timeline);
    svg_shape_document_destroy(&document);
    return true;
}

static bool test_duplicate_and_missing_ids_keep_source_identity(void)
{
    static const char source[] =
        "<svg width='16' height='8'>"
        "<rect id='same' x='0' y='0' width='2' height='2'><animate attributeName='x' values='0;8' dur='2s'/></rect>"
        "<rect id='same' x='3' y='0' width='2' height='2'><animate attributeName='x' values='3;9' dur='2s'/></rect>"
        "<circle cx='4' cy='4' r='1'><animate attributeName='cy' values='4;7' dur='2s'/></circle>"
        "</svg>";
    svg_anim_track tracks[3] = {0};
    const char *targets[] = {"<rect id='same' x='0'", "<rect id='same' x='3'",
        "<circle cx='4'"};
    for (size_t index = 0U; index < 3U; ++index) {
        const char *target = strstr(source, targets[index]);
        CHECK(target != NULL);
        const char *end = strchr(target, '>');
        CHECK(end != NULL);
        tracks[index].supported = true;
        tracks[index].kind = SVG_ANIM_KIND_ANIMATE;
        tracks[index].attribute = index == 2U ? SVG_ANIM_ATTR_CY : SVG_ANIM_ATTR_X;
        tracks[index].duration_seconds = 2.0;
        tracks[index].repeat_count = 1.0;
        tracks[index].target_offset = (size_t)(target - source);
        tracks[index].target_length = (size_t)(end - target) + 1U;
    }
    svg_shape_document document = {0};
    document.animation.source = (base_bytes){(const uint8_t *)source,
        sizeof(source) - 1U};
    document.animation.tracks = tracks;
    document.animation.track_count = 3U;
    viewer_timeline timeline;
    viewer_timeline_init(&timeline);
    viewer_timeline_reset(&timeline, &document);
    CHECK(viewer_timeline_track_count(&timeline) == 3U);
    char first[128], duplicate[128], no_id[128];
    size_t first_length = 0U, duplicate_length = 0U, no_id_length = 0U;
    CHECK(viewer_timeline_track_label(&timeline, 0U, first, sizeof(first), &first_length));
    CHECK(viewer_timeline_track_label(&timeline, 1U, duplicate, sizeof(duplicate),
        &duplicate_length));
    CHECK(viewer_timeline_track_label(&timeline, 2U, no_id, sizeof(no_id), &no_id_length));
    CHECK(first_length == duplicate_length && memcmp(first, duplicate, first_length) == 0);
    CHECK(strcmp(no_id, "circle / cy") == 0);
    CHECK(document.animation.tracks[0].target_offset !=
        document.animation.tracks[1].target_offset);

    CHECK(memcmp(source, document.animation.source.ptr, sizeof(source) - 1U) == 0);
    viewer_timeline_destroy(&timeline);
    return true;
}

int main(void)
{
    if (!test_tracks_labels_and_duration() || !test_seek_play_pause_loop_and_keys() ||
        !test_layout_pointer_scrub_and_rows() || !test_frame_preview_and_bounds() ||
        !test_accessibility_labels() || !test_indefinite_and_unsupported_accessibility() ||
        !test_docked_layout_controls_and_navigation() ||
        !test_modes_speed_virtualization_and_selection_sync() ||
        !test_reduce_motion_keeps_explicit_preview() ||
        !test_duplicate_and_missing_ids_keep_source_identity())
        return 1;
    (void)puts("viewer timeline: passed");
    return 0;
}
