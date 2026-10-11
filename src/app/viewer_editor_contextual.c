#include "viewer_editor_internal.h"

#include "document_model.h"
#include "viewer_chrome.h"
#include "ui/ui_widgets.h"
#include "platform/platform_text_ui.h"

#include <math.h>
#include <stdio.h>
#include <string.h>

#define EDITOR_CONTEXT_ACTION_CAP 4U
#define EDITOR_CONTEXT_ACTION_HEIGHT 32U
#define EDITOR_CONTEXT_ACTION_GAP 4U
#define EDITOR_CONTEXT_CANVAS_INSET 4U

static bool contextual_add(editor_context_action actions[EDITOR_CONTEXT_ACTION_CAP],
    size_t *count, uint32_t id, const char *label)
{
    if (actions == NULL || count == NULL || label == NULL ||
        *count >= EDITOR_CONTEXT_ACTION_CAP) return false;
    size_t length = strlen(label);
    if (length == 0U || length > UI_WIDGET_LABEL_MAX_BYTES) return false;
    actions[*count] = (editor_context_action){id, label, length, {0U, 0U, 0U, 0U}};
    ++*count;
    return true;
}

static bool contextual_selected_handle(viewer_editor *editor,
    size_t selection_index, document_handle *out_handle)
{
    return editor != NULL && out_handle != NULL &&
        selection_index < editor->canvas_selection_count &&
        document_edit_element_at_offset(editor->edit,
            editor->canvas_selection_offsets[selection_index], out_handle);
}

static bool contextual_find_attribute(viewer_editor *editor,
    document_handle handle, const char *name, size_t *out_index)
{
    if (editor == NULL || name == NULL || out_index == NULL) return false;
    size_t length = strlen(name);
    size_t count = document_edit_attribute_count(editor->edit, handle);
    if (count > 4096U) count = 4096U;
    for (size_t index = 0U; index < count; ++index) {
        document_edit_attribute attribute;
        base_bytes value;
        if (document_edit_attribute_at(editor->edit, handle, index, &attribute) &&
            document_edit_span_bytes(editor->edit, attribute.name, &value) &&
            value.len == length && memcmp(value.ptr, name, length) == 0) {
            *out_index = index;
            return true;
        }
    }
    return false;
}

static bool contextual_same_parent(viewer_editor *editor)
{
    if (editor == NULL || editor->canvas_selection_count < 2U ||
        editor->canvas_selection_count > EDITOR_CANVAS_SELECTION_CAP) return false;
    document_handle first = {0U};
    if (!contextual_selected_handle(editor, 0U, &first)) return false;
    document_handle parent = {0U};
    if (!document_model_parent(editor_model(editor), first, &parent)) return false;
    for (size_t index = 1U; index < editor->canvas_selection_count; ++index) {
        document_handle current = {0U}, current_parent = {0U};
        if (!contextual_selected_handle(editor, index, &current) ||
            !document_model_parent(editor_model(editor), current, &current_parent) ||
            !editor_same_handle(parent, current_parent)) return false;
    }
    return true;
}

static bool contextual_build(viewer_editor *editor,
    const svg_shape_document *document,
    editor_context_action actions[EDITOR_CONTEXT_ACTION_CAP], size_t *out_count)
{
    if (editor == NULL || actions == NULL || out_count == NULL) return false;
    *out_count = 0U;
    size_t selected_count = editor->canvas_selection_count;
    if (editor->draw_tool != VIEWER_EDITOR_TOOL_SELECT ||
        editor->active_tool != UI_ICON_SELECT || editor->node_edit_active ||
        editor->field_kind != EDITOR_FIELD_NONE ||
        editor->canvas_gesture != EDITOR_CANVAS_GESTURE_NONE ||
        selected_count == 0U || selected_count > EDITOR_CANVAS_SELECTION_CAP)
        return true;
    bool locked = editor_canvas_selection_is_locked(editor);
    if (selected_count > 1U) {
        svg_shape_selection bounds[EDITOR_CANVAS_SELECTION_CAP];
        bool align_valid = document != NULL;
        for (size_t index = 0U; align_valid && index < selected_count; ++index)
            align_valid = svg_shape_element_at_offset(document,
                editor->canvas_selection_offsets[index], &bounds[index]);
        if (!locked && align_valid)
            (void)contextual_add(actions, out_count, EDITOR_CONTEXT_ALIGN, "Align");
        if (!locked && contextual_same_parent(editor))
            (void)contextual_add(actions, out_count, EDITOR_CONTEXT_GROUP, "Group");
        if (!locked)
            (void)contextual_add(actions, out_count, EDITOR_CONTEXT_DUPLICATE,
                "Duplicate");
        if (!locked)
            (void)contextual_add(actions, out_count, EDITOR_CONTEXT_DELETE, "Delete");
        return true;
    }
    document_handle handle = {0U};
    if (!contextual_selected_handle(editor, 0U, &handle)) return true;
    base_bytes tag = {0};
    bool path = document_edit_tag_name(editor->edit, handle, &tag) &&
        tag.len == 4U && memcmp(tag.ptr, "path", 4U) == 0;
    if (path && !locked) {
        size_t data_index = 0U;
        if (contextual_find_attribute(editor, handle, "d", &data_index))
            (void)contextual_add(actions, out_count, EDITOR_CONTEXT_EDIT_NODES,
                "Edit nodes");
    }
    if (!locked && contextual_find_attribute(editor, handle, "fill", &(size_t){0U}))
        (void)contextual_add(actions, out_count, EDITOR_CONTEXT_FILL, "Fill");
    if (!locked && contextual_find_attribute(editor, handle, "stroke", &(size_t){0U}))
        (void)contextual_add(actions, out_count, EDITOR_CONTEXT_STROKE, "Stroke");
    if (!path && !locked && *out_count < EDITOR_CONTEXT_ACTION_CAP) {
        (void)contextual_add(actions, out_count, EDITOR_CONTEXT_DUPLICATE,
            "Duplicate");
        if (*out_count < EDITOR_CONTEXT_ACTION_CAP)
            (void)contextual_add(actions, out_count, EDITOR_CONTEXT_DELETE, "Delete");
    }
    return true;
}

static uint32_t contextual_action_width_points(const editor_context_action *action)
{
    if (action == NULL) return 0U;
    uint32_t width = (uint32_t)action->label_length * 7U + 20U;
    if (width < 44U) width = 44U;
    if (width > 92U) width = 92U;
    return width;
}

static bool rect_intersects(ui_rect left, ui_rect right)
{
    return left.x < right.x + right.width && right.x < left.x + left.width &&
        left.y < right.y + right.height && right.y < left.y + left.height;
}

static bool point_inside(ui_rect rect, double x, double y)
{
    return x >= rect.x && y >= rect.y &&
        x - rect.x < rect.width && y - rect.y < rect.height;
}

static uint32_t contextual_total_width(const editor_context_action *actions,
                                      size_t count, uint32_t scale)
{
    uint64_t total = (uint64_t)(count > 0U ? count - 1U : 0U) *
        EDITOR_CONTEXT_ACTION_GAP * scale + 8U * scale;
    for (size_t index = 0U; index < count; ++index)
        total += (uint64_t)contextual_action_width_points(&actions[index]) * scale;
    return total > UINT32_MAX ? UINT32_MAX : (uint32_t)total;
}

static bool contextual_layout_fallback(viewer_editor *editor,
    const editor_context_action *actions, size_t count, uint32_t scale,
    ui_rect rects[EDITOR_CONTEXT_ACTION_CAP])
{
    viewer_chrome_layout layout;
    if (editor == NULL || actions == NULL || rects == NULL || count == 0U ||
        !viewer_chrome_layout_compute(editor->width, editor->height,
            editor->backing_scale, editor->want_structure, editor->want_inspector,
            &layout)) return false;
    if (!editor->tree_ready) editor_build_tree(editor);
    uint32_t right = layout.options.x + layout.options.width -
        EDITOR_CONTEXT_CANVAS_INSET * scale;
    for (size_t index = 0U; index < editor->tree.count; ++index) {
        const ui_node *node = &editor->tree.nodes[index];
        if (node->spec.id == EDITOR_BUTTON_SNAPPING) {
            right = node->bounds.x > EDITOR_CONTEXT_ACTION_GAP * scale ?
                node->bounds.x - EDITOR_CONTEXT_ACTION_GAP * scale : 0U;
            break;
        }
    }
    uint32_t total = contextual_total_width(actions, count, scale);
    uint32_t left_limit = layout.options.x + EDITOR_CONTEXT_CANVAS_INSET * scale;
    if (total > right - (right > left_limit ? left_limit : right)) return false;
    uint32_t x = right - total + 4U * scale;
    uint32_t h = EDITOR_CONTEXT_ACTION_HEIGHT * scale;
    uint32_t y = layout.options.y + (layout.options.height > h ?
        (layout.options.height - h) / 2U : 0U);
    for (size_t index = 0U; index < count; ++index) {
        uint32_t width = contextual_action_width_points(&actions[index]) * scale;
        rects[index] = (ui_rect){x, y, width, h};
        x += width + EDITOR_CONTEXT_ACTION_GAP * scale;
    }
    return true;
}

static bool contextual_cache_layout(viewer_editor *editor,
    const svg_shape_document *document, const ui_viewer_navigation *navigation,
    ui_rect canvas)
{
    editor_context_action actions[EDITOR_CONTEXT_ACTION_CAP] = {{0}};
    size_t count = 0U;
    editor->context_action_count = 0U;
    editor->context_actions_fallback = false;
    if (!contextual_build(editor, document, actions, &count) || count == 0U ||
        navigation == NULL || !isfinite(navigation->scale) ||
        !(navigation->scale > 0.0)) return false;
    svg_shape_selection selection;
    if (!editor_canvas_selection_bounds(editor, document, &selection)) return false;
    uint32_t scale = (uint32_t)llround(editor->backing_scale);
    if (scale == 0U) scale = 1U;
    if (scale > 8U) scale = 8U;
    uint32_t width = contextual_total_width(actions, count, scale);
    uint32_t height = EDITOR_CONTEXT_ACTION_HEIGHT * scale;
    uint32_t inset = EDITOR_CONTEXT_CANVAS_INSET * scale;
    uint32_t canvas_right = canvas.x + canvas.width;
    uint32_t canvas_bottom = canvas.y + canvas.height;
    double left = (double)canvas.x + navigation->pan_x +
        selection.device_bounds.x * navigation->scale;
    double top = (double)canvas.y + navigation->pan_y +
        selection.device_bounds.y * navigation->scale;
    double right = left + selection.device_bounds.width * navigation->scale;
    double bottom = top + selection.device_bounds.height * navigation->scale;
    if (!isfinite(left) || !isfinite(top) || !isfinite(right) || !isfinite(bottom))
        return false;
    if ((uint64_t)width + 2U * inset > canvas.width ||
        (uint64_t)height + 2U * inset > canvas.height) {
        editor->context_actions_fallback = true;
    } else {
        double min_x = (double)canvas.x + inset;
        double max_x = (double)canvas_right - width - inset;
        double desired_x = (left + right - width) * 0.5;
        if (!isfinite(desired_x) || max_x < min_x)
            editor->context_actions_fallback = true;
        uint32_t x = 0U;
        if (!editor->context_actions_fallback)
            x = (uint32_t)floor(fmax(min_x, fmin(desired_x, max_x)));
        double desired_y = 0.0;
        if (!editor->context_actions_fallback &&
            top >= (double)canvas.y + height + 2U * inset)
            desired_y = top - height - 2U * inset;
        else if (!editor->context_actions_fallback &&
                 bottom + height + 2U * inset <= canvas_bottom)
            desired_y = bottom + 2U * inset;
        else editor->context_actions_fallback = true;
        if (!isfinite(desired_y)) editor->context_actions_fallback = true;
        uint32_t y = 0U;
        if (!editor->context_actions_fallback) {
            double min_y = (double)canvas.y + inset;
            double max_y = (double)canvas_bottom - height - inset;
            y = (uint32_t)floor(fmax(min_y, fmin(desired_y, max_y)));
        }
        if (!editor->context_actions_fallback) {
            ui_rect strip = {x, y, width, height};
            double selected_left = fmin(fmax(left, (double)canvas.x),
                (double)canvas_right);
            double selected_top = fmin(fmax(top, (double)canvas.y),
                (double)canvas_bottom);
            double selected_right = fmin(right, (double)canvas_right);
            double selected_bottom = fmin(bottom, (double)canvas_bottom);
            ui_rect selected = {
                (uint32_t)selected_left, (uint32_t)selected_top,
                selected_right > selected_left ?
                    (uint32_t)(selected_right - selected_left) : 0U,
                selected_bottom > selected_top ?
                    (uint32_t)(selected_bottom - selected_top) : 0U};
            bool pointer_hit = editor->context_pointer_valid &&
                point_inside(strip, (double)canvas.x + editor->context_pointer_x,
                    (double)canvas.y + editor->context_pointer_y);
            if (rect_intersects(strip, selected) || pointer_hit) {
                uint32_t alt_y = y >= canvas.y + height + 2U * inset ?
                    y - height - 2U * inset :
                    y + height + 2U * inset;
                bool alt_valid = alt_y >= canvas.y + inset &&
                    (uint64_t)alt_y + height <= (uint64_t)canvas_bottom - inset;
                ui_rect alternate = {x, alt_y, width, height};
                bool alt_pointer_hit = editor->context_pointer_valid &&
                    point_inside(alternate,
                        (double)canvas.x + editor->context_pointer_x,
                        (double)canvas.y + editor->context_pointer_y);
                if (alt_valid && !rect_intersects(alternate, selected) &&
                    !alt_pointer_hit) strip = alternate;
                else editor->context_actions_fallback = true;
            }
            if (!editor->context_actions_fallback) {
                uint32_t action_x = strip.x + 4U * scale;
                for (size_t index = 0U; index < count; ++index) {
                    uint32_t action_width = contextual_action_width_points(&actions[index]) * scale;
                    actions[index].bounds = (ui_rect){action_x,
                        strip.y + 2U * scale, action_width,
                        EDITOR_CONTEXT_ACTION_HEIGHT * scale - 4U * scale};
                    action_x += action_width + EDITOR_CONTEXT_ACTION_GAP * scale;
                }
            }
        }
    }
    if (editor->context_actions_fallback) {
        ui_rect fallback[EDITOR_CONTEXT_ACTION_CAP] = {{0}};
        if (!contextual_layout_fallback(editor, actions, count, scale, fallback))
            return false;
        for (size_t index = 0U; index < count; ++index)
            actions[index].bounds = fallback[index];
    }
    editor->context_action_count = count;
    memcpy(editor->context_actions, actions, count * sizeof(actions[0]));
    return true;
}

static uint32_t contextual_widget_scale(const viewer_editor *editor)
{
    double raw = editor->backing_scale;
    if (!isfinite(raw) || raw < 0.5) raw = 1.0;
    uint32_t scale = (uint32_t)llround(raw);
    if (scale == 0U) scale = 1U;
    if (scale > 8U) scale = 8U;
    return scale;
}

static ui_rect contextual_points(const viewer_editor *editor, ui_rect pixels)
{
    double scale = editor->backing_scale > 0.0 ? editor->backing_scale : 1.0;
    return (ui_rect){(uint32_t)llround((double)pixels.x / scale),
        (uint32_t)llround((double)pixels.y / scale),
        (uint32_t)llround((double)pixels.width / scale),
        (uint32_t)llround((double)pixels.height / scale)};
}

static bool contextual_paint_cached(viewer_editor *editor,
    uint8_t *rgba, uint32_t width, uint32_t height, size_t stride)
{
    if (editor == NULL || rgba == NULL || editor->context_action_count == 0U ||
        stride < (size_t)width * 4U || (height > 0U && stride > SIZE_MAX / height))
        return false;
    ui_widget_canvas canvas;
    if (!ui_widget_canvas_init(&canvas, rgba, stride * height, width, height, stride,
            contextual_widget_scale(editor), editor->theme,
            editor->increased_contrast, platform_text_ui_draw_widget, NULL)) return false;
    for (size_t index = 0U; index < editor->context_action_count; ++index) {
        const editor_context_action *action = &editor->context_actions[index];
        ui_control_state state = action->id == editor->context_action_hovered ?
            UI_CONTROL_HOVER : UI_CONTROL_REST;
        ui_widget_draw_spec spec = {.kind = UI_WIDGET_BUTTON_SECONDARY,
            .state = state, .bounds = contextual_points(editor, action->bounds),
            .label = action->label, .label_length = action->label_length,
            .font_size_pt = 12U, .font_family = UI_FONT_SYSTEM,
            .font_weight = UI_FONT_SEMIBOLD};
        if (!ui_widget_paint(&canvas, &spec)) return false;
    }
    return true;
}

void editor_paint_context_actions(viewer_editor *editor,
    const svg_shape_document *document, const ui_viewer_navigation *navigation,
    ui_rect canvas, uint8_t *rgba, uint32_t width, uint32_t height,
    size_t stride)
{
    if (editor == NULL || document == NULL || navigation == NULL || rgba == NULL)
        return;
    (void)contextual_cache_layout(editor, document, navigation, canvas);
    (void)contextual_paint_cached(editor, rgba, width, height, stride);
}

uint32_t editor_context_action_at(viewer_editor *editor, uint32_t x, uint32_t y)
{
    if (editor == NULL || editor->snap_popover_open ||
        editor->context_action_count == 0U ||
        editor->canvas_gesture != EDITOR_CANVAS_GESTURE_NONE ||
        editor->draw_tool != VIEWER_EDITOR_TOOL_SELECT || editor->node_edit_active)
        return 0U;
    for (size_t index = 0U; index < editor->context_action_count; ++index) {
        ui_rect bounds = editor->context_actions[index].bounds;
        if (x >= bounds.x && y >= bounds.y && x - bounds.x < bounds.width &&
            y - bounds.y < bounds.height) return editor->context_actions[index].id;
    }
    return 0U;
}

void editor_context_action_pointer_move(viewer_editor *editor,
    uint32_t x, uint32_t y)
{
    if (editor == NULL) return;
    uint32_t hovered = editor_context_action_at(editor, x, y);
    if (hovered == editor->context_action_hovered) return;
    editor->context_action_hovered = hovered;
    editor->redraw_requested = true;
}

static bool contextual_action_is_current(viewer_editor *editor,
    const svg_shape_document *document, uint32_t action)
{
    editor_context_action actions[EDITOR_CONTEXT_ACTION_CAP] = {{0}};
    size_t count = 0U;
    if (!contextual_build(editor, document, actions, &count)) return false;
    for (size_t index = 0U; index < count; ++index)
        if (actions[index].id == action) return true;
    return false;
}

static bool contextual_open_attribute(viewer_editor *editor, const char *name)
{
    size_t attribute = 0U;
    if (!contextual_find_attribute(editor, editor->selected, name, &attribute))
        return false;
    if (!editor->want_inspector)
        (void)viewer_editor_set_sidebar_visibility(editor, editor->want_structure, true);
    if (!editor_begin_field(editor, EDITOR_FIELD_ATTRIBUTE, attribute)) return false;
    editor->focus_id = EDITOR_ATTRIBUTE_ROW_BASE + (uint32_t)attribute;
    editor->tree_ready = false;
    editor->redraw_requested = true;
    editor_status_ok(editor, name[0] == 'f' ? "Editing fill" : "Editing stroke");
    return true;
}

uint32_t editor_context_action_run(viewer_editor *editor,
    const svg_shape_document *document, uint32_t action)
{
    if (editor == NULL || document == NULL ||
        !contextual_action_is_current(editor, document, action))
        return VIEWER_EDITOR_HANDLED;
    if (action == EDITOR_CONTEXT_EDIT_NODES) {
        (void)viewer_editor_set_draw_tool(editor, VIEWER_EDITOR_TOOL_PEN);
        editor->node_edit_active = true;
        editor->tree_ready = false;
        editor->redraw_requested = true;
        return VIEWER_EDITOR_HANDLED;
    }
    if (action == EDITOR_CONTEXT_FILL)
        return contextual_open_attribute(editor, "fill") ?
            VIEWER_EDITOR_HANDLED : VIEWER_EDITOR_HANDLED;
    if (action == EDITOR_CONTEXT_STROKE)
        return contextual_open_attribute(editor, "stroke") ?
            VIEWER_EDITOR_HANDLED : VIEWER_EDITOR_HANDLED;
    if (action == EDITOR_CONTEXT_ALIGN)
        return viewer_editor_align_selection(editor, document,
            VIEWER_EDITOR_ALIGN_HORIZONTAL_CENTER);
    if (action == EDITOR_CONTEXT_GROUP) return editor_canvas_multi_group(editor);
    if (action == EDITOR_CONTEXT_DUPLICATE) {
        if (editor->canvas_selection_count > 1U)
            return editor_canvas_multi_duplicate(editor);
        size_t old_offset = editor->selected_offset;
        char old_id[EDITOR_NAME_CAP];
        editor_set_text(old_id, sizeof(old_id), editor->selected_has_id ?
            editor->selected_id : "");
        document_handle duplicate = {0U};
        document_edit_error error = document_edit_duplicate_element(editor->edit,
            editor->selected, &duplicate);
        return editor_apply_result(editor, error, old_offset, old_id, duplicate);
    }
    if (action == EDITOR_CONTEXT_DELETE)
        return editor->canvas_selection_count > 1U ?
            editor_canvas_multi_delete(editor) :
            viewer_editor_key(editor, 51U, 0U, NULL, 0U);
    return VIEWER_EDITOR_HANDLED;
}

uint32_t viewer_editor_context_action_at(viewer_editor *editor,
    uint32_t x, uint32_t y)
{
    return editor_context_action_at(editor, x, y);
}

uint32_t viewer_editor_context_action_run(viewer_editor *editor,
    const svg_shape_document *document, uint32_t action)
{
    return editor_context_action_run(editor, document, action);
}
