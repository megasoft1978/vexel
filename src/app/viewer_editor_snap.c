#include "viewer_editor_internal.h"

#include <math.h>

#define EDITOR_SNAP_SHAPE_SCAN_LIMIT 8192U

static size_t editor_selected_ranges(viewer_editor *editor,
    document_source_span ranges[EDITOR_CANVAS_SELECTION_CAP])
{
    if (editor == NULL || ranges == NULL) return 0U;
    size_t count = editor->canvas_selection_count;
    if (count == 0U && editor->canvas_selection_valid) {
        ranges[0] = (document_source_span){editor->selected_offset, 0U};
        count = 1U;
    }
    if (count > EDITOR_CANVAS_SELECTION_CAP) count = EDITOR_CANVAS_SELECTION_CAP;
    for (size_t index = 0U; index < count; ++index) {
        document_handle handle = {0};
        document_source_span span = {editor->canvas_selection_offsets[index], 0U};
        if (!document_edit_element_at_offset(editor->edit,
                editor->canvas_selection_offsets[index], &handle) ||
            !document_edit_source_span(editor->edit, handle, &span)) {
            ranges[index] = (document_source_span){SIZE_MAX, 0U};
        } else ranges[index] = span;
    }
    return count;
}

static bool editor_shape_is_selected(size_t offset,
    const document_source_span *ranges, size_t count)
{
    for (size_t index = 0U; index < count; ++index) {
        if (ranges[index].offset == SIZE_MAX || offset < ranges[index].offset) continue;
        if (ranges[index].length == 0U ||
            offset - ranges[index].offset < ranges[index].length) return true;
    }
    return false;
}

static void snap_axis_from_bounds(viewer_editor *editor,
    const svg_shape_document *document, bool horizontal, const double moving[3],
    double tolerance, double *correction, double *guide)
{
    double best = tolerance;
    double chosen = 0.0;
    double chosen_delta = 0.0;
    for (size_t index = 0U; index < 3U; ++index) {
        double grid = nearbyint(moving[index]);
        double delta = grid - moving[index];
        if (fabs(delta) < best) {
            best = fabs(delta); chosen = grid; chosen_delta = delta;
        }
    }
    document_source_span selected[EDITOR_CANVAS_SELECTION_CAP];
    size_t selected_count = editor_selected_ranges(editor, selected);
    size_t scan = document->shape_count < EDITOR_SNAP_SHAPE_SCAN_LIMIT ?
        document->shape_count : EDITOR_SNAP_SHAPE_SCAN_LIMIT;
    for (size_t shape = 0U; shape < scan; ++shape) {
        if (document->shape_sources == NULL || editor_shape_is_selected(
                document->shape_sources[shape].offset, selected, selected_count)) continue;
        svg_shape_selection candidate;
        if (!svg_shape_bounds_query(document, shape, &candidate)) continue;
        double left = candidate.user_bounds.x;
        double top = candidate.user_bounds.y;
        double width = candidate.user_bounds.width;
        double height = candidate.user_bounds.height;
        double targets[3] = {horizontal ? left : top,
            horizontal ? left + width * 0.5 : top + height * 0.5,
            horizontal ? left + width : top + height};
        for (size_t moving_index = 0U; moving_index < 3U; ++moving_index) {
            for (size_t target_index = 0U; target_index < 3U; ++target_index) {
                double delta = targets[target_index] - moving[moving_index];
                if (fabs(delta) <= best) {
                    best = fabs(delta);
                    chosen = targets[target_index];
                    chosen_delta = delta;
                }
            }
        }
    }
    *correction = 0.0;
    if (best < tolerance) {
        *correction = chosen_delta;
        *guide = chosen;
        if (horizontal) editor->snap_guide_x_visible = true;
        else editor->snap_guide_y_visible = true;
    }
}

void editor_canvas_snap_translation(viewer_editor *editor,
    const svg_shape_document *document, double tolerance,
    double *translate_x, double *translate_y)
{
    if (editor == NULL || document == NULL || translate_x == NULL ||
        translate_y == NULL) return;
    editor->snap_guide_x_visible = false;
    editor->snap_guide_y_visible = false;
    if (!editor->snap_enabled || !isfinite(tolerance) || !(tolerance > 0.0)) return;
    svg_shape_rect bounds = editor->canvas_selection.user_bounds;
    double moving_x[3] = {bounds.x + *translate_x,
        bounds.x + bounds.width * 0.5 + *translate_x,
        bounds.x + bounds.width + *translate_x};
    double moving_y[3] = {bounds.y + *translate_y,
        bounds.y + bounds.height * 0.5 + *translate_y,
        bounds.y + bounds.height + *translate_y};
    double correction_x = 0.0, correction_y = 0.0;
    snap_axis_from_bounds(editor, document, true, moving_x, tolerance,
        &correction_x, &editor->snap_guide_x);
    snap_axis_from_bounds(editor, document, false, moving_y, tolerance,
        &correction_y, &editor->snap_guide_y);
    *translate_x += correction_x;
    *translate_y += correction_y;
}

static double snap_axis_point(viewer_editor *editor,
    const svg_shape_document *document, bool horizontal, double value,
    double tolerance, double *guide, bool *visible)
{
    double nearest = nearbyint(value);
    double best = fabs(nearest - value);
    double target = nearest;
    if (best >= tolerance) best = tolerance;
    document_source_span selected[EDITOR_CANVAS_SELECTION_CAP];
    size_t selected_count = editor_selected_ranges(editor, selected);
    size_t scan = document->shape_count < EDITOR_SNAP_SHAPE_SCAN_LIMIT ?
        document->shape_count : EDITOR_SNAP_SHAPE_SCAN_LIMIT;
    for (size_t shape = 0U; shape < scan; ++shape) {
        if (document->shape_sources == NULL || editor_shape_is_selected(
                document->shape_sources[shape].offset, selected, selected_count)) continue;
        svg_shape_selection candidate;
        if (!svg_shape_bounds_query(document, shape, &candidate)) continue;
        double start = horizontal ? candidate.user_bounds.x : candidate.user_bounds.y;
        double size = horizontal ? candidate.user_bounds.width : candidate.user_bounds.height;
        double targets[3] = {start, start + size * 0.5, start + size};
        for (size_t index = 0U; index < 3U; ++index) {
            double distance = fabs(targets[index] - value);
            if (distance < best) { best = distance; target = targets[index]; }
        }
    }
    if (best < tolerance) {
        *guide = target;
        *visible = true;
        return target;
    }
    return value;
}

geom_point editor_canvas_snap_point(viewer_editor *editor,
    const svg_shape_document *document, geom_point point, double tolerance)
{
    if (editor == NULL || document == NULL || !isfinite(point.x) ||
        !isfinite(point.y) || !isfinite(tolerance) || !(tolerance > 0.0)) return point;
    editor->snap_guide_x_visible = false;
    editor->snap_guide_y_visible = false;
    if (!editor->snap_enabled) return point;
    point.x = snap_axis_point(editor, document, true, point.x, tolerance,
        &editor->snap_guide_x, &editor->snap_guide_x_visible);
    point.y = snap_axis_point(editor, document, false, point.y, tolerance,
        &editor->snap_guide_y, &editor->snap_guide_y_visible);
    return point;
}
