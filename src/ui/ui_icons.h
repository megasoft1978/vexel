#ifndef VEXEL_UI_ICONS_H
#define VEXEL_UI_ICONS_H

#include "ui_core.h"

/* Draws one cached 24 pt view-box icon centered in bounds. */
bool ui_icon_paint_into(ui_icon_kind icon, ui_theme theme, double backing_scale,
                        ui_rect bounds, uint32_t color, uint8_t *rgba,
                        uint32_t width, uint32_t height, size_t stride);
uint64_t ui_icon_asset_hash(ui_theme theme, uint32_t integer_scale);

#endif
