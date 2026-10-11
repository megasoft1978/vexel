#ifndef VEXEL_UI_DRAW_H
#define VEXEL_UI_DRAW_H

#include "ui_core.h"

/* Deterministic ASCII fallback used by headless galleries and legacy clients. */
bool ui_scalar_text_draw(const char *text, size_t length, ui_rect clip,
    uint32_t font_size_px, uint32_t color, uint8_t *rgba,
    uint32_t width, uint32_t height, size_t stride);

#endif
