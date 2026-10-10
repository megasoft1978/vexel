#include "viewer_editor_internal.h"

#include <math.h>
#include <stdio.h>
#include <string.h>

#define EDITOR_PATH_DRAFT_LIMIT (64U * 1024U)
static bool path_append(uint8_t *buffer, size_t capacity, size_t *used,
                        const char *bytes, size_t length)
{
    size_t end;
    if (buffer == NULL || used == NULL || (length > 0U && bytes == NULL) ||
        !base_size_add(*used, length, &end) || end > capacity) return false;
    if (length > 0U) memcpy(buffer + *used, bytes, length);
    *used = end;
    return true;
}

static bool path_append_number(uint8_t *buffer, size_t capacity, size_t *used,
                               double value)
{
    if (!isfinite(value) || fabs(value) > 2000000.0) return false;
    char number[64];
    int amount = snprintf(number, sizeof(number), "%.9g", value);
    return amount > 0 && (size_t)amount < sizeof(number) &&
        path_append(buffer, capacity, used, number, (size_t)amount);
}

static bool path_append_xy(uint8_t *buffer, size_t capacity, size_t *used,
                           double x, double y, char separator)
{
    return path_append_number(buffer, capacity, used, x) &&
        path_append(buffer, capacity, used, &separator, 1U) &&
        path_append_number(buffer, capacity, used, y);
}

bool editor_canvas_path_active(const viewer_editor *editor)
{
    return editor != NULL && editor->canvas_gesture == EDITOR_CANVAS_GESTURE_PATH &&
        editor->path_anchor_count > 0U;
}

static bool path_near_first(const viewer_editor *editor, geom_point user,
                            double screen_x, double screen_y)
{
    if (editor == NULL || editor->path_anchor_count < 2U ||
        !isfinite(screen_x) || !isfinite(screen_y)) return false;
    const editor_path_anchor *first = &editor->path_anchors[0];
    double view_scale = editor->canvas_zoom > 0.0 ? editor->canvas_zoom : 1.0;
    double backing_scale = editor->backing_scale > 0.0 ? editor->backing_scale : 1.0;
    double dx = user.x - first->x, dy = user.y - first->y;
    double document_distance = hypot(dx, dy);
    return isfinite(document_distance) &&
        document_distance <= 8.0 / (view_scale * backing_scale);
}

static bool path_add_anchor(viewer_editor *editor, geom_point user, bool drag_handle)
{
    if (editor == NULL || editor->path_anchor_count >= VIEWER_EDITOR_PATH_POINT_CAP ||
        !isfinite(user.x) || !isfinite(user.y) || fabs(user.x) > 1000000.0 ||
        fabs(user.y) > 1000000.0) return false;
    editor->path_anchors[editor->path_anchor_count++] = (editor_path_anchor){
        .x = user.x, .y = user.y};
    editor->path_current_x = user.x;
    editor->path_current_y = user.y;
    editor->path_anchor_dragging = drag_handle;
    editor->path_close_preview = false;
    return true;
}

uint32_t editor_canvas_path_pointer_down(viewer_editor *editor, geom_point user,
    double screen_x, double screen_y, uint8_t click_count)
{
    if (editor == NULL || !isfinite(user.x) || !isfinite(user.y) ||
        !isfinite(screen_x) || !isfinite(screen_y)) return VIEWER_EDITOR_HANDLED;
    bool pen = editor->draw_tool == VIEWER_EDITOR_TOOL_PEN;
    bool polygon = editor->draw_tool == VIEWER_EDITOR_TOOL_POLYGON;
    if (editor->path_anchor_count == 0U) {
        editor->canvas_gesture = EDITOR_CANVAS_GESTURE_PATH;
        if (!path_add_anchor(editor, user, pen)) {
            editor_status_ok(editor, "Path point limit or coordinate limit reached");
            return VIEWER_EDITOR_HANDLED;
        }
        editor->redraw_requested = true;
        return VIEWER_EDITOR_HANDLED;
    }
    if ((pen || polygon) && path_near_first(editor, user, screen_x, screen_y)) {
        editor->path_closed = true;
        editor->path_anchor_dragging = false;
        return editor_canvas_path_finish(editor, true);
    }
    if (editor->path_anchor_count >= VIEWER_EDITOR_PATH_POINT_CAP) {
        editor_status_ok(editor, "Path point limit reached (128 anchors)");
        return VIEWER_EDITOR_HANDLED;
    }
    const editor_path_anchor *last =
        &editor->path_anchors[editor->path_anchor_count - 1U];
    bool at_last = hypot(user.x - last->x, user.y - last->y) < 1.0e-9;
    if (click_count >= 2U && at_last) {
        return editor_canvas_path_finish(editor, polygon);
    }
    if (!path_add_anchor(editor, user, pen)) {
        editor_status_ok(editor, "Path point limit or coordinate limit reached");
        return VIEWER_EDITOR_HANDLED;
    }
    editor->canvas_gesture = EDITOR_CANVAS_GESTURE_PATH;
    if (click_count >= 2U) return editor_canvas_path_finish(editor, polygon);
    editor->redraw_requested = true;
    return VIEWER_EDITOR_HANDLED;
}

uint32_t editor_canvas_path_pointer_move(viewer_editor *editor, geom_point user,
    double screen_x, double screen_y)
{
    if (editor == NULL || editor->path_anchor_count == 0U ||
        !isfinite(user.x) || !isfinite(user.y) || !isfinite(screen_x) ||
        !isfinite(screen_y)) return VIEWER_EDITOR_HANDLED;
    editor->path_current_x = user.x;
    editor->path_current_y = user.y;
    editor->path_close_preview = !editor->path_anchor_dragging &&
        (editor->draw_tool == VIEWER_EDITOR_TOOL_POLYGON ||
         editor->draw_tool == VIEWER_EDITOR_TOOL_PEN) &&
        path_near_first(editor, user, screen_x, screen_y);
    if (editor->path_anchor_dragging && editor->path_anchor_count > 0U) {
        editor_path_anchor *anchor = &editor->path_anchors[editor->path_anchor_count - 1U];
        anchor->handle_x = user.x - anchor->x;
        anchor->handle_y = user.y - anchor->y;
        anchor->has_handle = hypot(anchor->handle_x, anchor->handle_y) >
            2.0 / fmax(editor->backing_scale, 1.0);
    }
    editor->redraw_requested = true;
    return VIEWER_EDITOR_HANDLED;
}

uint32_t editor_canvas_path_pointer_up(viewer_editor *editor)
{
    if (editor == NULL || editor->canvas_gesture != EDITOR_CANVAS_GESTURE_PATH)
        return 0U;
    editor->path_anchor_dragging = false;
    editor->redraw_requested = true;
    return VIEWER_EDITOR_HANDLED;
}

static bool path_append_segment(uint8_t *buffer, size_t capacity, size_t *used,
    const editor_path_anchor *from, const editor_path_anchor *to)
{
    if (from == NULL || to == NULL) return false;
    if (from->has_handle || to->has_handle) {
        double control1_x = from->has_handle ? from->x + from->handle_x : from->x;
        double control1_y = from->has_handle ? from->y + from->handle_y : from->y;
        double control2_x = to->has_handle ? to->x - to->handle_x : to->x;
        double control2_y = to->has_handle ? to->y - to->handle_y : to->y;
        return path_append(buffer, capacity, used, " C ", 3U) &&
            path_append_xy(buffer, capacity, used, control1_x, control1_y, ' ') &&
            path_append(buffer, capacity, used, " ", 1U) &&
            path_append_xy(buffer, capacity, used, control2_x, control2_y, ' ') &&
            path_append(buffer, capacity, used, " ", 1U) &&
            path_append_xy(buffer, capacity, used, to->x, to->y, ' ');
    }
    return path_append(buffer, capacity, used, " L ", 3U) &&
        path_append_xy(buffer, capacity, used, to->x, to->y, ' ');
}

static bool path_serialize_fragment(viewer_editor *editor, bool closed,
                                    uint8_t *buffer, size_t capacity, size_t *out_length)
{
    if (editor == NULL || buffer == NULL || out_length == NULL ||
        editor->path_anchor_count == 0U) return false;
    size_t used = 0U;
    if (editor->draw_tool == VIEWER_EDITOR_TOOL_POLYGON ||
        editor->draw_tool == VIEWER_EDITOR_TOOL_POLYLINE) {
        bool polygon = editor->draw_tool == VIEWER_EDITOR_TOOL_POLYGON;
        if (!path_append(buffer, capacity, &used,
                polygon ? "<polygon points=\"" : "<polyline points=\"",
                polygon ? sizeof("<polygon points=\"") - 1U :
                    sizeof("<polyline points=\"") - 1U)) return false;
        for (size_t index = 0U; index < editor->path_anchor_count; ++index) {
            const editor_path_anchor *point = &editor->path_anchors[index];
            if (index > 0U && !path_append(buffer, capacity, &used, " ", 1U)) return false;
            if (!path_append_xy(buffer, capacity, &used, point->x, point->y, ','))
                return false;
        }
        static const char polygon_style[] =
            "\" fill=\"#4f7cff\" stroke=\"#172033\" stroke-width=\"1\"/>";
        static const char polyline_style[] =
            "\" fill=\"none\" stroke=\"#172033\" stroke-width=\"2\"/>";
        const char *style = polygon ? polygon_style : polyline_style;
        size_t style_length = polygon ? sizeof(polygon_style) - 1U :
            sizeof(polyline_style) - 1U;
        if (!path_append(buffer, capacity, &used, style, style_length)) return false;
        *out_length = used;
        return true;
    }
    if (!path_append(buffer, capacity, &used, "<path d=\"M ", 11U) ||
        !path_append_xy(buffer, capacity, &used, editor->path_anchors[0].x,
            editor->path_anchors[0].y, ' ')) return false;
    for (size_t index = 1U; index < editor->path_anchor_count; ++index)
        if (!path_append_segment(buffer, capacity, &used,
                &editor->path_anchors[index - 1U], &editor->path_anchors[index]))
            return false;
    if (closed) {
        if (!path_append_segment(buffer, capacity, &used,
                &editor->path_anchors[editor->path_anchor_count - 1U],
                &editor->path_anchors[0]) ||
            !path_append(buffer, capacity, &used, " Z", 2U)) return false;
    }
    const char *style = closed ?
        "\" fill=\"#4f7cff\" stroke=\"#172033\" stroke-width=\"1\"/>" :
        "\" fill=\"none\" stroke=\"#172033\" stroke-width=\"2\"/>";
    size_t style_length = strlen(style);
    if (!path_append(buffer, capacity, &used, style, style_length)) return false;
    *out_length = used;
    return true;
}

uint32_t editor_canvas_path_finish(viewer_editor *editor, bool closed)
{
    if (editor == NULL || editor->path_anchor_count == 0U)
        return VIEWER_EDITOR_HANDLED;
    bool polygon = editor->draw_tool == VIEWER_EDITOR_TOOL_POLYGON;
    size_t minimum = polygon ? 3U : 2U;
    if (editor->path_anchor_count < minimum) {
        editor_status_ok(editor, polygon ? "A polygon needs at least three points" :
            "A path needs at least two points");
        return VIEWER_EDITOR_HANDLED;
    }
    if (polygon) closed = true;
    uint8_t fragment[EDITOR_PATH_DRAFT_LIMIT];
    size_t fragment_length = 0U;
    if (!path_serialize_fragment(editor, closed, fragment, sizeof(fragment),
            &fragment_length)) {
        editor_status_ok(editor, "Path exceeds the 64 KiB editor limit");
        return VIEWER_EDITOR_HANDLED;
    }
    document_handle root = {0U}, parent = {0U}, inserted = {0U};
    if (!document_edit_root(editor->edit, &root)) return VIEWER_EDITOR_HANDLED;
    parent = root;
    base_bytes tag = {0};
    if (document_edit_tag_name(editor->edit, editor->selected, &tag) &&
        tag.len == 1U && tag.ptr[0] == 'g') parent = editor->selected;
    size_t old_offset = editor->selected_offset;
    char old_id[EDITOR_NAME_CAP];
    editor_set_text(old_id, sizeof(old_id), editor->selected_has_id ?
        editor->selected_id : "");
    document_edit_error error = document_edit_insert_xml_fragment(editor->edit,
        parent, (base_bytes){fragment, fragment_length}, &inserted);
    if (error != DOCUMENT_EDIT_OK) {
        editor_status_error(editor, error);
        return VIEWER_EDITOR_HANDLED;
    }
    editor->path_anchor_count = 0U;
    editor->path_anchor_dragging = false;
    editor->path_closed = closed;
    editor->path_close_preview = false;
    editor->canvas_gesture = EDITOR_CANVAS_GESTURE_NONE;
    editor->canvas_selection_count = 0U;
    editor->canvas_selection_valid = false;
    editor_status_ok(editor, "Path created · preview updating");
    return editor_apply_result(editor, DOCUMENT_EDIT_OK, old_offset, old_id, inserted);
}

void editor_canvas_path_cancel(viewer_editor *editor)
{
    if (editor == NULL) return;
    editor->path_anchor_count = 0U;
    editor->path_anchor_dragging = false;
    editor->path_closed = false;
    editor->path_close_preview = false;
    editor->canvas_gesture = EDITOR_CANVAS_GESTURE_NONE;
    editor->redraw_requested = true;
}
