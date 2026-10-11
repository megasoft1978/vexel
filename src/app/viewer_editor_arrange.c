#include "viewer_editor_internal.h"

#include <math.h>

typedef struct editor_arrange_item {
    size_t selection_index;
    double center;
    double translate;
} editor_arrange_item;

static bool editor_arrange_bounds(viewer_editor *editor,
    const svg_shape_document *document, svg_shape_selection bounds[
        EDITOR_CANVAS_SELECTION_CAP], size_t *out_count)
{
    if (editor == NULL || document == NULL || bounds == NULL ||
        out_count == NULL || editor->canvas_selection_count < 2U ||
        editor->canvas_selection_count > EDITOR_CANVAS_SELECTION_CAP) return false;
    for (size_t index = 0U; index < editor->canvas_selection_count; ++index) {
        if (!svg_shape_element_at_offset(document,
                editor->canvas_selection_offsets[index], &bounds[index]) ||
            !isfinite(bounds[index].user_bounds.x) ||
            !isfinite(bounds[index].user_bounds.y) ||
            !isfinite(bounds[index].user_bounds.width) ||
            !isfinite(bounds[index].user_bounds.height) ||
            bounds[index].user_bounds.width < 0.0 ||
            bounds[index].user_bounds.height < 0.0) return false;
    }
    *out_count = editor->canvas_selection_count;
    return true;
}

static bool editor_arrange_delta_valid(double value)
{
    return isfinite(value) && fabs(value) <= 1000000.0;
}

uint32_t viewer_editor_align_selection(viewer_editor *editor,
    const svg_shape_document *document, viewer_editor_alignment alignment)
{
    if (editor == NULL || document == NULL ||
        (unsigned)alignment > (unsigned)VIEWER_EDITOR_ALIGN_BOTTOM)
        return VIEWER_EDITOR_HANDLED;
    svg_shape_selection bounds[EDITOR_CANVAS_SELECTION_CAP];
    double translate_x[EDITOR_CANVAS_SELECTION_CAP] = {0};
    double translate_y[EDITOR_CANVAS_SELECTION_CAP] = {0};
    size_t count = 0U;
    if (!editor_arrange_bounds(editor, document, bounds, &count)) {
        editor_status_ok(editor, "Align needs at least two rendered selections");
        return VIEWER_EDITOR_HANDLED;
    }
    double minimum_x = bounds[0].user_bounds.x;
    double maximum_x = minimum_x + bounds[0].user_bounds.width;
    double minimum_y = bounds[0].user_bounds.y;
    double maximum_y = minimum_y + bounds[0].user_bounds.height;
    if (!isfinite(maximum_x) || !isfinite(maximum_y)) return VIEWER_EDITOR_HANDLED;
    for (size_t index = 1U; index < count; ++index) {
        double right = bounds[index].user_bounds.x + bounds[index].user_bounds.width;
        double bottom = bounds[index].user_bounds.y + bounds[index].user_bounds.height;
        if (!isfinite(right) || !isfinite(bottom)) return VIEWER_EDITOR_HANDLED;
        if (bounds[index].user_bounds.x < minimum_x)
            minimum_x = bounds[index].user_bounds.x;
        if (right > maximum_x) maximum_x = right;
        if (bounds[index].user_bounds.y < minimum_y)
            minimum_y = bounds[index].user_bounds.y;
        if (bottom > maximum_y) maximum_y = bottom;
    }
    double target_x = minimum_x;
    double target_y = minimum_y;
    if (alignment == VIEWER_EDITOR_ALIGN_HORIZONTAL_CENTER)
        target_x = (minimum_x + maximum_x) * 0.5;
    else if (alignment == VIEWER_EDITOR_ALIGN_RIGHT) target_x = maximum_x;
    else if (alignment == VIEWER_EDITOR_ALIGN_VERTICAL_CENTER)
        target_y = (minimum_y + maximum_y) * 0.5;
    else if (alignment == VIEWER_EDITOR_ALIGN_BOTTOM) target_y = maximum_y;
    for (size_t index = 0U; index < count; ++index) {
        const svg_shape_rect item = bounds[index].user_bounds;
        if (alignment == VIEWER_EDITOR_ALIGN_LEFT)
            translate_x[index] = target_x - item.x;
        else if (alignment == VIEWER_EDITOR_ALIGN_HORIZONTAL_CENTER)
            translate_x[index] = target_x - (item.x + item.width * 0.5);
        else if (alignment == VIEWER_EDITOR_ALIGN_RIGHT)
            translate_x[index] = target_x - (item.x + item.width);
        else if (alignment == VIEWER_EDITOR_ALIGN_TOP)
            translate_y[index] = target_y - item.y;
        else if (alignment == VIEWER_EDITOR_ALIGN_VERTICAL_CENTER)
            translate_y[index] = target_y - (item.y + item.height * 0.5);
        else translate_y[index] = target_y - (item.y + item.height);
        if (!editor_arrange_delta_valid(translate_x[index]) ||
            !editor_arrange_delta_valid(translate_y[index])) {
            editor_status_ok(editor, "Align exceeds the editor coordinate limit");
            return VIEWER_EDITOR_HANDLED;
        }
    }
    uint32_t result = editor_canvas_multi_translate_each(editor, translate_x,
        translate_y, count);
    if ((result & VIEWER_EDITOR_MUTATED) != 0U)
        editor_status_ok(editor, "Selection aligned · preview updating");
    return result;
}

static bool arrange_item_before(const editor_arrange_item *left,
    const editor_arrange_item *right)
{
    return left->center < right->center ||
        (left->center == right->center &&
         left->selection_index < right->selection_index);
}

uint32_t viewer_editor_distribute_selection(viewer_editor *editor,
    const svg_shape_document *document,
    viewer_editor_distribution distribution)
{
    if (editor == NULL || document == NULL ||
        (unsigned)distribution >
            (unsigned)VIEWER_EDITOR_DISTRIBUTE_VERTICAL)
        return VIEWER_EDITOR_HANDLED;
    svg_shape_selection bounds[EDITOR_CANVAS_SELECTION_CAP];
    double translate_x[EDITOR_CANVAS_SELECTION_CAP] = {0};
    double translate_y[EDITOR_CANVAS_SELECTION_CAP] = {0};
    editor_arrange_item items[EDITOR_CANVAS_SELECTION_CAP];
    size_t count = 0U;
    if (!editor_arrange_bounds(editor, document, bounds, &count) || count < 3U) {
        editor_status_ok(editor, "Distribute needs at least three rendered selections");
        return VIEWER_EDITOR_HANDLED;
    }
    for (size_t index = 0U; index < count; ++index) {
        const svg_shape_rect item = bounds[index].user_bounds;
        double center = distribution == VIEWER_EDITOR_DISTRIBUTE_HORIZONTAL ?
            item.x + item.width * 0.5 : item.y + item.height * 0.5;
        if (!isfinite(center)) return VIEWER_EDITOR_HANDLED;
        items[index] = (editor_arrange_item){index, center, 0.0};
    }
    for (size_t index = 1U; index < count; ++index) {
        editor_arrange_item value = items[index];
        size_t position = index;
        while (position > 0U && arrange_item_before(&value,
                &items[position - 1U])) {
            items[position] = items[position - 1U];
            --position;
        }
        items[position] = value;
    }
    double first = items[0].center;
    double last = items[count - 1U].center;
    double interval = (last - first) / (double)(count - 1U);
    if (!isfinite(interval)) return VIEWER_EDITOR_HANDLED;
    for (size_t index = 1U; index + 1U < count; ++index) {
        size_t selection_index = items[index].selection_index;
        double target = first + interval * (double)index;
        items[index].translate = target - items[index].center;
        if (!editor_arrange_delta_valid(items[index].translate)) {
            editor_status_ok(editor,
                "Distribute exceeds the editor coordinate limit");
            return VIEWER_EDITOR_HANDLED;
        }
        if (distribution == VIEWER_EDITOR_DISTRIBUTE_HORIZONTAL)
            translate_x[selection_index] = items[index].translate;
        else translate_y[selection_index] = items[index].translate;
    }
    uint32_t result = editor_canvas_multi_translate_each(editor, translate_x,
        translate_y, count);
    if ((result & VIEWER_EDITOR_MUTATED) != 0U)
        editor_status_ok(editor, "Selection distributed · preview updating");
    return result;
}
