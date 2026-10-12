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

static bool open_easing_dock(viewer_timeline *timeline)
{
    return viewer_timeline_dock_resize(timeline, 100);
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
    const uint32_t width = 900U, height = 1200U;
    size_t stride = (size_t)width * 4U;
    size_t capacity = stride * height;
    uint8_t *rgba = calloc(capacity, 1U);
    CHECK(rgba != NULL);
    svg_shape_document document = {0};
    CHECK(decode(source, &document));
    viewer_timeline timeline;
    viewer_timeline_init(&timeline);
    viewer_timeline_reset(&timeline, &document);
    CHECK(open_easing_dock(&timeline));
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
    CHECK(open_easing_dock(&timeline));
    timeline.selected_key = 1U;
    CHECK(viewer_timeline_easing_enabled(&timeline));
    viewer_timeline_easing_sync(&timeline);
    CHECK(fabs(timeline.easing_fields[0].value - 0.22) < 1e-12 &&
        fabs(timeline.easing_fields[3].value - 0.52) < 1e-12);
    viewer_timeline_layout layout;
    CHECK(viewer_timeline_layout_compute(&timeline, 900U, 1200U, 2.0, &layout));
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
    CHECK(hashes[0] == UINT64_C(0xdf5fbc87552662d8) &&
        hashes[1] == UINT64_C(0xa327472149f7d66d) &&
        hashes[2] == UINT64_C(0xffaf725096979e54));
    CHECK(hashes[0] != hashes[1] && hashes[1] != hashes[2]);

    static const char paced[] =
        "<svg width='16' height='8'><rect id='dot' x='0' y='0' width='4' height='4'>"
        "<animate attributeName='x' values='0;10' "
        "dur='1s' calcMode='paced'/></rect></svg>";
    document = (svg_shape_document){0};
    CHECK(decode(paced, &document));
    viewer_timeline_init(&timeline);
    viewer_timeline_reset(&timeline, &document);
    CHECK(open_easing_dock(&timeline));
    timeline.selected_key = 0U;
    CHECK(!viewer_timeline_easing_enabled(&timeline));
    CHECK(viewer_timeline_layout_compute(&timeline, 900U, 1200U, 2.0, &layout));
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
    viewer_editor *editor = viewer_editor_create(bytes, UI_THEME_DARK, 900U, 1200U);
    CHECK(editor != NULL);
    viewer_timeline timeline;
    viewer_timeline_init(&timeline);
    viewer_timeline_reset(&timeline, &document);
    CHECK(open_easing_dock(&timeline));
    timeline.selected_key = 1U;
    viewer_timeline_layout layout;
    CHECK(viewer_timeline_layout_compute(&timeline, 900U, 1200U, 2.0, &layout));
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
    editor = viewer_editor_create(bytes, UI_THEME_DARK, 900U, 1200U);
    CHECK(editor != NULL);
    viewer_timeline_init(&timeline);
    viewer_timeline_reset(&timeline, &document);
    CHECK(open_easing_dock(&timeline));
    timeline.selected_key = 1U;
    CHECK(viewer_timeline_layout_compute(&timeline, 900U, 1200U, 2.0, &layout));
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
    editor = viewer_editor_create(linear_bytes, UI_THEME_DARK, 900U, 1200U);
    CHECK(editor != NULL);
    viewer_timeline_init(&timeline);
    viewer_timeline_reset(&timeline, &document);
    CHECK(open_easing_dock(&timeline));
    timeline.selected_key = 1U;
    CHECK(viewer_timeline_layout_compute(&timeline, 900U, 1200U, 2.0, &layout));
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

static bool test_docked_timeline_key_drag_edits_source(void)
{
    static const char source[] =
        "<svg width='48' height='16'><rect id='dot' x='0' y='2' width='8' height='8'>"
        "<animate attributeName='x' values='0;5;10;15' keyTimes='0;0.25;0.5;1' "
        "dur='2s' calcMode='linear'/></rect></svg>";
    base_bytes bytes = {(const uint8_t *)source, sizeof(source) - 1U};
    svg_shape_document document = {0};
    CHECK(decode(source, &document));
    viewer_editor *editor = viewer_editor_create(bytes, UI_THEME_DARK, 900U, 1200U);
    CHECK(editor != NULL);
    viewer_timeline timeline;
    viewer_timeline_init(&timeline);
    viewer_timeline_reset(&timeline, &document);
    CHECK(timeline.dock_open || viewer_timeline_dock_set_open(&timeline, true));
    viewer_timeline_layout layout;
    CHECK(viewer_timeline_layout_compute(&timeline, 900U, 1200U, 2.0, &layout));
    CHECK(layout.visible_track_count == 1U);
    double key_x = (double)layout.ruler.x + 0.25 * layout.ruler.width;
    double row_y = (double)layout.track_rows[0].y +
        (double)layout.track_rows[0].height / 2.0;
    platform_event event = {
        .type = PLATFORM_EVENT_POINTER_DOWN,
        .button = 0U,
        .logical_x = (int32_t)floor(key_x / 2.0),
        .logical_y = (int32_t)floor(row_y / 2.0)
    };
    CHECK(viewer_timeline_process_event(&timeline, &event, editor,
        900U, 1200U, 2.0));
    CHECK(timeline.dragging_key && timeline.selected_key == 1U);

    event.type = PLATFORM_EVENT_POINTER_MOVE;
    event.logical_x = (int32_t)floor(((double)layout.ruler.x +
        0.375 * layout.ruler.width) / 2.0);
    CHECK(viewer_timeline_process_event(&timeline, &event, editor,
        900U, 1200U, 2.0));
    event.type = PLATFORM_EVENT_POINTER_UP;
    CHECK(viewer_timeline_process_event(&timeline, &event, editor,
        900U, 1200U, 2.0));
    CHECK(viewer_timeline_take_editor_mutation(&timeline));

    size_t capacity = viewer_editor_byte_length(editor) + 1U;
    uint8_t *edited = malloc(capacity);
    CHECK(edited != NULL);
    size_t length = 0U;
    CHECK(viewer_editor_materialize(editor, edited, capacity, &length));
    edited[length] = 0U;
    svg_shape_document changed = {0};
    CHECK(decode((const char *)edited, &changed));
    CHECK(fabs(changed.animation.tracks[0].keys[1].time - 0.375) < 0.01);
    free(edited);
    svg_shape_document_destroy(&changed);
    viewer_timeline_destroy(&timeline);
    viewer_editor_destroy(editor);
    svg_shape_document_destroy(&document);
    return true;
}

static const char field_edit_source[] =
    "<svg width='48' height='16'><rect id='dot' x='0' y='2' width='8' height='8'>"
    "<animate attributeName='x' values='0;5;10;15' keyTimes='0;0.25;0.5;1' "
    "dur='2s' calcMode='spline' keySplines='0.10 0.20 0.30 0.40; "
    "0.22 0.32 0.42 0.52;0.40 0.50 0.60 0.70'/></rect></svg>";

static bool key_event(viewer_timeline *timeline, viewer_editor *editor,
    const viewer_timeline_layout *layout, uint16_t key_code, char text)
{
    platform_event event = {.type = PLATFORM_EVENT_KEY_DOWN, .key_code = key_code};
    if (text != '\0') {
        event.text[0] = text;
        event.text_len = 1U;
    }
    return viewer_timeline_easing_process_event(timeline, editor, &event, layout);
}

/* Splits the keySplines attribute of edited into trimmed tuples. */
static bool split_splines(const char *edited, char tuples[3][96])
{
    const char *start = strstr(edited, "keySplines='");
    if (start == NULL) return false;
    start += strlen("keySplines='");
    const char *end = strchr(start, '\'');
    if (end == NULL) return false;
    for (size_t index = 0U; index < 3U; ++index) {
        while (start < end && *start == ' ') start++;
        const char *stop = start;
        while (stop < end && *stop != ';') stop++;
        const char *trim = stop;
        while (trim > start && trim[-1] == ' ') trim--;
        if ((size_t)(trim - start) >= 96U) return false;
        memcpy(tuples[index], start, (size_t)(trim - start));
        tuples[index][trim - start] = '\0';
        start = stop < end ? stop + 1 : stop;
    }
    return true;
}

/* Types input into easing field 0 of key 1 and commits with Return. */
static bool type_into_field(const char *input, bool *out_mutated,
    char tuples[3][96], double *out_decoded, bool *out_snapshot_matches)
{
    base_bytes bytes = {(const uint8_t *)field_edit_source,
        sizeof(field_edit_source) - 1U};
    svg_shape_document document = {0};
    CHECK(decode(field_edit_source, &document));
    viewer_editor *editor = viewer_editor_create(bytes, UI_THEME_DARK, 900U, 1200U);
    CHECK(editor != NULL);
    viewer_timeline timeline;
    viewer_timeline_init(&timeline);
    viewer_timeline_reset(&timeline, &document);
    CHECK(open_easing_dock(&timeline));
    timeline.selected_key = 1U;
    viewer_timeline_layout layout;
    CHECK(viewer_timeline_layout_compute(&timeline, 900U, 1200U, 2.0, &layout));
    viewer_timeline_easing_layout easing;
    CHECK(viewer_timeline_easing_layout_compute(&layout, &easing));
    platform_event event = {.type = PLATFORM_EVENT_POINTER_DOWN,
        .logical_x = (int32_t)((easing.fields[0].x + easing.fields[0].width / 2U) / 2U),
        .logical_y = (int32_t)((easing.fields[0].y + easing.fields[0].height / 2U) / 2U)};
    CHECK(viewer_timeline_easing_process_event(&timeline, editor, &event, &layout));
    event.type = PLATFORM_EVENT_POINTER_UP;
    CHECK(viewer_timeline_easing_process_event(&timeline, editor, &event, &layout));
    CHECK(timeline.easing_has_focus && timeline.easing_focused_field == 0U);
    (void)viewer_timeline_take_editor_mutation(&timeline);
    for (size_t index = 0U; index < 24U; ++index)
        CHECK(key_event(&timeline, editor, &layout, 51U, '\0'));
    for (const char *cursor = input; *cursor != '\0'; ++cursor)
        CHECK(key_event(&timeline, editor, &layout, 0U, *cursor));
    CHECK(key_event(&timeline, editor, &layout, 36U, '\0'));
    *out_mutated = viewer_timeline_take_editor_mutation(&timeline);
    size_t capacity = viewer_editor_byte_length(editor) + 1U;
    uint8_t *edited = malloc(capacity);
    CHECK(edited != NULL);
    size_t length = 0U;
    CHECK(viewer_editor_materialize(editor, edited, capacity, &length));
    edited[length] = 0U;
    bool okay = split_splines((const char *)edited, tuples);
    svg_shape_document decoded = {0};
    okay = okay && decode((const char *)edited, &decoded);
    if (okay) *out_decoded = decoded.animation.tracks[0].splines[1][0];
    char expected[40];
    (void)snprintf(expected, sizeof(expected), "%.17g",
        timeline.easing_fields[0].value);
    *out_snapshot_matches = okay && viewer_timeline_accessibility_snapshot(&timeline,
        &layout, UI_THEME_DARK, true, &semantics_scratch) &&
        snapshot_has_value(&semantics_scratch, UI_ROLE_SLIDER, "Spline x1", expected);
    free(edited);
    svg_shape_document_destroy(&decoded);
    viewer_timeline_destroy(&timeline);
    viewer_editor_destroy(editor);
    svg_shape_document_destroy(&document);
    return okay;
}

static bool check_accepted_field_value(const char *input, double expected)
{
    char tuples[3][96];
    bool mutated = false, snapshot_matches = false;
    double decoded = -1.0;
    CHECK(type_into_field(input, &mutated, tuples, &decoded, &snapshot_matches));
    CHECK(mutated);
    CHECK(strcmp(tuples[0], "0.10 0.20 0.30 0.40") == 0);
    CHECK(strcmp(tuples[2], "0.40 0.50 0.60 0.70") == 0);
    char *rest = NULL;
    CHECK(strtod(tuples[1], &rest) == expected);
    CHECK(strtod(rest, &rest) == 0.32 && strtod(rest, &rest) == 0.42 &&
        strtod(rest, &rest) == 0.52 && *rest == '\0');
    CHECK(decoded == expected);
    CHECK(snapshot_matches);
    return true;
}

static bool check_rejected_field_value(const char *input)
{
    char tuples[3][96];
    bool mutated = true, snapshot_matches = false;
    double decoded = -1.0;
    CHECK(type_into_field(input, &mutated, tuples, &decoded, &snapshot_matches));
    CHECK(!mutated);
    CHECK(strcmp(tuples[0], "0.10 0.20 0.30 0.40") == 0);
    CHECK(strcmp(tuples[1], "0.22 0.32 0.42 0.52") == 0);
    CHECK(strcmp(tuples[2], "0.40 0.50 0.60 0.70") == 0);
    CHECK(decoded == 0.22);
    return true;
}

static bool test_easing_numeric_field_edits(void)
{
    CHECK(check_accepted_field_value("0.37", 0.37));
    CHECK(check_accepted_field_value("0", 0.0));
    CHECK(check_accepted_field_value("1", 1.0));
    CHECK(check_rejected_field_value("1.5"));
    CHECK(check_rejected_field_value("-0.1"));
    CHECK(check_rejected_field_value(""));
    CHECK(check_rejected_field_value("abc"));
    return true;
}

static bool click_field(viewer_timeline *timeline, viewer_editor *editor,
    const viewer_timeline_layout *layout, const viewer_timeline_easing_layout *easing,
    size_t field)
{
    ui_rect rect = easing->fields[field];
    platform_event event = {.type = PLATFORM_EVENT_POINTER_DOWN,
        .logical_x = (int32_t)((rect.x + rect.width / 2U) / 2U),
        .logical_y = (int32_t)((rect.y + rect.height / 2U) / 2U)};
    CHECK(viewer_timeline_easing_process_event(timeline, editor, &event, layout));
    event.type = PLATFORM_EVENT_POINTER_UP;
    CHECK(viewer_timeline_easing_process_event(timeline, editor, &event, layout));
    return timeline->easing_focused_field == field;
}

static bool type_and_return(viewer_timeline *timeline, viewer_editor *editor,
    const viewer_timeline_layout *layout, const char *input)
{
    for (size_t index = 0U; index < 24U; ++index)
        CHECK(key_event(timeline, editor, layout, 51U, '\0'));
    for (const char *cursor = input; *cursor != '\0'; ++cursor)
        CHECK(key_event(timeline, editor, layout, 0U, *cursor));
    CHECK(key_event(timeline, editor, layout, 36U, '\0'));
    return true;
}

/* Rejected or never-committed x1 draft, then a valid commit of x2; x1 keeps its source value. */
static bool check_unentered_field_does_not_leak(const char *draft, bool press_return)
{
    base_bytes bytes = {(const uint8_t *)field_edit_source,
        sizeof(field_edit_source) - 1U};
    svg_shape_document document = {0};
    CHECK(decode(field_edit_source, &document));
    viewer_editor *editor = viewer_editor_create(bytes, UI_THEME_DARK, 900U, 1200U);
    CHECK(editor != NULL);
    viewer_timeline timeline;
    viewer_timeline_init(&timeline);
    viewer_timeline_reset(&timeline, &document);
    CHECK(open_easing_dock(&timeline));
    timeline.selected_key = 1U;
    viewer_timeline_layout layout;
    CHECK(viewer_timeline_layout_compute(&timeline, 900U, 1200U, 2.0, &layout));
    viewer_timeline_easing_layout easing;
    CHECK(viewer_timeline_easing_layout_compute(&layout, &easing));
    CHECK(click_field(&timeline, editor, &layout, &easing, 0U));
    if (press_return) CHECK(type_and_return(&timeline, editor, &layout, draft));
    else {
        for (size_t index = 0U; index < 24U; ++index)
            CHECK(key_event(&timeline, editor, &layout, 51U, '\0'));
        for (const char *cursor = draft; *cursor != '\0'; ++cursor)
            CHECK(key_event(&timeline, editor, &layout, 0U, *cursor));
    }
    CHECK(click_field(&timeline, editor, &layout, &easing, 2U));
    CHECK(type_and_return(&timeline, editor, &layout, "0.41"));
    size_t capacity = viewer_editor_byte_length(editor) + 1U;
    uint8_t *edited = malloc(capacity);
    CHECK(edited != NULL);
    size_t length = 0U;
    CHECK(viewer_editor_materialize(editor, edited, capacity, &length));
    edited[length] = 0U;
    char tuples[3][96];
    svg_shape_document decoded = {0};
    bool okay = split_splines((const char *)edited, tuples) &&
        decode((const char *)edited, &decoded);
    if (okay) {
        const double *tuple = decoded.animation.tracks[0].splines[1];
        if (tuple[0] != 0.22)
            (void)fprintf(stderr, "draft '%s': x1 became %.17g\n", draft, tuple[0]);
        okay = tuple[0] == 0.22 && tuple[1] == 0.32 && tuple[2] == 0.41 &&
            tuple[3] == 0.52 && strcmp(tuples[0], "0.10 0.20 0.30 0.40") == 0 &&
            strcmp(tuples[2], "0.40 0.50 0.60 0.70") == 0;
    }
    free(edited);
    svg_shape_document_destroy(&decoded);
    viewer_timeline_destroy(&timeline);
    viewer_editor_destroy(editor);
    svg_shape_document_destroy(&document);
    return okay;
}

static bool test_easing_rejected_field_does_not_leak(void)
{
    CHECK(check_unentered_field_does_not_leak("1.5", true));
    CHECK(check_unentered_field_does_not_leak("-0.1", true));
    CHECK(check_unentered_field_does_not_leak("abc", true));
    CHECK(check_unentered_field_does_not_leak("0.3", false));
    return true;
}

int main(void)
{
    if (!test_easing_accessibility_and_raster_goldens() ||
        !test_easing_pointer_and_keyboard_edits() ||
        !test_easing_numeric_field_edits() ||
        !test_easing_rejected_field_does_not_leak() ||
        !test_docked_timeline_key_drag_edits_source()) return 1;
    (void)puts("viewer timeline keyframes: passed");
    return 0;
}
