#include "viewer_editor_internal.h"

#include <math.h>
#include <stdio.h>
#include <string.h>

#define EDITOR_SNAP_SHAPE_SCAN_LIMIT 8192U
#define EDITOR_SNAP_SHAPE_CANDIDATE_CAP 128U

typedef struct editor_snap_shape {
    svg_shape_rect bounds;
    size_t source_offset;
    double distance;
} editor_snap_shape;

typedef enum editor_snap_kind {
    EDITOR_SNAP_NONE,
    EDITOR_SNAP_GUIDE,
    EDITOR_SNAP_OBJECT,
    EDITOR_SNAP_GRID,
    EDITOR_SNAP_EQUAL_GAP
} editor_snap_kind;

typedef struct editor_snap_candidate {
    editor_snap_kind kind;
    bool axis_x;
    double target;
    double delta;
    double distance;
    double gap;
    size_t source_offset;
    uint8_t moving_index;
    uint8_t target_index;
} editor_snap_candidate;

static size_t editor_selected_ranges(viewer_editor *editor,
    document_source_span ranges[EDITOR_CANVAS_SELECTION_CAP])
{
    if (editor == NULL || ranges == NULL) return 0U;
    size_t count = editor->canvas_selection_count;
    if (count == 0U && editor->canvas_selection_valid) {
        document_handle handle = {0U};
        document_source_span span = {editor->selected_offset, 0U};
        if (document_edit_element_at_offset(editor->edit,
                editor->selected_offset, &handle))
            (void)document_edit_source_span(editor->edit, handle, &span);
        ranges[0] = span;
        return 1U;
    }
    if (count > EDITOR_CANVAS_SELECTION_CAP)
        count = EDITOR_CANVAS_SELECTION_CAP;
    for (size_t index = 0U; index < count; ++index) {
        document_handle handle = {0U};
        document_source_span span = {editor->canvas_selection_offsets[index], 0U};
        if (!document_edit_element_at_offset(editor->edit,
                editor->canvas_selection_offsets[index], &handle) ||
            !document_edit_source_span(editor->edit, handle, &span))
            ranges[index] = (document_source_span){SIZE_MAX, 0U};
        else ranges[index] = span;
    }
    return count;
}

static bool editor_shape_is_selected(size_t offset,
    const document_source_span *ranges, size_t count)
{
    for (size_t index = 0U; index < count; ++index) {
        if (ranges[index].offset == SIZE_MAX || offset < ranges[index].offset)
            continue;
        if (ranges[index].length == 0U ||
            offset - ranges[index].offset < ranges[index].length) return true;
    }
    return false;
}

static double interval_gap(double a0, double a1, double b0, double b1)
{
    if (a1 < b0) return b0 - a1;
    if (b1 < a0) return a0 - b1;
    return 0.0;
}

static double rect_distance(svg_shape_rect a, svg_shape_rect b)
{
    double dx = interval_gap(a.x, a.x + a.width, b.x, b.x + b.width);
    double dy = interval_gap(a.y, a.y + a.height, b.y, b.y + b.height);
    return hypot(dx, dy);
}

static bool snap_shape_before(const editor_snap_shape *left,
                              const editor_snap_shape *right)
{
    return left->distance < right->distance ||
        (left->distance == right->distance &&
         left->source_offset < right->source_offset);
}

static size_t snap_collect_shapes(viewer_editor *editor,
    const svg_shape_document *document, svg_shape_rect near,
    editor_snap_shape out[EDITOR_SNAP_SHAPE_CANDIDATE_CAP])
{
    if (editor == NULL || document == NULL || out == NULL ||
        document->shape_sources == NULL) return 0U;
    document_source_span selected[EDITOR_CANVAS_SELECTION_CAP];
    size_t selected_count = editor_selected_ranges(editor, selected);
    size_t scan = document->shape_count < EDITOR_SNAP_SHAPE_SCAN_LIMIT ?
        document->shape_count : EDITOR_SNAP_SHAPE_SCAN_LIMIT;
    size_t count = 0U;
    for (size_t shape = 0U; shape < scan; ++shape) {
        size_t offset = document->shape_sources[shape].offset;
        if (editor_shape_is_selected(offset, selected, selected_count)) continue;
        document_handle handle = {0U};
        if (!document_edit_element_at_offset(editor->edit, offset, &handle) ||
            !editor_outline_is_visible(editor, handle)) continue;
        svg_shape_selection selection;
        if (!svg_shape_bounds_query(document, shape, &selection) ||
            !isfinite(selection.user_bounds.x) ||
            !isfinite(selection.user_bounds.y) ||
            !isfinite(selection.user_bounds.width) ||
            !isfinite(selection.user_bounds.height) ||
            selection.user_bounds.width < 0.0 ||
            selection.user_bounds.height < 0.0) continue;
        editor_snap_shape candidate = {selection.user_bounds, offset,
            rect_distance(selection.user_bounds, near)};
        if (!isfinite(candidate.distance)) continue;
        size_t position = 0U;
        while (position < count && !snap_shape_before(&candidate, &out[position]))
            ++position;
        if (position >= EDITOR_SNAP_SHAPE_CANDIDATE_CAP) continue;
        if (count < EDITOR_SNAP_SHAPE_CANDIDATE_CAP) ++count;
        for (size_t move = count - 1U; move > position; --move)
            out[move] = out[move - 1U];
        out[position] = candidate;
    }
    return count;
}

static unsigned snap_priority(editor_snap_kind kind)
{
    switch (kind) {
        case EDITOR_SNAP_GUIDE: return 0U;
        case EDITOR_SNAP_OBJECT: return 1U;
        case EDITOR_SNAP_GRID: return 2U;
        case EDITOR_SNAP_EQUAL_GAP: return 3U;
        case EDITOR_SNAP_NONE: break;
    }
    return UINT32_MAX;
}

static bool snap_candidate_before(const editor_snap_candidate *candidate,
                                 const editor_snap_candidate *current)
{
    if (current->kind == EDITOR_SNAP_NONE) return true;
    if (candidate->distance != current->distance)
        return candidate->distance < current->distance;
    unsigned candidate_priority = snap_priority(candidate->kind);
    unsigned current_priority = snap_priority(current->kind);
    if (candidate_priority != current_priority)
        return candidate_priority < current_priority;
    if (candidate->source_offset != current->source_offset)
        return candidate->source_offset < current->source_offset;
    if (candidate->axis_x != current->axis_x) return candidate->axis_x;
    if (candidate->moving_index != current->moving_index)
        return candidate->moving_index < current->moving_index;
    return candidate->target_index < current->target_index;
}

static void snap_consider(editor_snap_candidate *best, editor_snap_kind kind,
    bool axis_x, double moving, double target, double tolerance,
    size_t source_offset, uint8_t moving_index, uint8_t target_index,
    double gap)
{
    if (best == NULL || !isfinite(moving) || !isfinite(target) ||
        !isfinite(tolerance) || !(tolerance > 0.0)) return;
    double delta = target - moving;
    double distance = fabs(delta);
    if (!(distance < tolerance)) return;
    editor_snap_candidate candidate = {kind, axis_x, target, delta, distance,
        gap, source_offset, moving_index, target_index};
    if (snap_candidate_before(&candidate, best)) *best = candidate;
}

static void snap_clear(viewer_editor *editor)
{
    if (editor == NULL) return;
    editor->snap_guide_x_visible = false;
    editor->snap_guide_y_visible = false;
    editor->snap_label_visible = false;
    editor->snap_label_is_gap = false;
    editor->snap_label[0] = '\0';
}

void editor_canvas_snap_clear(viewer_editor *editor)
{
    snap_clear(editor);
}

static void snap_store(viewer_editor *editor,
                       const editor_snap_candidate *candidate)
{
    if (editor == NULL || candidate == NULL ||
        candidate->kind == EDITOR_SNAP_NONE) return;
    if (candidate->axis_x) {
        editor->snap_guide_x_visible = true;
        editor->snap_guide_x = candidate->target;
    } else {
        editor->snap_guide_y_visible = true;
        editor->snap_guide_y = candidate->target;
    }
    editor->snap_label_is_gap = candidate->kind == EDITOR_SNAP_EQUAL_GAP;
    int length = editor->snap_label_is_gap ?
        snprintf(editor->snap_label, sizeof(editor->snap_label), "Gap %.4g",
            candidate->gap) :
        snprintf(editor->snap_label, sizeof(editor->snap_label), "Δ%c %+.4g",
            candidate->axis_x ? 'x' : 'y', candidate->delta);
    editor->snap_label_visible = length > 0 &&
        (size_t)length < sizeof(editor->snap_label);
}

static void snap_offer_grid(editor_snap_candidate *best, bool axis_x,
    const double moving[3], double tolerance)
{
    for (uint8_t index = 0U; index < 3U; ++index) {
        double target = nearbyint(moving[index]);
        snap_consider(best, EDITOR_SNAP_GRID, axis_x, moving[index], target,
            tolerance, SIZE_MAX, index, 0U, 0.0);
    }
}

static void snap_offer_guides(viewer_editor *editor,
    editor_snap_candidate *best, bool axis_x, const double moving[3],
    double tolerance)
{
    if (editor == NULL || !editor->snap_guides_enabled) return;
    size_t count = editor->snap_guide_count;
    if (count > EDITOR_SNAP_GUIDE_CAP) count = EDITOR_SNAP_GUIDE_CAP;
    for (size_t guide = 0U; guide < count; ++guide) {
        const editor_snap_guide *value = &editor->snap_guides[guide];
        if (value->vertical != axis_x || !isfinite(value->position)) continue;
        for (uint8_t index = 0U; index < 3U; ++index)
            snap_consider(best, EDITOR_SNAP_GUIDE, axis_x, moving[index],
                value->position, tolerance, guide, index, 0U, 0.0);
    }
    if (!editor->guides.guides_visible) return;
    viewer_editor_guide_axis_kind axis = axis_x ?
        VIEWER_EDITOR_GUIDE_VERTICAL : VIEWER_EDITOR_GUIDE_HORIZONTAL;
    size_t ruler_count = viewer_editor_guides_count(&editor->guides, axis);
    for (size_t guide = 0U; guide < ruler_count; ++guide) {
        double target = 0.0;
        if (!viewer_editor_guides_get(&editor->guides, axis, guide, &target))
            continue;
        for (uint8_t index = 0U; index < 3U; ++index)
            snap_consider(best, EDITOR_SNAP_GUIDE, axis_x, moving[index],
                target, tolerance, guide, index, 0U, 0.0);
    }
}

static void snap_offer_objects(editor_snap_candidate *best, bool axis_x,
    const double moving[3], double tolerance,
    const editor_snap_shape *shapes, size_t count)
{
    for (size_t shape = 0U; shape < count; ++shape) {
        const svg_shape_rect bounds = shapes[shape].bounds;
        double start = axis_x ? bounds.x : bounds.y;
        double size = axis_x ? bounds.width : bounds.height;
        double targets[3] = {start, start + size * 0.5, start + size};
        for (uint8_t moving_index = 0U; moving_index < 3U; ++moving_index)
            for (uint8_t target_index = 0U; target_index < 3U; ++target_index)
                snap_consider(best, EDITOR_SNAP_OBJECT, axis_x,
                    moving[moving_index], targets[target_index], tolerance,
                    shapes[shape].source_offset, moving_index, target_index, 0.0);
    }
}

static bool ranges_overlap(double a0, double a1, double b0, double b1)
{
    return a0 <= b1 && b0 <= a1;
}

static void snap_offer_equal_gaps(editor_snap_candidate *best_x,
    editor_snap_candidate *best_y,
    const svg_shape_rect moving, double tolerance,
    const editor_snap_shape *shapes, size_t count)
{
    for (size_t left_index = 0U; left_index < count; ++left_index) {
        svg_shape_rect left = shapes[left_index].bounds;
        if (!ranges_overlap(moving.y, moving.y + moving.height,
                left.y, left.y + left.height)) continue;
        for (size_t right_index = 0U; right_index < count; ++right_index) {
            if (left_index == right_index) continue;
            svg_shape_rect right = shapes[right_index].bounds;
            if (left.x + left.width > right.x ||
                !ranges_overlap(moving.y, moving.y + moving.height,
                    right.y, right.y + right.height)) continue;
            double available = right.x - (left.x + left.width);
            double gap = (available - moving.width) * 0.5;
            if (!isfinite(gap) || gap < 0.0) continue;
            double target = left.x + left.width + gap;
            snap_consider(best_x, EDITOR_SNAP_EQUAL_GAP, true,
                moving.x, target, tolerance,
                shapes[left_index].source_offset, 0U,
                (uint8_t)(right_index < UINT8_MAX ? right_index : UINT8_MAX), gap);
        }
        /* The vertical case is evaluated with the same fixed bounded set. */
        for (size_t upper_index = 0U; upper_index < count; ++upper_index) {
            if (left_index == upper_index) continue;
            svg_shape_rect lower = shapes[upper_index].bounds;
            if (left.y + left.height > lower.y ||
                !ranges_overlap(moving.x, moving.x + moving.width,
                    left.x, left.x + left.width) ||
                !ranges_overlap(moving.x, moving.x + moving.width,
                    lower.x, lower.x + lower.width)) continue;
            double available = lower.y - (left.y + left.height);
            double gap = (available - moving.height) * 0.5;
            if (!isfinite(gap) || gap < 0.0) continue;
            double target = left.y + left.height + gap;
            snap_consider(best_y, EDITOR_SNAP_EQUAL_GAP, false,
                moving.y, target, tolerance,
                shapes[left_index].source_offset, 0U,
                (uint8_t)(upper_index < UINT8_MAX ? upper_index : UINT8_MAX), gap);
        }
    }
}

static editor_snap_candidate snap_choose_for_rect(viewer_editor *editor,
    const svg_shape_document *document, svg_shape_rect moving,
    double tolerance, bool include_gap, bool prefer_x)
{
    editor_snap_candidate best_x = {0}, best_y = {0};
    double x[3] = {moving.x, moving.x + moving.width * 0.5,
        moving.x + moving.width};
    double y[3] = {moving.y, moving.y + moving.height * 0.5,
        moving.y + moving.height};
    if (editor->snap_grid_enabled) {
        snap_offer_grid(&best_x, true, x, tolerance);
        snap_offer_grid(&best_y, false, y, tolerance);
    }
    editor_snap_shape shapes[EDITOR_SNAP_SHAPE_CANDIDATE_CAP];
    size_t shape_count = editor->snap_objects_enabled ?
        snap_collect_shapes(editor, document, moving, shapes) : 0U;
    if (editor->snap_objects_enabled) {
        snap_offer_objects(&best_x, true, x, tolerance, shapes, shape_count);
        snap_offer_objects(&best_y, false, y, tolerance, shapes, shape_count);
        if (include_gap && shape_count >= 2U)
            snap_offer_equal_gaps(&best_x, &best_y, moving, tolerance,
                shapes, shape_count);
    }
    snap_offer_guides(editor, &best_x, true, x, tolerance);
    snap_offer_guides(editor, &best_y, false, y, tolerance);
    if (prefer_x && best_x.kind != EDITOR_SNAP_NONE) return best_x;
    if (!prefer_x && best_y.kind != EDITOR_SNAP_NONE) return best_y;
    return prefer_x ? best_y : best_x;
}

void editor_canvas_snap_translation(viewer_editor *editor,
    const svg_shape_document *document, double tolerance,
    double *translate_x, double *translate_y)
{
    if (editor == NULL || document == NULL || translate_x == NULL ||
        translate_y == NULL) return;
    snap_clear(editor);
    if (!editor->snap_enabled || editor->snap_bypass ||
        !isfinite(tolerance) || !(tolerance > 0.0)) return;
    svg_shape_selection selection;
    if (!editor_canvas_selection_bounds(editor, document, &selection))
        selection = editor->canvas_selection;
    svg_shape_rect moving = selection.user_bounds;
    moving.x += *translate_x;
    moving.y += *translate_y;
    if (!isfinite(moving.x) || !isfinite(moving.y) ||
        !isfinite(moving.width) || !isfinite(moving.height) ||
        moving.width < 0.0 || moving.height < 0.0) return;
    editor_snap_candidate candidate = snap_choose_for_rect(editor, document,
        moving, tolerance, true, fabs(*translate_x) >= fabs(*translate_y));
    if (candidate.kind == EDITOR_SNAP_NONE) return;
    if (candidate.axis_x) *translate_x += candidate.delta;
    else *translate_y += candidate.delta;
    snap_store(editor, &candidate);
}

static editor_snap_candidate snap_choose_for_point(viewer_editor *editor,
    const svg_shape_document *document, geom_point point, double tolerance)
{
    editor_snap_candidate best = {0};
    double x[3] = {point.x, point.x, point.x};
    double y[3] = {point.y, point.y, point.y};
    if (editor->snap_grid_enabled) {
        snap_offer_grid(&best, true, x, tolerance);
        snap_offer_grid(&best, false, y, tolerance);
    }
    svg_shape_rect near = {point.x, point.y, 0.0, 0.0};
    editor_snap_shape shapes[EDITOR_SNAP_SHAPE_CANDIDATE_CAP];
    size_t shape_count = editor->snap_objects_enabled ?
        snap_collect_shapes(editor, document, near, shapes) : 0U;
    if (editor->snap_objects_enabled) {
        snap_offer_objects(&best, true, x, tolerance, shapes, shape_count);
        snap_offer_objects(&best, false, y, tolerance, shapes, shape_count);
    }
    snap_offer_guides(editor, &best, true, x, tolerance);
    snap_offer_guides(editor, &best, false, y, tolerance);
    return best;
}

geom_point editor_canvas_snap_point(viewer_editor *editor,
    const svg_shape_document *document, geom_point point, double tolerance)
{
    if (editor == NULL || document == NULL || !isfinite(point.x) ||
        !isfinite(point.y) || !isfinite(tolerance) || !(tolerance > 0.0))
        return point;
    snap_clear(editor);
    if (!editor->snap_enabled || editor->snap_bypass) return point;
    editor_snap_candidate candidate = snap_choose_for_point(editor, document,
        point, tolerance);
    if (candidate.kind == EDITOR_SNAP_NONE) return point;
    if (candidate.axis_x) point.x += candidate.delta;
    else point.y += candidate.delta;
    snap_store(editor, &candidate);
    return point;
}

bool editor_canvas_snap_guide_add(viewer_editor *editor, bool vertical,
                                  double position)
{
    if (editor == NULL || !isfinite(position) || fabs(position) > 1000000.0)
        return false;
    if (editor->snap_guide_count > EDITOR_SNAP_GUIDE_CAP)
        editor->snap_guide_count = EDITOR_SNAP_GUIDE_CAP;
    for (size_t index = 0U; index < editor->snap_guide_count; ++index)
        if (editor->snap_guides[index].vertical == vertical &&
            editor->snap_guides[index].position == position) return true;
    if (editor->snap_guide_count >= EDITOR_SNAP_GUIDE_CAP) {
        editor_status_ok(editor, "Snap guide limit reached (128)");
        return false;
    }
    editor->snap_guides[editor->snap_guide_count++] =
        (editor_snap_guide){vertical, position};
    editor->redraw_requested = true;
    return true;
}

void editor_canvas_snap_guide_clear(viewer_editor *editor)
{
    if (editor == NULL || editor->snap_guide_count == 0U) return;
    editor->snap_guide_count = 0U;
    snap_clear(editor);
    editor->redraw_requested = true;
}
