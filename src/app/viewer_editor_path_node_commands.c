#include "viewer_editor_internal.h"

#include <math.h>
#include <stdio.h>
#include <string.h>

static bool path_selected_transform(viewer_editor *editor,
    const svg_shape_document *document, svg_shape_selection *out_selection)
{
    base_bytes tag = {0};
    return editor != NULL && document != NULL && out_selection != NULL &&
        editor->node_edit_active && editor->draw_tool == VIEWER_EDITOR_TOOL_PEN &&
        document_edit_tag_name(editor->edit, editor->selected, &tag) &&
        tag.len == 4U && memcmp(tag.ptr, "path", 4U) == 0 &&
        svg_shape_element_at_offset(document, editor->selected_offset,
            out_selection);
}

static double point_segment_distance(geom_point point, geom_edge edge)
{
    double dx = edge.b.x - edge.a.x, dy = edge.b.y - edge.a.y;
    double length_squared = dx * dx + dy * dy;
    double t = length_squared > 0.0 ?
        ((point.x - edge.a.x) * dx + (point.y - edge.a.y) * dy) /
            length_squared : 0.0;
    if (t < 0.0) t = 0.0;
    if (t > 1.0) t = 1.0;
    return hypot(point.x - (edge.a.x + t * dx),
        point.y - (edge.a.y + t * dy));
}

uint32_t editor_canvas_path_node_hover(viewer_editor *editor,
    const svg_shape_document *document, geom_point user, double tolerance)
{
    svg_shape_selection selection;
    if (!path_selected_transform(editor, document, &selection) ||
        !isfinite(user.x) || !isfinite(user.y) || !isfinite(tolerance) ||
        !(tolerance > 0.0)) return VIEWER_EDITOR_HANDLED;
    editor_path_node_ref nodes[VIEWER_EDITOR_PATH_POINT_CAP];
    size_t count = 0U;
    if (!editor_canvas_path_node_collect(editor, document, nodes, &count))
        return VIEWER_EDITOR_HANDLED;
    double best = tolerance;
    size_t hovered_node = SIZE_MAX;
    uint8_t hovered_handle = 0U;
    for (size_t index = 0U; index < count; ++index) {
        geom_point candidate;
        if (!geom_affine_map_point(selection.transform,
                (geom_point){nodes[index].x, nodes[index].y}, &candidate)) continue;
        double distance = hypot(user.x - candidate.x, user.y - candidate.y);
        if (distance < best) {
            best = distance;
            hovered_node = nodes[index].segment_index;
            hovered_handle = 0U;
        }
        if (nodes[index].has_out && geom_affine_map_point(selection.transform,
                nodes[index].out_control, &candidate)) {
            distance = hypot(user.x - candidate.x, user.y - candidate.y);
            if (distance < best) {
                best = distance;
                hovered_node = nodes[index].segment_index;
                hovered_handle = 1U;
            }
        }
        if (nodes[index].has_in && geom_affine_map_point(selection.transform,
                nodes[index].in_control, &candidate)) {
            distance = hypot(user.x - candidate.x, user.y - candidate.y);
            if (distance < best) {
                best = distance;
                hovered_node = nodes[index].segment_index;
                hovered_handle = 2U;
            }
        }
    }
    bool segment_valid = false;
    size_t hovered_segment = SIZE_MAX;
    if (hovered_node == SIZE_MAX) {
        double segment_best = tolerance;
        for (size_t index = 0U; index < document->path_segment_count; ++index) {
            const svg_shape_path_segment *segment = &document->path_segments[index];
            if (segment->shape_index != selection.shape_index ||
                segment->edge_count == 0U || segment->first_edge > document->edge_count ||
                segment->edge_count > document->edge_count - segment->first_edge)
                continue;
            for (size_t edge_index = 0U; edge_index < segment->edge_count; ++edge_index) {
                double distance = point_segment_distance(user,
                    document->edges[segment->first_edge + edge_index]);
                if (distance < segment_best) {
                    segment_best = distance;
                    hovered_segment = index;
                    segment_valid = true;
                }
            }
        }
    }
    bool changed = editor->path_hovered_node_segment != hovered_node ||
        editor->path_hovered_handle_kind != hovered_handle ||
        editor->path_hovered_segment_valid != segment_valid ||
        (segment_valid && editor->path_hovered_segment != hovered_segment);
    if (changed) {
        editor->path_hovered_node_segment = hovered_node;
        editor->path_hovered_handle_kind = hovered_handle;
        editor->path_hovered_segment = hovered_segment;
        editor->path_hovered_segment_valid = segment_valid;
        editor->tree_ready = false;
        editor->redraw_requested = true;
    }
    return VIEWER_EDITOR_HANDLED;
}

uint32_t viewer_editor_node_edit_hover(viewer_editor *editor,
    const svg_shape_document *document, geom_point user, double tolerance)
{
    return editor_canvas_path_node_hover(editor, document, user, tolerance);
}

uint32_t editor_canvas_path_node_key(viewer_editor *editor,
    const svg_shape_document *document, uint16_t key_code, uint32_t modifiers)
{
    if (editor == NULL || document == NULL || !editor->node_edit_active) return 0U;
    if (key_code == 48U) {
        editor_path_node_ref nodes[VIEWER_EDITOR_PATH_POINT_CAP];
        size_t count = 0U, current = SIZE_MAX;
        if (!editor_canvas_path_node_collect(editor, document, nodes, &count) ||
            count == 0U) return VIEWER_EDITOR_HANDLED;
        for (size_t index = 0U; index < count; ++index)
            if (nodes[index].segment_index == editor->path_selected_node_segment) {
                current = index;
                break;
            }
        bool reverse = (modifiers & PLATFORM_MODIFIER_SHIFT) != 0U;
        if (current == SIZE_MAX)
            return editor_canvas_path_node_select_index(editor, document,
                reverse ? count - 1U : 0U) ? VIEWER_EDITOR_HANDLED :
                    VIEWER_EDITOR_HANDLED;
        if ((reverse && current == 0U) || (!reverse && current + 1U >= count)) {
            editor->node_edit_active = false;
            editor->path_hovered_node_segment = SIZE_MAX;
            editor->path_hovered_segment_valid = false;
            editor->tree_ready = false;
            editor->redraw_requested = true;
            editor->focus_id = EDITOR_NODE_CANVAS;
            editor_build_tree(editor);
            if (ui_focus_next(&editor->tree, reverse)) {
                editor->focus_id = editor->tree.focused_id;
                editor->tree_ready = false;
            }
            editor_status_ok(editor, "Tab left node editing");
            return VIEWER_EDITOR_HANDLED;
        }
        size_t next = reverse ? current - 1U : current + 1U;
        return editor_canvas_path_node_select_index(editor, document, next) ?
            VIEWER_EDITOR_HANDLED : VIEWER_EDITOR_HANDLED;
    }
    if (key_code >= 123U && key_code <= 126U) {
        double step = (modifiers & PLATFORM_MODIFIER_SHIFT) != 0U ? 10.0 : 1.0;
        double dx = key_code == 123U ? -step : key_code == 124U ? step : 0.0;
        double dy = key_code == 126U ? -step : key_code == 125U ? step : 0.0;
        return editor_canvas_path_node_nudge(editor, document, dx, dy);
    }
    return 0U;
}

bool viewer_editor_node_edit_active(const viewer_editor *editor)
{
    return editor != NULL && editor->node_edit_active;
}

uint32_t viewer_editor_node_edit_key(viewer_editor *editor,
    const svg_shape_document *document, uint16_t key_code, uint32_t modifiers)
{
    return editor_canvas_path_node_key(editor, document, key_code, modifiers);
}

static bool path_span_current(viewer_editor *editor,
    const svg_shape_document *document, document_source_span span)
{
    base_bytes current = {0};
    return span.offset <= document->source.len &&
        span.length <= document->source.len - span.offset &&
        document_edit_span_bytes(editor->edit, span, &current) &&
        (span.length == 0U || memcmp(current.ptr,
            document->source.ptr + span.offset, span.length) == 0);
}

static uint32_t path_commit_replacement(viewer_editor *editor,
    const svg_shape_document *document, document_source_span span,
    const char *replacement, size_t replacement_length, const char *status)
{
    if (editor == NULL || document == NULL || replacement == NULL ||
        replacement_length == 0U || !path_span_current(editor, document, span)) {
        if (editor != NULL) editor_status_ok(editor,
            "Path source changed; wait for preview");
        return VIEWER_EDITOR_HANDLED;
    }
    size_t old_offset = editor->selected_offset;
    char old_id[EDITOR_NAME_CAP];
    editor_set_text(old_id, sizeof(old_id), editor->selected_has_id ?
        editor->selected_id : "");
    document_edit_error error = document_edit_replace_source_span(editor->edit,
        span, (base_bytes){(const uint8_t *)replacement, replacement_length});
    if (error != DOCUMENT_EDIT_OK)
        return editor_apply_result(editor, error, old_offset, old_id,
            (document_handle){0});
    editor_status_ok(editor, status);
    size_t anchor = editor->path_node_anchor_segment;
    uint32_t result = editor_apply_result(editor, DOCUMENT_EDIT_OK, old_offset,
        old_id, (document_handle){0});
    if (editor->node_edit_active && anchor < document->path_segment_count) {
        editor->path_selected_node_segment = anchor;
        editor->path_node_anchor_segment = anchor;
    }
    return result;
}

static bool path_format_absolute_segment(const editor_path_node_ref *from,
    const editor_path_node_ref *to, const svg_shape_document *document,
    char output[256], size_t *out_length)
{
    if (from == NULL || to == NULL || document == NULL || output == NULL ||
        out_length == NULL || to->in_segment >= document->path_segment_count)
        return false;
    const svg_shape_path_segment *segment =
        &document->path_segments[to->in_segment];
    char command = segment->command;
    char upper = command >= 'a' && command <= 'z' ?
        (char)(command - 'a' + 'A') : command;
    int amount = -1;
    if (upper == 'S') {
        amount = snprintf(output, 256U, "C %.9g %.9g %.9g %.9g %.9g %.9g",
            from->out_control.x, from->out_control.y, to->in_control.x,
            to->in_control.y, to->x, to->y);
    } else if (upper == 'T') {
        amount = snprintf(output, 256U, "Q %.9g %.9g %.9g %.9g",
            from->out_control.x, from->out_control.y, to->x, to->y);
    }
    if (amount <= 0 || amount >= 256) return false;
    *out_length = (size_t)amount;
    return true;
}

static uint32_t path_corner(viewer_editor *editor,
    const svg_shape_document *document, editor_path_node_ref *nodes,
    size_t count, size_t selected)
{
    editor_path_node_ref *node = &nodes[selected];
    if (node->out_virtual) {
        size_t next = SIZE_MAX;
        for (size_t index = selected + 1U; index < count; ++index)
            if (nodes[index].segment_index == node->out_segment) {
                next = index;
                break;
            }
        char replacement[256];
        size_t replacement_length = 0U;
        if (next == SIZE_MAX || !path_format_absolute_segment(node,
                &nodes[next], document, replacement, &replacement_length))
            return VIEWER_EDITOR_HANDLED;
        const svg_shape_path_segment *segment =
            &document->path_segments[node->out_segment];
        return path_commit_replacement(editor, document,
            (document_source_span){segment->source_offset, segment->source_length},
            replacement, replacement_length,
            "Corner anchor converted · preview updating");
    }
    if (node->in_virtual) {
        const svg_shape_path_segment *segment =
            &document->path_segments[node->in_segment];
        char replacement[128];
        int amount = snprintf(replacement, sizeof(replacement),
            "Q %.9g %.9g %.9g %.9g", node->in_control.x,
            node->in_control.y, node->x, node->y);
        if (amount <= 0 || (size_t)amount >= sizeof(replacement))
            return VIEWER_EDITOR_HANDLED;
        return path_commit_replacement(editor, document,
            (document_source_span){segment->source_offset, segment->source_length},
            replacement, (size_t)amount,
            "Corner anchor converted · preview updating");
    }
    return VIEWER_EDITOR_HANDLED;
}

static uint32_t path_smooth(viewer_editor *editor,
    const svg_shape_document *document, editor_path_node_ref *nodes,
    size_t count, size_t selected)
{
    editor_path_node_ref *node = &nodes[selected];
    if (!node->has_in || !node->has_out ||
        node->out_segment >= document->path_segment_count) return VIEWER_EDITOR_HANDLED;
    double tangent_x = node->x - node->in_control.x;
    double tangent_y = node->y - node->in_control.y;
    double tangent_length = hypot(tangent_x, tangent_y);
    double outgoing_length = hypot(node->out_control.x - node->x,
        node->out_control.y - node->y);
    if (!(tangent_length > 1.0e-12) || !isfinite(tangent_length) ||
        !isfinite(outgoing_length)) {
        editor_status_ok(editor, "This anchor has no usable tangent to smooth");
        return VIEWER_EDITOR_HANDLED;
    }
    if (!(outgoing_length > 1.0e-12)) outgoing_length = tangent_length;
    geom_point smooth = {node->x + tangent_x / tangent_length * outgoing_length,
        node->y + tangent_y / tangent_length * outgoing_length};
    const svg_shape_path_segment *segment =
        &document->path_segments[node->out_segment];
    char command = segment->command;
    char upper = command >= 'a' && command <= 'z' ?
        (char)(command - 'a' + 'A') : command;
    if (node->out_virtual) {
        size_t next = SIZE_MAX;
        for (size_t index = selected + 1U; index < count; ++index)
            if (nodes[index].segment_index == node->out_segment) {
                next = index;
                break;
            }
        if (next == SIZE_MAX) return VIEWER_EDITOR_HANDLED;
        editor_path_node_ref *end = &nodes[next];
        char replacement[256];
        int amount = upper == 'S' ?
            snprintf(replacement, sizeof(replacement),
                "C %.9g %.9g %.9g %.9g %.9g %.9g", smooth.x, smooth.y,
                end->in_control.x, end->in_control.y, end->x, end->y) :
            snprintf(replacement, sizeof(replacement),
                "Q %.9g %.9g %.9g %.9g", smooth.x, smooth.y, end->x, end->y);
        if (amount <= 0 || (size_t)amount >= sizeof(replacement))
            return VIEWER_EDITOR_HANDLED;
        return path_commit_replacement(editor, document,
            (document_source_span){segment->source_offset, segment->source_length},
            replacement, (size_t)amount,
            "Smooth anchor converted · preview updating");
    }
    if (node->out_number_x >= document->path_number_count ||
        node->out_number_y >= document->path_number_count)
        return VIEWER_EDITOR_HANDLED;
    const svg_shape_path_number *x_number =
        &document->path_numbers[node->out_number_x];
    const svg_shape_path_number *y_number =
        &document->path_numbers[node->out_number_y];
    if (x_number->offset + x_number->length > y_number->offset) return VIEWER_EDITOR_HANDLED;
    double x = smooth.x, y = smooth.y;
    if (command >= 'a' && command <= 'z') {
        x -= node->out_origin_x;
        y -= node->out_origin_y;
    }
    char x_text[64], y_text[64];
    int x_amount = snprintf(x_text, sizeof(x_text), "%.9g", x);
    int y_amount = snprintf(y_text, sizeof(y_text), "%.9g", y);
    if (x_amount <= 0 || (size_t)x_amount >= sizeof(x_text) ||
        y_amount <= 0 || (size_t)y_amount >= sizeof(y_text))
        return VIEWER_EDITOR_HANDLED;
    size_t gap_length = y_number->offset - x_number->offset - x_number->length;
    base_bytes gap = {0};
    if (!document_edit_span_bytes(editor->edit,
            (document_source_span){x_number->offset + x_number->length,
                gap_length}, &gap)) return VIEWER_EDITOR_HANDLED;
    char replacement[256];
    size_t used = 0U;
    memcpy(replacement + used, x_text, (size_t)x_amount); used += (size_t)x_amount;
    if (gap_length > sizeof(replacement) - used ||
        (size_t)y_amount > sizeof(replacement) - used - gap_length)
        return VIEWER_EDITOR_HANDLED;
    memcpy(replacement + used, gap.ptr, gap_length); used += gap_length;
    memcpy(replacement + used, y_text, (size_t)y_amount); used += (size_t)y_amount;
    document_source_span span = {x_number->offset,
        y_number->offset + y_number->length - x_number->offset};
    return path_commit_replacement(editor, document, span, replacement, used,
        "Smooth anchor adjusted · preview updating");
}

uint32_t editor_canvas_path_node_convert(viewer_editor *editor,
    const svg_shape_document *document, bool smooth)
{
    if (editor == NULL || document == NULL || !editor->node_edit_active ||
        editor->path_selected_node_segment == SIZE_MAX) return VIEWER_EDITOR_HANDLED;
    editor_path_node_ref nodes[VIEWER_EDITOR_PATH_POINT_CAP];
    size_t count = 0U, selected = SIZE_MAX;
    if (!editor_canvas_path_node_collect(editor, document, nodes, &count))
        return VIEWER_EDITOR_HANDLED;
    for (size_t index = 0U; index < count; ++index)
        if (nodes[index].segment_index == editor->path_selected_node_segment) {
            selected = index;
            break;
        }
    if (selected == SIZE_MAX) return VIEWER_EDITOR_HANDLED;
    return smooth ? path_smooth(editor, document, nodes, count, selected) :
        path_corner(editor, document, nodes, count, selected);
}

uint32_t viewer_editor_convert_selected_path_node(viewer_editor *editor,
    const svg_shape_document *document, bool smooth)
{
    return editor_canvas_path_node_convert(editor, document, smooth);
}
