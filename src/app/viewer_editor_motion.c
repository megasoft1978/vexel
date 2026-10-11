#include "viewer_editor_internal.h"

#include "platform/platform_text_ui.h"
#include "viewer_chrome.h"

#include <math.h>

#define EDITOR_SIDEBAR_MOTION_MS 160U
#define EDITOR_SPLITTER_KEY_STEP 8U

static double editor_ui_scale(const viewer_editor *editor)
{
    return (double)viewer_editor_ui_scale_percent(editor) / 100.0;
}

static uint16_t editor_clamp_panel_width(uint32_t id, double proposed)
{
    double minimum = id == EDITOR_SPLITTER_STRUCTURE_ID ? 184.0 : 240.0;
    double maximum = id == EDITOR_SPLITTER_STRUCTURE_ID ? 320.0 : 440.0;
    if (!isfinite(proposed)) return (uint16_t)minimum;
    if (proposed < minimum) proposed = minimum;
    if (proposed > maximum) proposed = maximum;
    return (uint16_t)llround(proposed);
}

static uint32_t editor_transition_color(const viewer_editor *editor,
                                        ui_control_state state)
{
    ui_tokens tokens = ui_theme_tokens_accented(editor->theme,
        editor->increased_contrast, editor->system_accent_srgb);
    if (state == UI_CONTROL_HOVER) return tokens.panel_hover;
    if (state == UI_CONTROL_PRESSED) return tokens.pressed_fill;
    if (state == UI_CONTROL_SELECTED) return tokens.selection_fill;
    if (state == UI_CONTROL_DISABLED) return tokens.panel_raised;
    return 0U;
}

static uint32_t mix_color(uint32_t left, uint32_t right, double amount)
{
    if (amount < 0.0) amount = 0.0;
    if (amount > 1.0) amount = 1.0;
    uint32_t out = 0U;
    for (unsigned shift = 0U; shift < 32U; shift += 8U) {
        uint32_t a = (left >> shift) & 255U;
        uint32_t b = (right >> shift) & 255U;
        uint32_t channel = (uint32_t)llround((double)a * (1.0 - amount) +
            (double)b * amount);
        out |= channel << shift;
    }
    return out;
}

static bool point_in_rect(ui_rect bounds, uint32_t x, uint32_t y)
{
    return x >= bounds.x && y >= bounds.y &&
        x - bounds.x < bounds.width && y - bounds.y < bounds.height;
}

bool editor_splitter_pointer_down(viewer_editor *editor, uint32_t x, uint32_t y,
                                  uint8_t click_count)
{
    if (editor == NULL || !editor->modern_style) return false;
    if (!editor->tree_ready) editor_build_tree(editor);
    if (!editor->tree_ready) return false;
    uint32_t id = 0U;
    for (size_t index = 0U; index < editor->tree.count; ++index) {
        uint32_t candidate = editor->tree.nodes[index].spec.id;
        if ((candidate == EDITOR_SPLITTER_STRUCTURE_ID ||
             candidate == EDITOR_SPLITTER_INSPECTOR_ID) &&
            point_in_rect(editor->tree.nodes[index].bounds, x, y)) {
            id = candidate;
            break;
        }
    }
    if (id == 0U) return false;
    if (click_count >= 2U) {
        uint16_t structure = EDITOR_LEFT_WIDTH;
        uint16_t inspector = EDITOR_RIGHT_WIDTH;
        if (viewer_editor_set_panel_widths(editor, structure, inspector))
            (void)platform_text_ui_set_panel_widths(structure, inspector);
        editor->focus_id = id;
        editor->tree_ready = false;
        editor->redraw_requested = true;
        return true;
    }
    editor->splitter_drag_id = id;
    editor->splitter_drag_start_x = x;
    editor->splitter_drag_start_width = id == EDITOR_SPLITTER_STRUCTURE_ID ?
        editor->structure_width_points : editor->inspector_width_points;
    editor->splitter_dragging = true;
    editor->focus_id = id;
    editor->tree_ready = false;
    editor->redraw_requested = true;
    return true;
}

bool editor_splitter_pointer_move(viewer_editor *editor, uint32_t x, uint32_t y)
{
    (void)y;
    if (editor == NULL || !editor->splitter_dragging) return false;
    double scale = editor->backing_scale * editor_ui_scale(editor);
    if (!isfinite(scale) || scale <= 0.0) return false;
    double delta = ((double)x - (double)editor->splitter_drag_start_x) / scale;
    if (editor->splitter_drag_id == EDITOR_SPLITTER_INSPECTOR_ID) delta = -delta;
    double proposed = (double)editor->splitter_drag_start_width + delta;
    uint16_t width = editor_clamp_panel_width(editor->splitter_drag_id, proposed);
    uint16_t structure = editor->structure_width_points;
    uint16_t inspector = editor->inspector_width_points;
    if (editor->splitter_drag_id == EDITOR_SPLITTER_STRUCTURE_ID) structure = width;
    else inspector = width;
    bool changed = viewer_editor_set_panel_widths(editor, structure, inspector);
    if (changed) editor->focus_id = editor->splitter_drag_id;
    return changed;
}

void editor_splitter_pointer_up(viewer_editor *editor)
{
    if (editor == NULL || !editor->splitter_dragging) return;
    editor->splitter_dragging = false;
    editor->splitter_drag_id = 0U;
    (void)platform_text_ui_set_panel_widths(editor->structure_width_points,
        editor->inspector_width_points);
    editor->redraw_requested = true;
}

bool editor_splitter_key(viewer_editor *editor, uint16_t key_code,
                         uint32_t modifiers)
{
    if (editor == NULL || (editor->focus_id != EDITOR_SPLITTER_STRUCTURE_ID &&
        editor->focus_id != EDITOR_SPLITTER_INSPECTOR_ID) ||
        (key_code != 123U && key_code != 124U)) return false;
    int direction = key_code == 124U ? 1 : -1;
    if (editor->focus_id == EDITOR_SPLITTER_INSPECTOR_ID) direction = -direction;
    uint16_t step = (modifiers & PLATFORM_MODIFIER_OPTION) != 0U ? 1U :
        EDITOR_SPLITTER_KEY_STEP;
    uint16_t current = editor->focus_id == EDITOR_SPLITTER_STRUCTURE_ID ?
        editor->structure_width_points : editor->inspector_width_points;
    double proposed = (double)current + (double)direction * (double)step;
    uint16_t next = editor_clamp_panel_width(editor->focus_id, proposed);
    if (next == current) return true;
    uint16_t structure = editor->structure_width_points;
    uint16_t inspector = editor->inspector_width_points;
    if (editor->focus_id == EDITOR_SPLITTER_STRUCTURE_ID) structure = next;
    else inspector = next;
    (void)viewer_editor_set_panel_widths(editor, structure, inspector);
    (void)platform_text_ui_set_panel_widths(structure, inspector);
    return true;
}

void viewer_editor_set_motion_time(viewer_editor *editor, double seconds)
{
    if (editor == NULL || !isfinite(seconds) || seconds < 0.0) return;
    editor->motion_time_seconds = seconds;
}

bool viewer_editor_motion_tick(viewer_editor *editor, double seconds)
{
    if (editor == NULL || !isfinite(seconds) || seconds < 0.0) return false;
    editor->motion_time_seconds = seconds;
    bool changed = false;
    if (editor->sidebar_motion_active) {
        double elapsed = seconds - editor->sidebar_motion_start_seconds;
        if (elapsed < 0.0) elapsed = 0.0;
        double progress = elapsed * 1000.0 / (double)EDITOR_SIDEBAR_MOTION_MS;
        if (progress > 1.0) progress = 1.0;
        double inverse = 1.0 - progress;
        double structure = (double)editor->sidebar_motion_start_structure * inverse +
            (double)editor->sidebar_motion_target_structure * progress;
        double inspector = (double)editor->sidebar_motion_start_inspector * inverse +
            (double)editor->sidebar_motion_target_inspector * progress;
        editor->layout_structure_width_points = (uint16_t)llround(structure);
        editor->layout_inspector_width_points = (uint16_t)llround(inspector);
        if (progress >= 1.0) {
            editor->sidebar_motion_active = false;
            editor->layout_structure_width_points = editor->sidebar_motion_target_structure;
            editor->layout_inspector_width_points = editor->sidebar_motion_target_inspector;
        }
        changed = true;
    }
    for (size_t index = 0U; index < EDITOR_MOTION_CONTROL_CAP; ++index) {
        editor_control_transition *transition = &editor->control_transitions[index];
        if (!transition->active) continue;
        changed = true;
        double elapsed = seconds - transition->start_seconds;
        if (elapsed * 1000.0 >= (double)transition->duration_ms) {
            transition->active = false;
            changed = true;
        }
    }
    if (changed) {
        editor->tree_ready = false;
        editor->redraw_requested = true;
    }
    return changed;
}

bool viewer_editor_motion_active(const viewer_editor *editor)
{
    if (editor == NULL) return false;
    if (editor->sidebar_motion_active) return true;
    for (size_t index = 0U; index < EDITOR_MOTION_CONTROL_CAP; ++index)
        if (editor->control_transitions[index].active) return true;
    return false;
}

void editor_motion_control_transition(viewer_editor *editor, uint32_t id,
    ui_control_state from_state, ui_control_state to_state, uint16_t duration_ms)
{
    if (editor == NULL || editor->reduced_motion || duration_ms == 0U) return;
    size_t slot = EDITOR_MOTION_CONTROL_CAP;
    uint32_t from_color = editor_transition_color(editor, from_state);
    for (size_t index = 0U; index < EDITOR_MOTION_CONTROL_CAP; ++index) {
        if (editor->control_transitions[index].active &&
            editor->control_transitions[index].id == id) {
            editor_control_transition *prior = &editor->control_transitions[index];
            double elapsed = editor->motion_time_seconds - prior->start_seconds;
            double progress = elapsed <= 0.0 ? 0.0 :
                elapsed * 1000.0 / (double)prior->duration_ms;
            if (progress > 1.0) progress = 1.0;
            from_color = mix_color(prior->from_color, prior->to_color, progress);
            slot = index;
            break;
        }
        if (!editor->control_transitions[index].active && slot == EDITOR_MOTION_CONTROL_CAP)
            slot = index;
    }
    if (slot == EDITOR_MOTION_CONTROL_CAP) slot = 0U;
    editor->control_transitions[slot] = (editor_control_transition){.id = id,
        .from_state = from_state, .to_state = to_state,
        .start_seconds = editor->motion_time_seconds,
        .duration_ms = duration_ms, .from_color = from_color,
        .to_color = editor_transition_color(editor, to_state), .active = true};
}
