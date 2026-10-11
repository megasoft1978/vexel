#include "viewer_editor_internal.h"

#include "ui_widgets.h"
#include "platform/platform_text_ui.h"

#include <math.h>
#include <stdio.h>
#include <string.h>

static ui_rect editor_widget_bounds(const viewer_editor *editor, ui_rect pixels)
{
    double scale = editor->backing_scale;
    if (!isfinite(scale) || scale <= 0.0) scale = 1.0;
    return (ui_rect){
        (uint32_t)llround((double)pixels.x / scale),
        (uint32_t)llround((double)pixels.y / scale),
        (uint32_t)llround((double)pixels.width / scale),
        (uint32_t)llround((double)pixels.height / scale)};
}

static const char *node_text(const viewer_editor *editor, const ui_node *node,
                             bool value, size_t *out_length)
{
    size_t offset = value ? node->value_offset : node->label_offset;
    size_t length = value ? node->full_value_len : node->full_label_len;
    *out_length = length;
    return editor->tree.text_storage + offset;
}

static ui_control_state node_state(const ui_node *node)
{
    uint32_t state = node->spec.state;
    bool tree_row = node->spec.widget == UI_WIDGET_TREE_ROW ||
        node->spec.role == UI_ROLE_ROW;
    if ((state & UI_STATE_INVALID) != 0U) return UI_CONTROL_INVALID;
    if ((state & UI_STATE_DISABLED) != 0U) return UI_CONTROL_DISABLED;
    if (tree_row && (state & UI_STATE_CHECKED) != 0U) return UI_CONTROL_SELECTED;
    if ((state & UI_STATE_PRESSED) != 0U) return UI_CONTROL_PRESSED;
    if ((state & UI_STATE_CHECKED) != 0U) return UI_CONTROL_SELECTED;
    if ((state & UI_STATE_HOVERED) != 0U) return UI_CONTROL_HOVER;
    return UI_CONTROL_REST;
}

static ui_widget_kind node_widget(const ui_node *node)
{
    if (node->spec.widget != UI_WIDGET_AUTO) return node->spec.widget;
    if (node->spec.role == UI_ROLE_BUTTON)
        return node->spec.icon == UI_ICON_NONE ? UI_WIDGET_BUTTON_SECONDARY :
            UI_WIDGET_ICON_BUTTON;
    if (node->spec.role == UI_ROLE_TEXT_FIELD) return UI_WIDGET_TEXT_FIELD;
    if (node->spec.role == UI_ROLE_ROW) return UI_WIDGET_TREE_ROW;
    if (node->spec.role == UI_ROLE_TEXT || node->spec.role == UI_ROLE_STATUS)
        return UI_WIDGET_LABEL;
    return UI_WIDGET_AUTO;
}

static bool editor_node_has_children(const viewer_editor *editor, size_t index)
{
    return document_model_first_child(editor_model(editor), editor->outline[index],
        &(document_handle){0});
}

static bool editor_node_hidden(viewer_editor *editor, document_handle handle)
{
    return !editor_outline_is_visible(editor, handle);
}

static bool editor_is_number_attribute(const char *name, size_t length)
{
    static const char *const names[] = {"x", "y", "width", "height", "cx", "cy",
        "r", "rx", "ry", "x1", "y1", "x2", "y2", "stroke-width", "font-size"};
    for (size_t index = 0U; index < sizeof(names) / sizeof(names[0]); ++index)
        if (strlen(names[index]) == length && memcmp(name, names[index], length) == 0)
            return true;
    return false;
}

static void editor_number_label(const char *name, size_t length,
                                char out[16], size_t *out_length)
{
    static const struct { const char *source; const char *label; } names[] = {
        {"x", "X"}, {"y", "Y"}, {"width", "Width"}, {"height", "Height"},
        {"cx", "Center X"}, {"cy", "Center Y"}, {"r", "Radius"},
        {"rx", "Radius X"}, {"ry", "Radius Y"}, {"x1", "Start X"},
        {"y1", "Start Y"}, {"x2", "End X"}, {"y2", "End Y"},
        {"stroke-width", "Stroke width"}, {"font-size", "Font size"}
    };
    for (size_t index = 0U; index < sizeof(names) / sizeof(names[0]); ++index) {
        size_t source_length = strlen(names[index].source);
        if (length == source_length && memcmp(name, names[index].source, length) == 0) {
            size_t label_length = strlen(names[index].label);
            memcpy(out, names[index].label, label_length);
            *out_length = label_length;
            return;
        }
    }
    size_t take = length < 15U ? length : 15U;
    if (take > 0U) memcpy(out, name, take);
    if (take > 0U && out[0] >= 'a' && out[0] <= 'z') out[0] -= 'a' - 'A';
    *out_length = take;
}

static bool editor_row_attribute_name(viewer_editor *editor, const ui_node *node,
                                      char name[EDITOR_NAME_CAP], size_t *out_length)
{
    if (node->spec.id < EDITOR_ATTRIBUTE_ROW_BASE ||
        node->spec.id >= EDITOR_ATTRIBUTE_ROW_BASE + 2048U) return false;
    size_t index = node->spec.id - EDITOR_ATTRIBUTE_ROW_BASE;
    document_edit_attribute attribute;
    base_bytes bytes;
    if (!document_edit_attribute_at(editor->edit, editor->selected, index, &attribute) ||
        !document_edit_span_bytes(editor->edit, attribute.name, &bytes) ||
        bytes.len >= EDITOR_NAME_CAP) return false;
    memcpy(name, bytes.ptr, bytes.len);
    name[bytes.len] = '\0';
    *out_length = bytes.len;
    return true;
}

static ui_widget_draw_spec editor_draw_spec(viewer_editor *editor,
    const ui_node *node, ui_widget_kind kind)
{
    size_t label_length = 0U, value_length = 0U;
    const char *label = node_text(editor, node, false, &label_length);
    const char *value = node_text(editor, node, true, &value_length);
    ui_widget_draw_spec spec = {.kind = kind, .state = node_state(node),
        .bounds = editor_widget_bounds(editor, node->bounds),
        .icon = node->spec.icon, .label = label, .label_length = label_length,
        .value = value, .value_length = value_length, .font_size_pt = 13U,
        .font_family = UI_FONT_SYSTEM, .font_weight = UI_FONT_REGULAR,
        .tone = UI_WIDGET_TONE_NEUTRAL, .visible = true,
        .focus_visible = (node->spec.state & UI_STATE_FOCUSED) != 0U,
        .expanded = (node->spec.state & UI_STATE_EXPANDED) != 0U};
    if (kind == UI_WIDGET_LABEL && node->spec.id == EDITOR_NODE_STATUS)
        spec.font_size_pt = 12U;
    if (kind == UI_WIDGET_LABEL && node->spec.id == EDITOR_NODE_STATUS + 101U)
        spec.text_color = editor->tree.tokens.text_secondary;
    if ((node->spec.id == EDITOR_NODE_SELECTED_TAG ||
         node->spec.id == EDITOR_NODE_INSPECTOR_TITLE ||
         node->spec.id == EDITOR_NODE_OUTLINE_TITLE) && kind == UI_WIDGET_LABEL) {
        spec.font_size_pt = 13U;
        spec.font_weight = UI_FONT_SEMIBOLD;
    }
    if (node->spec.id >= 22000U && node->spec.id < 22100U &&
        kind == UI_WIDGET_LABEL) {
        spec.font_size_pt = 12U;
        spec.font_weight = UI_FONT_SEMIBOLD;
    }
    if (kind == UI_WIDGET_EMPTY_STATE) {
        spec.secondary = value;
        spec.secondary_length = value_length;
    }
    if (node->spec.id == EDITOR_OUTLINE_SEARCH_ID && kind == UI_WIDGET_TEXT_FIELD) {
        spec.label = NULL;
        spec.label_length = 0U;
        spec.icon = UI_ICON_SEARCH;
        if (value_length == 0U) {
            spec.value = "Search layers";
            spec.value_length = 13U;
            spec.placeholder = true;
        }
    }
    if (kind == UI_WIDGET_TREE_ROW) {
        size_t row = node->spec.id - EDITOR_OUTLINE_ROW_BASE;
        size_t item = 0U;
        if (editor_outline_visible_model_index(editor,
                editor->outline_visible_start + row, &item)) {
            spec.indent = editor->outline_depth[item] > 8U ? 8U :
                (uint8_t)editor->outline_depth[item];
            spec.has_children = editor_node_has_children(editor, item);
            spec.expanded = spec.has_children &&
                !editor_outline_is_collapsed(editor, item);
            spec.visible = !editor_node_hidden(editor, editor->outline[item]);
            spec.locked = editor_outline_is_locked(editor, editor->outline[item]);
            spec.show_row_actions = true;
        }
    } else if (kind == UI_WIDGET_SCRUB_LABEL) {
        char name[EDITOR_NAME_CAP] = {0};
        size_t name_length = 0U;
        (void)editor_row_attribute_name(editor, node, name, &name_length);
        char short_name[16] = {0};
        size_t short_length = 0U;
        editor_number_label(name, name_length, short_name, &short_length);
        static _Thread_local char number_label[16];
        if (short_length > 0U) memcpy(number_label, short_name, short_length);
        spec.label = number_label;
        spec.label_length = short_length;
        spec.unit = NULL;
        spec.unit_length = 0U;
    } else if (kind == UI_WIDGET_COLOR_FIELD) {
        uint32_t rgba = 0U;
        if (ui_widget_hex_parse(value, value_length, &rgba)) spec.rgba = rgba;
        else spec.kind = UI_WIDGET_TEXT_FIELD;
    }
    if (kind == UI_WIDGET_LABEL && (node->spec.state & UI_STATE_INVALID) != 0U)
        spec.tone = UI_WIDGET_TONE_ERROR;
    return spec;
}

static const char *tooltip_for_id(uint32_t id)
{
    switch (id) {
        case EDITOR_BUTTON_ZOOM_HUD: return "Choose a zoom preset";
        case EDITOR_BUTTON_SAVE: return "Save  ⌘S";
        case EDITOR_BUTTON_UNDO: return "Undo  ⌘Z";
        case EDITOR_BUTTON_REDO: return "Redo  ⇧⌘Z";
        case EDITOR_BUTTON_DELETE: return "Delete  Delete";
        case EDITOR_BUTTON_DUPLICATE: return "Duplicate  ⌘D";
        case EDITOR_BUTTON_GROUP: return "Group  ⌘G";
        case EDITOR_BUTTON_UNGROUP: return "Ungroup  ⇧⌘G";
        case EDITOR_BUTTON_SAVE_AS: return "Save as";
        case EDITOR_BUTTON_TOOL_SELECT: return "Select  V";
        case EDITOR_BUTTON_TOOL_PAN: return "Pan  H / Space";
        case EDITOR_BUTTON_TOOL_ZOOM: return "Zoom  Z";
        case EDITOR_BUTTON_TOGGLE_STRUCTURE: return "Show or hide Layers panel";
        case EDITOR_BUTTON_TOGGLE_INSPECTOR: return "Show or hide Inspector panel";
        case EDITOR_BUTTON_HIDE_STRUCTURE: return "Collapse Layers panel";
        case EDITOR_BUTTON_HIDE_INSPECTOR: return "Collapse Inspector panel";
        case EDITOR_BUTTON_TOOL_RECTANGLE: return "Rectangle  R";
        case EDITOR_BUTTON_TOOL_ELLIPSE: return "Ellipse  O";
        case EDITOR_BUTTON_TOOL_LINE: return "Line  L";
        case EDITOR_BUTTON_TOOL_PEN: return "Pen  P";
        case EDITOR_BUTTON_TOOL_NODE: return "Node edit tool  N";
        case EDITOR_BUTTON_TOOL_TEXT: return "Text  T";
        case EDITOR_BUTTON_TOOL_POLYGON: return "Polygon  Y";
        case EDITOR_BUTTON_TOOL_POLYLINE: return "Polyline  U";
        default: return NULL;
    }
}

static bool tooltip_is_tool_rail_command(uint32_t id)
{
    switch (id) {
        case EDITOR_BUTTON_TOOL_SELECT:
        case EDITOR_BUTTON_TOOL_PAN:
        case EDITOR_BUTTON_TOOL_ZOOM:
        case EDITOR_BUTTON_TOOL_RECTANGLE:
        case EDITOR_BUTTON_TOOL_ELLIPSE:
        case EDITOR_BUTTON_TOOL_LINE:
        case EDITOR_BUTTON_TOOL_POLYGON:
        case EDITOR_BUTTON_TOOL_POLYLINE:
        case EDITOR_BUTTON_TOOL_PEN:
        case EDITOR_BUTTON_TOOL_NODE:
        case EDITOR_BUTTON_TOOL_TEXT:
            return true;
        default: return false;
    }
}

static const char *row_action_tooltip(viewer_editor *editor,
    const ui_node *node)
{
    if (node == NULL || node->spec.id < EDITOR_OUTLINE_ROW_BASE ||
        node->spec.id >= EDITOR_OUTLINE_ROW_BASE + EDITOR_OUTLINE_VISIBLE) return NULL;
    size_t scale = (size_t)ceil(editor->backing_scale > 0.0 ?
        editor->backing_scale : 1.0);
    size_t action_start = node->bounds.x + node->bounds.width -
        (node->bounds.width > EDITOR_OUTLINE_ACTION_WIDTH_POINTS * scale ?
            EDITOR_OUTLINE_ACTION_WIDTH_POINTS * scale : node->bounds.width);
    if (editor->hovered_x < action_start) return NULL;
    size_t row = node->spec.id - EDITOR_OUTLINE_ROW_BASE;
    size_t item = 0U;
    if (!editor_outline_visible_model_index(editor,
            editor->outline_visible_start + row, &item)) return NULL;
    if ((size_t)editor->hovered_x < action_start +
            EDITOR_OUTLINE_VISIBILITY_WIDTH_POINTS * scale)
        return editor_outline_is_visible(editor, editor->outline[item]) ?
            "Hide layer" : "Show layer";
    return editor_outline_is_locked(editor, editor->outline[item]) ?
        "Unlock layer" : "Lock layer";
}

void editor_paint_widgets(viewer_editor *editor, uint8_t *rgba,
                          size_t capacity, size_t stride)
{
    if (editor == NULL || rgba == NULL || !editor->tree_ready) return;
    double raw_scale = isfinite(editor->backing_scale) ? editor->backing_scale : 1.0;
    uint32_t scale = (uint32_t)llround(raw_scale);
    if (scale < 1U) scale = 1U;
    if (scale > 3U) scale = 3U;
    ui_widget_canvas canvas;
    if (!ui_widget_canvas_init(&canvas, rgba, capacity, editor->width, editor->height,
            stride, scale, editor->theme, editor->increased_contrast,
            platform_text_ui_draw_widget, NULL)) return;
    ui_widget_draw_spec tooltip_spec = {0};
    bool tooltip_ready = false;
    for (size_t index = 1U; index < editor->tree.count; ++index) {
        const ui_node *node = &editor->tree.nodes[index];
        ui_widget_kind kind = node_widget(node);
        if (kind == UI_WIDGET_AUTO || node->bounds.width == 0U ||
            node->bounds.height == 0U) continue;
        if (kind == UI_WIDGET_TEXT_FIELD) {
            size_t name_length = 0U, value_length = 0U;
            const char *name = node_text(editor, node, false, &name_length);
            const char *value = node_text(editor, node, true, &value_length);
            char source_name[EDITOR_NAME_CAP] = {0};
            size_t source_name_length = name_length;
            if (editor_row_attribute_name(editor, node, source_name,
                    &source_name_length)) {
                name = source_name;
                name_length = source_name_length;
            }
            if (editor_is_number_attribute(name, name_length)) {
                kind = UI_WIDGET_SCRUB_LABEL;
            } else if (((name_length == 4U && memcmp(name, "fill", 4U) == 0) ||
                        (name_length == 6U && memcmp(name, "stroke", 6U) == 0)) &&
                ui_widget_hex_parse(value, value_length, &(uint32_t){0})) {
                kind = UI_WIDGET_COLOR_FIELD;
            }
        }
        ui_widget_draw_spec spec = editor_draw_spec(editor, node, kind);
        if (!ui_widget_paint(&canvas, &spec)) return;
        if ((node->spec.state & UI_STATE_HOVERED) != 0U) {
            const char *tooltip = tooltip_for_id(node->spec.id);
            if (tooltip == NULL && node->spec.role == UI_ROLE_ROW)
                tooltip = row_action_tooltip(editor, node);
            if (tooltip != NULL) {
                size_t length = strlen(tooltip);
                ui_rect bounds = editor_widget_bounds(editor, node->bounds);
                uint32_t tooltip_width = (uint32_t)(length * 7U + 20U);
                if (tooltip_width > 240U) tooltip_width = 240U;
                if (tooltip_width > canvas.width / scale)
                    tooltip_width = canvas.width / scale;
                uint32_t x = tooltip_is_tool_rail_command(node->spec.id) ?
                    EDITOR_TOOL_RAIL_WIDTH +
                        (editor->effective_structure ? EDITOR_LEFT_WIDTH : 0U) + 8U :
                    bounds.x;
                if (x + tooltip_width > canvas.width / scale)
                    x = canvas.width / scale > tooltip_width ?
                        canvas.width / scale - tooltip_width : 0U;
                uint32_t y = bounds.y + bounds.height + 4U;
                if (y + 28U > canvas.height / scale)
                    y = bounds.y > 32U ? bounds.y - 32U : 0U;
                tooltip_spec = (ui_widget_draw_spec){.kind = UI_WIDGET_TOOLTIP,
                    .state = UI_CONTROL_REST,
                    .bounds = {x, y, tooltip_width, 28U},
                    .label = tooltip, .label_length = length};
                tooltip_ready = true;
            }
        }
    }
    if (tooltip_ready) (void)ui_widget_paint(&canvas, &tooltip_spec);
}
