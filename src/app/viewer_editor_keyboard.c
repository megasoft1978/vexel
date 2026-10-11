#include "viewer_editor_internal.h"

#include "platform/platform_localization.h"

#include <math.h>
#include <string.h>

double editor_presentation_scale(const viewer_editor *editor)
{
    double backing = editor != NULL && isfinite(editor->backing_scale) &&
        editor->backing_scale > 0.0 ? editor->backing_scale : 1.0;
    uint8_t text_percent = editor == NULL ? 100U : editor->interface_scale_percent;
    if (text_percent < 100U || text_percent > 200U || (text_percent % 25U) != 0U)
        text_percent = 100U;
    double scale = backing * ((double)text_percent / 100.0);
    return !isfinite(scale) || scale > 4.0 ? 4.0 : scale;
}

bool editor_localize_slice(const char *source, size_t length,
    char *output, size_t capacity, size_t *out_length)
{
    if (source == NULL || output == NULL || capacity == 0U || out_length == NULL)
        return false;
    return platform_localization_copy_default(source, length, output, capacity,
        out_length);
}

static bool focused_outline_position(viewer_editor *editor, size_t *out_visible)
{
    if (editor == NULL || out_visible == NULL ||
        editor->focus_id < EDITOR_OUTLINE_ROW_BASE ||
        editor->focus_id >= EDITOR_OUTLINE_ROW_BASE + EDITOR_OUTLINE_VISIBLE)
        return false;
    size_t row = editor->focus_id - EDITOR_OUTLINE_ROW_BASE;
    size_t visible = editor->outline_visible_start + row;
    size_t model_index = 0U;
    if (!editor_outline_visible_model_index(editor, visible, &model_index)) return false;
    *out_visible = visible;
    return true;
}

static bool editor_focus_outline_row(viewer_editor *editor, size_t visible)
{
    if (editor == NULL || visible >= editor->outline_visible_count) return false;
    size_t maximum = editor->outline_visible_count > EDITOR_OUTLINE_VISIBLE ?
        editor->outline_visible_count - EDITOR_OUTLINE_VISIBLE : 0U;
    if (visible < editor->outline_visible_start)
        editor->outline_visible_start = visible;
    else if (visible >= editor->outline_visible_start + EDITOR_OUTLINE_VISIBLE)
        editor->outline_visible_start = visible - EDITOR_OUTLINE_VISIBLE + 1U;
    size_t model_index = 0U;
    if (!editor_outline_visible_model_index(editor, visible, &model_index)) return false;
    if (editor->outline_visible_start > maximum)
        editor->outline_visible_start = maximum;
    (void)editor_select_outline_index(editor, model_index);
    editor->focus_id = EDITOR_OUTLINE_ROW_BASE +
        (uint32_t)(visible - editor->outline_visible_start);
    editor->tree_ready = false;
    editor->redraw_requested = true;
    return true;
}

static bool focused_outline_has_children(viewer_editor *editor, size_t visible,
                                          size_t *out_model_index,
                                          document_handle *out_child)
{
    size_t model_index = 0U;
    if (!editor_outline_visible_model_index(editor, visible, &model_index)) return false;
    if (!document_model_first_child(editor_model(editor), editor->outline[model_index],
            out_child)) return false;
    *out_model_index = model_index;
    return true;
}

static bool focus_outline_parent(viewer_editor *editor, size_t model_index)
{
    document_handle parent = {0};
    if (!document_model_parent(editor_model(editor), editor->outline[model_index],
            &parent)) return false;
    for (size_t visible = 0U; visible < editor->outline_visible_count; ++visible) {
        size_t candidate = 0U;
        if (editor_outline_visible_model_index(editor, visible, &candidate) &&
            editor_same_handle(editor->outline[candidate], parent))
            return editor_focus_outline_row(editor, visible);
    }
    return false;
}

static bool focus_outline_child(viewer_editor *editor, size_t model_index,
                                size_t current_visible, document_handle child)
{
    for (size_t visible = current_visible + 1U;
         visible < editor->outline_visible_count; ++visible) {
        size_t candidate = 0U;
        if (editor_outline_visible_model_index(editor, visible, &candidate) &&
            editor_same_handle(editor->outline[candidate], child))
            return editor_focus_outline_row(editor, visible);
    }
    (void)model_index;
    return false;
}

static uint32_t handle_outline_arrow(viewer_editor *editor, uint16_t key_code)
{
    size_t visible = 0U;
    if (!focused_outline_position(editor, &visible)) return 0U;
    if (key_code == 126U) {
        if (visible > 0U) (void)editor_focus_outline_row(editor, visible - 1U);
        return VIEWER_EDITOR_HANDLED;
    }
    if (key_code == 125U) {
        if (visible + 1U < editor->outline_visible_count)
            (void)editor_focus_outline_row(editor, visible + 1U);
        return VIEWER_EDITOR_HANDLED;
    }
    size_t model_index = 0U;
    document_handle child = {0};
    bool has_children = focused_outline_has_children(editor, visible,
        &model_index, &child);
    if (key_code == 123U) {
        if (has_children && !editor_outline_is_collapsed(editor, model_index)) {
            (void)editor_outline_toggle_collapsed(editor, model_index);
            editor->focus_id = EDITOR_OUTLINE_ROW_BASE +
                (uint32_t)(visible - editor->outline_visible_start);
            return VIEWER_EDITOR_HANDLED;
        }
        if (focus_outline_parent(editor, model_index)) return VIEWER_EDITOR_HANDLED;
        return VIEWER_EDITOR_HANDLED;
    }
    if (key_code == 124U) {
        if (!has_children) return VIEWER_EDITOR_HANDLED;
        if (editor_outline_is_collapsed(editor, model_index)) {
            (void)editor_outline_toggle_collapsed(editor, model_index);
            editor->focus_id = EDITOR_OUTLINE_ROW_BASE +
                (uint32_t)(visible - editor->outline_visible_start);
            return VIEWER_EDITOR_HANDLED;
        }
        (void)focus_outline_child(editor, model_index, visible, child);
        return VIEWER_EDITOR_HANDLED;
    }
    return 0U;
}

static bool begin_focused_text_field(viewer_editor *editor)
{
    if (editor->focus_id == EDITOR_OUTLINE_SEARCH_ID)
        return editor_begin_field(editor, EDITOR_FIELD_SEARCH, 0U);
    if (editor->focus_id == EDITOR_RENAME_ID)
        return editor_begin_field(editor, EDITOR_FIELD_RENAME, 0U);
    if (editor->focus_id == EDITOR_TEXT_FIELD_ID)
        return editor_begin_field(editor, EDITOR_FIELD_TEXT, 0U);
    if (editor->focus_id == EDITOR_NEW_NAME_ID)
        return editor_begin_field(editor, EDITOR_FIELD_NEW_NAME, 0U);
    if (editor->focus_id == EDITOR_NEW_VALUE_ID)
        return editor_begin_field(editor, EDITOR_FIELD_NEW_VALUE, 0U);
    if (editor->focus_id >= EDITOR_ATTRIBUTE_ROW_BASE &&
        editor->focus_id < EDITOR_ATTRIBUTE_ROW_BASE + 2048U) {
        size_t attribute = editor->focus_id - EDITOR_ATTRIBUTE_ROW_BASE;
        return editor_begin_field(editor, EDITOR_FIELD_ATTRIBUTE, attribute);
    }
    return false;
}

static bool focused_text_key(uint16_t key_code, uint32_t modifiers,
                             const char *text, size_t text_length)
{
    if ((modifiers & (PLATFORM_MODIFIER_OPTION | PLATFORM_MODIFIER_CONTROL)) != 0U)
        return false;
    if ((modifiers & PLATFORM_MODIFIER_COMMAND) != 0U)
        return text_length == 1U && text != NULL &&
            (text[0] == 'a' || text[0] == 'A');
    return text_length > 0U || key_code == 123U || key_code == 124U ||
        key_code == 125U || key_code == 126U || key_code == 115U ||
        key_code == 119U || key_code == 51U || key_code == 117U ||
        key_code == 36U || key_code == 76U || key_code == 49U;
}

uint32_t editor_handle_focused_key(viewer_editor *editor, uint16_t key_code,
    uint32_t modifiers, const char *text, size_t text_length)
{
    if (editor == NULL || editor->field_kind != EDITOR_FIELD_NONE ||
        editor->focus_id == 0U) return 0U;
    if (!editor->tree_ready) editor_build_tree(editor);
    const ui_node *focused = ui_tree_find(&editor->tree, editor->focus_id);
    if (focused == NULL || (focused->spec.state & UI_STATE_DISABLED) != 0U)
        return 0U;
    if (focused->spec.role == UI_ROLE_ROW && key_code >= 123U && key_code <= 126U &&
        (modifiers & (PLATFORM_MODIFIER_COMMAND | PLATFORM_MODIFIER_OPTION |
            PLATFORM_MODIFIER_CONTROL)) == 0U)
        return handle_outline_arrow(editor, key_code);
    if (focused->spec.role == UI_ROLE_TEXT_FIELD &&
        focused_text_key(key_code, modifiers, text, text_length)) {
        if (key_code == 36U || key_code == 76U) {
            (void)begin_focused_text_field(editor);
            return VIEWER_EDITOR_HANDLED;
        }
        if (!begin_focused_text_field(editor)) return VIEWER_EDITOR_HANDLED;
        return viewer_editor_key(editor, key_code, modifiers, text, text_length);
    }
    if (key_code == 36U || key_code == 76U || key_code == 49U) {
        if (focused->spec.role == UI_ROLE_ROW) {
            size_t visible = 0U, model_index = 0U;
            if (focused_outline_position(editor, &visible) &&
                editor_outline_visible_model_index(editor, visible, &model_index))
                (void)editor_select_outline_index(editor, model_index);
            editor->tree_ready = false;
            return VIEWER_EDITOR_HANDLED;
        }
        if (focused->spec.role == UI_ROLE_BUTTON) {
            uint32_t x = focused->bounds.x + focused->bounds.width / 2U;
            uint32_t y = focused->bounds.y + focused->bounds.height / 2U;
            uint32_t result = viewer_editor_pointer_down(editor, x, y, 0U);
            viewer_editor_pointer_up(editor);
            return result | VIEWER_EDITOR_HANDLED;
        }
    }
    if (key_code >= 123U && key_code <= 126U) return VIEWER_EDITOR_HANDLED;
    return 0U;
}
