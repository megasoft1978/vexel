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
        !overlay_rect_valid(overlay.bounds)) return;
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
