#include "viewer_editor_internal.h"

#include <string.h>
#include "viewer_timeline.h"

#include "document_model.h"
#include "viewer_chrome.h"
#include "platform/macos/platform_accessibility.h"
#include "platform/platform_text_ui.h"

#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static size_t bounded_length(const char *text, size_t capacity)
{
    size_t length = 0U;
    if (text == NULL) return 0U;
    while (length < capacity && text[length] != '\0') ++length;
    return length;
}

void editor_set_text(char *target, size_t capacity, const char *source)
{
    if (target == NULL || capacity == 0U) return;
    size_t length = bounded_length(source, capacity - 1U);
    if (length > 0U) memcpy(target, source, length);
    target[length] = '\0';
}

size_t editor_utf8_prefix(const uint8_t *bytes, size_t length, size_t maximum)
{
    size_t result = length < maximum ? length : maximum;
    while (result > 0U && result < length && (bytes[result] & 0xc0U) == 0x80U) --result;
    return result;
}

bool editor_same_handle(document_handle left, document_handle right)
{
    return left.document_id == right.document_id && left.slot == right.slot &&
        left.generation == right.generation;
}

const document_model *editor_model(const viewer_editor *editor)
{
    return editor == NULL || editor->edit == NULL ? NULL :
        document_edit_model(editor->edit);
}

bool editor_begin_field(viewer_editor *editor, editor_field_kind kind,
                               size_t attribute_index)
{
    if (kind == EDITOR_FIELD_NEW_NAME || kind == EDITOR_FIELD_NEW_VALUE) {
        const char *source = kind == EDITOR_FIELD_NEW_NAME ? editor->new_name : editor->new_value;
        size_t length = kind == EDITOR_FIELD_NEW_NAME ? editor->new_name_length : editor->new_value_length;
        if (length >= sizeof(editor->field)) return false;
        if (length > 0U) memcpy(editor->field, source, length);
        editor->field[length] = '\0';
        editor->field_length = length;
    } else if (kind == EDITOR_FIELD_ATTRIBUTE) {
        document_edit_attribute attribute;
        base_bytes name;
        uint8_t decoded[EDITOR_FIELD_CAP];
        size_t length = 0U;
        if (!document_edit_attribute_at(editor->edit, editor->selected,
                attribute_index, &attribute) ||
            !document_edit_span_bytes(editor->edit, attribute.name, &name) ||
            name.len >= sizeof(editor->selected_attribute_name) ||
            !document_edit_decode_span(editor->edit, attribute.value,
                decoded, sizeof(decoded), &length) || length >= sizeof(editor->field)) {
            editor_status_ok(editor, "Value exceeds the inline editor limit");
            return false;
        }
        memcpy(editor->selected_attribute_name, name.ptr, name.len);
        editor->selected_attribute_name[name.len] = '\0';
        if (length > 0U) memcpy(editor->field, decoded, length);
        editor->field[length] = '\0';
        editor->field_length = length;
        editor->field_attribute = attribute_index;
        editor->selected_attribute = attribute_index;
    } else if (kind == EDITOR_FIELD_TEXT) {
        size_t length = 0U;
        if (!document_edit_text_content(editor->edit, editor->selected,
                (uint8_t *)editor->field, sizeof(editor->field) - 1U, &length)) {
            editor_status_ok(editor, "Text exceeds the inline editor limit");
            return false;
        }
        editor->field[length] = '\0';
        editor->field_length = length;
    } else if (kind == EDITOR_FIELD_SEARCH) {
        if (editor->outline_filter_length >= EDITOR_OUTLINE_FILTER_CAP) return false;
        memcpy(editor->field, editor->outline_filter, editor->outline_filter_length);
        editor->field[editor->outline_filter_length] = '\0';
        editor->field_length = editor->outline_filter_length;
        memcpy(editor->outline_filter_original, editor->outline_filter,
            editor->outline_filter_length + 1U);
        editor->outline_filter_original_length = editor->outline_filter_length;
    } else if (kind == EDITOR_FIELD_RENAME) {
        size_t length = editor->selected_has_id ? strlen(editor->selected_id) : 0U;
        if (length >= sizeof(editor->field)) return false;
        if (length > 0U) memcpy(editor->field, editor->selected_id, length);
        editor->field[length] = '\0';
        editor->field_length = length;
    } else return false;
    editor->cursor = editor->field_length;
    editor->anchor = editor->cursor;
    editor->field_kind = kind;
    editor->focus_id = kind == EDITOR_FIELD_ATTRIBUTE ?
        EDITOR_ATTRIBUTE_ROW_BASE + (uint32_t)attribute_index :
        kind == EDITOR_FIELD_TEXT ? EDITOR_TEXT_FIELD_ID :
        kind == EDITOR_FIELD_NEW_NAME ? EDITOR_NEW_NAME_ID :
        kind == EDITOR_FIELD_NEW_VALUE ? EDITOR_NEW_VALUE_ID :
        kind == EDITOR_FIELD_SEARCH ? EDITOR_OUTLINE_SEARCH_ID : EDITOR_RENAME_ID;
    editor->tree_ready = false;
    return true;
}

bool editor_store_pending_field(viewer_editor *editor)
{
    if (editor->field_kind == EDITOR_FIELD_NEW_NAME) {
        if (editor->field_length >= sizeof(editor->new_name)) return false;
        memcpy(editor->new_name, editor->field, editor->field_length);
        editor->new_name[editor->field_length] = '\0';
        editor->new_name_length = editor->field_length;
    } else if (editor->field_kind == EDITOR_FIELD_NEW_VALUE) {
        if (editor->field_length >= sizeof(editor->new_value)) return false;
        memcpy(editor->new_value, editor->field, editor->field_length);
        editor->new_value[editor->field_length] = '\0';
        editor->new_value_length = editor->field_length;
    }
    return true;
}

uint32_t editor_apply_result(viewer_editor *editor, document_edit_error result,
                                    size_t old_offset, const char *old_id,
                                    document_handle preferred)
{
    if (result != DOCUMENT_EDIT_OK) {
        editor_status_error(editor, result);
        return VIEWER_EDITOR_HANDLED;
    }
    editor_rebind_after_change(editor, old_offset, old_id, preferred);
    editor_status_ok(editor, "Edited · preview updating");
    return VIEWER_EDITOR_HANDLED | VIEWER_EDITOR_MUTATED;
}


bool editor_reorder(viewer_editor *editor, bool forward)
{
    if (editor_canvas_selection_is_locked(editor)) {
        editor_status_ok(editor, "Unlock the selected layer to reorder it");
        return false;
    }
    const document_model *model = editor_model(editor);
    document_handle parent, sibling = {0};
    if (!document_model_parent(model, editor->selected, &parent)) return false;
    if (forward) {
        if (!document_model_next_sibling(model, editor->selected, &sibling)) return false;
        size_t old_offset = editor->selected_offset;
        char old_id[EDITOR_NAME_CAP];
        editor_set_text(old_id, sizeof(old_id), editor->selected_has_id ? editor->selected_id : "");
        document_edit_error result = document_edit_reorder_sibling(editor->edit,
            editor->selected, sibling, false);
        (void)editor_apply_result(editor, result, old_offset, old_id, (document_handle){0});
        return result == DOCUMENT_EDIT_OK;
    }
    document_handle cursor;
    if (!document_model_first_child(model, parent, &cursor)) return false;
    document_handle previous = {0};
    while (!editor_same_handle(cursor, editor->selected)) {
        previous = cursor;
        if (!document_model_next_sibling(model, cursor, &cursor)) return false;
    }
    if (previous.document_id == 0U) return false;
    size_t old_offset = editor->selected_offset;
    char old_id[EDITOR_NAME_CAP];
    editor_set_text(old_id, sizeof(old_id), editor->selected_has_id ? editor->selected_id : "");
    document_edit_error result = document_edit_reorder_sibling(editor->edit,
        editor->selected, previous, true);
    (void)editor_apply_result(editor, result, old_offset, old_id, (document_handle){0});
    return result == DOCUMENT_EDIT_OK;
}

bool editor_reorder_to_edge(viewer_editor *editor, bool front)
{
    if (editor_canvas_selection_is_locked(editor)) {
        editor_status_ok(editor, "Unlock the selected layer to reorder it");
        return false;
    }
    const document_model *model = editor_model(editor);
    document_handle parent = {0U}, target = {0U}, cursor = {0U};
    if (model == NULL || !document_model_parent(model, editor->selected, &parent) ||
        !document_model_first_child(model, parent, &cursor)) return false;
    if (!front) {
        if (editor_same_handle(cursor, editor->selected)) return false;
        target = cursor;
    } else {
        target = cursor;
        while (document_model_next_sibling(model, cursor, &cursor)) target = cursor;
        if (editor_same_handle(target, editor->selected)) return false;
    }
    size_t old_offset = editor->selected_offset;
    char old_id[EDITOR_NAME_CAP];
    editor_set_text(old_id, sizeof(old_id), editor->selected_has_id ?
        editor->selected_id : "");
    document_edit_error result = document_edit_reorder_sibling(editor->edit,
        editor->selected, target, !front);
    (void)editor_apply_result(editor, result, old_offset, old_id,
        (document_handle){0U});
    return result == DOCUMENT_EDIT_OK;
}

static void editor_preferences_changed(
    const platform_text_ui_preferences *preferences, void *context)
{
    viewer_editor *editor = context;
    if (editor == NULL || preferences == NULL) return;
    (void)viewer_editor_set_appearance(editor, preferences->appearance,
        preferences->increased_contrast, preferences->reduced_motion);
    (void)viewer_editor_set_system_accent(editor, preferences->accent_srgb,
        preferences->material_allowed);
    (void)viewer_editor_set_ui_scale(editor, preferences->ui_scale_percent);
    (void)viewer_editor_set_panel_widths(editor,
        preferences->structure_width_points, preferences->inspector_width_points);
}

static void editor_view_menu_changed(bool structure_visible,
                                     bool inspector_visible, void *context)
{
    viewer_editor *editor = context;
    (void)viewer_editor_set_sidebar_visibility(editor, structure_visible,
                                               inspector_visible);
}

static bool editor_accessibility_command(uint32_t command_id, void *context)
{
    viewer_editor *editor = context;
    if (editor == NULL) return false;
    if (command_id == EDITOR_BUTTON_TOOL_SELECT)
        return viewer_editor_set_tool(editor, UI_ICON_SELECT) ||
            editor->active_tool == UI_ICON_SELECT;
    if (command_id == EDITOR_BUTTON_TOOL_PAN)
        return viewer_editor_set_tool(editor, UI_ICON_PAN) ||
            editor->active_tool == UI_ICON_PAN;
    if (command_id == EDITOR_BUTTON_TOOL_ZOOM)
        return viewer_editor_set_tool(editor, UI_ICON_ZOOM) ||
            editor->active_tool == UI_ICON_ZOOM;
    if (command_id == EDITOR_BUTTON_SNAPPING) {
        editor->snap_popover_open = !editor->snap_popover_open;
        editor->snap_popover_hovered = false;
        editor->snap_popover_hovered_row = UINT8_MAX;
        editor->tree_ready = false;
        editor->redraw_requested = true;
        return true;
    }
    if (command_id == EDITOR_BUTTON_TOGGLE_STRUCTURE ||
        command_id == EDITOR_BUTTON_HIDE_STRUCTURE) {
        (void)viewer_editor_set_sidebar_visibility(editor,
            !editor->want_structure, editor->want_inspector);
        return true;
    }
    if (command_id == EDITOR_BUTTON_TOGGLE_INSPECTOR ||
        command_id == EDITOR_BUTTON_HIDE_INSPECTOR) {
        (void)viewer_editor_set_sidebar_visibility(editor,
            editor->want_structure, !editor->want_inspector);
        return true;
    }
    if (command_id == EDITOR_BUTTON_INSPECTOR_OTHER_ATTRIBUTES) {
        editor->inspector_other_attributes_expanded =
            !editor->inspector_other_attributes_expanded;
        editor->tree_ready = false;
        editor->redraw_requested = true;
        return true;
    }
    if (command_id >= EDITOR_OUTLINE_ROW_BASE &&
        command_id < EDITOR_OUTLINE_ROW_BASE + EDITOR_OUTLINE_VISIBLE) {
        size_t index = editor->outline_start + command_id - EDITOR_OUTLINE_ROW_BASE;
        if (editor->modern_style && !editor_outline_visible_model_index(editor,
                editor->outline_visible_start + command_id - EDITOR_OUTLINE_ROW_BASE,
                &index)) return false;
        return editor_select_outline_index(editor, index);
    }
    if (command_id >= EDITOR_ATTRIBUTE_ROW_BASE &&
        command_id < EDITOR_ATTRIBUTE_ROW_BASE + 2048U) {
        size_t index = command_id - EDITOR_ATTRIBUTE_ROW_BASE;
        if (index >= document_edit_attribute_count(editor->edit, editor->selected)) return false;
        editor->selected_attribute = index;
        if (!editor_begin_field(editor, EDITOR_FIELD_ATTRIBUTE, index)) return false;
        editor->focus_id = command_id;
        editor->tree_ready = false;
        return true;
    }
    return false;
}

viewer_editor *viewer_editor_create(base_bytes source, ui_theme theme,
                                    uint32_t width, uint32_t height)
{
    return viewer_editor_create_styled(source, theme, width, height,
        platform_text_ui_app_is_active());
}

viewer_editor *viewer_editor_create_styled(base_bytes source, ui_theme theme,
                                           uint32_t width, uint32_t height,
                                           bool modern_style)
{
    if (source.ptr == NULL || source.len == 0U || width == 0U || height == 0U ||
        (theme != UI_THEME_DARK && theme != UI_THEME_LIGHT)) return NULL;
    viewer_editor *editor = calloc(1U, sizeof(*editor));
    if (editor == NULL) return NULL;
    document_edit_result created = document_edit_create(source, document_edit_default_limits());
    if (created.error != DOCUMENT_EDIT_OK) {
        free(editor);
        return NULL;
    }
    editor->edit = created.edit;
    editor->theme = theme;
    editor->modern_style = modern_style;
    editor->active_tool = UI_ICON_SELECT;
    editor->want_structure = true;
    editor->want_inspector = true;
    editor->workspace = viewer_workspace_default();
    editor->want_structure = viewer_workspace_get(editor->workspace)->structure;
    editor->want_inspector = viewer_workspace_get(editor->workspace)->inspector;
    editor->timeline_height_points = VIEWER_TIMELINE_DOCK_DEFAULT_POINTS;
    editor->width = width;
    editor->height = height;
    editor->backing_scale = 1.0;
    editor->path_selected_node_segment = SIZE_MAX;
    editor->snap_enabled = true;
    viewer_editor_guides_init(&editor->guides);
    editor->snap_grid_enabled = true;
    editor->snap_objects_enabled = true;
    editor->snap_guides_enabled = true;
    editor->snap_popover_hovered_row = UINT8_MAX;
    editor->ui_scale_percent = 100U;
    editor->structure_width_points = EDITOR_LEFT_WIDTH;
    editor->inspector_width_points = EDITOR_RIGHT_WIDTH;
    editor->layout_structure_width_points = EDITOR_LEFT_WIDTH;
    editor->layout_inspector_width_points = EDITOR_RIGHT_WIDTH;
    editor->inspector_geometry_expanded = true;
    editor->inspector_appearance_expanded = true;
    if (modern_style && platform_text_ui_app_is_active()) {
        platform_text_ui_preferences preferences = {0};
        if (platform_text_ui_get_extended_preferences(&preferences))
            editor_preferences_changed(&preferences, editor);
    }
    editor_status_ok(editor, "Ready");
    if (!document_edit_root(editor->edit, &editor->selected)) {
        viewer_editor_destroy(editor);
        return NULL;
    }
    editor_capture_selection_identity(editor);
    editor_refresh_outline(editor);
    if (modern_style)
        (void)platform_text_ui_watch_preferences(editor_preferences_changed, editor);
    if (modern_style) {
        platform_accessibility_set_view_callback(editor_view_menu_changed, editor,
            editor->want_structure, editor->want_inspector);
        platform_accessibility_set_command_callback(editor_accessibility_command, editor);
    }
    return editor;
}

void viewer_editor_destroy(viewer_editor *editor)
{
    if (editor == NULL) return;
    platform_text_ui_unwatch_preferences(editor);
    platform_accessibility_clear_view_callback(editor);
    platform_accessibility_set_command_callback(NULL, NULL);
    document_edit_destroy(editor->edit);
    free(editor);
}

bool viewer_editor_set_appearance(viewer_editor *editor, ui_theme theme,
                                   bool increased_contrast, bool reduced_motion)
{
    if (editor == NULL || (theme != UI_THEME_DARK && theme != UI_THEME_LIGHT)) return false;
    if (!editor->modern_style) return true;
    if (editor->theme == theme && editor->increased_contrast == increased_contrast &&
        editor->reduced_motion == reduced_motion) return false;
    editor->theme = theme;
    editor->increased_contrast = increased_contrast;
    editor->reduced_motion = reduced_motion;
    if (reduced_motion && editor->sidebar_motion_active) {
        editor->layout_structure_width_points = editor->sidebar_motion_target_structure;
        editor->layout_inspector_width_points = editor->sidebar_motion_target_inspector;
        editor->sidebar_motion_active = false;
    }
    if (reduced_motion) {
        for (size_t index = 0U; index < EDITOR_MOTION_CONTROL_CAP; ++index)
            editor->control_transitions[index].active = false;
    }
    editor->tree_ready = false;
    editor->redraw_requested = true;
    return true;
}

bool viewer_editor_set_system_accent(viewer_editor *editor, uint32_t accent_srgb,
                                     bool material_allowed)
{
    if (editor == NULL) return false;
    if (editor->system_accent_srgb == accent_srgb &&
        editor->native_material_allowed == material_allowed) return false;
    editor->system_accent_srgb = accent_srgb;
    editor->native_material_allowed = material_allowed;
    editor->tree_ready = false;
    editor->redraw_requested = true;
    return true;
}

bool viewer_editor_set_ui_scale(viewer_editor *editor, uint16_t percent)
{
    bool valid = percent == 100U || percent == 110U || percent == 120U ||
        (percent >= 125U && percent <= 200U && (percent % 25U) == 0U);
    if (editor == NULL || !valid) return false;
    if (editor->ui_scale_percent == percent) return false;
    editor->ui_scale_percent = percent;
    editor->tree_ready = false;
    editor->redraw_requested = true;
    return true;
}

bool viewer_editor_set_interface_scale(viewer_editor *editor,
                                       uint8_t scale_percent)
{
    if (editor == NULL) return false;
    if (editor->ui_scale_percent == scale_percent) return true;
    return viewer_editor_set_ui_scale(editor, (uint16_t)scale_percent);
}

bool viewer_editor_set_panel_widths(viewer_editor *editor,
    uint16_t structure_width_points, uint16_t inspector_width_points)
{
    if (editor == NULL || structure_width_points < 184U ||
        structure_width_points > 320U || inspector_width_points < 240U ||
        inspector_width_points > 440U) return false;
    if (editor->structure_width_points == structure_width_points &&
        editor->inspector_width_points == inspector_width_points) return false;
    editor->structure_width_points = structure_width_points;
    editor->inspector_width_points = inspector_width_points;
    editor->layout_structure_width_points = structure_width_points;
    editor->layout_inspector_width_points = inspector_width_points;
    editor->tree_ready = false;
    editor->redraw_requested = true;
    return true;
}

bool viewer_editor_native_material_enabled(const viewer_editor *editor)
{
    return editor != NULL && editor->modern_style &&
        editor->native_material_allowed && !editor->increased_contrast;
}

uint16_t viewer_editor_ui_scale_percent(const viewer_editor *editor)
{
    return editor == NULL || editor->ui_scale_percent == 0U ? 100U :
        editor->ui_scale_percent;
}

static uint32_t density_scale(const viewer_editor *editor, uint32_t comfortable,
                              uint32_t compact)
{
    return editor != NULL && editor->density == VIEWER_EDITOR_DENSITY_COMPACT ?
        compact : comfortable;
}

uint32_t editor_row_height(const viewer_editor *editor)
{
    return density_scale(editor, 28U, 23U);
}

uint32_t editor_field_height(const viewer_editor *editor)
{
    return density_scale(editor, 30U, 25U);
}

uint32_t editor_section_height(const viewer_editor *editor)
{
    return density_scale(editor, 24U, 21U);
}

void viewer_editor_set_density(viewer_editor *editor, uint8_t density)
{
    if (editor == NULL) return;
    if (density != VIEWER_EDITOR_DENSITY_COMFORTABLE)
        density = VIEWER_EDITOR_DENSITY_COMPACT;
    if (editor->density == density) return;
    editor->density = density;
    editor->tree_ready = false;
    editor->redraw_requested = true;
    if (editor->density_changed != NULL)
        editor->density_changed(editor->density_context, density);
}

void viewer_editor_set_density_callback(viewer_editor *editor,
    viewer_editor_density_callback callback, void *context)
{
    if (editor == NULL) return;
    editor->density_changed = callback;
    editor->density_context = context;
}

uint8_t viewer_editor_density(const viewer_editor *editor)
{
    return editor == NULL ? VIEWER_EDITOR_DENSITY_COMPACT : editor->density;
}

bool viewer_editor_set_focus_mode(viewer_editor *editor, bool focus)
{
    if (editor == NULL) return false;
    if (editor->focus_mode == focus) return false;
    if (focus) {
        editor->focus_saved_structure = editor->want_structure;
        editor->focus_saved_inspector = editor->want_inspector;
        editor->focus_saved_timeline = editor->timeline_visible;
        editor->focus_mode = true;
        (void)viewer_editor_set_sidebar_visibility(editor, false, false);
        if (editor->timeline_available)
            (void)viewer_editor_set_timeline_state(editor, true, false,
                editor->timeline_height_points);
    } else {
        editor->focus_mode = false;
        (void)viewer_editor_set_sidebar_visibility(editor,
            editor->focus_saved_structure, editor->focus_saved_inspector);
        if (editor->timeline_available)
            (void)viewer_editor_set_timeline_state(editor, true,
                editor->focus_saved_timeline, editor->timeline_height_points);
    }
    editor->tree_ready = false;
    editor->redraw_requested = true;
    return true;
}

bool viewer_editor_toggle_focus_mode(viewer_editor *editor)
{
    return editor == NULL ? false :
        viewer_editor_set_focus_mode(editor, !editor->focus_mode);
}

bool viewer_editor_focus_mode(const viewer_editor *editor)
{
    return editor != NULL && editor->focus_mode;
}

bool viewer_editor_set_workspace(viewer_editor *editor, viewer_workspace_kind workspace)
{
    if (editor == NULL) return false;
    if ((size_t)workspace >= viewer_workspace_count())
        workspace = viewer_workspace_default();
    const viewer_workspace_spec *spec = viewer_workspace_get(workspace);
    bool changed = editor->workspace != workspace;
    editor->workspace = workspace;
    /* A workspace only selects panes; it never changes the document. */
    bool sidebar_changed = editor->want_structure != spec->structure ||
        editor->want_inspector != spec->inspector;
    if (sidebar_changed)
        (void)viewer_editor_set_sidebar_visibility(editor, spec->structure,
            spec->inspector);
    if (editor->timeline_available && editor->timeline_visible != spec->timeline)
        (void)viewer_editor_set_timeline_state(editor, true, spec->timeline,
            editor->timeline_height_points);
    /* The workspace also selects the inspector tab and, for Nodes, the
     * geometry section; a NULL record keeps what the user has open. */
    if (spec->inspector_tab != NULL) {
        uint8_t tab = strcmp(spec->inspector_tab, "Document") == 0 ?
            (uint8_t)(EDITOR_BUTTON_INSPECTOR_TAB_DOCUMENT -
                EDITOR_BUTTON_INSPECTOR_TAB_OBJECT) : 0U;
        if (editor->inspector_tab != tab) {
            editor->inspector_tab = tab;
            changed = true;
        }
    }
    if (spec->inspector_section != NULL &&
        strcmp(spec->inspector_section, "Geometry") == 0)
        editor->inspector_geometry_expanded = true;
    if (editor->workspace_changed != NULL)
        editor->workspace_changed(editor->workspace_context,
            viewer_workspace_name(workspace));
    if (changed) {
        editor->tree_ready = false;
        editor->redraw_requested = true;
    }
    return changed || sidebar_changed;
}

bool viewer_editor_cycle_workspace(viewer_editor *editor, bool backwards)
{
    if (editor == NULL) return false;
    return viewer_editor_set_workspace(editor,
        viewer_workspace_cycle(editor->workspace, backwards));
}

void viewer_editor_set_workspace_callback(viewer_editor *editor,
    viewer_editor_workspace_callback callback, void *context)
{
    if (editor == NULL) return;
    editor->workspace_changed = callback;
    editor->workspace_context = context;
}

bool viewer_editor_apply_workspace_name(viewer_editor *editor,
    const char *name, size_t length)
{
    if (editor == NULL) return false;
    return viewer_editor_set_workspace(editor,
        viewer_workspace_from_name(name, length));
}

viewer_workspace_kind viewer_editor_workspace(const viewer_editor *editor)
{
    return editor == NULL ? viewer_workspace_default() : editor->workspace;
}

bool viewer_editor_set_sidebar_visibility(viewer_editor *editor,
                                          bool structure_visible,
                                          bool inspector_visible)
{
    if (editor == NULL) return false;
    if (editor->want_structure == structure_visible &&
        editor->want_inspector == inspector_visible) return false;
    editor->want_structure = structure_visible;
    editor->want_inspector = inspector_visible;
    uint16_t current_structure = editor->layout_structure_width_points;
    uint16_t current_inspector = editor->layout_inspector_width_points;
    if (!editor->reduced_motion) {
        editor->sidebar_motion_start_structure = current_structure;
        editor->sidebar_motion_start_inspector = current_inspector;
        editor->sidebar_motion_target_structure = structure_visible ?
            editor->structure_width_points : 0U;
        editor->sidebar_motion_target_inspector = inspector_visible ?
            editor->inspector_width_points : 0U;
        editor->sidebar_motion_start_seconds = editor->motion_time_seconds;
        editor->sidebar_motion_show_structure = structure_visible || current_structure > 0U;
        editor->sidebar_motion_show_inspector = inspector_visible || current_inspector > 0U;
        editor->sidebar_motion_active = true;
    } else {
        editor->layout_structure_width_points = structure_visible ?
            editor->structure_width_points : 0U;
        editor->layout_inspector_width_points = inspector_visible ?
            editor->inspector_width_points : 0U;
        editor->sidebar_motion_active = false;
    }
    editor->tree_ready = false;
    editor->redraw_requested = true;
    if (editor->modern_style)
        platform_accessibility_set_view_callback(editor_view_menu_changed, editor,
            structure_visible, inspector_visible);
    return true;
}

bool viewer_editor_set_timeline_state(viewer_editor *editor, bool available,
                                      bool visible, uint32_t height_points)
{
    if (editor == NULL || height_points < VIEWER_TIMELINE_DOCK_MIN_POINTS ||
        height_points > VIEWER_TIMELINE_DOCK_MAX_POINTS) return false;
    visible = available && visible;
    if (editor->timeline_available == available &&
        editor->timeline_visible == visible &&
        editor->timeline_height_points == height_points) return false;
    editor->timeline_available = available;
    editor->timeline_visible = visible;
    editor->timeline_height_points = (uint16_t)height_points;
    editor->tree_ready = false;
    editor->redraw_requested = true;
    return true;
}

bool viewer_editor_timeline_visible(const viewer_editor *editor)
{
    return editor != NULL && editor->timeline_visible;
}

bool viewer_editor_timeline_available(const viewer_editor *editor)
{
    return editor != NULL && editor->timeline_available;
}

bool viewer_editor_reduced_motion(const viewer_editor *editor)
{
    return editor != NULL && editor->reduced_motion;
}

bool viewer_editor_set_tool(viewer_editor *editor, ui_icon_kind tool)
{
    if (editor == NULL || tool < UI_ICON_SELECT || tool > UI_ICON_ZOOM) return false;
    if (editor->active_tool == tool &&
        editor->draw_tool == VIEWER_EDITOR_TOOL_SELECT) return false;
    editor->active_tool = tool;
    if (editor_canvas_path_active(editor)) editor_canvas_path_cancel(editor);
    editor->draw_tool = VIEWER_EDITOR_TOOL_SELECT;
    editor->node_edit_active = false;
    editor->tree_ready = false;
    editor->redraw_requested = true;
    return true;
}

ui_icon_kind viewer_editor_get_tool(const viewer_editor *editor)
{
    return editor == NULL ? UI_ICON_SELECT : editor->active_tool;
}

bool viewer_editor_text_input_active(const viewer_editor *editor)
{
    return editor != NULL && editor->field_kind != EDITOR_FIELD_NONE;
}

bool viewer_editor_set_draw_tool(viewer_editor *editor,
                                 viewer_editor_draw_tool tool)
{
    if (editor == NULL || tool < VIEWER_EDITOR_TOOL_SELECT ||
        tool > VIEWER_EDITOR_TOOL_TEXT) return false;
    bool node_mode_changed = editor->node_edit_active;
    editor->node_edit_active = false;
    if (editor->draw_tool == tool) {
        if (node_mode_changed) {
            editor->tree_ready = false;
            editor->redraw_requested = true;
        }
        return node_mode_changed;
    }
    if (editor_canvas_path_active(editor)) editor_canvas_path_cancel(editor);
    editor->draw_tool = tool;
    editor->active_tool = UI_ICON_SELECT;
    editor->tree_ready = false;
    editor->redraw_requested = true;
    return true;
}

viewer_editor_draw_tool viewer_editor_get_draw_tool(const viewer_editor *editor)
{
    return editor == NULL ? VIEWER_EDITOR_TOOL_SELECT : editor->draw_tool;
}

bool viewer_editor_get_chrome_layout(viewer_editor *editor,
                                     struct viewer_chrome_layout *out_layout)
{
    if (editor == NULL || out_layout == NULL) return false;
    uint32_t timeline_points = editor->timeline_visible ?
        editor->timeline_height_points : 32U;
    double ui_scale = (double)viewer_editor_ui_scale_percent(editor) / 100.0;
    uint16_t structure = editor->layout_structure_width_points;
    uint16_t inspector = editor->layout_inspector_width_points;
    bool want_structure = editor->sidebar_motion_active ?
        (editor->sidebar_motion_start_structure > 0U ||
         editor->sidebar_motion_target_structure > 0U) : editor->want_structure;
    bool want_inspector = editor->sidebar_motion_active ?
        (editor->sidebar_motion_start_inspector > 0U ||
         editor->sidebar_motion_target_inspector > 0U) : editor->want_inspector;
    return viewer_chrome_layout_compute_docked_sized(editor->width, editor->height,
        editor->backing_scale, ui_scale, structure,
        inspector, want_structure,
        want_inspector, editor->timeline_available, timeline_points,
        (viewer_chrome_layout *)out_layout);
}
