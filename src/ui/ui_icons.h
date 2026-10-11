#ifndef VEXEL_UI_ICONS_H
#define VEXEL_UI_ICONS_H

#include "ui_core.h"

typedef struct ui_icon_bounds {
    double left, top, right, bottom;
} ui_icon_bounds;

size_t ui_icon_path_count(void);
bool ui_icon_path_at(size_t index, const char **out_name,
                     const char **out_path, size_t *out_path_length);
bool ui_icon_path_validate(const char *path, size_t path_length,
                           ui_icon_bounds *out_bounds);
bool ui_icon_validate_all(void);
/* Draws one cached 24 pt view-box icon centered in bounds. */
bool ui_icon_paint_into(ui_icon_kind icon, ui_theme theme, double backing_scale,
                        ui_rect bounds, uint32_t color, uint8_t *rgba,
                        uint32_t width, uint32_t height, size_t stride);
/* Frozen painter for existing editor goldens until chrome adopts the path kit. */
bool ui_icon_paint_legacy_into(ui_icon_kind icon, ui_theme theme,
    double backing_scale, ui_rect bounds, uint32_t color, uint8_t *rgba,
    uint32_t width, uint32_t height, size_t stride);
bool ui_icon_paint_state_into(ui_icon_kind icon, ui_theme theme,
    bool increased_contrast, double backing_scale, ui_rect bounds,
    ui_control_state state, const ui_tokens *tokens, uint8_t *rgba,
    uint32_t width, uint32_t height, size_t stride);
uint64_t ui_icon_asset_hash(ui_theme theme, uint32_t integer_scale);
void ui_icon_cache_clear(void);

#endif
