#include "viewer_editor_internal.h"

#include <math.h>
#include <stdio.h>
#include <string.h>

static bool text_number(char output[64], double value)
{
    if (output == NULL || !isfinite(value) || fabs(value) > 1000000.0) return false;
    int amount = snprintf(output, 64U, "%.9g", value);
    return amount > 0 && amount < 64;
}

uint32_t editor_canvas_text_place(viewer_editor *editor,
    const svg_shape_document *document, geom_point user)
{
    if (editor == NULL || editor->edit == NULL || document == NULL ||
        !isfinite(user.x) || !isfinite(user.y) || fabs(user.x) > 1000000.0 ||
        fabs(user.y) > 1000000.0) return VIEWER_EDITOR_HANDLED;
    editor->text_draft_active = true;
    editor->text_draft_x = user.x;
    editor->text_draft_y = user.y;
    editor->field_kind = EDITOR_FIELD_TEXT;
    editor->field_length = 0U;
    editor->field[0] = '\0';
    editor->cursor = 0U;
    editor->anchor = 0U;
    editor->focus_id = EDITOR_TEXT_FIELD_ID;
    editor->tree_ready = false;
    editor->redraw_requested = true;
    editor_status_ok(editor, "Type text, then press Return");
    return VIEWER_EDITOR_HANDLED;
}

uint32_t editor_commit_text_draft(viewer_editor *editor)
{
    if (editor == NULL || editor->edit == NULL || !editor->text_draft_active)
        return VIEWER_EDITOR_HANDLED;
    char x_text[64], y_text[64];
    if (!text_number(x_text, editor->text_draft_x) ||
        !text_number(y_text, editor->text_draft_y)) return VIEWER_EDITOR_HANDLED;
    document_edit_template_attribute attributes[4] = {
        {{(const uint8_t *)"x", 1U}, {(const uint8_t *)x_text, strlen(x_text)}},
        {{(const uint8_t *)"y", 1U}, {(const uint8_t *)y_text, strlen(y_text)}},
        {{(const uint8_t *)"font-size", 9U}, {(const uint8_t *)"16", 2U}},
        {{(const uint8_t *)"fill", 4U}, {(const uint8_t *)"#172033", 7U}}
    };
    document_handle parent = {0}, root = {0}, inserted = {0};
    if (!document_edit_root(editor->edit, &root)) return VIEWER_EDITOR_HANDLED;
    parent = root;
    base_bytes tag = {0};
    if (document_edit_tag_name(editor->edit, editor->selected, &tag) &&
        tag.len == 1U && tag.ptr[0] == 'g') parent = editor->selected;
    size_t old_offset = editor->selected_offset;
    char old_id[EDITOR_NAME_CAP];
    editor_set_text(old_id, sizeof(old_id), editor->selected_has_id ?
        editor->selected_id : "");
    document_edit_error error = document_edit_insert_text_element(editor->edit,
        parent, attributes, 4U,
        (base_bytes){(const uint8_t *)editor->field, editor->field_length},
        &inserted);
    if (error != DOCUMENT_EDIT_OK)
        return editor_apply_result(editor, error, old_offset, old_id,
            (document_handle){0});
    editor->text_draft_active = false;
    uint32_t result = editor_apply_result(editor, DOCUMENT_EDIT_OK,
        old_offset, old_id, inserted);
    editor_status_ok(editor, "Text created · preview updating");
    return result;
}
