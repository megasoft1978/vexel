#include "viewer_editor_internal.h"

#include "viewer_chrome.h"

#include <math.h>
#include <stdio.h>
#include <string.h>

static bool editor_add_icon(viewer_editor *editor, uint32_t id, uint32_t parent,
                            ui_icon_kind icon, const char *label,
                            uint32_t state)
{
    return editor_add_widget_node(editor, id, parent, UI_AXIS_HORIZONTAL,
        32U, 32U, 0U, 0U, 0U, UI_ROLE_BUTTON, UI_WIDGET_ICON_BUTTON,
        icon, label, strlen(label), NULL, 0U, state, id);
}

static bool editor_add_divider(viewer_editor *editor, uint32_t id, uint32_t parent,
                               uint32_t height)
{
    return editor_add_widget_node(editor, id, parent, UI_AXIS_VERTICAL,
        1U, height, 0U, 0U, 0U, UI_ROLE_NONE, UI_WIDGET_DIVIDER,
        UI_ICON_NONE, NULL, 0U, NULL, 0U, 0U, 0U);
}

static bool editor_draw_tool_selected(viewer_editor *editor,
                                      viewer_editor_draw_tool tool)
{
    return editor->draw_tool == tool;
}

static const char *editor_tool_hint(viewer_editor *editor)
{
    switch (editor->draw_tool) {
        case VIEWER_EDITOR_TOOL_RECT: return "Rectangle tool · drag to draw · R";
        case VIEWER_EDITOR_TOOL_ELLIPSE: return "Ellipse tool · drag to draw · O";
        case VIEWER_EDITOR_TOOL_LINE: return "Line tool · drag to draw · L";
        case VIEWER_EDITOR_TOOL_POLYGON: return "Polygon tool · click points, then Return · Y";
        case VIEWER_EDITOR_TOOL_POLYLINE: return "Polyline tool · click points, then Return · U";
        case VIEWER_EDITOR_TOOL_PEN: return "Pen tool · click or drag anchors · P";
        case VIEWER_EDITOR_TOOL_TEXT: return "Text tool · click to place text · T";
        case VIEWER_EDITOR_TOOL_SELECT: break;
    }
    if (editor->active_tool == UI_ICON_PAN) return "Pan tool · drag the canvas · Space";
    if (editor->active_tool == UI_ICON_ZOOM) return "Zoom tool · scroll over the canvas · Z";
    return "Selection tool · choose a layer or edit it on the canvas · V";
}

static void editor_canvas_status(viewer_editor *editor,
                                 char hud[UI_TEXT_MAX_BYTES],
                                 char detail[UI_TEXT_MAX_BYTES])
{
    int amount;
    if (editor->canvas_pointer_valid) {
        amount = snprintf(hud, UI_TEXT_MAX_BYTES, "%.0f%%  ·  %.1f, %.1f",
            editor->canvas_zoom * 100.0, editor->canvas_pointer_x,
            editor->canvas_pointer_y);
    } else {
        amount = snprintf(hud, UI_TEXT_MAX_BYTES, "%.0f%% zoom",
            editor->canvas_zoom * 100.0);
    }
    if (amount < 0 || (size_t)amount >= UI_TEXT_MAX_BYTES)
        (void)snprintf(hud, UI_TEXT_MAX_BYTES, "Canvas ready");
    if (editor->canvas_document_width > 0U && editor->canvas_document_height > 0U) {
        amount = snprintf(detail, UI_TEXT_MAX_BYTES, "%u × %u",
            editor->canvas_document_width, editor->canvas_document_height);
        if (amount < 0 || (size_t)amount >= UI_TEXT_MAX_BYTES)
            (void)snprintf(detail, UI_TEXT_MAX_BYTES, "Document size unavailable");
    } else {
        (void)snprintf(detail, UI_TEXT_MAX_BYTES, "No document size");
    }
}

static void editor_status_labels(viewer_editor *editor,
                                 char primary[UI_TEXT_MAX_BYTES],
                                 char secondary[UI_TEXT_MAX_BYTES])
{
    if (strncmp(editor->status, "Edit rejected:", 14U) == 0 ||
        strncmp(editor->status, "Save failed:", 12U) == 0 ||
        strncmp(editor->status, "Preview update failed:", 22U) == 0 ||
        strncmp(editor->status, "Unsupported SVG content", 23U) == 0) {
        editor_set_text(primary, UI_TEXT_MAX_BYTES, editor->status);
    } else {
        const char *state = document_edit_is_dirty(editor->edit) ?
            "Unsaved changes" : "Saved";
        int amount = snprintf(primary, UI_TEXT_MAX_BYTES, "%s · %s",
            editor->status[0] == '\0' || strcmp(editor->status, "Ready") == 0 ?
                "Ready" : editor->status, state);
        if (amount < 0 || (size_t)amount >= UI_TEXT_MAX_BYTES)
            editor_set_text(primary, UI_TEXT_MAX_BYTES, state);
    }
    if (editor->canvas_selection_count > 1U) {
        int amount = snprintf(secondary, UI_TEXT_MAX_BYTES, "%zu layers selected",
            editor->canvas_selection_count);
        if (amount < 0 || (size_t)amount >= UI_TEXT_MAX_BYTES)
            editor_set_text(secondary, UI_TEXT_MAX_BYTES, "Multiple layers selected");
    } else if (editor->canvas_selection_count == 1U) {
        editor_set_text(secondary, UI_TEXT_MAX_BYTES, "1 layer selected");
    } else editor_set_text(secondary, UI_TEXT_MAX_BYTES, "No selection");
}

void editor_build_tree_modern(viewer_editor *editor)
{
    if (editor == NULL || editor->edit == NULL || editor->width == 0U || editor->height == 0U)
        return;
    viewer_chrome_layout layout;
    if (!viewer_chrome_layout_compute(editor->width, editor->height,
            editor->backing_scale, editor->want_structure, editor->want_inspector,
            &layout) || !ui_tree_init_accessible(&editor->tree, editor->theme,
                editor->increased_contrast)) return;
    editor->effective_structure = layout.structure_visible;
    editor->effective_inspector = layout.inspector_visible;
    editor->tree.nodes[0].spec.color = 0U;
    editor->tree.nodes[0].spec.label_len = 0U;
    editor->tree.backing_scale = editor->backing_scale;
    editor->tree.tokens.font_px = (uint32_t)ceil(13.0 * editor->backing_scale);
    editor->tree.tokens.radius = (uint32_t)ceil(6.0 * editor->backing_scale);
    editor->tree.tokens.spacing = (uint32_t)ceil(8.0 * editor->backing_scale);
    const ui_tokens *tokens = &editor->tree.tokens;
    bool okay = editor_add_node(editor, EDITOR_NODE_TOOLBAR, UI_ROOT_ID,
            UI_AXIS_HORIZONTAL, 0U, VIEWER_CHROME_TOOLBAR_POINTS, 6U, 4U,
            tokens->panel, UI_ROLE_GROUP, NULL, 0U, NULL, 0U, 0U, 0U) &&
        editor_add_icon(editor, EDITOR_BUTTON_SAVE, EDITOR_NODE_TOOLBAR,
            UI_ICON_SAVE, "Save · Command-S", 0U) &&
        editor_add_divider(editor, 30U, EDITOR_NODE_TOOLBAR, 22U) &&
        editor_add_icon(editor, EDITOR_BUTTON_UNDO, EDITOR_NODE_TOOLBAR,
            UI_ICON_UNDO, "Undo · Command-Z", 0U) &&
        editor_add_icon(editor, EDITOR_BUTTON_REDO, EDITOR_NODE_TOOLBAR,
            UI_ICON_REDO, "Redo · Shift-Command-Z", 0U) &&
        editor_add_divider(editor, 31U, EDITOR_NODE_TOOLBAR, 22U) &&
        editor_add_icon(editor, EDITOR_BUTTON_DUPLICATE, EDITOR_NODE_TOOLBAR,
            UI_ICON_DUPLICATE, "Duplicate · Command-D", 0U) &&
        editor_add_icon(editor, EDITOR_BUTTON_GROUP, EDITOR_NODE_TOOLBAR,
            UI_ICON_GROUP, "Group · Command-G", 0U) &&
        editor_add_icon(editor, EDITOR_BUTTON_UNGROUP, EDITOR_NODE_TOOLBAR,
            UI_ICON_UNGROUP, "Ungroup · Shift-Command-G", 0U) &&
        editor_add_icon(editor, EDITOR_BUTTON_DELETE, EDITOR_NODE_TOOLBAR,
            UI_ICON_TRASH, "Delete · Delete", 0U) &&
        editor_add_node(editor, 32U, EDITOR_NODE_TOOLBAR, UI_AXIS_HORIZONTAL,
            0U, 32U, 0U, 0U, 0U, UI_ROLE_GROUP, NULL, 0U, NULL, 0U, 0U, 0U) &&
        editor_add_icon(editor, EDITOR_BUTTON_TOGGLE_STRUCTURE,
            EDITOR_NODE_TOOLBAR, UI_ICON_LAYERS, "Layers panel",
            editor->want_structure ? UI_STATE_CHECKED : 0U) &&
        editor_add_icon(editor, EDITOR_BUTTON_TOGGLE_INSPECTOR,
            EDITOR_NODE_TOOLBAR, UI_ICON_RULER, "Inspector panel",
            editor->want_inspector ? UI_STATE_CHECKED : 0U);
    if (!okay) return;
    okay = editor_add_node(editor, EDITOR_NODE_OPTIONS, UI_ROOT_ID,
            UI_AXIS_HORIZONTAL, 0U, VIEWER_CHROME_OPTIONS_POINTS, 8U, 8U,
            tokens->panel_raised, UI_ROLE_GROUP, NULL, 0U, NULL, 0U, 0U, 0U) &&
        editor_add_widget_node(editor, EDITOR_NODE_OPTIONS + 100U,
            EDITOR_NODE_OPTIONS, UI_AXIS_HORIZONTAL, 0U, 0U, 0U, 0U, 0U,
            UI_ROLE_TEXT, UI_WIDGET_LABEL, UI_ICON_NONE, editor_tool_hint(editor),
            strlen(editor_tool_hint(editor)), NULL, 0U, 0U, 0U) &&
        editor_add_node(editor, EDITOR_NODE_WORKSPACE, UI_ROOT_ID,
            UI_AXIS_HORIZONTAL, 0U, 0U, 0U, 0U, 0U, UI_ROLE_GROUP,
            NULL, 0U, NULL, 0U, 0U, 0U) &&
        editor_add_node(editor, EDITOR_NODE_TOOL_RAIL, EDITOR_NODE_WORKSPACE,
            UI_AXIS_VERTICAL, EDITOR_TOOL_RAIL_WIDTH, 0U, 6U, 4U,
            tokens->panel, UI_ROLE_GROUP, NULL, 0U, NULL, 0U, 0U, 0U);
    if (!okay) return;
    ui_control_state select_state = editor->draw_tool == VIEWER_EDITOR_TOOL_SELECT &&
        editor->active_tool == UI_ICON_SELECT ? UI_STATE_CHECKED : 0U;
    okay = editor_add_icon(editor, EDITOR_BUTTON_TOOL_SELECT, EDITOR_NODE_TOOL_RAIL,
            UI_ICON_SELECT, "Selection tool · V", select_state) &&
        editor_add_icon(editor, EDITOR_BUTTON_TOOL_PAN, EDITOR_NODE_TOOL_RAIL,
            UI_ICON_PAN, "Pan tool · Space", editor->active_tool == UI_ICON_PAN ?
                UI_STATE_CHECKED : 0U) &&
        editor_add_icon(editor, EDITOR_BUTTON_TOOL_ZOOM, EDITOR_NODE_TOOL_RAIL,
            UI_ICON_ZOOM_IN, "Zoom tool · Z", editor->active_tool == UI_ICON_ZOOM ?
                UI_STATE_CHECKED : 0U) &&
        editor_add_divider(editor, 33U, EDITOR_NODE_TOOL_RAIL, 1U) &&
        editor_add_icon(editor, EDITOR_BUTTON_TOOL_RECTANGLE, EDITOR_NODE_TOOL_RAIL,
            UI_ICON_RECTANGLE, "Rectangle tool · R",
            editor_draw_tool_selected(editor, VIEWER_EDITOR_TOOL_RECT) ? UI_STATE_CHECKED : 0U) &&
        editor_add_icon(editor, EDITOR_BUTTON_TOOL_ELLIPSE, EDITOR_NODE_TOOL_RAIL,
            UI_ICON_ELLIPSE, "Ellipse tool · O",
            editor_draw_tool_selected(editor, VIEWER_EDITOR_TOOL_ELLIPSE) ? UI_STATE_CHECKED : 0U) &&
        editor_add_icon(editor, EDITOR_BUTTON_TOOL_LINE, EDITOR_NODE_TOOL_RAIL,
            UI_ICON_LINE, "Line tool · L",
            editor_draw_tool_selected(editor, VIEWER_EDITOR_TOOL_LINE) ? UI_STATE_CHECKED : 0U) &&
        editor_add_icon(editor, EDITOR_BUTTON_TOOL_PEN, EDITOR_NODE_TOOL_RAIL,
            UI_ICON_PEN, "Pen tool · P",
            editor_draw_tool_selected(editor, VIEWER_EDITOR_TOOL_PEN) ? UI_STATE_CHECKED : 0U) &&
        editor_add_icon(editor, EDITOR_BUTTON_TOOL_NODE, EDITOR_NODE_TOOL_RAIL,
            UI_ICON_NODE_EDIT, "Edit path nodes · P",
            0U) &&
        editor_add_icon(editor, EDITOR_BUTTON_TOOL_TEXT, EDITOR_NODE_TOOL_RAIL,
            UI_ICON_TEXT, "Text tool · T",
            editor_draw_tool_selected(editor, VIEWER_EDITOR_TOOL_TEXT) ? UI_STATE_CHECKED : 0U);
    if (!okay) return;
    if (layout.structure_visible) editor_build_structure_panel(editor, tokens);
    okay = editor_add_node(editor, EDITOR_NODE_CANVAS, EDITOR_NODE_WORKSPACE,
            UI_AXIS_VERTICAL, 0U, 0U, 0U, 0U, 0U, UI_ROLE_GROUP,
            NULL, 0U, NULL, 0U, 0U, 0U);
    if (!okay) return;
    char hud[UI_TEXT_MAX_BYTES] = {0}, document_size[UI_TEXT_MAX_BYTES] = {0};
    char detail[UI_TEXT_MAX_BYTES] = {0};
    editor_canvas_status(editor, hud, document_size);
    if (editor->canvas_tiles_total > 0U) {
        int amount = snprintf(detail, sizeof(detail),
            editor->canvas_tiles_active ? "Rendering sharp detail · %zu of %zu" :
                "Sharp detail ready · %zu of %zu",
            editor->canvas_tiles_done, editor->canvas_tiles_total);
        if (amount < 0 || (size_t)amount >= sizeof(detail))
            editor_set_text(detail, sizeof(detail), "Rendering sharp detail");
    } else {
        editor_set_text(detail, sizeof(detail), "Sharp detail ready");
    }
    okay = editor_add_node(editor, EDITOR_NODE_CANVAS_HUD, EDITOR_NODE_CANVAS,
            UI_AXIS_HORIZONTAL, 0U, 32U, 8U, 8U, 0U, UI_ROLE_GROUP,
            NULL, 0U, NULL, 0U, 0U, 0U) &&
        editor_add_widget_node(editor, EDITOR_NODE_CANVAS_HUD + 100U,
            EDITOR_NODE_CANVAS_HUD, UI_AXIS_HORIZONTAL, 0U, 28U,
            0U, 0U, 0U, UI_ROLE_STATUS, UI_WIDGET_HUD_PILL, UI_ICON_NONE,
            hud, strlen(hud), NULL, 0U, 0U, 0U) &&
        editor_add_widget_node(editor, EDITOR_NODE_CANVAS_HUD + 101U,
            EDITOR_NODE_CANVAS_HUD, UI_AXIS_HORIZONTAL, 0U, 28U,
            0U, 0U, 0U, UI_ROLE_STATUS, UI_WIDGET_HUD_PILL, UI_ICON_NONE,
            document_size, strlen(document_size), NULL, 0U, 0U, 0U) &&
        editor_add_widget_node(editor, EDITOR_NODE_CANVAS_TILE_STATUS,
            EDITOR_NODE_CANVAS, UI_AXIS_HORIZONTAL, 0U, 24U, 8U, 0U,
            0U, UI_ROLE_STATUS, UI_WIDGET_LABEL, UI_ICON_NONE,
            detail, strlen(detail), NULL, 0U, 0U, 0U);
    if (!okay) return;
    if (editor->outline_count <= 1U) {
        okay = editor_add_widget_node(editor, EDITOR_NODE_CANVAS + 100U,
            EDITOR_NODE_CANVAS, UI_AXIS_VERTICAL, 0U, 0U, 0U, 0U,
            0U, UI_ROLE_GROUP, UI_WIDGET_EMPTY_STATE, UI_ICON_LAYERS,
            "Empty canvas", 12U, "Add a shape or open an SVG document.", 34U,
            0U, 0U);
        if (!okay) return;
    }
    if (layout.inspector_visible && !editor_build_inspector_panel(editor, tokens)) return;
    char status[UI_TEXT_MAX_BYTES] = {0}, selection[UI_TEXT_MAX_BYTES] = {0};
    editor_status_labels(editor, status, selection);
    bool status_error = strncmp(editor->status, "Edit rejected:", 14U) == 0 ||
        strncmp(editor->status, "Save failed:", 12U) == 0 ||
        strncmp(editor->status, "Preview update failed:", 22U) == 0 ||
        strncmp(editor->status, "Unsupported SVG content", 23U) == 0;
    okay = editor_add_node(editor, EDITOR_NODE_STATUS, UI_ROOT_ID,
            UI_AXIS_HORIZONTAL, 0U, VIEWER_CHROME_STATUS_POINTS, 8U, 8U,
            tokens->panel_raised, UI_ROLE_GROUP, NULL, 0U, NULL, 0U, 0U, 0U) &&
        editor_add_widget_node(editor, EDITOR_NODE_STATUS + 100U,
            EDITOR_NODE_STATUS, UI_AXIS_HORIZONTAL, 0U, 0U, 0U, 0U,
            0U, UI_ROLE_STATUS, UI_WIDGET_LABEL, UI_ICON_NONE,
            status, strlen(status), NULL, 0U,
            status_error ? UI_STATE_INVALID : 0U, 0U) &&
        editor_add_widget_node(editor, EDITOR_NODE_STATUS + 101U,
            EDITOR_NODE_STATUS, UI_AXIS_HORIZONTAL, 0U, 24U, 0U, 0U,
            0U, UI_ROLE_STATUS, UI_WIDGET_LABEL, UI_ICON_NONE,
            selection, strlen(selection), NULL, 0U, 0U, 0U);
    if (okay && ui_tree_layout(&editor->tree, editor->width, editor->height)) {
        if (editor->focus_id != 0U) {
            for (size_t index = 0U; index < editor->tree.count; ++index) {
                ui_node *node = &editor->tree.nodes[index];
                if (node->spec.id == editor->focus_id &&
                    (node->spec.state & UI_STATE_DISABLED) == 0U) {
                    node->spec.state |= UI_STATE_FOCUSED;
                    editor->tree.focused_id = editor->focus_id;
                    break;
                }
            }
        }
        editor->tree_ready = true;
    }
}
