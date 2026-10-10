#ifndef VEXEL_PLATFORM_TEXT_UI_H
#define VEXEL_PLATFORM_TEXT_UI_H

#include "ui/ui_core.h"

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

typedef void (*platform_text_ui_preferences_callback)(ui_theme appearance,
    bool increased_contrast, bool reduced_motion, void *context);

bool platform_text_ui_app_is_active(void);
bool platform_text_ui_get_preferences(ui_theme *out_appearance,
    bool *out_increased_contrast, bool *out_reduced_motion);
bool platform_text_ui_watch_preferences(platform_text_ui_preferences_callback callback,
                                        void *context);
void platform_text_ui_unwatch_preferences(void *context);

/* Draws CoreText-shaped SF or SF Mono glyph paths into caller-owned RGBA pixels. */
bool platform_text_ui_draw(const char *utf8, size_t length, bool monospaced,
    bool centered,
    ui_theme theme, uint32_t font_size_px, uint32_t color,
    uint8_t *rgba, uint32_t width, uint32_t height, size_t stride,
    ui_rect clip);
void platform_text_ui_cache_clear(void);

#endif
