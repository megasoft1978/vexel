#include "viewer_editor_internal.h"

#include "document_model.h"
#include "viewer_chrome.h"

#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

void viewer_editor_set_canvas_status(viewer_editor *editor, double zoom,
    bool pointer_valid, double pointer_x, double pointer_y,
    uint32_t document_width, uint32_t document_height,
    size_t tiles_done, size_t tiles_total, bool tiles_active)
{
    if (editor == NULL || !isfinite(zoom) || !(zoom > 0.0) ||
        (pointer_valid && (!isfinite(pointer_x) || !isfinite(pointer_y)))) return;
    if (editor->canvas_zoom == zoom && editor->canvas_pointer_valid == pointer_valid &&
        editor->canvas_pointer_x == pointer_x && editor->canvas_pointer_y == pointer_y &&
        editor->canvas_document_width == document_width &&
        editor->canvas_document_height == document_height &&
        editor->canvas_tiles_done == tiles_done &&
        editor->canvas_tiles_total == tiles_total &&
        editor->canvas_tiles_active == tiles_active) return;
    editor->canvas_zoom = zoom;
    editor->canvas_pointer_valid = pointer_valid;
    editor->canvas_pointer_x = pointer_x;
    editor->canvas_pointer_y = pointer_y;
    editor->canvas_document_width = document_width;
    editor->canvas_document_height = document_height;
    editor->canvas_tiles_done = tiles_done;
    editor->canvas_tiles_total = tiles_total;
    editor->canvas_tiles_active = tiles_active;
    editor->tree_ready = false;
    editor->redraw_requested = true;
}

static bool editor_node_contains_canvas(const ui_node *node, uint32_t x,
                                        uint32_t y)
{
    return node != NULL && x >= node->bounds.x && y >= node->bounds.y &&
        x - node->bounds.x < node->bounds.width &&
        y - node->bounds.y < node->bounds.height;
}

/* Applies a horizontal drag to the armed numeric field. The value is shown live
 * and committed once on release, so a scrub is one undo command. */
static bool editor_field_scrub_move(viewer_editor *editor, uint32_t x)
{
    if (!editor->field_scrub_armed) return false;
    if (x > editor->field_scrub_start_x)
        editor->field_scrub_moved = true;
    else if (editor->field_scrub_start_x > x)
        editor->field_scrub_moved = true;
    int64_t delta = (int64_t)x - (int64_t)editor->field_scrub_start_x;
    /* One document unit per pixel, a tenth of a unit with Option held. */
    double step = (editor->scrub_fine ? 0.1 : 1.0) * (double)delta;
    double value = editor->field_scrub_start_value + step;
    char text[64] = {0};
    int amount = snprintf(text, sizeof(text), "%.2f", value);
    if (amount <= 0 || (size_t)amount >= sizeof(text)) return true;
    editor_set_text(editor->field, sizeof(editor->field), text);
    editor->field_length = strlen(editor->field);
    editor->tree_ready = false;
    editor->redraw_requested = true;
    return true;
}

void viewer_editor_pointer_move(viewer_editor *editor, uint32_t x, uint32_t y)
{
    if (editor == NULL) return;
    if (editor_field_scrub_move(editor, x)) return;
    if (editor_splitter_pointer_move(editor, x, y)) return;
    editor_snap_popover_pointer_move(editor, x, y);
    editor_context_action_pointer_move(editor, x, y);
    if (editor->modern_style) {
        viewer_chrome_layout layout;
        if (viewer_editor_get_chrome_layout(editor, &layout) &&
            x >= layout.canvas.x && y >= layout.canvas.y &&
            x - layout.canvas.x < layout.canvas.width &&
            y - layout.canvas.y < layout.canvas.height) {
            editor->context_pointer_valid = true;
            editor->context_pointer_x = x - layout.canvas.x;
            editor->context_pointer_y = y - layout.canvas.y;
        } else editor->context_pointer_valid = false;
    }
    if (!editor->tree_ready) editor_build_tree(editor);
    if (!editor->tree_ready) return;
    uint32_t hovered = 0U;
    for (size_t reverse = editor->tree.count; reverse > 0U; --reverse) {
        const ui_node *node = &editor->tree.nodes[reverse - 1U];
        bool interactive = node->spec.role == UI_ROLE_BUTTON ||
            node->spec.role == UI_ROLE_TEXT_FIELD || node->spec.role == UI_ROLE_ROW;
        if (interactive && editor_node_contains_canvas(node, x, y)) {
            hovered = node->spec.id;
            break;
        }
    }
    bool over_reveal = false;
    if (editor->reveal_active) {
        const ui_node *body = ui_tree_find(&editor->tree, EDITOR_NODE_ID_REVEAL);
        over_reveal = editor_node_contains_canvas(body, x, y);
    }
    bool reveal_changed = false;
    if (hovered >= EDITOR_OUTLINE_ROW_BASE &&
        hovered < EDITOR_OUTLINE_ROW_BASE + EDITOR_OUTLINE_VISIBLE) {
        size_t row = hovered - EDITOR_OUTLINE_ROW_BASE;
        size_t model_index = 0U;
        if (editor_outline_visible_model_index(editor,
                editor->outline_visible_start + row, &model_index)) {
            document_handle handle = editor->outline[model_index];
            if (!editor->reveal_active ||
                !editor_same_handle(editor->reveal_handle, handle)) {
                (void)editor_reveal_prepare(editor, handle);
                reveal_changed = true;
            }
        }
    } else if (!over_reveal && editor->reveal_active) {
        editor->reveal_active = false;
        editor->reveal_has_id = false;
        reveal_changed = true;
    } else if (over_reveal && hovered == 0U) {
        hovered = editor->hovered_id;
    }
    if (hovered != editor->hovered_id) {
        const ui_node *old_node = ui_tree_find(&editor->tree, editor->hovered_id);
        const ui_node *new_node = ui_tree_find(&editor->tree, hovered);
        if (editor->hovered_id != 0U && old_node != NULL &&
            (old_node->spec.state & UI_STATE_CHECKED) == 0U)
            editor_motion_control_transition(editor, editor->hovered_id,
                UI_CONTROL_HOVER, UI_CONTROL_REST, 80U);
        if (hovered != 0U && new_node != NULL &&
            (new_node->spec.state & UI_STATE_CHECKED) == 0U)
            editor_motion_control_transition(editor, hovered,
                UI_CONTROL_REST, UI_CONTROL_HOVER, 80U);
    }
    if (hovered == editor->hovered_id && x == editor->hovered_x &&
        y == editor->hovered_y && !reveal_changed) return;
    editor->hovered_id = hovered;
    editor->hovered_x = x;
    editor->hovered_y = y;
    editor->tree_ready = false;
    editor->redraw_requested = true;
}

void viewer_editor_pointer_up(viewer_editor *editor)
{
    if (editor == NULL) return;
    if (editor->field_scrub_armed) {
        bool moved = editor->field_scrub_moved;
        editor->field_scrub_armed = false;
        editor->field_scrub_moved = false;
        if (moved) (void)editor_commit_field(editor);
        editor->tree_ready = false;
        editor->redraw_requested = true;
        return;
    }
    if (editor->splitter_dragging) {
        editor_splitter_pointer_up(editor);
        editor->pressed_id = 0U;
        editor->tree_ready = false;
        editor->redraw_requested = true;
        return;
    }
    if (editor->pressed_id == 0U) return;
    editor->pressed_id = 0U;
    editor->tree_ready = false;
    editor->redraw_requested = true;
}
