#ifndef VEXEL_UI_WIDGET_PAINT_INTERNAL_H
#define VEXEL_UI_WIDGET_PAINT_INTERNAL_H

#include "ui_widgets.h"

void ui_widget_paint_round(const ui_widget_canvas *canvas, ui_rect bounds,
    uint32_t radius_points, uint32_t fill, uint32_t edge, uint32_t edge_points);
void ui_widget_paint_rect(const ui_widget_canvas *canvas, ui_rect bounds, uint32_t color);
uint32_t ui_widget_paint_state_foreground(const ui_widget_canvas *canvas,
    ui_control_state state);
bool ui_widget_paint_text(const ui_widget_canvas *canvas, const char *text,
    size_t length, ui_rect bounds, uint32_t color, ui_font_family family,
    ui_font_weight weight, uint32_t font_points);
bool ui_widget_paint_icon(const ui_widget_canvas *canvas, ui_icon_kind icon,
    ui_rect bounds, ui_control_state state);
bool ui_widget_paint_icon_color(const ui_widget_canvas *canvas, ui_icon_kind icon,
    ui_rect bounds, uint32_t color);
bool ui_widget_paint_tree_row(const ui_widget_canvas *canvas,
    const ui_widget_draw_spec *spec);
bool ui_widget_paint_empty_state(const ui_widget_canvas *canvas,
    const ui_widget_draw_spec *spec);

#endif
