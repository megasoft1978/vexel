#include "viewer_editor_internal.h"

#include <math.h>
#include <stdlib.h>

static bool overlay_rect_valid(svg_shape_rect rect)
{
    return isfinite(rect.x) && isfinite(rect.y) && isfinite(rect.width) &&
        isfinite(rect.height) && rect.width >= 0.0 && rect.height >= 0.0;
}

static bool clip_line(double *x0, double *y0, double *x1, double *y1,
                      ui_rect clip)
{
    double left = (double)clip.x, top = (double)clip.y;
    double right = (double)clip.x + clip.width - 1.0;
    double bottom = (double)clip.y + clip.height - 1.0;
    double dx = *x1 - *x0, dy = *y1 - *y0;
    double p[4] = {-dx, dx, -dy, dy};
    double q[4] = {*x0 - left, right - *x0, *y0 - top, bottom - *y0};
    double low = 0.0, high = 1.0;
    for (size_t index = 0U; index < 4U; ++index) {
        if (p[index] == 0.0) {
            if (q[index] < 0.0) return false;
            continue;
        }
        double ratio = q[index] / p[index];
        if (p[index] < 0.0) low = fmax(low, ratio);
        else high = fmin(high, ratio);
        if (low > high) return false;
    }
    double start_x = *x0, start_y = *y0;
    *x0 = start_x + low * dx;
    *y0 = start_y + low * dy;
    *x1 = start_x + high * dx;
    *y1 = start_y + high * dy;
    return true;
}

static void paint_line(uint8_t *rgba, uint32_t width, uint32_t height,
                       size_t stride, ui_rect clip, geom_point from,
                       geom_point to, uint32_t color)
{
    double x0 = from.x, y0 = from.y, x1 = to.x, y1 = to.y;
    if (clip.width == 0U || clip.height == 0U ||
        !clip_line(&x0, &y0, &x1, &y1, clip)) return;
    int64_t ix0 = (int64_t)llround(x0), iy0 = (int64_t)llround(y0);
    int64_t ix1 = (int64_t)llround(x1), iy1 = (int64_t)llround(y1);
    int64_t dx = llabs(ix1 - ix0), sx = ix0 < ix1 ? 1 : -1;
    int64_t dy = -llabs(iy1 - iy0), sy = iy0 < iy1 ? 1 : -1;
    int64_t error = dx + dy;
    for (;;) {
        if (ix0 >= 0 && iy0 >= 0 && (uint64_t)ix0 < width && (uint64_t)iy0 < height) {
            uint8_t *pixel = rgba + (size_t)iy0 * stride + (size_t)ix0 * 4U;
            pixel[0] = (uint8_t)(color >> 24U);
            pixel[1] = (uint8_t)(color >> 16U);
            pixel[2] = (uint8_t)(color >> 8U);
            pixel[3] = 255U;
        }
        if (ix0 == ix1 && iy0 == iy1) break;
        int64_t twice = 2 * error;
        if (twice >= dy) { error += dy; ix0 += sx; }
        if (twice <= dx) { error += dx; iy0 += sy; }
    }
}

static void paint_handle(uint8_t *rgba, uint32_t width, uint32_t height,
                         size_t stride, ui_rect clip, geom_point center,
                         uint32_t size, bool circle)
{
    int64_t half = (int64_t)(size / 2U);
    int64_t center_x = (int64_t)llround(center.x);
    int64_t center_y = (int64_t)llround(center.y);
    int64_t left = center_x - half, top = center_y - half;
    int64_t right = left + (int64_t)size, bottom = top + (int64_t)size;
    int64_t clip_left = (int64_t)clip.x, clip_top = (int64_t)clip.y;
    int64_t clip_right = clip_left + clip.width, clip_bottom = clip_top + clip.height;
    uint64_t radius_squared = (uint64_t)(half * half);
    for (int64_t y = top; y < bottom; ++y) {
        for (int64_t x = left; x < right; ++x) {
            if (x < clip_left || y < clip_top || x >= clip_right || y >= clip_bottom ||
                x < 0 || y < 0 || (uint64_t)x >= width || (uint64_t)y >= height) continue;
            int64_t dx = x - center_x, dy = y - center_y;
            if (circle && (uint64_t)(dx * dx + dy * dy) > radius_squared) continue;
            uint8_t *pixel = rgba + (size_t)y * stride + (size_t)x * 4U;
            bool edge = x == left || x + 1 == right || y == top || y + 1 == bottom;
            pixel[0] = edge ? 36U : 248U;
            pixel[1] = edge ? 105U : 250U;
            pixel[2] = edge ? 218U : 255U;
            pixel[3] = 255U;
        }
    }
}

static bool overlay_user_to_screen(const svg_shape_document *document,
    const ui_viewer_navigation *navigation, ui_rect canvas, geom_point user,
    geom_point *out)
{
    if (document == NULL || navigation == NULL || out == NULL ||
        document->width == 0U || document->height == 0U ||
        !(document->view_width > 0.0) || !(document->view_height > 0.0) ||
        !(navigation->scale > 0.0)) return false;
    double scale_x = (double)document->width / document->view_width;
    double scale_y = (double)document->height / document->view_height;
    double offset_x = 0.0, offset_y = 0.0;
    svg_shape_aspect aspect = document->preserve_aspect;
    if (!aspect.set) aspect = (svg_shape_aspect){0.5, 0.5, false, false, true};
    if (!aspect.none) {
        double scale = aspect.slice ? fmax(scale_x, scale_y) : fmin(scale_x, scale_y);
        scale_x = scale;
        scale_y = scale;
        offset_x = ((double)document->width - document->view_width * scale) * aspect.align_x;
        offset_y = ((double)document->height - document->view_height * scale) * aspect.align_y;
    }
    double device_x = offset_x + (user.x - document->view_x) * scale_x;
    double device_y = offset_y + (user.y - document->view_y) * scale_y;
    out->x = (double)canvas.x + navigation->pan_x + device_x * navigation->scale;
    out->y = (double)canvas.y + navigation->pan_y + device_y * navigation->scale;
    return isfinite(out->x) && isfinite(out->y);
}

static geom_point cubic_point(geom_point start, geom_point control1,
    geom_point control2, geom_point end, double t)
{
    double inverse = 1.0 - t;
    double a = inverse * inverse * inverse;
    double b = 3.0 * inverse * inverse * t;
    double c = 3.0 * inverse * t * t;
    double d = t * t * t;
    return (geom_point){a * start.x + b * control1.x + c * control2.x + d * end.x,
        a * start.y + b * control1.y + c * control2.y + d * end.y};
}

static void paint_path_preview(const viewer_editor_canvas_overlay_state *overlay,
    const svg_shape_document *document, const ui_viewer_navigation *navigation,
    ui_rect canvas, uint8_t *rgba, uint32_t width, uint32_t height, size_t stride,
    double backing_scale)
{
    if (overlay == NULL || overlay->path_point_count == 0U ||
        overlay->path_point_count > VIEWER_EDITOR_PATH_POINT_CAP) return;
    geom_point screen[VIEWER_EDITOR_PATH_POINT_CAP];
    for (size_t index = 0U; index < overlay->path_point_count; ++index)
        if (!overlay_user_to_screen(document, navigation, canvas,
                overlay->path_points[index], &screen[index])) return;
    for (size_t index = 1U; !overlay->path_nodes_only &&
         index < overlay->path_point_count; ++index) {
        geom_point from = screen[index - 1U], to = screen[index];
        bool curve = overlay->path_has_handles[index - 1U] ||
            (index < overlay->path_point_count - 1U &&
             overlay->path_has_handles[index]);
        if (!curve) {
            paint_line(rgba, width, height, stride, canvas, from, to,
                UINT32_C(0x246bdaff));
            continue;
        }
        geom_point control1 = from, control2 = to;
        if (overlay->path_has_handles[index - 1U] &&
            !overlay_user_to_screen(document, navigation, canvas,
                overlay->path_handles[index - 1U], &control1)) return;
        if (index < overlay->path_point_count - 1U &&
            overlay->path_has_handles[index]) {
            geom_point vector = {
                overlay->path_handles[index].x - overlay->path_points[index].x,
                overlay->path_handles[index].y - overlay->path_points[index].y};
            geom_point incoming = {overlay->path_points[index].x - vector.x,
                overlay->path_points[index].y - vector.y};
            if (!overlay_user_to_screen(document, navigation, canvas,
                    incoming, &control2)) return;
        }
        geom_point previous = from;
        for (size_t step = 1U; step <= 16U; ++step) {
            geom_point next = cubic_point(from, control1, control2, to,
                (double)step / 16.0);
            paint_line(rgba, width, height, stride, canvas, previous, next,
                UINT32_C(0x246bdaff));
            previous = next;
        }
    }
    if (!overlay->path_nodes_only && overlay->path_closed_preview &&
        overlay->path_point_count > 2U)
        paint_line(rgba, width, height, stride, canvas,
            screen[overlay->path_point_count - 1U], screen[0], UINT32_C(0x246bdaff));
    uint32_t node_size = (uint32_t)ceil(6.0 * fmax(backing_scale, 1.0));
    if (node_size < 4U) node_size = 4U;
    for (size_t index = 0U; index < overlay->path_point_count; ++index) {
        if (index < VIEWER_EDITOR_PATH_POINT_CAP && overlay->path_has_handles[index]) {
            geom_point control;
            if (overlay_user_to_screen(document, navigation, canvas,
                    overlay->path_handles[index], &control)) {
                paint_line(rgba, width, height, stride, canvas, screen[index], control,
                    UINT32_C(0x4a8cefff));
                paint_handle(rgba, width, height, stride, canvas, control,
                    node_size, true);
            }
        }
        if (index < VIEWER_EDITOR_PATH_POINT_CAP &&
            overlay->path_has_in_handles[index]) {
            geom_point control;
            if (overlay_user_to_screen(document, navigation, canvas,
                    overlay->path_in_handles[index], &control)) {
                paint_line(rgba, width, height, stride, canvas, screen[index], control,
                    UINT32_C(0x4a8cefff));
                paint_handle(rgba, width, height, stride, canvas, control,
                    node_size, true);
            }
        }
        paint_handle(rgba, width, height, stride, canvas, screen[index],
            node_size, false);
    }
}

static geom_point rotate_screen_point(geom_point point, geom_point center,
                                      double angle)
{
    double dx = point.x - center.x, dy = point.y - center.y;
    double cosine = cos(angle), sine = sin(angle);
    return (geom_point){center.x + dx * cosine - dy * sine,
        center.y + dx * sine + dy * cosine};
}

void viewer_editor_paint_canvas_overlay(viewer_editor *editor,
    const svg_shape_document *document, const ui_viewer_navigation *navigation,
    ui_rect canvas, uint8_t *rgba, uint32_t width, uint32_t height,
    size_t stride)
{
    if (editor == NULL || document == NULL || navigation == NULL || rgba == NULL ||
        canvas.width == 0U || canvas.height == 0U || !isfinite(navigation->scale) ||
        !(navigation->scale > 0.0) || stride < (size_t)width * 4U) return;
    viewer_editor_canvas_overlay_state overlay;
    if (!viewer_editor_get_canvas_overlay(editor, document, &overlay) ||
        (overlay.path_point_count == 0U && !overlay_rect_valid(overlay.bounds))) return;
    if (overlay.snap_guide_x_visible) {
        geom_point guide;
        if (overlay_user_to_screen(document, navigation, canvas,
                (geom_point){overlay.snap_guide_x, document->view_y}, &guide))
            paint_line(rgba, width, height, stride, canvas, guide,
                (geom_point){guide.x, (double)canvas.y + canvas.height},
                UINT32_C(0x12a99aff));
    }
    if (overlay.snap_guide_y_visible) {
        geom_point guide;
        if (overlay_user_to_screen(document, navigation, canvas,
                (geom_point){document->view_x, overlay.snap_guide_y}, &guide))
            paint_line(rgba, width, height, stride, canvas, guide,
                (geom_point){(double)canvas.x + canvas.width, guide.y},
                UINT32_C(0x12a99aff));
    }
    if (overlay.path_point_count > 0U) {
        paint_path_preview(&overlay, document, navigation, canvas, rgba,
            width, height, stride, editor->backing_scale);
        return;
    }
    geom_point center = {
        (double)canvas.x + navigation->pan_x +
            (overlay.bounds.x + overlay.bounds.width * 0.5) * navigation->scale,
        (double)canvas.y + navigation->pan_y +
            (overlay.bounds.y + overlay.bounds.height * 0.5) * navigation->scale
    };
    double left = (double)canvas.x + navigation->pan_x +
        overlay.bounds.x * navigation->scale;
    double top = (double)canvas.y + navigation->pan_y +
        overlay.bounds.y * navigation->scale;
    double right = left + overlay.bounds.width * navigation->scale;
    double bottom = top + overlay.bounds.height * navigation->scale;
    geom_point corners[4] = {{left, top}, {right, top}, {right, bottom}, {left, bottom}};
    if (overlay.rotation_radians != 0.0) {
        for (size_t index = 0U; index < 4U; ++index)
            corners[index] = rotate_screen_point(corners[index], center,
                overlay.rotation_radians);
    }
    const uint32_t blue = UINT32_C(0x246bdaff);
    for (size_t index = 0U; index < 4U; ++index)
        paint_line(rgba, width, height, stride, canvas, corners[index],
            corners[(index + 1U) % 4U], blue);
    if (overlay.drawing_preview) return;
    geom_point handles[8] = {
        corners[0],
        {(corners[0].x + corners[1].x) * 0.5,
         (corners[0].y + corners[1].y) * 0.5},
        corners[1],
        {(corners[1].x + corners[2].x) * 0.5,
         (corners[1].y + corners[2].y) * 0.5},
        corners[2],
        {(corners[2].x + corners[3].x) * 0.5,
         (corners[2].y + corners[3].y) * 0.5},
        corners[3],
        {(corners[3].x + corners[0].x) * 0.5,
         (corners[3].y + corners[0].y) * 0.5}
    };
    geom_point top_middle = handles[1];
    double normal_x = top_middle.x - center.x;
    double normal_y = top_middle.y - center.y;
    double normal_length = hypot(normal_x, normal_y);
    if (normal_length > 0.0) {
        normal_x /= normal_length;
        normal_y /= normal_length;
    } else { normal_x = 0.0; normal_y = -1.0; }
    geom_point rotate_handle = {top_middle.x + normal_x * 22.0 * editor->backing_scale,
        top_middle.y + normal_y * 22.0 * editor->backing_scale};
    paint_line(rgba, width, height, stride, canvas, top_middle, rotate_handle, blue);
    uint32_t handle_size = (uint32_t)ceil(6.0 * editor->backing_scale);
    if (handle_size < 4U) handle_size = 4U;
    for (size_t index = 0U; index < 8U; ++index)
        paint_handle(rgba, width, height, stride, canvas, handles[index],
            handle_size, false);
    paint_handle(rgba, width, height, stride, canvas, rotate_handle,
        handle_size + 2U, true);
}
