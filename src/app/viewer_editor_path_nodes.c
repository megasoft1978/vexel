#include "viewer_editor_internal.h"

#include <math.h>
#include <stdio.h>
#include <string.h>

#define EDITOR_PATH_NODE_RENDER_CAP VIEWER_EDITOR_PATH_POINT_CAP
typedef editor_path_node_ref path_node_ref;

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
    geom_point previous_cubic = {0.0, 0.0};
    geom_point previous_quadratic = {0.0, 0.0};
    bool previous_was_cubic = false, previous_was_quadratic = false;
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
            previous_was_cubic = false;
            previous_was_quadratic = false;
            continue;
        }
        size_t x_offset = 0U, y_offset = 1U;
        size_t first_control_offset = SIZE_MAX, second_control_offset = SIZE_MAX;
        bool has_x = true, has_y = true, has_control = false;
        geom_point first_control = {0.0, 0.0};
        geom_point second_control = {0.0, 0.0};
        if (upper == 'H') { y_offset = SIZE_MAX; has_y = false; }
        else if (upper == 'V') {
            x_offset = SIZE_MAX; y_offset = 0U; has_x = false;
        } else if (upper == 'C') {
            if (segment->number_count != 6U) return false;
            first_control_offset = 0U; second_control_offset = 2U;
            if (!path_numbers_at(document, segment, 0U, 2U,
                    &first_control.x, &first_control.y) ||
                !path_numbers_at(document, segment, 2U, 2U,
                    &second_control.x, &second_control.y)) return false;
            has_control = true; x_offset = 4U; y_offset = 5U;
        } else if (upper == 'S') {
            if (segment->number_count != 4U) return false;
            second_control_offset = 0U;
            if (!path_numbers_at(document, segment, 0U, 2U,
                    &second_control.x, &second_control.y)) return false;
            has_control = true; x_offset = 2U; y_offset = 3U;
        } else if (upper == 'Q') {
            if (segment->number_count != 4U) return false;
            first_control_offset = 0U; second_control_offset = 0U;
            if (!path_numbers_at(document, segment, 0U, 2U,
                    &first_control.x, &first_control.y)) return false;
            second_control = first_control;
            has_control = true; x_offset = 2U; y_offset = 3U;
        } else if (upper == 'T') {
            if (segment->number_count != 2U) return false;
            has_control = true;
        } else if (upper == 'A') {
            if (segment->number_count != 7U) return false;
            x_offset = 5U; y_offset = 6U;
        } else if (upper == 'M' || upper == 'L') {
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
        if (upper == 'S') first_control = previous_was_cubic ?
            (geom_point){2.0 * origin_x - previous_cubic.x,
                2.0 * origin_y - previous_cubic.y} :
            (geom_point){origin_x, origin_y};
        else if (upper == 'T') first_control = previous_was_quadratic ?
            (geom_point){2.0 * origin_x - previous_quadratic.x,
                2.0 * origin_y - previous_quadratic.y} :
            (geom_point){origin_x, origin_y};
        if (upper == 'T') second_control = first_control;
        if (relative) {
            if (first_control_offset != SIZE_MAX) {
                first_control.x += origin_x; first_control.y += origin_y;
            }
            if (second_control_offset != SIZE_MAX) {
                second_control.x += origin_x; second_control.y += origin_y;
            }
        }
        if (has_control && (!isfinite(first_control.x) ||
                !isfinite(first_control.y) || !isfinite(second_control.x) ||
                !isfinite(second_control.y) || fabs(first_control.x) > 1000000.0 ||
                fabs(first_control.y) > 1000000.0 ||
                fabs(second_control.x) > 1000000.0 ||
                fabs(second_control.y) > 1000000.0)) return false;
        if (count == EDITOR_PATH_NODE_RENDER_CAP) break;
        path_node_ref *node = &out[count];
        *node = (path_node_ref){.segment_index = index,
            .number_x = has_x ? first_number + x_offset : SIZE_MAX,
            .number_y = has_y ? first_number + y_offset : SIZE_MAX,
            .out_segment = SIZE_MAX, .out_number_x = SIZE_MAX,
            .out_number_y = SIZE_MAX, .in_segment = SIZE_MAX,
            .in_number_x = SIZE_MAX, .in_number_y = SIZE_MAX,
            .x = x, .y = y, .origin_x = origin_x, .origin_y = origin_y,
            .has_x = has_x, .has_y = has_y, .relative = relative,
            .command = upper};
        if (upper == 'M') {
            first_x = x; first_y = y; previous_node = SIZE_MAX;
        }
        if (has_control) {
            if (previous_node != SIZE_MAX) {
                path_node_ref *previous = &out[previous_node];
                previous->has_out = true;
                previous->out_control = first_control;
                previous->out_origin_x = origin_x;
                previous->out_origin_y = origin_y;
                previous->out_segment = index;
                previous->out_virtual = first_control_offset == SIZE_MAX;
                previous->out_number_x = first_control_offset == SIZE_MAX ?
                    SIZE_MAX : first_number + first_control_offset;
                previous->out_number_y = first_control_offset == SIZE_MAX ?
                    SIZE_MAX : first_number + first_control_offset + 1U;
            }
            node->has_in = true;
            node->in_control = second_control;
            node->in_origin_x = origin_x; node->in_origin_y = origin_y;
            node->in_segment = index;
            node->in_virtual = second_control_offset == SIZE_MAX;
            node->in_number_x = second_control_offset == SIZE_MAX ?
                SIZE_MAX : first_number + second_control_offset;
            node->in_number_y = second_control_offset == SIZE_MAX ?
                SIZE_MAX : first_number + second_control_offset + 1U;
        }
        ++count;
        current_x = x; current_y = y; previous_node = count - 1U;
        previous_was_cubic = upper == 'C' || upper == 'S';
        previous_was_quadratic = upper == 'Q' || upper == 'T';
        if (previous_was_cubic) previous_cubic = second_control;
        if (previous_was_quadratic) previous_quadratic = second_control;
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

static bool path_command_supported(uint8_t byte)
{
    return byte == 'M' || byte == 'm' || byte == 'L' || byte == 'l' ||
        byte == 'H' || byte == 'h' || byte == 'V' || byte == 'v' ||
        byte == 'C' || byte == 'c' || byte == 'S' || byte == 's' ||
        byte == 'Q' || byte == 'q' || byte == 'T' || byte == 't' ||
        byte == 'A' || byte == 'a' || byte == 'Z' || byte == 'z';
}

static bool path_selected_d_supported(viewer_editor *editor)
{
    base_bytes tag = {0};
    if (editor == NULL || editor->edit == NULL ||
        !document_edit_tag_name(editor->edit, editor->selected, &tag) ||
        tag.len != 4U || memcmp(tag.ptr, "path", 4U) != 0) return false;
    size_t count = document_edit_attribute_count(editor->edit, editor->selected);
    for (size_t index = 0U; index < count; ++index) {
        document_edit_attribute attribute;
        base_bytes name = {0}, value = {0};
        if (!document_edit_attribute_at(editor->edit, editor->selected, index,
                &attribute) ||
            !document_edit_span_bytes(editor->edit, attribute.name, &name) ||
            name.len != 1U || name.ptr[0] != 'd' ||
            !document_edit_span_bytes(editor->edit, attribute.value, &value))
            continue;
        bool has_command = false;
        for (size_t byte_index = 0U; byte_index < value.len; ++byte_index) {
            uint8_t byte = value.ptr[byte_index];
            bool letter = (byte >= 'A' && byte <= 'Z') ||
                (byte >= 'a' && byte <= 'z');
            if (!letter) continue;
            if (path_command_supported(byte)) { has_command = true; continue; }
            bool exponent = (byte == 'e' || byte == 'E') && byte_index > 0U &&
                byte_index + 1U < value.len &&
                value.ptr[byte_index - 1U] >= '0' &&
                value.ptr[byte_index - 1U] <= '9';
            if (exponent) {
                size_t next = byte_index + 1U;
                if (value.ptr[next] == '+' || value.ptr[next] == '-') ++next;
                exponent = next < value.len && value.ptr[next] >= '0' &&
                    value.ptr[next] <= '9';
            }
            if (!exponent) return false;
        }
        return has_command;
    }
    return false;
}

bool editor_canvas_path_node_mode_enter(viewer_editor *editor)
{
    if (editor == NULL) return false;
    if (!path_selected_d_supported(editor)) {
        editor_status_ok(editor,
            "Select a path with supported commands to edit nodes");
        return false;
    }
    (void)viewer_editor_set_draw_tool(editor, VIEWER_EDITOR_TOOL_PEN);
    editor->node_edit_active = true;
    editor->focus_id = EDITOR_NODE_CANVAS;
    editor->path_selected_node_segment = SIZE_MAX;
    editor->path_node_segment = SIZE_MAX;
    editor->path_node_anchor_segment = SIZE_MAX;
    editor->path_hovered_node_segment = SIZE_MAX;
    editor->path_hovered_segment_valid = false;
    editor->path_node_is_initial = false;
    editor->path_node_corner_available = false;
    editor->path_node_smooth_available = false;
    editor->tree_ready = false;
    editor->redraw_requested = true;
    editor_status_ok(editor,
        "Node editing · Tab anchors · arrows nudge · Return applies options");
    return true;
}

bool editor_canvas_path_node_collect(viewer_editor *editor,
    const svg_shape_document *document,
    editor_path_node_ref out_nodes[VIEWER_EDITOR_PATH_POINT_CAP],
    size_t *out_count)
{
    svg_shape_selection selection;
    if (out_nodes == NULL || out_count == NULL ||
        !path_selected_shape(editor, document, &selection)) return false;
    return path_collect_nodes(document, selection.shape_index, out_nodes,
        out_count);
}

static void path_select_node_ref(viewer_editor *editor,
    const svg_shape_document *document, const path_node_ref *node,
    uint8_t handle_kind, geom_point point)
{
    if (editor == NULL || document == NULL || node == NULL) return;
    editor->path_selected_node_segment = node->segment_index;
    editor->path_node_anchor_segment = node->segment_index;
    editor->path_node_handle_kind = handle_kind;
    editor->path_node_segment = node->segment_index;
    editor->path_node_number_x = node->number_x;
    editor->path_node_number_y = node->number_y;
    editor->path_node_origin_x = node->origin_x;
    editor->path_node_origin_y = node->origin_y;
    editor->path_node_relative = node->relative;
    editor->path_node_virtual_handle = false;
    if (handle_kind == 1U) {
        editor->path_node_segment = node->out_segment;
        editor->path_node_number_x = node->out_number_x;
        editor->path_node_number_y = node->out_number_y;
        editor->path_node_origin_x = node->out_origin_x;
        editor->path_node_origin_y = node->out_origin_y;
        editor->path_node_relative = document->path_segments[node->out_segment].command >= 'a' &&
            document->path_segments[node->out_segment].command <= 'z';
        editor->path_node_virtual_handle = node->out_virtual;
    } else if (handle_kind == 2U) {
        editor->path_node_segment = node->in_segment;
        editor->path_node_number_x = node->in_number_x;
        editor->path_node_number_y = node->in_number_y;
        editor->path_node_origin_x = node->in_origin_x;
        editor->path_node_origin_y = node->in_origin_y;
        editor->path_node_relative = document->path_segments[node->in_segment].command >= 'a' &&
            document->path_segments[node->in_segment].command <= 'z';
        editor->path_node_virtual_handle = node->in_virtual;
    }
    editor->path_node_has_x = handle_kind == 0U ? node->has_x : true;
    editor->path_node_has_y = handle_kind == 0U ? node->has_y : true;
    editor->path_node_is_initial = node->command == 'M';
    editor->path_node_current_x = point.x;
    editor->path_node_current_y = point.y;
    editor->path_node_corner_available = node->out_virtual || node->in_virtual;
    editor->path_node_smooth_available = node->has_in && node->has_out;
    if (node->has_in && node->has_out && node->in_segment <
            document->path_segment_count && node->out_segment <
            document->path_segment_count) {
        char in_command = document->path_segments[node->in_segment].command;
        char out_command = document->path_segments[node->out_segment].command;
        bool in_cubic = in_command == 'C' || in_command == 'c' ||
            in_command == 'S' || in_command == 's';
        bool out_cubic = out_command == 'C' || out_command == 'c' ||
            out_command == 'S' || out_command == 's';
        bool in_quadratic = in_command == 'Q' || in_command == 'q' ||
            in_command == 'T' || in_command == 't';
        bool out_quadratic = out_command == 'Q' || out_command == 'q' ||
            out_command == 'T' || out_command == 't';
        editor->path_node_smooth_available =
            (in_cubic && out_cubic) || (in_quadratic && out_quadratic);
    } else editor->path_node_smooth_available = false;
    editor->path_hovered_node_segment = SIZE_MAX;
    editor->path_hovered_segment_valid = false;
    editor->tree_ready = false;
    editor->redraw_requested = true;
}

bool editor_canvas_path_node_select_index(viewer_editor *editor,
    const svg_shape_document *document, size_t node_index)
{
    path_node_ref nodes[EDITOR_PATH_NODE_RENDER_CAP];
    size_t count = 0U;
    if (!editor_canvas_path_node_collect(editor, document, nodes, &count) ||
        node_index >= count) return false;
    path_select_node_ref(editor, document, &nodes[node_index], 0U,
        (geom_point){nodes[node_index].x, nodes[node_index].y});
    editor_status_ok(editor, "Path anchor selected");
    return true;
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

static uint32_t path_apply_edit_keep_anchor(viewer_editor *editor,
    const svg_shape_document *document, document_edit_error error,
    size_t old_offset, const char *old_id)
{
    size_t anchor = editor->path_node_anchor_segment;
    uint32_t result = editor_apply_result(editor, error, old_offset, old_id,
        (document_handle){0});
    if (error == DOCUMENT_EDIT_OK && editor->node_edit_active &&
        anchor < document->path_segment_count) {
        editor->path_selected_node_segment = anchor;
        editor->path_node_anchor_segment = anchor;
    }
    return result;
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
    return path_apply_edit_keep_anchor(editor, document, DOCUMENT_EDIT_OK,
        old_offset, old_id);
}

static uint32_t path_commit_virtual_handle(viewer_editor *editor,
    const svg_shape_document *document)
{
    if (editor == NULL || document == NULL ||
        editor->path_node_segment >= document->path_segment_count)
        return VIEWER_EDITOR_HANDLED;
    const svg_shape_path_segment *segment =
        &document->path_segments[editor->path_node_segment];
    char command = segment->command;
    char upper = command >= 'a' && command <= 'z' ?
        (char)(command - 'a' + 'A') : command;
    if (upper != 'S' && upper != 'T') return VIEWER_EDITOR_HANDLED;
    size_t endpoint_offset = upper == 'S' ? 2U : 0U;
    double end_x, end_y;
    if (!path_numbers_at(document, segment, endpoint_offset, 2U,
            &end_x, &end_y)) return VIEWER_EDITOR_HANDLED;
    bool relative = command >= 'a' && command <= 'z';
    if (relative) {
        end_x += editor->path_node_origin_x;
        end_y += editor->path_node_origin_y;
    }
    char replacement[256];
    int amount;
    if (upper == 'S') {
        double second_x, second_y;
        if (!path_numbers_at(document, segment, 0U, 2U,
                &second_x, &second_y)) return VIEWER_EDITOR_HANDLED;
        if (relative) {
            second_x += editor->path_node_origin_x;
            second_y += editor->path_node_origin_y;
        }
        amount = snprintf(replacement, sizeof(replacement),
            "C %.9g %.9g %.9g %.9g %.9g %.9g",
            editor->path_node_current_x, editor->path_node_current_y,
            second_x, second_y, end_x, end_y);
    } else {
        amount = snprintf(replacement, sizeof(replacement), "Q %.9g %.9g %.9g %.9g",
            editor->path_node_current_x, editor->path_node_current_y, end_x, end_y);
    }
    if (amount <= 0 || (size_t)amount >= sizeof(replacement))
        return VIEWER_EDITOR_HANDLED;
    document_source_span span = {segment->source_offset, segment->source_length};
    if (span.length == 0U || !path_stored_range_current(editor, document, span)) {
        editor_status_ok(editor, "Path handle source is stale; wait for preview");
        return VIEWER_EDITOR_HANDLED;
    }
    size_t old_offset = editor->selected_offset;
    char old_id[EDITOR_NAME_CAP];
    editor_set_text(old_id, sizeof(old_id), editor->selected_has_id ?
        editor->selected_id : "");
    document_edit_error error = document_edit_replace_source_span(editor->edit,
        span, (base_bytes){(const uint8_t *)replacement, (size_t)amount});
    editor->path_node_segment = SIZE_MAX;
    editor->path_node_virtual_handle = false;
    if (error != DOCUMENT_EDIT_OK) return editor_apply_result(editor, error,
        old_offset, old_id, (document_handle){0});
    editor_status_ok(editor, "Path handle committed · preview updating");
    return path_apply_edit_keep_anchor(editor, document, DOCUMENT_EDIT_OK,
        old_offset, old_id);
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
        if (distance < best) {
            best = distance; chosen = index; handle_kind = 0U;
            local = (geom_point){nodes[index].x, nodes[index].y};
        }
        if (nodes[index].has_out) {
            if (geom_affine_map_point(selection.transform, nodes[index].out_control,
                    &candidate)) {
                distance = hypot(user.x - candidate.x, user.y - candidate.y);
                if (distance < best) {
                    best = distance; chosen = index; handle_kind = 1U;
                    local = nodes[index].out_control;
                }
            }
        }
        if (nodes[index].has_in) {
            if (geom_affine_map_point(selection.transform, nodes[index].in_control,
                    &candidate)) {
                distance = hypot(user.x - candidate.x, user.y - candidate.y);
                if (distance < best) {
                    best = distance; chosen = index; handle_kind = 2U;
                    local = nodes[index].in_control;
                }
            }
        }
    }
    if (chosen == SIZE_MAX) return false;
    path_node_ref *node = &nodes[chosen];
    path_select_node_ref(editor, document, node, handle_kind, local);
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
    return editor->path_node_virtual_handle ?
        path_commit_virtual_handle(editor, document) :
        path_commit_numeric_edit(editor, document);
}

uint32_t editor_canvas_path_node_nudge(viewer_editor *editor,
    const svg_shape_document *document, double dx, double dy)
{
    if (editor == NULL || document == NULL || !isfinite(dx) || !isfinite(dy) ||
        (dx == 0.0 && dy == 0.0) ||
        editor->path_selected_node_segment == SIZE_MAX)
        return VIEWER_EDITOR_HANDLED;
    path_node_ref nodes[EDITOR_PATH_NODE_RENDER_CAP];
    size_t count = 0U, selected = SIZE_MAX;
    if (!editor_canvas_path_node_collect(editor, document, nodes, &count))
        return VIEWER_EDITOR_HANDLED;
    for (size_t index = 0U; index < count; ++index)
        if (nodes[index].segment_index == editor->path_selected_node_segment) {
            selected = index;
            break;
        }
    if (selected == SIZE_MAX) return VIEWER_EDITOR_HANDLED;
    path_node_ref *node = &nodes[selected];
    path_select_node_ref(editor, document, node, 0U,
        (geom_point){node->x, node->y});
    if ((!node->has_x && dx != 0.0) || (!node->has_y && dy != 0.0)) {
        if (!node->has_x) dx = 0.0;
        if (!node->has_y) dy = 0.0;
    }
    if (dx == 0.0 && dy == 0.0) return VIEWER_EDITOR_HANDLED;
    double next_x = node->x + dx, next_y = node->y + dy;
    if (!isfinite(next_x) || !isfinite(next_y) || fabs(next_x) > 1000000.0 ||
        fabs(next_y) > 1000000.0) {
        editor_status_ok(editor, "Path node exceeds the coordinate limit");
        return VIEWER_EDITOR_HANDLED;
    }
    editor->path_node_current_x = next_x;
    editor->path_node_current_y = next_y;
    return path_commit_numeric_edit(editor, document);
}

bool editor_canvas_path_nodes_overlay(viewer_editor *editor,
    const svg_shape_document *document,
    viewer_editor_canvas_overlay_state *out_overlay)
{
    svg_shape_selection selection;
    if (out_overlay == NULL || editor == NULL || !editor->node_edit_active ||
        !path_selected_shape(editor, document, &selection))
        return false;
    path_node_ref nodes[EDITOR_PATH_NODE_RENDER_CAP];
    size_t count = 0U;
    if (!path_collect_nodes(document, selection.shape_index, nodes, &count)) return false;
    out_overlay->path_point_count = count;
    out_overlay->path_nodes_only = true;
    out_overlay->visible = true;
    for (size_t index = 0U; index < count; ++index) {
        if (nodes[index].segment_index == editor->path_selected_node_segment)
            out_overlay->path_point_states[index] = 2U;
        else if (nodes[index].segment_index == editor->path_hovered_node_segment)
            out_overlay->path_point_states[index] = 1U;
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

