#include "viewer_editor_internal.h"

#include "viewer_chrome.h"
#include "ui_widgets.h"
#include "platform/platform_localization.h"

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
                               uint32_t width, uint32_t height)
{
    return editor_add_widget_node(editor, id, parent, UI_AXIS_VERTICAL,
        width, height, 0U, 0U, 0U, UI_ROLE_NONE, UI_WIDGET_DIVIDER,
        UI_ICON_NONE, NULL, 0U, NULL, 0U, 0U, 0U);
}

static bool editor_add_splitter(viewer_editor *editor, uint32_t id,
    uint16_t width_points, const char *label)
{
    char value[16] = {0};
    int amount = snprintf(value, sizeof(value), "%u pt", (unsigned)width_points);
    if (amount < 0 || (size_t)amount >= sizeof(value)) return false;
    double scale = editor->backing_scale *
        ((double)viewer_editor_ui_scale_percent(editor) / 100.0);
    ui_widget_spec spec = {.id = id, .parent_id = EDITOR_NODE_WORKSPACE,
        .axis = UI_AXIS_VERTICAL,
        .preferred_width = (uint32_t)ceil(VIEWER_CHROME_SPLITTER_POINTS * scale),
        .preferred_height = 0U, .widget = UI_WIDGET_DIVIDER,
        .label = label, .label_len = strlen(label),
        .value = value, .value_len = (size_t)amount};
    return ui_widget_add_slider(&editor->tree, &spec);
}

static void editor_set_node_bounds(viewer_editor *editor, uint32_t id, ui_rect bounds)
{
    for (size_t index = 0U; index < editor->tree.count; ++index) {
        if (editor->tree.nodes[index].spec.id == id) {
            editor->tree.nodes[index].bounds = bounds;
            return;
        }
    }
}

static bool editor_draw_tool_selected(viewer_editor *editor,
                                      viewer_editor_draw_tool tool)
{
    if (tool == VIEWER_EDITOR_TOOL_PEN)
        return editor->draw_tool == tool && !editor->node_edit_active;
    return editor->draw_tool == tool;
}

static const char *editor_tool_hint(viewer_editor *editor)
{
    if (editor->node_edit_active && editor->path_hovered_segment_valid)
        return "Double-click this segment to add an anchor";
    if (editor->node_edit_active)
        return "Node edit · Tab anchors · arrows nudge · Esc exits";
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
    if (editor->active_tool == UI_ICON_PAN)
        return "Pan tool · drag the canvas · H / Space";
    if (editor->active_tool == UI_ICON_ZOOM) return "Zoom tool · scroll over the canvas · Z";
    return "Selection tool · choose a layer or edit it on the canvas · V";
}

static uint32_t editor_toolbar_action_state(const viewer_editor *editor,
                                             uint32_t action)
{
    size_t cursor = document_edit_history_cursor(editor->edit);
    size_t history = document_edit_history_count(editor->edit);
    if ((action == EDITOR_BUTTON_UNDO && cursor == 0U) ||
        (action == EDITOR_BUTTON_REDO && cursor >= history))
        return UI_STATE_DISABLED;
    if (action == EDITOR_BUTTON_DELETE && editor->node_edit_active &&
        (editor->path_selected_node_segment == SIZE_MAX ||
         editor->path_node_is_initial)) return UI_STATE_DISABLED;
    bool selected = editor->canvas_selection_count > 0U ||
        (editor->selection_active && editor->selected_index > 0U);
    if ((action == EDITOR_BUTTON_DUPLICATE || action == EDITOR_BUTTON_DELETE) &&
        !selected) return UI_STATE_DISABLED;
    if (action == EDITOR_BUTTON_GROUP && editor->canvas_selection_count < 2U)
        return UI_STATE_DISABLED;
    if (action == EDITOR_BUTTON_UNGROUP) {
        base_bytes tag = {0};
        if (!selected || !document_edit_tag_name(editor->edit,
                editor->selected, &tag) || tag.len != 1U || tag.ptr[0] != 'g')
            return UI_STATE_DISABLED;
    }
    return 0U;
}

static void editor_canvas_status(viewer_editor *editor,
                                 char hud[UI_TEXT_MAX_BYTES])
{
    int amount;
    if (editor->canvas_pointer_valid) {
        amount = snprintf(hud, UI_TEXT_MAX_BYTES, "%.0f%% · %.1f, %.1f",
            editor->canvas_zoom * 100.0, editor->canvas_pointer_x,
            editor->canvas_pointer_y);
    } else {
        amount = snprintf(hud, UI_TEXT_MAX_BYTES, "%.0f%% zoom",
            editor->canvas_zoom * 100.0);
    }
    if (amount < 0 || (size_t)amount >= UI_TEXT_MAX_BYTES)
        (void)snprintf(hud, UI_TEXT_MAX_BYTES, "Canvas ready");
    size_t used = strlen(hud);
    if (editor->canvas_document_width > 0U && editor->canvas_document_height > 0U &&
        used < UI_TEXT_MAX_BYTES - 1U) {
        amount = snprintf(hud + used, UI_TEXT_MAX_BYTES - used, " · %u × %u",
            editor->canvas_document_width, editor->canvas_document_height);
        if (amount > 0 && (size_t)amount < UI_TEXT_MAX_BYTES - used)
            used += (size_t)amount;
    }
    if (used < UI_TEXT_MAX_BYTES - 1U) {
        if (editor->canvas_tiles_total > 0U) {
            amount = snprintf(hud + used, UI_TEXT_MAX_BYTES - used,
                editor->canvas_tiles_active ? " · Rendering %zu/%zu" : " · Sharp %zu/%zu",
                editor->canvas_tiles_done, editor->canvas_tiles_total);
        } else {
            amount = snprintf(hud + used, UI_TEXT_MAX_BYTES - used, " · Sharp ready");
        }
        if (amount < 0 || (size_t)amount >= UI_TEXT_MAX_BYTES - used)
            (void)snprintf(hud + used, UI_TEXT_MAX_BYTES - used, " · Sharp");
    }
}

static void editor_status_labels(viewer_editor *editor,
                                 char primary[EDITOR_STATUS_CAP],
                                 char secondary[PLATFORM_LOCALIZATION_TEXT_CAP])
{
    if (editor->status_error) {
        editor_set_text(primary, EDITOR_STATUS_CAP, editor->status);
    } else {
        const char *state_fallback = document_edit_is_dirty(editor->edit) ?
            "Unsaved changes" : "Saved";
        char state[128] = {0};
        size_t state_length = 0U;
        if (!editor_localize_slice(state_fallback, strlen(state_fallback), state,
                sizeof(state), &state_length))
            editor_set_text(state, sizeof(state), state_fallback);
        (void)state_length;
        const char *message = editor->status[0] == '\0' ? "Ready" : editor->status;
        int amount = snprintf(primary, EDITOR_STATUS_CAP, "%s · %s", message, state);
        if (amount < 0 || (size_t)amount >= EDITOR_STATUS_CAP)
            editor_set_text(primary, EDITOR_STATUS_CAP, state);
    }
    if (editor->canvas_selection_count == 0U) {
        size_t length = 0U;
        if (!editor_localize_slice("No selection", 12U, secondary,
                PLATFORM_LOCALIZATION_TEXT_CAP, &length))
            editor_set_text(secondary, PLATFORM_LOCALIZATION_TEXT_CAP, "No selection");
    } else {
        size_t length = 0U;
        if (!platform_localization_format_count_message(
                editor->canvas_selection_count,
                "editor.status.one_layer_selected", "1 layer selected",
                "editor.status.layers_selected", "%@ layers selected",
                "Canvas selection count status.", secondary,
                PLATFORM_LOCALIZATION_TEXT_CAP, &length))
            editor_set_text(secondary, PLATFORM_LOCALIZATION_TEXT_CAP,
                editor->canvas_selection_count == 1U ? "1 layer selected" :
                    "Multiple layers selected");
    }
}

void editor_build_tree_modern(viewer_editor *editor)
{
    if (editor == NULL || editor->edit == NULL || editor->width == 0U || editor->height == 0U)
        return;
    viewer_chrome_layout layout;
    uint32_t timeline_points = editor->timeline_visible ?
        editor->timeline_height_points : 32U;
    double ui_scale = (double)viewer_editor_ui_scale_percent(editor) / 100.0;
    double scale = editor->backing_scale > 0.0 ? editor->backing_scale : 1.0;
    if (!viewer_chrome_layout_compute_docked_sized(editor->width, editor->height,
            scale, ui_scale, editor->structure_width_points,
            editor->inspector_width_points, editor->want_structure, editor->want_inspector,
            editor->timeline_available, timeline_points, &layout) ||
        !ui_tree_init_accessible(&editor->tree, editor->theme,
                editor->increased_contrast)) return;
    editor->effective_structure = layout.structure_visible;
    editor->effective_inspector = layout.inspector_visible;
    editor->tree.nodes[0].spec.color = 0U;
    editor->tree.nodes[0].spec.label_len = 0U;
    editor->tree.backing_scale = scale;
    editor->tree.tokens = ui_theme_tokens_accented(editor->theme,
        editor->increased_contrast, editor->system_accent_srgb);
    editor->tree.tokens.font_px = (uint32_t)ceil(13.0 * scale * ui_scale);
    editor->tree.tokens.radius = (uint32_t)ceil(6.0 * scale * ui_scale);
    editor->tree.tokens.spacing = (uint32_t)ceil(8.0 * scale * ui_scale);
    const ui_tokens *tokens = &editor->tree.tokens;
    bool okay = editor_add_node(editor, EDITOR_NODE_TOOLBAR, UI_ROOT_ID,
            UI_AXIS_HORIZONTAL, 0U, VIEWER_CHROME_TOOLBAR_POINTS, 6U, 6U,
            viewer_editor_native_material_enabled(editor) ? 0U : tokens->panel,
            UI_ROLE_GROUP, NULL, 0U, NULL, 0U, 0U, 0U) &&
        editor_add_icon(editor, EDITOR_BUTTON_SAVE, EDITOR_NODE_TOOLBAR,
            UI_ICON_SAVE, "Save · Command-S", 0U) &&
        editor_add_icon(editor, EDITOR_BUTTON_UNDO, EDITOR_NODE_TOOLBAR,
            UI_ICON_UNDO, "Undo · Command-Z",
            editor_toolbar_action_state(editor, EDITOR_BUTTON_UNDO)) &&
        editor_add_icon(editor, EDITOR_BUTTON_REDO, EDITOR_NODE_TOOLBAR,
            UI_ICON_REDO, "Redo · Shift-Command-Z",
            editor_toolbar_action_state(editor, EDITOR_BUTTON_REDO)) &&
        editor_add_icon(editor, EDITOR_BUTTON_HISTORY, EDITOR_NODE_TOOLBAR,
            UI_ICON_CHEVRON_DOWN, "Undo History",
            editor_toolbar_action_state(editor, EDITOR_BUTTON_HISTORY)) &&
        editor_add_icon(editor, EDITOR_BUTTON_DUPLICATE, EDITOR_NODE_TOOLBAR,
            UI_ICON_DUPLICATE, "Duplicate · Command-D",
            editor_toolbar_action_state(editor, EDITOR_BUTTON_DUPLICATE)) &&
        editor_add_icon(editor, EDITOR_BUTTON_GROUP, EDITOR_NODE_TOOLBAR,
            UI_ICON_GROUP, "Group · Command-G",
            editor_toolbar_action_state(editor, EDITOR_BUTTON_GROUP)) &&
        editor_add_icon(editor, EDITOR_BUTTON_UNGROUP, EDITOR_NODE_TOOLBAR,
            UI_ICON_UNGROUP, "Ungroup · Shift-Command-G",
            editor_toolbar_action_state(editor, EDITOR_BUTTON_UNGROUP)) &&
        editor_add_icon(editor, EDITOR_BUTTON_DELETE, EDITOR_NODE_TOOLBAR,
            UI_ICON_TRASH, "Delete · Delete",
            editor_toolbar_action_state(editor, EDITOR_BUTTON_DELETE)) &&
        editor_add_node(editor, 32U, EDITOR_NODE_TOOLBAR, UI_AXIS_HORIZONTAL,
            0U, 32U, 0U, 0U, 0U, UI_ROLE_GROUP, NULL, 0U, NULL, 0U, 0U, 0U) &&
        editor_add_icon(editor, EDITOR_BUTTON_TOGGLE_STRUCTURE,
            EDITOR_NODE_TOOLBAR, UI_ICON_LAYERS, "Layers panel",
            editor->want_structure ? UI_STATE_CHECKED : 0U) &&
        editor_add_icon(editor, EDITOR_BUTTON_TOGGLE_INSPECTOR,
            EDITOR_NODE_TOOLBAR, UI_ICON_SIDEBAR_RIGHT, "Inspector panel",
            editor->want_inspector ? UI_STATE_CHECKED : 0U);
    if (!okay) return;
    if (editor->timeline_available && !editor_add_icon(editor,
            EDITOR_BUTTON_TOGGLE_TIMELINE, EDITOR_NODE_TOOLBAR, UI_ICON_RULER,
            "Animation timeline", editor->timeline_visible ? UI_STATE_CHECKED : 0U))
        return;
    okay = editor_add_node(editor, EDITOR_NODE_OPTIONS, UI_ROOT_ID,
        UI_AXIS_HORIZONTAL, 0U, VIEWER_CHROME_OPTIONS_POINTS, 8U, 8U,
        tokens->panel_raised, UI_ROLE_GROUP, NULL, 0U, NULL, 0U, 0U, 0U);
    if (!okay) return;
    /* Blender-style task workspaces: a tab strip, not a mode switch. The strip
     * replaces the duplicated tool-hint row; the hint now lives in the status
     * bar. Pane visibility and the inspector group come from the workspace
     * table, so the tabs and the layout can never disagree. */
    const viewer_workspace_spec *workspace_spec =
        viewer_workspace_get(editor->workspace);
    okay = editor_add_node(editor, EDITOR_NODE_WORKSPACES, EDITOR_NODE_OPTIONS,
        UI_AXIS_HORIZONTAL, 0U, 24U, 0U, 0U, 0U, UI_ROLE_GROUP,
        "Workspaces", 10U, NULL, 0U, 0U, 0U);
    (void)workspace_spec;
    for (size_t workspace_index = 0U;
         okay && workspace_index < viewer_workspace_count(); ++workspace_index) {
        const viewer_workspace_spec *spec =
            viewer_workspace_get((viewer_workspace_kind)workspace_index);
        char workspace_label[64] = {0};
        int label_amount = snprintf(workspace_label, sizeof(workspace_label),
            "%s · Control-Tab", spec->label);
        if (label_amount < 0 || (size_t)label_amount >= sizeof(workspace_label))
            snprintf(workspace_label, sizeof(workspace_label), "%s", spec->label);
        uint32_t tab_width = (uint32_t)(strlen(spec->label) * 9U + 28U);
        okay = editor_add_widget_node(editor,
            EDITOR_BUTTON_WORKSPACE_BASE + (uint32_t)workspace_index,
            EDITOR_NODE_WORKSPACES, UI_AXIS_HORIZONTAL, tab_width, 24U, 6U, 0U,
            (viewer_workspace_kind)workspace_index == editor->workspace ?
                tokens->panel_raised : 0U,
            UI_ROLE_BUTTON, UI_WIDGET_BUTTON_SECONDARY, UI_ICON_NONE,
            spec->label, strlen(spec->label), NULL, 0U,
            (viewer_workspace_kind)workspace_index == editor->workspace ?
                UI_STATE_CHECKED : 0U,
            EDITOR_BUTTON_WORKSPACE_BASE + (uint32_t)workspace_index);
    }
    if (okay && editor->node_edit_active && editor->path_node_corner_available)
        okay = editor_add_widget_node(editor, EDITOR_BUTTON_NODE_CORNER,
            EDITOR_NODE_OPTIONS, UI_AXIS_HORIZONTAL, 72U, 24U, 0U, 0U, 0U,
            UI_ROLE_BUTTON, UI_WIDGET_BUTTON_SECONDARY, UI_ICON_NONE,
            "Corner", 6U, NULL, 0U, 0U, EDITOR_BUTTON_NODE_CORNER);
    if (okay && editor->node_edit_active && editor->path_node_smooth_available)
        okay = editor_add_widget_node(editor, EDITOR_BUTTON_NODE_SMOOTH,
            EDITOR_NODE_OPTIONS, UI_AXIS_HORIZONTAL, 72U, 24U, 0U, 0U, 0U,
            UI_ROLE_BUTTON, UI_WIDGET_BUTTON_SECONDARY, UI_ICON_NONE,
            "Smooth", 6U, NULL, 0U, 0U, EDITOR_BUTTON_NODE_SMOOTH);
    if (okay)
        okay = editor_add_icon(editor, EDITOR_BUTTON_SNAPPING, EDITOR_NODE_OPTIONS,
            UI_ICON_SNAP, "Snapping settings · Grid, Objects, Guides",
            editor->snap_enabled ? UI_STATE_CHECKED : 0U);
    okay = okay && editor_add_node(editor, EDITOR_NODE_WORKSPACE, UI_ROOT_ID,
            UI_AXIS_HORIZONTAL, 0U, 0U, 0U, 0U, 0U, UI_ROLE_GROUP,
            NULL, 0U, NULL, 0U, 0U, 0U) &&
        editor_add_node(editor, EDITOR_NODE_TOOL_RAIL, EDITOR_NODE_WORKSPACE,
            UI_AXIS_VERTICAL, EDITOR_TOOL_RAIL_WIDTH, 0U, 6U, 6U,
            tokens->panel, UI_ROLE_GROUP, NULL, 0U, NULL, 0U, 0U, 0U);
    if (!okay) return;
    ui_control_state select_state = editor->draw_tool == VIEWER_EDITOR_TOOL_SELECT &&
        editor->active_tool == UI_ICON_SELECT ? UI_STATE_CHECKED : 0U;
    okay = editor_add_icon(editor, EDITOR_BUTTON_TOOL_SELECT, EDITOR_NODE_TOOL_RAIL,
            UI_ICON_SELECT, "Selection tool · V", select_state) &&
        editor_add_icon(editor, EDITOR_BUTTON_TOOL_PAN, EDITOR_NODE_TOOL_RAIL,
            UI_ICON_PAN, "Pan tool · H / Space", editor->active_tool == UI_ICON_PAN ?
                UI_STATE_CHECKED : 0U) &&
        editor_add_icon(editor, EDITOR_BUTTON_TOOL_ZOOM, EDITOR_NODE_TOOL_RAIL,
            UI_ICON_ZOOM_IN, "Zoom tool · Z", editor->active_tool == UI_ICON_ZOOM ?
                UI_STATE_CHECKED : 0U) &&
        editor_add_divider(editor, 33U, EDITOR_NODE_TOOL_RAIL, 32U, 1U) &&
        editor_add_icon(editor, EDITOR_BUTTON_TOOL_RECTANGLE, EDITOR_NODE_TOOL_RAIL,
            UI_ICON_RECTANGLE, "Rectangle tool · R",
            editor_draw_tool_selected(editor, VIEWER_EDITOR_TOOL_RECT) ? UI_STATE_CHECKED : 0U) &&
        editor_add_icon(editor, EDITOR_BUTTON_TOOL_ELLIPSE, EDITOR_NODE_TOOL_RAIL,
            UI_ICON_ELLIPSE, "Ellipse tool · O",
            editor_draw_tool_selected(editor, VIEWER_EDITOR_TOOL_ELLIPSE) ? UI_STATE_CHECKED : 0U) &&
        editor_add_icon(editor, EDITOR_BUTTON_TOOL_LINE, EDITOR_NODE_TOOL_RAIL,
            UI_ICON_LINE, "Line tool · L",
            editor_draw_tool_selected(editor, VIEWER_EDITOR_TOOL_LINE) ? UI_STATE_CHECKED : 0U) &&
        editor_add_icon(editor, EDITOR_BUTTON_TOOL_POLYGON, EDITOR_NODE_TOOL_RAIL,
            UI_ICON_PEN, "Polygon tool · Y",
            editor_draw_tool_selected(editor, VIEWER_EDITOR_TOOL_POLYGON) ? UI_STATE_CHECKED : 0U) &&
        editor_add_icon(editor, EDITOR_BUTTON_TOOL_POLYLINE, EDITOR_NODE_TOOL_RAIL,
            UI_ICON_POLYLINE, "Polyline tool · U",
            editor_draw_tool_selected(editor, VIEWER_EDITOR_TOOL_POLYLINE) ? UI_STATE_CHECKED : 0U) &&
        editor_add_divider(editor, 34U, EDITOR_NODE_TOOL_RAIL, 32U, 1U) &&
        editor_add_icon(editor, EDITOR_BUTTON_TOOL_PEN, EDITOR_NODE_TOOL_RAIL,
            UI_ICON_PENCIL, "Pen tool · P",
            editor_draw_tool_selected(editor, VIEWER_EDITOR_TOOL_PEN) ? UI_STATE_CHECKED : 0U) &&
        editor_add_icon(editor, EDITOR_BUTTON_TOOL_NODE, EDITOR_NODE_TOOL_RAIL,
            UI_ICON_NODE_EDIT, "Node edit tool · N",
            editor->node_edit_active ? UI_STATE_CHECKED : 0U) &&
        editor_add_icon(editor, EDITOR_BUTTON_TOOL_TEXT, EDITOR_NODE_TOOL_RAIL,
            UI_ICON_TEXT, "Text tool · T",
            editor_draw_tool_selected(editor, VIEWER_EDITOR_TOOL_TEXT) ? UI_STATE_CHECKED : 0U);
    if (!okay) return;
    if (layout.structure_visible) {
        editor_build_structure_panel(editor, tokens);
        if (!editor_add_splitter(editor, EDITOR_SPLITTER_STRUCTURE_ID,
                editor->layout_structure_width_points, "Layers panel width")) return;
    }
    okay = editor_add_node(editor, EDITOR_NODE_CANVAS, EDITOR_NODE_WORKSPACE,
            UI_AXIS_VERTICAL, 0U, 0U,
            editor->guides.rulers_visible ? VIEWER_EDITOR_RULER_LEFT_POINTS : 0U,
            0U, 0U, UI_ROLE_GROUP,
            NULL, 0U, NULL, 0U, 0U, 0U);
    if (!okay) return;
    char hud[UI_TEXT_MAX_BYTES] = {0};
    editor_canvas_status(editor, hud);
    uint32_t canvas_points = (uint32_t)((double)layout.canvas.width / (scale * ui_scale));
    uint32_t hud_width = (uint32_t)(strlen(hud) * 7U + 20U);
    uint32_t hud_max_width = canvas_points > 8U ? canvas_points - 8U : canvas_points;
    if (hud_width > hud_max_width) hud_width = hud_max_width;
    if (hud_width == 0U) hud_width = 1U;
    okay = editor_add_node(editor, EDITOR_NODE_CANVAS_HUD, EDITOR_NODE_CANVAS,
            UI_AXIS_HORIZONTAL, 0U, 36U, 4U, 0U, 0U, UI_ROLE_GROUP,
            NULL, 0U, NULL, 0U, 0U, 0U) &&
        editor_add_widget_node(editor, EDITOR_NODE_CANVAS_HUD + 100U,
            EDITOR_NODE_CANVAS_HUD, UI_AXIS_HORIZONTAL, hud_width, 28U,
            0U, 0U, 0U, UI_ROLE_BUTTON, UI_WIDGET_HUD_PILL, UI_ICON_NONE,
            hud, strlen(hud), NULL, 0U,
            editor->zoom_menu_open ? UI_STATE_CHECKED : 0U,
            EDITOR_BUTTON_ZOOM_HUD);
    if (!okay) return;
    if (editor->zoom_menu_open) {
        static const char *const zoom_labels[] = {
            "25%", "50%", "100%", "200%", "400%", "800%"};
        okay = editor_add_node(editor, EDITOR_NODE_CANVAS_ZOOM_MENU,
            EDITOR_NODE_CANVAS, UI_AXIS_VERTICAL, 132U, 164U, 4U, 2U,
            tokens->panel_raised, UI_ROLE_GROUP, NULL, 0U, NULL, 0U, 0U, 0U);
        for (size_t index = 0U; okay && index <
                sizeof(zoom_labels) / sizeof(zoom_labels[0]); ++index) {
            double preset = viewer_editor_zoom_preset_scale(index);
            uint32_t state = fabs(editor->canvas_zoom - preset) < 0.000001 ?
                UI_STATE_CHECKED : 0U;
            okay = editor_add_widget_node(editor,
                EDITOR_BUTTON_ZOOM_PRESET_BASE + (uint32_t)index,
                EDITOR_NODE_CANVAS_ZOOM_MENU, UI_AXIS_HORIZONTAL, 0U, 24U,
                2U, 0U, tokens->panel_raised, UI_ROLE_BUTTON,
                UI_WIDGET_BUTTON_SECONDARY, UI_ICON_NONE,
                zoom_labels[index], strlen(zoom_labels[index]), NULL, 0U,
                state, EDITOR_BUTTON_ZOOM_PRESET_BASE + (uint32_t)index);
        }
        if (!okay) return;
    }
    if (editor->outline_count <= 1U) {
        okay = editor_add_widget_node(editor, EDITOR_NODE_CANVAS + 100U,
            EDITOR_NODE_CANVAS, UI_AXIS_VERTICAL, 0U, 0U, 0U, 0U,
            0U, UI_ROLE_GROUP, UI_WIDGET_EMPTY_STATE, UI_ICON_LAYERS,
            "Empty canvas", 12U, "Add a shape or open an SVG document.", 34U,
            0U, 0U);
        if (!okay) return;
    }
    if (layout.inspector_visible && !editor_add_splitter(editor,
            EDITOR_SPLITTER_INSPECTOR_ID, editor->layout_inspector_width_points,
            "Inspector panel width")) return;
    if (layout.inspector_visible && !editor_build_inspector_panel(editor, tokens)) return;
    if (editor->timeline_available && layout.timeline.height > 0U) {
        uint32_t preferred_height = (uint32_t)ceil(
            (double)layout.timeline.height / scale);
        if (!editor_add_node(editor, EDITOR_NODE_TIMELINE_DOCK, UI_ROOT_ID,
                UI_AXIS_VERTICAL, 0U, preferred_height, 0U, 0U,
                tokens->panel, UI_ROLE_GROUP, "Animation timeline", 18U,
                NULL, 0U, 0U, 0U)) return;
    }
    char status[EDITOR_STATUS_CAP] = {0};
    char selection[PLATFORM_LOCALIZATION_TEXT_CAP] = {0};
    editor_status_labels(editor, status, selection);
    bool status_error = editor->status_error;
    /* One status bar with the three Blender regions: hint, report, readout. */
    const char *status_hint = editor_tool_hint(editor);
    char report[EDITOR_STATUS_CAP] = {0};
    if (editor->status_error) {
        editor_set_text(report, sizeof(report), editor->status);
    } else if (editor->status[0] != '\0' &&
               strcmp(editor->status, "Ready") != 0 && strcmp(editor->status, "Saved") != 0) {
        editor_set_text(report, sizeof(report), editor->status);
    }
    char readout[EDITOR_STATUS_CAP] = {0};
    int readout_amount = snprintf(readout, sizeof(readout), "%s · %s", status, selection);
    if (readout_amount < 0 || (size_t)readout_amount >= sizeof(readout))
        editor_set_text(readout, sizeof(readout), selection);
    uint32_t selection_width = (uint32_t)(strlen(selection) * 7U + 20U);
    uint32_t readout_width = (uint32_t)(strlen(readout) * 7U + 24U);
    uint32_t status_points = (uint32_t)((double)layout.status.width / (scale * ui_scale));
    uint32_t status_content_width = status_points > 24U ? status_points - 24U : 0U;
    if (selection_width > status_content_width) selection_width = status_content_width;
    okay = editor_add_node(editor, EDITOR_NODE_STATUS, UI_ROOT_ID,
            UI_AXIS_HORIZONTAL, 0U, VIEWER_CHROME_STATUS_POINTS, 0U, 0U,
            tokens->panel_raised, UI_ROLE_GROUP, NULL, 0U, NULL, 0U, 0U, 0U) &&
        editor_add_node(editor, EDITOR_NODE_STATUS + 102U,
            EDITOR_NODE_STATUS, UI_AXIS_HORIZONTAL, 12U, 0U, 0U, 0U, 0U,
            UI_ROLE_GROUP, NULL, 0U, NULL, 0U, 0U, 0U) &&
        editor_add_widget_node(editor, EDITOR_NODE_STATUS_HINT,
            EDITOR_NODE_STATUS, UI_AXIS_HORIZONTAL, 0U, 0U, 0U, 0U,
            0U, UI_ROLE_STATUS, UI_WIDGET_LABEL, UI_ICON_NONE,
            status_hint, strlen(status_hint), NULL, 0U, 0U, 0U) &&
        editor_add_node(editor, EDITOR_NODE_STATUS + 103U,
            EDITOR_NODE_STATUS, UI_AXIS_HORIZONTAL, 12U, 0U, 0U, 0U, 0U,
            UI_ROLE_GROUP, NULL, 0U, NULL, 0U, 0U, 0U) &&
        editor_add_widget_node(editor, EDITOR_NODE_STATUS_REPORT,
            EDITOR_NODE_STATUS, UI_AXIS_HORIZONTAL, 0U, 0U, 0U, 0U,
            0U, UI_ROLE_STATUS, UI_WIDGET_LABEL, UI_ICON_NONE,
            report, strlen(report), NULL, 0U,
            status_error ? UI_STATE_INVALID : 0U, 0U) &&
        editor_add_widget_node(editor, EDITOR_NODE_STATUS_READOUT,
            EDITOR_NODE_STATUS, UI_AXIS_HORIZONTAL, readout_width, 24U, 0U, 0U,
            0U, UI_ROLE_STATUS, UI_WIDGET_LABEL, UI_ICON_NONE,
            readout, strlen(readout), NULL, 0U, 0U, 0U) &&
        editor_add_node(editor, EDITOR_NODE_STATUS + 104U,
            EDITOR_NODE_STATUS, UI_AXIS_HORIZONTAL, 12U, 0U, 0U, 0U, 0U,
            UI_ROLE_GROUP, NULL, 0U, NULL, 0U, 0U, 0U);
    if (okay && ui_tree_layout(&editor->tree, editor->width, editor->height)) {
        editor_set_node_bounds(editor, EDITOR_NODE_TOOLBAR, layout.toolbar);
        editor_set_node_bounds(editor, EDITOR_NODE_OPTIONS, layout.options);
        editor_set_node_bounds(editor, EDITOR_NODE_WORKSPACE, layout.workspace);
        editor_set_node_bounds(editor, EDITOR_NODE_TOOL_RAIL, layout.rail);
        if (layout.structure_visible) {
            editor_set_node_bounds(editor, EDITOR_NODE_OUTLINE, layout.structure);
            editor_set_node_bounds(editor, EDITOR_SPLITTER_STRUCTURE_ID,
                layout.structure_splitter);
        }
        editor_set_node_bounds(editor, EDITOR_NODE_CANVAS, layout.canvas);
        if (layout.inspector_visible) {
            editor_set_node_bounds(editor, EDITOR_SPLITTER_INSPECTOR_ID,
                layout.inspector_splitter);
            editor_set_node_bounds(editor, EDITOR_NODE_INSPECTOR, layout.inspector);
        }
        editor_set_node_bounds(editor, EDITOR_NODE_STATUS, layout.status);
        editor_reveal_layout_nodes(editor);
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
