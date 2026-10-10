#include "ui_widgets.h"

#define UI_WIDGET_MAX_CANVAS_AXIS 16384U

bool ui_widget_canvas_init(ui_widget_canvas *canvas, uint8_t *rgba,
    size_t capacity, uint32_t width, uint32_t height, size_t stride,
    uint32_t backing_scale, ui_theme theme, bool increased_contrast,
    ui_widget_text_draw_fn draw_text, void *text_context)
{
    if (canvas == NULL || rgba == NULL || width == 0U || height == 0U ||
        width > UI_WIDGET_MAX_CANVAS_AXIS || height > UI_WIDGET_MAX_CANVAS_AXIS ||
        backing_scale == 0U || backing_scale > 3U || theme < UI_THEME_DARK ||
        theme > UI_THEME_LIGHT ||
        (size_t)width > SIZE_MAX / 4U || stride < (size_t)width * 4U ||
        (size_t)height > SIZE_MAX / stride || capacity < (size_t)height * stride) return false;
    *canvas = (ui_widget_canvas){.rgba = rgba, .capacity = capacity,
        .stride = stride, .width = width, .height = height,
        .backing_scale = backing_scale, .theme = theme,
        .increased_contrast = increased_contrast,
        .tokens = ui_theme_tokens(theme, increased_contrast),
        .draw_text = draw_text, .text_context = text_context};
    return true;
}
