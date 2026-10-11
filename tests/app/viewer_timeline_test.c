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

static bool snapshot_has_value(const ui_semantics_snapshot *snapshot,
    ui_role role, const char *label_text, const char *expected_value)
{
    for (size_t index = 0U; index < snapshot->count; ++index) {
        const ui_semantics_node *node = &snapshot->nodes[index];
        const char *label = NULL, *value = NULL;
        size_t label_length = 0U, value_length = 0U;
        if (node->role == role && ui_semantics_text(snapshot, node, false,
                &label, &label_length) && ui_semantics_text(snapshot, node, true,
                &value, &value_length) && label_length == strlen(label_text) &&
            memcmp(label, label_text, label_length) == 0 &&
            value_length == strlen(expected_value) &&
            memcmp(value, expected_value, value_length) == 0) return true;
    }
    return false;
}

static bool snapshot_has_disabled_label(const ui_semantics_snapshot *snapshot,
    ui_role role, const char *expected)
{
    for (size_t index = 0U; index < snapshot->count; ++index) {
        const ui_semantics_node *node = &snapshot->nodes[index];
        const char *label = NULL;
        size_t length = 0U;
        if (node->role == role && (node->state & UI_STATE_DISABLED) != 0U &&
            ui_semantics_text(snapshot, node, false, &label, &length) &&
            length == strlen(expected) && memcmp(label, expected, length) == 0)
            return true;
    }
    return false;
}

static bool write_easing_png(const char *directory, const char *filename,
    const uint8_t *rgba, size_t capacity, size_t stride,
    uint32_t width, uint32_t height)
{
    if (directory == NULL) return true;
    char path[1024];
    int amount = snprintf(path, sizeof(path), "%s/%s", directory, filename);
    if (amount <= 0 || (size_t)amount >= sizeof(path)) return false;
    platform_png_blob png = {0};
    if (!platform_png_encode_rgba8(rgba, capacity, stride, width, height, &png))
        return false;
    FILE *file = fopen(path, "wb");
    bool okay = file != NULL && fwrite(png.bytes, 1U, png.length, file) == png.length;
    if (file != NULL && fclose(file) != 0) okay = false;
    platform_png_blob_destroy(&png);
    return okay;
}

static uint64_t fnv1a64(const uint8_t *bytes, size_t length)
{
    uint64_t hash = UINT64_C(14695981039346656037);
    for (size_t index = 0U; index < length; ++index) {
        hash ^= bytes[index];
        hash *= UINT64_C(1099511628211);
    }
    return hash;
}

static bool render_easing_hash(ui_theme theme, bool increased_contrast,
    const char *filename, uint64_t *out_hash)
{
    static const char source[] =
        "<svg width='48' height='16'><rect id='dot' x='0' y='2' width='8' height='8'>"
        "<animate attributeName='x' values='0;5;10;15' keyTimes='0;0.25;0.5;1' "
        "dur='2s' calcMode='spline' keySplines='0.10 0.20 0.30 0.40; "
        "0.22 0.32 0.42 0.52;0.40 0.50 0.60 0.70'/></rect></svg>";
    const uint32_t width = 900U, height = 720U;
    size_t stride = (size_t)width * 4U;
    size_t capacity = stride * height;
    uint8_t *rgba = calloc(capacity, 1U);
    CHECK(rgba != NULL);
    svg_shape_document document = {0};
    CHECK(decode(source, &document));
    viewer_timeline timeline;
    viewer_timeline_init(&timeline);
    viewer_timeline_reset(&timeline, &document);
    timeline.selected_key = 1U;
    viewer_timeline_layout layout;
    CHECK(viewer_timeline_layout_compute(&timeline, width, height, 2.0, &layout));
    viewer_timeline_easing_layout easing;
    CHECK(viewer_timeline_easing_layout_compute(&layout, &easing) && easing.visible);
    ui_widget_canvas canvas;
    CHECK(ui_widget_canvas_init(&canvas, rgba, capacity, width, height, stride,
        2U, theme, increased_contrast, platform_text_ui_draw_widget, NULL));
    uint32_t panel = canvas.tokens.panel;
    for (size_t offset = 0U; offset < capacity; offset += 4U) {
        rgba[offset] = (uint8_t)(panel >> 24U);
        rgba[offset + 1U] = (uint8_t)(panel >> 16U);
        rgba[offset + 2U] = (uint8_t)(panel >> 8U);
        rgba[offset + 3U] = (uint8_t)panel;
    }
    viewer_timeline_easing_paint(&timeline, &layout, &easing, &canvas);
    *out_hash = fnv1a64(rgba, capacity);
    bool okay = write_easing_png(getenv("VEXEL_EASING_GOLDEN_DIR"), filename,
        rgba, capacity, stride, width, height);
    viewer_timeline_destroy(&timeline);
    svg_shape_document_destroy(&document);
    free(rgba);
    return okay;
}

static bool test_easing_accessibility_and_raster_goldens(void)
{
    static const char source[] =
        "<svg width='48' height='16'><rect id='dot' x='0' y='2' width='8' height='8'>"
        "<animate attributeName='x' values='0;5;10;15' keyTimes='0;0.25;0.5;1' "
        "dur='2s' calcMode='spline' keySplines='0.10 0.20 0.30 0.40; "
        "0.22 0.32 0.42 0.52;0.40 0.50 0.60 0.70'/></rect></svg>";
    svg_shape_document document = {0};
    CHECK(decode(source, &document));
    viewer_timeline timeline;
    viewer_timeline_init(&timeline);
    viewer_timeline_reset(&timeline, &document);
    timeline.selected_key = 1U;
    CHECK(viewer_timeline_easing_enabled(&timeline));
    viewer_timeline_easing_sync(&timeline);
    CHECK(fabs(timeline.easing_fields[0].value - 0.22) < 1e-12 &&
        fabs(timeline.easing_fields[3].value - 0.52) < 1e-12);
    viewer_timeline_layout layout;
    CHECK(viewer_timeline_layout_compute(&timeline, 900U, 720U, 2.0, &layout));
    viewer_timeline_easing_layout easing;
    CHECK(viewer_timeline_easing_layout_compute(&layout, &easing) && easing.visible);
    CHECK(easing.graph.width > 0U && easing.graph.height > 0U);
    CHECK(easing.mode_buttons[2].y + easing.mode_buttons[2].height <= easing.graph.y);
    CHECK(easing.graph.y + easing.graph.height <= easing.presets[0].y);
    CHECK(viewer_timeline_accessibility_snapshot(&timeline, &layout,
        UI_THEME_DARK, true, &semantics_scratch));
    CHECK(snapshot_has_value(&semantics_scratch, UI_ROLE_BUTTON,
        "Interpolation mode", "Spline"));
    CHECK(snapshot_has_label(&semantics_scratch, UI_ROLE_SLIDER, "Spline x1"));
    CHECK(snapshot_has_value(&semantics_scratch, UI_ROLE_SLIDER,
        "Spline y2", "0.52000000000000002"));
    viewer_timeline_destroy(&timeline);
    svg_shape_document_destroy(&document);

    uint64_t hashes[3] = {0U, 0U, 0U};
    CHECK(render_easing_hash(UI_THEME_LIGHT, false, "easing-light.png", &hashes[0]));
    CHECK(render_easing_hash(UI_THEME_DARK, false, "easing-dark.png", &hashes[1]));
    CHECK(render_easing_hash(UI_THEME_DARK, true,
        "easing-high-contrast.png", &hashes[2]));
    CHECK(hashes[0] == UINT64_C(0x7bc5c278646f35d8) &&
        hashes[1] == UINT64_C(0xeed8c47f12b6356d) &&
        hashes[2] == UINT64_C(0x0db2c8bad46cddd4));
    CHECK(hashes[0] != hashes[1] && hashes[1] != hashes[2]);

    static const char paced[] =
        "<svg width='16' height='8'><rect id='dot' x='0' y='0' width='4' height='4'>"
        "<animate attributeName='x' values='0;10' "
        "dur='1s' calcMode='paced'/></rect></svg>";
    document = (svg_shape_document){0};
    CHECK(decode(paced, &document));
    viewer_timeline_init(&timeline);
    viewer_timeline_reset(&timeline, &document);
    timeline.selected_key = 0U;
    CHECK(!viewer_timeline_easing_enabled(&timeline));
    CHECK(viewer_timeline_layout_compute(&timeline, 900U, 720U, 2.0, &layout));
    CHECK(viewer_timeline_accessibility_snapshot(&timeline, &layout,
        UI_THEME_DARK, true, &semantics_scratch));
    CHECK(snapshot_has_disabled_label(&semantics_scratch, UI_ROLE_BUTTON,
        "Interpolation mode"));
    CHECK(snapshot_has_disabled_label(&semantics_scratch, UI_ROLE_SLIDER, "Spline x1"));
    viewer_timeline_destroy(&timeline);
    svg_shape_document_destroy(&document);
    return true;
}

static bool test_easing_pointer_and_keyboard_edits(void)
{
    static const char source[] =
        "<svg width='48' height='16'><rect id='dot' x='0' y='2' width='8' height='8'>"
        "<animate attributeName='x' values='0;5;10;15' keyTimes='0;0.25;0.5;1' "
        "dur='2s' calcMode='spline' keySplines='0.10 0.20 0.30 0.40; "
        "0.22 0.32 0.42 0.52;0.40 0.50 0.60 0.70'/></rect></svg>";
    base_bytes bytes = {(const uint8_t *)source, sizeof(source) - 1U};
    svg_shape_document document = {0};
    CHECK(decode(source, &document));
    viewer_editor *editor = viewer_editor_create(bytes, UI_THEME_DARK, 900U, 720U);
    CHECK(editor != NULL);
    viewer_timeline timeline;
    viewer_timeline_init(&timeline);
    viewer_timeline_reset(&timeline, &document);
    timeline.selected_key = 1U;
    viewer_timeline_layout layout;
    CHECK(viewer_timeline_layout_compute(&timeline, 900U, 720U, 2.0, &layout));
    viewer_timeline_easing_layout easing;
    CHECK(viewer_timeline_easing_layout_compute(&layout, &easing));
    double target_x = (double)easing.graph.x + 7.0 + 0.58 * ((double)easing.graph.width - 14.0);
    double target_y = (double)easing.graph.y + (double)easing.graph.height - 7.0 -
        0.73 * ((double)easing.graph.height - 14.0);
    platform_event event = {.type = PLATFORM_EVENT_POINTER_DOWN,
        .logical_x = (int32_t)floor(target_x / 2.0),
        .logical_y = (int32_t)floor(target_y / 2.0)};
    CHECK(viewer_timeline_easing_process_event(&timeline, editor, &event, &layout));
    event.type = PLATFORM_EVENT_POINTER_MOVE;
    CHECK(viewer_timeline_easing_process_event(&timeline, editor, &event, &layout));
    event.type = PLATFORM_EVENT_POINTER_UP;
    CHECK(viewer_timeline_easing_process_event(&timeline, editor, &event, &layout));
    CHECK(viewer_timeline_take_editor_mutation(&timeline));
    size_t edited_capacity = viewer_editor_byte_length(editor) + 1U;
    uint8_t *edited = malloc(edited_capacity);
    CHECK(edited != NULL);
    size_t edited_length = 0U;
    CHECK(viewer_editor_materialize(editor, edited, edited_capacity, &edited_length));
    edited[edited_length] = 0U;
    CHECK(edited_length > bytes.len);
    CHECK(strstr((const char *)edited, "keySplines='0.10 0.20 0.30 0.40;") != NULL);
    CHECK(strstr((const char *)edited, ";0.40 0.50 0.60 0.70'") != NULL);
    free(edited);
    viewer_timeline_destroy(&timeline);
    viewer_editor_destroy(editor);
    svg_shape_document_destroy(&document);

    CHECK(decode(source, &document));
    editor = viewer_editor_create(bytes, UI_THEME_DARK, 900U, 720U);
    CHECK(editor != NULL);
    viewer_timeline_init(&timeline);
    viewer_timeline_reset(&timeline, &document);
    timeline.selected_key = 1U;
    CHECK(viewer_timeline_layout_compute(&timeline, 900U, 720U, 2.0, &layout));
    CHECK(viewer_timeline_easing_layout_compute(&layout, &easing));
    double handle_x = (double)easing.graph.x + 7.0 +
        0.42 * ((double)easing.graph.width - 14.0);
    double handle_y = (double)easing.graph.y + (double)easing.graph.height - 7.0 -
        0.52 * ((double)easing.graph.height - 14.0);
    event = (platform_event){.type = PLATFORM_EVENT_POINTER_DOWN,
        .logical_x = (int32_t)floor(handle_x / 2.0),
        .logical_y = (int32_t)floor(handle_y / 2.0)};
    CHECK(viewer_timeline_easing_process_event(&timeline, editor, &event, &layout));
    event = (platform_event){.type = PLATFORM_EVENT_KEY_DOWN, .key_code = 124U};
    CHECK(viewer_timeline_easing_process_event(&timeline, editor, &event, &layout));
    double changed_x2 = timeline.easing_fields[2].value;
    CHECK(changed_x2 > 0.42 && changed_x2 <= 1.0);
    event.key_code = 36U;
    CHECK(viewer_timeline_easing_process_event(&timeline, editor, &event, &layout));
    CHECK(viewer_timeline_take_editor_mutation(&timeline));
    edited_capacity = viewer_editor_byte_length(editor) + 1U;
    edited = malloc(edited_capacity);
    CHECK(edited != NULL);
    edited_length = 0U;
    CHECK(viewer_editor_materialize(editor, edited, edited_capacity, &edited_length));
    edited[edited_length] = 0U;
    svg_shape_document edited_document = {0};
    CHECK(decode((const char *)edited, &edited_document));
    CHECK(fabs(edited_document.animation.tracks[0].splines[1][2] - changed_x2) < 1e-12);
    free(edited);
    svg_shape_document_destroy(&edited_document);
    viewer_timeline_destroy(&timeline);
    viewer_editor_destroy(editor);
    svg_shape_document_destroy(&document);

    static const char linear_source[] =
        "<svg width='48' height='16'><rect id='dot' x='0' y='2' width='8' height='8'>"
        "<animate attributeName='x' values='0;5;10;15' keyTimes='0;0.25;0.5;1' "
        "dur='2s' calcMode='linear'/></rect></svg>";
    base_bytes linear_bytes = {(const uint8_t *)linear_source,
        sizeof(linear_source) - 1U};
    CHECK(decode(linear_source, &document));
    editor = viewer_editor_create(linear_bytes, UI_THEME_DARK, 900U, 720U);
    CHECK(editor != NULL);
    viewer_timeline_init(&timeline);
    viewer_timeline_reset(&timeline, &document);
    timeline.selected_key = 1U;
    CHECK(viewer_timeline_layout_compute(&timeline, 900U, 720U, 2.0, &layout));
    CHECK(viewer_timeline_easing_layout_compute(&layout, &easing));
    event = (platform_event){.type = PLATFORM_EVENT_POINTER_DOWN,
        .logical_x = (int32_t)((easing.mode_buttons[2].x +
            easing.mode_buttons[2].width / 2U) / 2U),
        .logical_y = (int32_t)((easing.mode_buttons[2].y +
            easing.mode_buttons[2].height / 2U) / 2U)};
    CHECK(viewer_timeline_easing_process_event(&timeline, editor, &event, &layout));
    CHECK(viewer_timeline_take_editor_mutation(&timeline));
    edited_capacity = viewer_editor_byte_length(editor) + 1U;
    edited = malloc(edited_capacity);
    CHECK(edited != NULL);
    edited_length = 0U;
    CHECK(viewer_editor_materialize(editor, edited, edited_capacity, &edited_length));
    edited[edited_length] = 0U;
    edited_document = (svg_shape_document){0};
    CHECK(decode((const char *)edited, &edited_document));
    CHECK(edited_document.animation.tracks[0].calc_mode == SVG_ANIM_CALC_SPLINE &&
        fabs(edited_document.animation.tracks[0].splines[1][0] - 0.42) < 1e-12 &&
        fabs(edited_document.animation.tracks[0].splines[1][2] - 0.58) < 1e-12);
    free(edited);
    svg_shape_document_destroy(&edited_document);
    viewer_timeline_destroy(&timeline);
    viewer_editor_destroy(editor);
    svg_shape_document_destroy(&document);
    return true;
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
        !test_accessibility_labels() || !test_easing_accessibility_and_raster_goldens() ||
        !test_easing_pointer_and_keyboard_edits())
        return 1;
    (void)puts("viewer timeline: passed");
    return 0;
}
