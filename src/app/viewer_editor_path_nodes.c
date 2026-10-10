#include "viewer_editor_internal.h"

#include <math.h>
#include <stdio.h>
#include <string.h>

#define EDITOR_PATH_NODE_RENDER_CAP VIEWER_EDITOR_PATH_POINT_CAP

typedef struct path_node_ref {
    size_t segment_index;
    size_t number_x;
    size_t number_y;
    size_t out_segment;
    size_t out_number_x;
    size_t out_number_y;
    size_t in_segment;
    size_t in_number_x;
    size_t in_number_y;
    double x;
    double y;
    double origin_x;
    double origin_y;
    geom_point out_control;
    geom_point in_control;
    double out_origin_x;
    double out_origin_y;
    double in_origin_x;
    double in_origin_y;
    bool has_x;
    bool has_y;
    bool has_out;
    bool has_in;
    bool relative;
} path_node_ref;

static bool path_inverse_map(geom_affine matrix, geom_point point,
                             geom_point *out)
{
    double determinant = matrix.a * matrix.d - matrix.b * matrix.c;
    if (out == NULL || !geom_affine_valid(matrix) || !isfinite(determinant) ||
        fabs(determinant) < 1.0e-12) return false;
    double x = point.x - matrix.e, y = point.y - matrix.f;
    out->x = (matrix.d * x - matrix.c * y) / determinant;
    out->y = (-matrix.b * x + matrix.a * y) / determinant;
    return isfinite(out->x) && isfinite(out->y) && fabs(out->x) <= 1000000.0 &&
        fabs(out->y) <= 1000000.0;
}

static bool path_numbers_at(const svg_shape_document *document,
    const svg_shape_path_segment *segment, size_t offset, size_t count,
    double *x, double *y)
{
    if (document == NULL || segment == NULL || offset > segment->number_count ||
        count > segment->number_count - offset || count == 0U || x == NULL || y == NULL)
        return false;
    size_t first = segment->first_number + offset;
    if (first > document->path_number_count ||
        count > document->path_number_count - first) return false;
    *x = document->path_numbers[first].value;
    *y = count > 1U ? document->path_numbers[first + 1U].value : 0.0;
    return isfinite(*x) && isfinite(*y);
}

static bool path_collect_nodes(const svg_shape_document *document, size_t shape,
    path_node_ref out[EDITOR_PATH_NODE_RENDER_CAP], size_t *out_count)
{
    if (document == NULL || out == NULL || out_count == NULL ||
        document->path_segments == NULL || document->path_numbers == NULL ||
        shape >= document->shape_count) return false;
    size_t count = 0U, previous_node = SIZE_MAX;
    double current_x = 0.0, current_y = 0.0;
    double first_x = 0.0, first_y = 0.0;
    for (size_t index = 0U; index < document->path_segment_count; ++index) {
        const svg_shape_path_segment *segment = &document->path_segments[index];
        if (segment->shape_index != shape) continue;
        char command = segment->command;
        char upper = command >= 'a' && command <= 'z' ?
            (char)(command - 'a' + 'A') : command;
        bool relative = command >= 'a' && command <= 'z';
        if (upper == 'Z') {
            current_x = first_x;
            current_y = first_y;
            previous_node = SIZE_MAX;
            continue;
        }
        size_t x_offset = 0U, y_offset = 1U;
        bool has_x = true, has_y = true, has_control = false;
        double control_x = 0.0, control_y = 0.0;
        size_t control_offset = SIZE_MAX;
        if (upper == 'H') { y_offset = SIZE_MAX; has_y = false; }
        else if (upper == 'V') {
            x_offset = SIZE_MAX; y_offset = 0U; has_x = false;
        }
        else if (upper == 'C') {
            if (segment->number_count != 6U) return false;
            control_offset = 0U;
            if (!path_numbers_at(document, segment, 0U, 2U,
                    &control_x, &control_y)) return false;
            has_control = true;
            x_offset = 4U; y_offset = 5U;
        } else if (upper == 'S') {
            if (segment->number_count != 4U) return false;
            control_offset = 0U;
            if (!path_numbers_at(document, segment, 0U, 2U,
                    &control_x, &control_y)) return false;
            has_control = true;
            x_offset = 2U; y_offset = 3U;
        } else if (upper == 'Q') {
            if (segment->number_count != 4U) return false;
            control_offset = 0U;
            if (!path_numbers_at(document, segment, 0U, 2U,
                    &control_x, &control_y)) return false;
            has_control = true;
            x_offset = 2U; y_offset = 3U;
        } else if (upper == 'A') {
            if (segment->number_count != 7U) return false;
            x_offset = 5U; y_offset = 6U;
        } else if (upper == 'M' || upper == 'L' || upper == 'T') {
            if (segment->number_count != 2U) return false;
        } else return false;
        if (segment->number_count == 0U) continue;
        double origin_x = current_x, origin_y = current_y;
        double x = current_x, y = current_y;
        size_t first_number = segment->first_number;
        if (first_number > document->path_number_count ||
            segment->number_count > document->path_number_count - first_number)
            return false;
        if (has_x) {
            if (x_offset >= segment->number_count) return false;
            double value = document->path_numbers[first_number + x_offset].value;
            x = relative ? current_x + value : value;
        }
        if (has_y) {
            if (y_offset >= segment->number_count) return false;
            double value = document->path_numbers[first_number + y_offset].value;
            y = relative ? current_y + value : value;
        }
        if (!isfinite(x) || !isfinite(y) || fabs(x) > 1000000.0 ||
            fabs(y) > 1000000.0) return false;
        if (count == EDITOR_PATH_NODE_RENDER_CAP) break;
        path_node_ref *node = &out[count];
        *node = (path_node_ref){.segment_index = index,
            .number_x = has_x ? first_number + x_offset : SIZE_MAX,
            .number_y = has_y ? first_number + y_offset : SIZE_MAX,
            .out_segment = SIZE_MAX, .out_number_x = SIZE_MAX,
            .out_number_y = SIZE_MAX, .in_segment = SIZE_MAX,
            .in_number_x = SIZE_MAX, .in_number_y = SIZE_MAX,
            .x = x, .y = y, .origin_x = origin_x, .origin_y = origin_y,
            .has_x = has_x, .has_y = has_y, .relative = relative};
        if (upper == 'M') {
            first_x = x; first_y = y;
            previous_node = SIZE_MAX;
        }
        if (has_control) {
            if (relative) { control_x += origin_x; control_y += origin_y; }
            if (previous_node != SIZE_MAX &&
                (upper == 'C' || upper == 'Q')) {
                path_node_ref *previous = &out[previous_node];
                previous->has_out = true;
                previous->out_control = (geom_point){control_x, control_y};
                previous->out_origin_x = origin_x;
                previous->out_origin_y = origin_y;
                previous->out_segment = index;
                previous->out_number_x = first_number + control_offset;
                previous->out_number_y = first_number + control_offset + 1U;
            }
            node->has_in = true;
            node->in_control = (geom_point){control_x, control_y};
            node->in_origin_x = origin_x;
            node->in_origin_y = origin_y;
            node->in_segment = index;
            node->in_number_x = first_number + control_offset;
            node->in_number_y = first_number + control_offset + 1U;
        }
        ++count;
        current_x = x; current_y = y;
        previous_node = count - 1U;
    }
    *out_count = count;
    return count > 0U;
}

static bool path_selected_shape(viewer_editor *editor,
    const svg_shape_document *document, svg_shape_selection *selection)
{
    base_bytes tag = {0};
    return editor != NULL && document != NULL && selection != NULL &&
        editor->draw_tool == VIEWER_EDITOR_TOOL_PEN &&
        document_edit_tag_name(editor->edit, editor->selected, &tag) &&
        tag.len == 4U && memcmp(tag.ptr, "path", 4U) == 0 &&
        svg_shape_element_at_offset(document, editor->selected_offset, selection);
}

static bool path_stored_range_current(viewer_editor *editor,
    const svg_shape_document *document, document_source_span span)
{
    base_bytes current = {0};
    return editor != NULL && document != NULL && span.offset <= document->source.len &&
        span.length <= document->source.len - span.offset &&
        document_edit_span_bytes(editor->edit, span, &current) &&
        (span.length == 0U || memcmp(current.ptr, document->source.ptr + span.offset,
            span.length) == 0);
}

static size_t path_format_number(char output[64], double value)
{
    if (output == NULL || !isfinite(value) || fabs(value) > 1000000.0) return 0U;
    int amount = snprintf(output, 64U, "%.9g", value);
    return amount > 0 && amount < 64 ? (size_t)amount : 0U;
}

static uint32_t path_commit_numeric_edit(viewer_editor *editor,
    const svg_shape_document *document)
{
    if (editor == NULL || document == NULL ||
        editor->path_node_segment >= document->path_segment_count ||
        (editor->path_node_number_x == SIZE_MAX &&
         editor->path_node_number_y == SIZE_MAX)) return VIEWER_EDITOR_HANDLED;
    const svg_shape_path_segment *segment =
        &document->path_segments[editor->path_node_segment];
    size_t first_index = editor->path_node_number_x != SIZE_MAX ?
        editor->path_node_number_x : editor->path_node_number_y;
    size_t second_index = editor->path_node_number_y;
    if (first_index >= document->path_number_count ||
        (second_index != SIZE_MAX && second_index >= document->path_number_count))
        return VIEWER_EDITOR_HANDLED;
    const svg_shape_path_number *first = &document->path_numbers[first_index];
    const svg_shape_path_number *second = second_index == SIZE_MAX ? NULL :
        &document->path_numbers[second_index];
    document_source_span full_span = {first->offset,
        (second == NULL ? first->offset + first->length :
            second->offset + second->length) - first->offset};
    if (!path_stored_range_current(editor, document, full_span)) {
        editor_status_ok(editor, "Path node source is stale; wait for preview");
        return VIEWER_EDITOR_HANDLED;
    }
    char x_text[64], y_text[64];
    double x = editor->path_node_current_x;
    double y = editor->path_node_current_y;
    if (editor->path_node_relative) {
        x -= editor->path_node_origin_x;
        y -= editor->path_node_origin_y;
    }
    size_t x_length = editor->path_node_number_x == SIZE_MAX ? 0U :
        path_format_number(x_text, x);
    size_t y_length = editor->path_node_number_y == SIZE_MAX ? 0U :
        path_format_number(y_text, y);
    if ((editor->path_node_number_x != SIZE_MAX && x_length == 0U) ||
        (editor->path_node_number_y != SIZE_MAX && y_length == 0U)) {
        editor_status_ok(editor, "Path node exceeds the coordinate limit");
        return VIEWER_EDITOR_HANDLED;
    }
    uint8_t replacement[256];
    size_t used = 0U;
    if (editor->path_node_number_x != SIZE_MAX) {
        memcpy(replacement + used, x_text, x_length); used += x_length;
    }
    if (second != NULL) {
        if (first->offset + first->length > second->offset ||
            second->offset + second->length - first->offset > full_span.length) {
            editor_status_ok(editor, "Path node number spans are invalid");
            return VIEWER_EDITOR_HANDLED;
        }
        size_t gap_length = second->offset - first->offset - first->length;
        base_bytes gap = {0};
        if (!document_edit_span_bytes(editor->edit,
                (document_source_span){first->offset + first->length, gap_length}, &gap) ||
            gap_length > sizeof(replacement) - used ||
            y_length > sizeof(replacement) - used - gap_length) {
            editor_status_ok(editor, "Path node replacement exceeds the editor limit");
            return VIEWER_EDITOR_HANDLED;
        }
        if (gap_length > 0U) memcpy(replacement + used, gap.ptr, gap_length);
        used += gap_length;
        if (editor->path_node_number_y != SIZE_MAX) {
            memcpy(replacement + used, y_text, y_length); used += y_length;
        }
    } else if (editor->path_node_number_x == SIZE_MAX) {
        memcpy(replacement + used, y_text, y_length); used += y_length;
    }
    size_t old_offset = editor->selected_offset;
    char old_id[EDITOR_NAME_CAP];
    editor_set_text(old_id, sizeof(old_id), editor->selected_has_id ?
        editor->selected_id : "");
    document_edit_error error = document_edit_replace_source_span(editor->edit,
        full_span, (base_bytes){replacement, used});
    editor->path_node_segment = SIZE_MAX;
    if (error != DOCUMENT_EDIT_OK) return editor_apply_result(editor, error,
        old_offset, old_id, (document_handle){0});
    editor_status_ok(editor, "Path node committed · preview updating");
    (void)segment;
    return editor_apply_result(editor, DOCUMENT_EDIT_OK, old_offset, old_id,
        (document_handle){0});
}

bool editor_canvas_path_node_begin(viewer_editor *editor,
    const svg_shape_document *document, geom_point user, double tolerance)
{
    svg_shape_selection selection;
    if (!path_selected_shape(editor, document, &selection) ||
        !isfinite(user.x) || !isfinite(user.y) || !isfinite(tolerance) ||
        !(tolerance > 0.0)) return false;
    path_node_ref nodes[EDITOR_PATH_NODE_RENDER_CAP];
    size_t count = 0U;
    if (!path_collect_nodes(document, selection.shape_index, nodes, &count)) return false;
    double best = tolerance;
    size_t chosen = SIZE_MAX;
    uint8_t handle_kind = 0U;
    geom_point local = {0.0, 0.0};
    for (size_t index = 0U; index < count; ++index) {
        geom_point candidate;
        if (!geom_affine_map_point(selection.transform,
                (geom_point){nodes[index].x, nodes[index].y}, &candidate)) continue;
        double distance = hypot(user.x - candidate.x, user.y - candidate.y);
        if (distance <= best) {
            best = distance; chosen = index; handle_kind = 0U;
            local = (geom_point){nodes[index].x, nodes[index].y};
        }
        if (nodes[index].has_out) {
            if (geom_affine_map_point(selection.transform, nodes[index].out_control,
                    &candidate)) {
                distance = hypot(user.x - candidate.x, user.y - candidate.y);
                if (distance <= best) {
                    best = distance; chosen = index; handle_kind = 1U;
                    local = nodes[index].out_control;
                }
            }
        }
        if (nodes[index].has_in) {
            if (geom_affine_map_point(selection.transform, nodes[index].in_control,
                    &candidate)) {
                distance = hypot(user.x - candidate.x, user.y - candidate.y);
                if (distance <= best) {
                    best = distance; chosen = index; handle_kind = 2U;
                    local = nodes[index].in_control;
                }
            }
        }
    }
    if (chosen == SIZE_MAX) return false;
    path_node_ref *node = &nodes[chosen];
    editor->path_selected_node_segment = node->segment_index;
    editor->path_node_anchor_segment = node->segment_index;
    editor->path_node_handle_kind = handle_kind;
    editor->path_node_relative = node->relative;
    if (handle_kind == 1U) {
        editor->path_node_segment = node->out_segment;
        editor->path_node_number_x = node->out_number_x;
        editor->path_node_number_y = node->out_number_y;
        editor->path_node_origin_x = node->out_origin_x;
        editor->path_node_origin_y = node->out_origin_y;
        editor->path_node_relative =
            document->path_segments[node->out_segment].command >= 'a' &&
            document->path_segments[node->out_segment].command <= 'z';
    } else if (handle_kind == 2U) {
        editor->path_node_segment = node->in_segment;
        editor->path_node_number_x = node->in_number_x;
        editor->path_node_number_y = node->in_number_y;
        editor->path_node_origin_x = node->in_origin_x;
        editor->path_node_origin_y = node->in_origin_y;
        editor->path_node_relative =
            document->path_segments[node->in_segment].command >= 'a' &&
            document->path_segments[node->in_segment].command <= 'z';
    } else {
        editor->path_node_segment = node->segment_index;
        editor->path_node_number_x = node->number_x;
        editor->path_node_number_y = node->number_y;
        editor->path_node_origin_x = node->origin_x;
        editor->path_node_origin_y = node->origin_y;
    }
    editor->path_node_has_x = editor->path_node_number_x != SIZE_MAX;
    editor->path_node_has_y = editor->path_node_number_y != SIZE_MAX;
    editor->path_node_current_x = local.x;
    editor->path_node_current_y = local.y;
    editor->gesture_start_x = local.x;
    editor->gesture_start_y = local.y;
    editor->gesture_current_x = local.x;
    editor->gesture_current_y = local.y;
    editor->canvas_gesture_moved = false;
    editor->canvas_gesture = EDITOR_CANVAS_GESTURE_PATH_NODE;
    editor_status_ok(editor, handle_kind == 0U ? "Path node selected" :
        "Path handle selected");
    return true;
}

uint32_t editor_canvas_path_node_pointer_move(viewer_editor *editor,
    const svg_shape_document *document, geom_point user)
{
    svg_shape_selection selection;
    geom_point local;
    if (editor == NULL || document == NULL ||
        editor->canvas_gesture != EDITOR_CANVAS_GESTURE_PATH_NODE ||
        !path_selected_shape(editor, document, &selection) ||
        !path_inverse_map(selection.transform, user, &local)) return VIEWER_EDITOR_HANDLED;
    if (!editor->path_node_has_x) local.x = editor->path_node_current_x;
    if (!editor->path_node_has_y) local.y = editor->path_node_current_y;
    editor->path_node_current_x = local.x;
    editor->path_node_current_y = local.y;
    editor->gesture_current_x = local.x;
    editor->gesture_current_y = local.y;
    editor->canvas_gesture_moved = fabs(local.x - editor->gesture_start_x) > 1.0e-9 ||
        fabs(local.y - editor->gesture_start_y) > 1.0e-9;
    editor->redraw_requested = true;
    return VIEWER_EDITOR_HANDLED;
}

uint32_t editor_canvas_path_node_pointer_up(viewer_editor *editor,
    const svg_shape_document *document)
{
    if (editor == NULL || document == NULL ||
        editor->canvas_gesture != EDITOR_CANVAS_GESTURE_PATH_NODE)
        return VIEWER_EDITOR_HANDLED;
    bool moved = editor->canvas_gesture_moved;
    editor->canvas_gesture = EDITOR_CANVAS_GESTURE_NONE;
    editor->canvas_gesture_moved = false;
    editor->redraw_requested = true;
    if (!moved) return VIEWER_EDITOR_HANDLED;
    return path_commit_numeric_edit(editor, document);
}

bool editor_canvas_path_nodes_overlay(viewer_editor *editor,
    const svg_shape_document *document,
    viewer_editor_canvas_overlay_state *out_overlay)
{
    svg_shape_selection selection;
    if (out_overlay == NULL || !path_selected_shape(editor, document, &selection))
        return false;
    path_node_ref nodes[EDITOR_PATH_NODE_RENDER_CAP];
    size_t count = 0U;
    if (!path_collect_nodes(document, selection.shape_index, nodes, &count)) return false;
    out_overlay->path_point_count = count;
    out_overlay->path_nodes_only = true;
    out_overlay->visible = true;
    for (size_t index = 0U; index < count; ++index) {
        geom_point point = {nodes[index].x, nodes[index].y};
        if (editor->canvas_gesture == EDITOR_CANVAS_GESTURE_PATH_NODE &&
            editor->path_node_handle_kind == 0U &&
            editor->path_node_anchor_segment == nodes[index].segment_index)
            point = (geom_point){editor->path_node_current_x,
                editor->path_node_current_y};
        if (!geom_affine_map_point(selection.transform, point,
                &out_overlay->path_points[index])) return false;
        if (nodes[index].has_out) {
            geom_point control = nodes[index].out_control;
            if (editor->canvas_gesture == EDITOR_CANVAS_GESTURE_PATH_NODE &&
                editor->path_node_handle_kind == 1U &&
                editor->path_node_anchor_segment == nodes[index].segment_index)
                control = (geom_point){editor->path_node_current_x,
                    editor->path_node_current_y};
            if (!geom_affine_map_point(selection.transform, control,
                    &out_overlay->path_handles[index])) return false;
            out_overlay->path_has_handles[index] = true;
        }
        if (nodes[index].has_in) {
            geom_point control = nodes[index].in_control;
            if (editor->canvas_gesture == EDITOR_CANVAS_GESTURE_PATH_NODE &&
                editor->path_node_handle_kind == 2U &&
                editor->path_node_anchor_segment == nodes[index].segment_index)
                control = (geom_point){editor->path_node_current_x,
                    editor->path_node_current_y};
            if (!geom_affine_map_point(selection.transform, control,
                    &out_overlay->path_in_handles[index])) return false;
            out_overlay->path_has_in_handles[index] = true;
        }
    }
    return true;
}

uint32_t editor_canvas_path_node_delete(viewer_editor *editor,
    const svg_shape_document *document)
{
    return editor_canvas_path_curve_delete_node(editor, document);
}

uint32_t viewer_editor_delete_selected_path_node(viewer_editor *editor,
    const svg_shape_document *document)
{
    return editor_canvas_path_node_delete(editor, document);
}

uint32_t editor_canvas_path_add_node(viewer_editor *editor,
    const svg_shape_document *document, geom_point user, double tolerance)
{
    return editor_canvas_path_curve_add_node(editor, document, user, tolerance);
}

