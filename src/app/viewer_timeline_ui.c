#include "viewer_timeline.h"

#include "platform/macos/platform_accessibility.h"
#include "platform/platform_text_ui.h"
#include "ui/ui_widgets.h"
#include "viewer_chrome.h"
#include "viewer_editor.h"

#include <math.h>
#include <stdio.h>
#include <string.h>

#define VIEWER_TIMELINE_A11Y_BASE UINT32_C(0x71000000)
#define VIEWER_TIMELINE_A11Y_TEXT_LIMIT 512U

enum {
    TIMELINE_A11Y_PANEL = VIEWER_TIMELINE_A11Y_BASE + 1U,
    TIMELINE_A11Y_PLAY,
    TIMELINE_A11Y_SLIDER,
    TIMELINE_A11Y_LOOP,
    TIMELINE_A11Y_TRACKS,
    TIMELINE_A11Y_ROW_BASE = VIEWER_TIMELINE_A11Y_BASE + 16U
};

/* The app draws and publishes accessibility on its main thread. */
static ui_tree timeline_accessibility_tree;
static ui_semantics_snapshot editor_semantics_scratch;
static ui_semantics_snapshot timeline_semantics_scratch;

static uint32_t widget_scale_for(double backing_scale)
{
    long scale = lround(backing_scale);
    if (scale < 1L) scale = 1L;
    if (scale > 3L) scale = 3L;
    return (uint32_t)scale;
}

static bool rect_to_widget(ui_rect pixels, uint32_t scale, ui_rect *out)
{
    if (out == NULL || scale == 0U || pixels.width == 0U || pixels.height == 0U)
        return false;
    *out = (ui_rect){pixels.x / scale, pixels.y / scale,
        pixels.width / scale + (pixels.width % scale != 0U ? 1U : 0U),
        pixels.height / scale + (pixels.height % scale != 0U ? 1U : 0U)};
    return out->width > 0U && out->height > 0U;
}

static void blend_panel_pixel(uint8_t *destination, uint32_t color)
{
    uint32_t alpha = color & 255U;
    if (alpha == 255U) {
        destination[0] = (uint8_t)(color >> 24U);
        destination[1] = (uint8_t)(color >> 16U);
        destination[2] = (uint8_t)(color >> 8U);
        destination[3] = 255U;
        return;
    }
    if (alpha == 0U) return;
    uint32_t inverse = 255U - alpha;
    uint32_t old_alpha = destination[3];
    uint32_t out_alpha = alpha + (old_alpha * inverse + 127U) / 255U;
    if (out_alpha == 0U) return;
    for (size_t channel = 0U; channel < 3U; ++channel) {
        uint32_t shift = 24U - (uint32_t)channel * 8U;
        uint32_t source = (color >> shift) & 255U;
        uint32_t numerator = source * alpha * 255U +
            (uint32_t)destination[channel] * old_alpha * inverse;
        destination[channel] = (uint8_t)((numerator + out_alpha * 127U) /
            (out_alpha * 255U));
    }
    destination[3] = (uint8_t)out_alpha;
}

static void paint_panel_background(const ui_widget_canvas *canvas, ui_rect panel)
{
    uint64_t right = (uint64_t)panel.x + panel.width;
    uint64_t bottom = (uint64_t)panel.y + panel.height;
    uint32_t x_end = right > canvas->width ? canvas->width : (uint32_t)right;
    uint32_t y_end = bottom > canvas->height ? canvas->height : (uint32_t)bottom;
    if (panel.x >= x_end || panel.y >= y_end) return;
    for (uint32_t y = panel.y; y < y_end; ++y) {
        for (uint32_t x = panel.x; x < x_end; ++x) {
            blend_panel_pixel(canvas->rgba + (size_t)y * canvas->stride +
                (size_t)x * 4U, canvas->tokens.panel);
        }
    }
}

static bool paint_widget(ui_widget_canvas *canvas, ui_widget_kind kind,
    ui_rect pixels, ui_control_state state,
    const char *label, size_t label_length, const char *value, size_t value_length,
    double numeric_value, bool checked)
{
    ui_rect bounds;
    if (!rect_to_widget(pixels, canvas->backing_scale, &bounds)) return false;
    ui_widget_draw_spec spec = {
        .kind = kind,
        .state = state,
        .bounds = bounds,
        .label = label,
        .label_length = label_length,
        .value = value,
        .value_length = value_length,
        .numeric_value = numeric_value,
        .checked = checked,
        .expanded = true
    };
    return ui_widget_paint(canvas, &spec);
}

static size_t utf8_prefix(const char *text, size_t length, size_t maximum)
{
    size_t amount = length < maximum ? length : maximum;
    while (amount > 0U && amount < length &&
        (((uint8_t)text[amount] & 0xc0U) == 0x80U)) amount--;
    return amount;
}

static bool snapshot_append(ui_semantics_snapshot *destination,
    const ui_semantics_snapshot *source, const ui_semantics_node *node,
    uint32_t id, uint32_t parent_id)
{
    if (destination == NULL || source == NULL || node == NULL ||
        destination->count >= UI_MAX_NODES) return false;
    const char *label = NULL, *value = NULL;
    size_t label_length = 0U, value_length = 0U;
    if (!ui_semantics_text(source, node, false, &label, &label_length) ||
        !ui_semantics_text(source, node, true, &value, &value_length)) return false;
    label_length = utf8_prefix(label, label_length, VIEWER_TIMELINE_A11Y_TEXT_LIMIT);
    value_length = utf8_prefix(value, value_length, VIEWER_TIMELINE_A11Y_TEXT_LIMIT);
    if (label_length > UI_TEXT_TOTAL_MAX_BYTES - destination->text_bytes_used ||
        value_length > UI_TEXT_TOTAL_MAX_BYTES - destination->text_bytes_used - label_length)
        return false;
    ui_semantics_node *copy = &destination->nodes[destination->count];
    *copy = *node;
    copy->id = id;
    copy->parent_id = parent_id;
    copy->label_full_len = label_length;
    copy->value_full_len = value_length;
    copy->label_len = utf8_prefix(node->label, node->label_len, label_length);
    copy->value_len = utf8_prefix(node->value, node->value_len, value_length);
    copy->label_offset = destination->text_bytes_used;
    if (label_length > 0U)
        memcpy(destination->text_storage + copy->label_offset, label, label_length);
    copy->value_offset = copy->label_offset + label_length;
    if (value_length > 0U)
        memcpy(destination->text_storage + copy->value_offset, value, value_length);
    destination->text_bytes_used += label_length + value_length;
    destination->count++;
    return true;
}

static bool add_timeline_node(ui_tree *tree, uint32_t id, uint32_t parent,
    ui_role role, const char *label, const char *value, uint32_t state,
    ui_rect bounds)
{
    size_t label_length = label == NULL ? 0U : strlen(label);
    size_t value_length = value == NULL ? 0U : strlen(value);
    ui_node_spec spec = {
        .id = id,
        .parent_id = parent,
        .axis = UI_AXIS_VERTICAL,
        .role = role,
        .label = label,
        .label_len = label_length,
        .value = value,
        .value_len = value_length,
        .state = state
    };
    if (!ui_tree_add(tree, &spec)) return false;
    tree->nodes[tree->count - 1U].bounds = bounds;
    return true;
}

static bool build_timeline_tree(viewer_timeline *timeline,
    const viewer_timeline_layout *layout, ui_theme theme,
    bool increased_contrast, size_t row_limit)
{
    if (!ui_tree_init_accessible(&timeline_accessibility_tree, theme,
            increased_contrast)) return false;
    char time_value[48];
    double duration = viewer_timeline_duration(timeline);
    int time_amount = snprintf(time_value, sizeof(time_value), "%.2f / %.2fs",
        timeline->time_seconds, duration);
    if (time_amount < 0 || (size_t)time_amount >= sizeof(time_value)) return false;
    if (!add_timeline_node(&timeline_accessibility_tree, TIMELINE_A11Y_PANEL,
            UI_ROOT_ID, UI_ROLE_GROUP, "Animation timeline", NULL, 0U,
            layout->panel) ||
        !add_timeline_node(&timeline_accessibility_tree, TIMELINE_A11Y_PLAY,
            TIMELINE_A11Y_PANEL, UI_ROLE_BUTTON,
            timeline->playing ? "Pause animation" : "Play animation", NULL, 0U,
            layout->play_button) ||
        !add_timeline_node(&timeline_accessibility_tree, TIMELINE_A11Y_SLIDER,
            TIMELINE_A11Y_PANEL, UI_ROLE_SLIDER, "Timeline position",
            time_value, 0U, layout->scrubber) ||
        !add_timeline_node(&timeline_accessibility_tree, TIMELINE_A11Y_LOOP,
            TIMELINE_A11Y_PANEL, UI_ROLE_CHECKBOX, "Loop animation", NULL,
            timeline->loop ? UI_STATE_CHECKED : 0U, layout->loop_toggle) ||
        !add_timeline_node(&timeline_accessibility_tree, TIMELINE_A11Y_TRACKS,
            TIMELINE_A11Y_PANEL, UI_ROLE_LIST, "Animation tracks", NULL, 0U,
            layout->panel)) return false;
    if (row_limit > layout->visible_track_count) row_limit = layout->visible_track_count;
    for (size_t row = 0U; row < row_limit; ++row) {
        size_t track_index = timeline->first_track + row;
        char label[128] = {0};
        size_t label_length = 0U;
        if (!viewer_timeline_track_label(timeline, track_index, label,
                sizeof(label), &label_length)) return false;
        uint32_t state = track_index == timeline->selected_track ? UI_STATE_CHECKED : 0U;
        if (!add_timeline_node(&timeline_accessibility_tree,
                TIMELINE_A11Y_ROW_BASE + (uint32_t)row, TIMELINE_A11Y_TRACKS,
                UI_ROLE_ROW, label, NULL, state, layout->track_rows[row])) return false;
    }
    return ui_semantics_snapshot_build(&timeline_accessibility_tree,
        &timeline_semantics_scratch);
}

bool viewer_timeline_accessibility_snapshot(viewer_timeline *timeline,
    const viewer_timeline_layout *layout, ui_theme theme,
    bool increased_contrast, ui_semantics_snapshot *out_snapshot)
{
    if (timeline == NULL || layout == NULL || out_snapshot == NULL ||
        viewer_timeline_track_count(timeline) == 0U ||
        (theme != UI_THEME_DARK && theme != UI_THEME_LIGHT)) return false;
    if (!build_timeline_tree(timeline, layout, theme, increased_contrast,
            layout->visible_track_count)) return false;
    *out_snapshot = timeline_semantics_scratch;
    return true;
}

static bool publish_merged_accessibility(viewer_timeline *timeline,
    viewer_editor *editor, const viewer_timeline_layout *layout,
    uint32_t width, uint32_t height, double backing_scale, ui_theme theme,
    bool increased_contrast)
{
    if (!viewer_editor_semantics(editor, &editor_semantics_scratch)) return false;
    size_t editor_count = editor_semantics_scratch.count;
    if (editor_count == 0U || editor_count > UI_MAX_NODES) return false;
    size_t extras = UI_MAX_NODES - editor_count;
    bool include_editor = extras >= 5U;
    size_t row_limit = 0U;
    if (include_editor && extras > 5U) {
        row_limit = extras - 5U;
        if (row_limit > layout->visible_track_count)
            row_limit = layout->visible_track_count;
    }
    if (!build_timeline_tree(timeline, layout, theme, increased_contrast,
            row_limit)) return false;
    uint32_t root_id = UI_ROOT_ID;
    if (include_editor) {
        root_id = editor_semantics_scratch.nodes[0].id;
    } else editor_semantics_scratch = (ui_semantics_snapshot){0};
    for (size_t index = 1U; index < timeline_semantics_scratch.count; ++index) {
        const ui_semantics_node *node = &timeline_semantics_scratch.nodes[index];
        if (node->id < UI_ROOT_ID || node->parent_id < UI_ROOT_ID) return false;
        uint32_t id = VIEWER_TIMELINE_A11Y_BASE + (node->id - UI_ROOT_ID);
        uint32_t parent = node->parent_id == UI_ROOT_ID ? root_id :
            VIEWER_TIMELINE_A11Y_BASE + (node->parent_id - UI_ROOT_ID);
        if (!snapshot_append(&editor_semantics_scratch,
                &timeline_semantics_scratch, node, id, parent)) return false;
    }
    return platform_accessibility_update(&editor_semantics_scratch, width, height,
        backing_scale);
}

bool viewer_timeline_paint(viewer_timeline *timeline, viewer_editor *editor,
                           uint8_t *rgba, size_t capacity,
                           uint32_t width, uint32_t height, size_t stride,
                           double backing_scale)
{
    if (timeline == NULL || editor == NULL || rgba == NULL ||
        viewer_timeline_track_count(timeline) == 0U ||
        !isfinite(backing_scale) || backing_scale < 0.5 || backing_scale > 4.0)
        return false;
    viewer_timeline_layout layout;
    if (!viewer_timeline_layout_compute(timeline, width, height,
            backing_scale, &layout)) return false;
    ui_theme theme = UI_THEME_DARK;
    bool increased_contrast = false, reduced_motion = false;
    (void)platform_text_ui_get_preferences(&theme, &increased_contrast, &reduced_motion);
    (void)reduced_motion;
    uint32_t widget_scale = widget_scale_for(backing_scale);
    ui_widget_canvas canvas;
    if (!ui_widget_canvas_init(&canvas, rgba, capacity, width, height, stride,
            widget_scale, theme, increased_contrast, platform_text_ui_draw_widget, NULL)) return false;
    paint_panel_background(&canvas, layout.panel);

    ui_rect header = {layout.panel.x + 12U * widget_scale,
        layout.panel.y + 2U * widget_scale,
        layout.panel.width > 24U * widget_scale ?
            layout.panel.width - 24U * widget_scale : 0U,
        22U * widget_scale};
    (void)paint_widget(&canvas, UI_WIDGET_SECTION_HEADER, header, UI_CONTROL_REST,
        "Timeline", sizeof("Timeline") - 1U, NULL, 0U, 0.0, false);

    char time_text[48];
    double duration = viewer_timeline_duration(timeline);
    int time_amount = snprintf(time_text, sizeof(time_text), "%.2f / %.2fs",
        timeline->time_seconds, duration);
    if (time_amount < 0 || (size_t)time_amount >= sizeof(time_text))
        (void)snprintf(time_text, sizeof(time_text), "-- / --s");
    (void)paint_widget(&canvas,
        timeline->preview_failed ? UI_WIDGET_INLINE_ERROR : UI_WIDGET_HUD_PILL,
        layout.time_readout, timeline->preview_failed ? UI_CONTROL_INVALID : UI_CONTROL_REST,
        timeline->preview_failed ? "Preview unavailable" : time_text,
        timeline->preview_failed ? sizeof("Preview unavailable") - 1U : strlen(time_text),
        NULL, 0U, 0.0, false);
    (void)paint_widget(&canvas, UI_WIDGET_BUTTON_PRIMARY, layout.play_button,
        UI_CONTROL_REST, timeline->playing ? "Pause" : "Play",
        timeline->playing ? sizeof("Pause") - 1U : sizeof("Play") - 1U,
        NULL, 0U, 0.0, false);
    (void)paint_widget(&canvas, UI_WIDGET_TOGGLE, layout.loop_toggle,
        UI_CONTROL_REST, "Loop", sizeof("Loop") - 1U, NULL, 0U, 0.0,
        timeline->loop);
    double normalized = duration > 0.0 ? timeline->time_seconds / duration : 0.0;
    (void)paint_widget(&canvas, UI_WIDGET_SLIDER, layout.scrubber,
        UI_CONTROL_REST, "Timeline position", sizeof("Timeline position") - 1U,
        time_text, strlen(time_text), normalized, false);

    for (size_t row = 0U; row < layout.visible_track_count; ++row) {
        size_t track_index = timeline->first_track + row;
        char label[128] = {0};
        size_t label_length = 0U;
        if (!viewer_timeline_track_label(timeline, track_index, label,
                sizeof(label), &label_length)) continue;
        ui_control_state state = track_index == timeline->selected_track ?
            UI_CONTROL_SELECTED : UI_CONTROL_REST;
        (void)paint_widget(&canvas, UI_WIDGET_BUTTON_SECONDARY,
            layout.track_rows[row], state, label, label_length, NULL, 0U, 0.0, false);
    }
    (void)publish_merged_accessibility(timeline, editor, &layout, width, height,
        backing_scale, theme, increased_contrast);
    return true;
}
