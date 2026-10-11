#include "viewer_editor_internal.h"

#include "document_model.h"

#include <math.h>
#include <stdio.h>
#include <string.h>

static bool format_number(char out[64], double value)
{
    if (out == NULL || !isfinite(value) || fabs(value) > 1000000.0) return false;
    int amount = snprintf(out, 64U, "%.9g", value);
    return amount > 0 && amount < 64;
}

uint32_t editor_insert_drawn_shape(viewer_editor *editor,
    const svg_shape_document *document, geom_point start, geom_point end)
{
    if (editor == NULL || document == NULL) return VIEWER_EDITOR_HANDLED;
    double left = fmin(start.x, end.x), top = fmin(start.y, end.y);
    double width = fabs(end.x - start.x), height = fabs(end.y - start.y);
    if (editor->draw_tool != VIEWER_EDITOR_TOOL_LINE &&
        (!(width >= 0.5) || !(height >= 0.5))) {
        editor_status_ok(editor, "Drag farther to create a shape");
        return VIEWER_EDITOR_HANDLED;
    }
    if (editor->draw_tool == VIEWER_EDITOR_TOOL_LINE &&
        width < 0.5 && height < 0.5) {
        editor_status_ok(editor, "Drag farther to create a line");
        return VIEWER_EDITOR_HANDLED;
    }
    char values[8][64] = {{0}};
    const char *names[8] = {0};
    size_t name_lengths[8] = {0U};
    size_t count = 0U;
    document_edit_template_kind kind;
    if (editor->draw_tool == VIEWER_EDITOR_TOOL_RECT) {
        kind = DOCUMENT_EDIT_TEMPLATE_RECT;
        names[count] = "x"; name_lengths[count] = 1U;
        if (!format_number(values[count++], left)) return VIEWER_EDITOR_HANDLED;
        names[count] = "y"; name_lengths[count] = 1U;
        if (!format_number(values[count++], top)) return VIEWER_EDITOR_HANDLED;
        names[count] = "width"; name_lengths[count] = 5U;
        if (!format_number(values[count++], width)) return VIEWER_EDITOR_HANDLED;
        names[count] = "height"; name_lengths[count] = 6U;
        if (!format_number(values[count++], height)) return VIEWER_EDITOR_HANDLED;
        names[count] = "fill"; name_lengths[count] = 4U;
        memcpy(values[count++], "#4f7cff", 8U);
        names[count] = "stroke"; name_lengths[count] = 6U;
        memcpy(values[count++], "#172033", 8U);
        names[count] = "stroke-width"; name_lengths[count] = 12U;
        memcpy(values[count++], "1", 2U);
    } else if (editor->draw_tool == VIEWER_EDITOR_TOOL_ELLIPSE) {
        kind = DOCUMENT_EDIT_TEMPLATE_ELLIPSE;
        names[count] = "cx"; name_lengths[count] = 2U;
        if (!format_number(values[count++], (start.x + end.x) * 0.5)) return VIEWER_EDITOR_HANDLED;
        names[count] = "cy"; name_lengths[count] = 2U;
        if (!format_number(values[count++], (start.y + end.y) * 0.5)) return VIEWER_EDITOR_HANDLED;
        names[count] = "rx"; name_lengths[count] = 2U;
        if (!format_number(values[count++], width * 0.5)) return VIEWER_EDITOR_HANDLED;
        names[count] = "ry"; name_lengths[count] = 2U;
        if (!format_number(values[count++], height * 0.5)) return VIEWER_EDITOR_HANDLED;
        names[count] = "fill"; name_lengths[count] = 4U;
        memcpy(values[count++], "#4f7cff", 8U);
        names[count] = "stroke"; name_lengths[count] = 6U;
        memcpy(values[count++], "#172033", 8U);
        names[count] = "stroke-width"; name_lengths[count] = 12U;
        memcpy(values[count++], "1", 2U);
    } else {
        kind = DOCUMENT_EDIT_TEMPLATE_LINE;
        names[count] = "x1"; name_lengths[count] = 2U;
        if (!format_number(values[count++], start.x)) return VIEWER_EDITOR_HANDLED;
        names[count] = "y1"; name_lengths[count] = 2U;
        if (!format_number(values[count++], start.y)) return VIEWER_EDITOR_HANDLED;
        names[count] = "x2"; name_lengths[count] = 2U;
        if (!format_number(values[count++], end.x)) return VIEWER_EDITOR_HANDLED;
        names[count] = "y2"; name_lengths[count] = 2U;
        if (!format_number(values[count++], end.y)) return VIEWER_EDITOR_HANDLED;
        names[count] = "fill"; name_lengths[count] = 4U;
        memcpy(values[count++], "none", 5U);
        names[count] = "stroke"; name_lengths[count] = 6U;
        memcpy(values[count++], "#172033", 8U);
        names[count] = "stroke-width"; name_lengths[count] = 12U;
        memcpy(values[count++], "2", 2U);
    }
    document_edit_template_attribute attributes[8];
    for (size_t index = 0U; index < count; ++index) {
        attributes[index] = (document_edit_template_attribute){
            .name = {(const uint8_t *)names[index], name_lengths[index]},
            .value = {(const uint8_t *)values[index], strlen(values[index])}
        };
    }
    document_handle parent = {0U}, root = {0U}, inserted = {0U};
    if (!document_edit_root(editor->edit, &root)) return VIEWER_EDITOR_HANDLED;
    parent = root;
    base_bytes parent_tag = {0};
    if (document_edit_tag_name(editor->edit, editor->selected, &parent_tag) &&
        parent_tag.len == 1U && parent_tag.ptr[0] == 'g') parent = editor->selected;
    size_t old_offset = editor->selected_offset;
    char old_id[EDITOR_NAME_CAP];
    editor_set_text(old_id, sizeof(old_id), editor->selected_has_id ?
        editor->selected_id : "");
    document_edit_error error = document_edit_insert_template(editor->edit,
        parent, kind, attributes, count, &inserted);
    if (error != DOCUMENT_EDIT_OK)
        return editor_apply_result(editor, error, old_offset, old_id,
            (document_handle){0U});
    editor->canvas_selection_valid = false;
    editor_status_ok(editor, "Shape created · preview updating");
    return editor_apply_result(editor, error, old_offset, old_id, inserted);
}

