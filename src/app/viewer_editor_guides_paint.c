#include "viewer_editor_internal.h"

#include "ui/ui_draw.h"
#include "platform/platform_text_ui.h"

#include <math.h>
#include <stdio.h>

typedef struct guide_paint_transform {
    double scale_x;
    double scale_y;
    double offset_x;
    double offset_y;
} guide_paint_transform;

static bool paint_transform_get(const svg_shape_document *document,
                                guide_paint_transform *out)
{
    if (document == NULL || out == NULL || document->width == 0U ||
        document->height == 0U || !isfinite(document->view_x) ||
        !isfinite(document->view_y) || !isfinite(document->view_width) ||
        !isfinite(document->view_height) || !(document->view_width > 0.0) ||
        !(document->view_height > 0.0)) return false;
    double sx = (double)document->width / document->view_width;
    double sy = (double)document->height / document->view_height;
    double ox = 0.0, oy = 0.0;
    svg_shape_aspect aspect = document->preserve_aspect;
    if (!aspect.set) aspect = (svg_shape_aspect){0.5, 0.5, false, false, true};
    if (!aspect.none) {
        double scale = aspect.slice ? fmax(sx, sy) : fmin(sx, sy);
        sx = scale;
        sy = scale;
        ox = ((double)document->width - document->view_width * scale) * aspect.align_x;
        oy = ((double)document->height - document->view_height * scale) * aspect.align_y;
    }
    if (!isfinite(sx) || !(sx > 0.0) || !isfinite(sy) || !(sy > 0.0) ||
        !isfinite(ox) || !isfinite(oy)) return false;
    *out = (guide_paint_transform){sx, sy, ox, oy};
    return true;
}

static bool user_to_screen(const svg_shape_document *document,
    const ui_viewer_navigation *navigation, ui_rect canvas,
    const guide_paint_transform *transform, geom_point user, geom_point *out)
{
    if (navigation == NULL || transform == NULL || out == NULL ||
        !isfinite(user.x) || !isfinite(user.y) ||
        !isfinite(navigation->scale) || !(navigation->scale > 0.0)) return false;
    double device_x = transform->offset_x +
        (user.x - document->view_x) * transform->scale_x;
    double device_y = transform->offset_y +
        (user.y - document->view_y) * transform->scale_y;
    double x = (double)canvas.x + navigation->pan_x + device_x * navigation->scale;
    double y = (double)canvas.y + navigation->pan_y + device_y * navigation->scale;
    if (!isfinite(x) || !isfinite(y)) return false;
    *out = (geom_point){x, y};
    return true;
}

static bool screen_to_user(const svg_shape_document *document,
    const ui_viewer_navigation *navigation, ui_rect canvas,
    const guide_paint_transform *transform, double screen_x, double screen_y,
    geom_point *out)
{
    if (navigation == NULL || transform == NULL || out == NULL ||
        !isfinite(screen_x) || !isfinite(screen_y) ||
        !isfinite(navigation->scale) || !(navigation->scale > 0.0)) return false;
    double dx = (screen_x - canvas.x - navigation->pan_x) / navigation->scale;
    double dy = (screen_y - canvas.y - navigation->pan_y) / navigation->scale;
    double x = document->view_x + (dx - transform->offset_x) / transform->scale_x;
    double y = document->view_y + (dy - transform->offset_y) / transform->scale_y;
    if (!isfinite(x) || !isfinite(y) || fabs(x) > 1000000.0 || fabs(y) > 1000000.0)
        return false;
    *out = (geom_point){x, y};
    return true;
}

static void fill_rect(uint8_t *rgba, uint32_t width, uint32_t height,
    size_t stride, ui_rect rect, uint32_t color)
{
    if (rect.x > width || rect.y > height || rect.width > width - rect.x ||
        rect.height > height - rect.y) return;
    uint8_t red = (uint8_t)(color >> 24U), green = (uint8_t)(color >> 16U);
    uint8_t blue = (uint8_t)(color >> 8U);
    for (uint32_t y = rect.y; y < rect.y + rect.height; ++y) {
        uint8_t *row = rgba + (size_t)y * stride + (size_t)rect.x * 4U;
        for (uint32_t x = 0U; x < rect.width; ++x) {
            row[(size_t)x * 4U] = red;
            row[(size_t)x * 4U + 1U] = green;
            row[(size_t)x * 4U + 2U] = blue;
            row[(size_t)x * 4U + 3U] = 255U;
        }
    }
}

static void line_h(uint8_t *rgba, uint32_t width, uint32_t height, size_t stride,
    uint32_t x, uint32_t y, uint32_t length, uint32_t color)
{
    if (y >= height || x >= width) return;
    if (length > width - x) length = width - x;
    uint8_t red = (uint8_t)(color >> 24U), green = (uint8_t)(color >> 16U);
    uint8_t blue = (uint8_t)(color >> 8U);
    for (uint32_t offset = 0U; offset < length; ++offset) {
        uint8_t *pixel = rgba + (size_t)y * stride + ((size_t)x + offset) * 4U;
        pixel[0] = red; pixel[1] = green; pixel[2] = blue; pixel[3] = 255U;
    }
}

static void line_v(uint8_t *rgba, uint32_t width, uint32_t height, size_t stride,
    uint32_t x, uint32_t y, uint32_t length, uint32_t color)
{
    if (x >= width || y >= height) return;
    if (length > height - y) length = height - y;
    uint8_t red = (uint8_t)(color >> 24U), green = (uint8_t)(color >> 16U);
    uint8_t blue = (uint8_t)(color >> 8U);
    for (uint32_t offset = 0U; offset < length; ++offset) {
        uint8_t *pixel = rgba + ((size_t)y + offset) * stride + (size_t)x * 4U;
        pixel[0] = red; pixel[1] = green; pixel[2] = blue; pixel[3] = 255U;
    }
}

static void paint_guide_lines(viewer_editor *editor,
    const svg_shape_document *document, const ui_viewer_navigation *navigation,
    ui_rect canvas, const guide_paint_transform *transform, uint8_t *rgba,
    uint32_t width, uint32_t height, size_t stride, const ui_tokens *tokens)
{
    if (!editor->guides.guides_visible) return;
    const viewer_editor_guide_axis_kind axes[2] = {
        VIEWER_EDITOR_GUIDE_HORIZONTAL, VIEWER_EDITOR_GUIDE_VERTICAL};
    for (size_t axis_index = 0U; axis_index < 2U; ++axis_index) {
        viewer_editor_guide_axis_kind axis = axes[axis_index];
        size_t count = viewer_editor_guides_count(&editor->guides, axis);
        for (size_t index = 0U; index <= count; ++index) {
            bool preview = editor->guides.dragging && editor->guides.drag_value_valid &&
                editor->guides.drag_axis == axis &&
                (editor->guides.drag_creating || editor->guides.drag_index == index);
            if (index == count && !preview) break;
            if (!preview && editor->guides.dragging &&
                editor->guides.drag_axis == axis && !editor->guides.drag_creating &&
                editor->guides.drag_index == index) continue;
            double value = 0.0;
            if (preview) value = editor->guides.drag_value;
            else if (index >= count || !viewer_editor_guides_get(
                    &editor->guides, axis, index, &value)) continue;
            geom_point user = axis == VIEWER_EDITOR_GUIDE_HORIZONTAL ?
                (geom_point){document->view_x, value} :
                (geom_point){value, document->view_y};
            geom_point screen;
            if (!user_to_screen(document, navigation, canvas, transform, user, &screen))
                continue;
            bool selected = preview || (editor->guides.selected &&
                editor->guides.selected_axis == axis &&
                editor->guides.selected_index == index);
            uint32_t color = selected ? tokens->accent : tokens->guide;
            if (axis == VIEWER_EDITOR_GUIDE_HORIZONTAL) {
                if (screen.y < canvas.y || screen.y >= (double)canvas.y + canvas.height)
                    continue;
                uint32_t y = (uint32_t)llround(screen.y);
                line_h(rgba, width, height, stride, canvas.x, y, canvas.width, color);
            } else {
                if (screen.x < canvas.x || screen.x >= (double)canvas.x + canvas.width)
                    continue;
                uint32_t x = (uint32_t)llround(screen.x);
                line_v(rgba, width, height, stride, x, canvas.y, canvas.height, color);
            }
        }
    }
}

static bool draw_label(const char *label, size_t length, viewer_editor *editor,
    uint32_t color, uint8_t *rgba, uint32_t width, uint32_t height, size_t stride,
    ui_rect bounds, bool scalar_text)
{
    uint32_t font_px = (uint32_t)ceil(11.0 * editor->backing_scale);
    if (font_px == 0U) font_px = 1U;
    if (font_px > 256U) return false;
    if (scalar_text)
        return ui_scalar_text_draw(label, length, bounds, font_px, color,
            rgba, width, height, stride);
    return platform_text_ui_draw(label, length, UI_FONT_MONOSPACED, UI_FONT_REGULAR,
        false, editor->theme, font_px, color, rgba, width, height, stride, bounds);
}

static void paint_top_ticks(viewer_editor *editor,
    const svg_shape_document *document, const ui_viewer_navigation *navigation,
    ui_rect canvas, const guide_paint_transform *transform, ui_rect ruler,
    uint8_t *rgba, uint32_t width, uint32_t height, size_t stride,
    const ui_tokens *tokens, bool scalar_text)
{
    double ppu = transform->scale_x * navigation->scale;
    double step = 0.0;
    if (!viewer_editor_guides_tick_step(ppu, editor->backing_scale, 64.0, &step)) return;
    geom_point left, right;
    if (!screen_to_user(document, navigation, canvas, transform,
            (double)canvas.x, (double)canvas.y + canvas.height * 0.5, &left) ||
        !screen_to_user(document, navigation, canvas, transform,
            (double)canvas.x + canvas.width, (double)canvas.y + canvas.height * 0.5,
            &right)) return;
    double low = fmin(left.x, right.x), high = fmax(left.x, right.x);
    double first = floor(low / step) * step;
    for (size_t tick = 0U; tick < VIEWER_EDITOR_GUIDE_TICK_CAP; ++tick) {
        double value = first + (double)tick * step;
        if (!isfinite(value) || value > high + step) break;
        geom_point screen;
        if (!user_to_screen(document, navigation, canvas, transform,
                (geom_point){value, document->view_y}, &screen)) continue;
        if (screen.x < (double)ruler.x || screen.x >= (double)ruler.x + ruler.width)
            continue;
        uint32_t x = (uint32_t)llround(screen.x);
        uint32_t mark = (uint32_t)ceil(7.0 * editor->backing_scale);
        if (mark > ruler.height / 2U) mark = ruler.height / 2U;
        line_v(rgba, width, height, stride, x, ruler.y + ruler.height - mark,
            mark, tokens->text_tertiary);
        char label[64];
        int amount = snprintf(label, sizeof(label), "%.6g", value);
        if (amount > 0 && (size_t)amount < sizeof(label)) {
            uint32_t label_width = (uint32_t)ceil(56.0 * editor->backing_scale);
            if (label_width > ruler.x + ruler.width - x)
                label_width = ruler.x + ruler.width - x;
            uint32_t inset = (uint32_t)ceil(3.0 * editor->backing_scale);
            (void)draw_label(label, (size_t)amount, editor, tokens->text_secondary,
                rgba, width, height, stride,
                (ui_rect){x + inset, ruler.y, label_width > inset ? label_width - inset : 0U,
                    ruler.height > 2U ? ruler.height - 2U : ruler.height}, scalar_text);
        }
    }
}

static void paint_left_ticks(viewer_editor *editor,
    const svg_shape_document *document, const ui_viewer_navigation *navigation,
    ui_rect canvas, const guide_paint_transform *transform, ui_rect ruler,
    uint8_t *rgba, uint32_t width, uint32_t height, size_t stride,
    const ui_tokens *tokens, bool scalar_text)
{
    double ppu = transform->scale_y * navigation->scale;
    double step = 0.0;
    if (!viewer_editor_guides_tick_step(ppu, editor->backing_scale, 64.0, &step)) return;
    geom_point top, bottom;
    if (!screen_to_user(document, navigation, canvas, transform,
            (double)canvas.x + canvas.width * 0.5, (double)canvas.y, &top) ||
        !screen_to_user(document, navigation, canvas, transform,
            (double)canvas.x + canvas.width * 0.5,
            (double)canvas.y + canvas.height, &bottom)) return;
    double low = fmin(top.y, bottom.y), high = fmax(top.y, bottom.y);
    double first = floor(low / step) * step;
    for (size_t tick = 0U; tick < VIEWER_EDITOR_GUIDE_TICK_CAP; ++tick) {
        double value = first + (double)tick * step;
        if (!isfinite(value) || value > high + step) break;
        geom_point screen;
        if (!user_to_screen(document, navigation, canvas, transform,
                (geom_point){document->view_x, value}, &screen)) continue;
        if (screen.y < (double)ruler.y || screen.y >= (double)ruler.y + ruler.height)
            continue;
        uint32_t y = (uint32_t)llround(screen.y);
        uint32_t mark = (uint32_t)ceil(7.0 * editor->backing_scale);
        if (mark > ruler.width / 2U) mark = ruler.width / 2U;
        line_h(rgba, width, height, stride, ruler.x + ruler.width - mark, y,
            mark, tokens->text_tertiary);
        char label[64];
        int amount = snprintf(label, sizeof(label), "%.6g", value);
        if (amount > 0 && (size_t)amount < sizeof(label)) {
            uint32_t inset = (uint32_t)ceil(4.0 * editor->backing_scale);
            uint32_t label_height = (uint32_t)ceil(14.0 * editor->backing_scale);
            uint32_t y_top = y > label_height / 2U ? y - label_height / 2U : 0U;
            (void)draw_label(label, (size_t)amount, editor, tokens->text_secondary,
                rgba, width, height, stride,
                (ui_rect){ruler.x + inset, y_top,
                    ruler.width > inset + mark ? ruler.width - inset - mark : 0U,
                    label_height}, scalar_text);
        }
    }
}

static void paint_rulers(viewer_editor *editor,
    const svg_shape_document *document, const ui_viewer_navigation *navigation,
    ui_rect canvas, const guide_paint_transform *transform, uint8_t *rgba,
    uint32_t width, uint32_t height, size_t stride, const ui_tokens *tokens,
    bool scalar_text)
{
    if (!editor->guides.rulers_visible || !editor->modern_style) return;
    double scale = editor->backing_scale;
    if (!isfinite(scale) || scale < 0.5 || scale > 4.0) return;
    uint32_t inset = (uint32_t)ceil(36.0 * scale);
    if (canvas.x < inset || canvas.y < inset) return;
    ui_rect top = {canvas.x, canvas.y - inset, canvas.width, inset};
    ui_rect left = {canvas.x - inset, canvas.y, inset, canvas.height};
    ui_rect corner = {canvas.x - inset, canvas.y - inset, inset, inset};
    fill_rect(rgba, width, height, stride, top, tokens->panel_raised);
    fill_rect(rgba, width, height, stride, left, tokens->panel_raised);
    fill_rect(rgba, width, height, stride, corner, tokens->panel);
    if (top.y + top.height > 0U)
        line_h(rgba, width, height, stride, top.x, top.y + top.height - 1U,
            top.width, tokens->separator);
    if (left.x + left.width > 0U)
        line_v(rgba, width, height, stride, left.x + left.width - 1U, left.y,
            left.height, tokens->separator);
    paint_top_ticks(editor, document, navigation, canvas, transform, top,
        rgba, width, height, stride, tokens, scalar_text);
    paint_left_ticks(editor, document, navigation, canvas, transform, left,
        rgba, width, height, stride, tokens, scalar_text);
}

static void guides_paint(viewer_editor *editor,
    const svg_shape_document *document, const ui_viewer_navigation *navigation,
    ui_rect canvas, uint8_t *rgba, uint32_t width, uint32_t height, size_t stride,
    bool scalar_text)
{
    if (editor == NULL || document == NULL || navigation == NULL || rgba == NULL ||
        width == 0U || height == 0U || (size_t)width > SIZE_MAX / 4U ||
        stride < (size_t)width * 4U || (size_t)height > SIZE_MAX / stride ||
        !isfinite(navigation->scale) || !(navigation->scale > 0.0)) return;
    guide_paint_transform transform;
    if (!paint_transform_get(document, &transform)) return;
    ui_tokens tokens = ui_theme_tokens(editor->theme, editor->increased_contrast);
    paint_guide_lines(editor, document, navigation, canvas, &transform, rgba,
        width, height, stride, &tokens);
    paint_rulers(editor, document, navigation, canvas, &transform, rgba,
        width, height, stride, &tokens, scalar_text);
}

void viewer_editor_guides_paint(viewer_editor *editor,
    const svg_shape_document *document, const ui_viewer_navigation *navigation,
    ui_rect canvas, uint8_t *rgba, uint32_t width, uint32_t height, size_t stride)
{
    guides_paint(editor, document, navigation, canvas, rgba, width, height,
        stride, false);
}

void viewer_editor_guides_paint_scalar(viewer_editor *editor,
    const svg_shape_document *document, const ui_viewer_navigation *navigation,
    ui_rect canvas, uint8_t *rgba, uint32_t width, uint32_t height, size_t stride)
{
    guides_paint(editor, document, navigation, canvas, rgba, width, height,
        stride, true);
}
