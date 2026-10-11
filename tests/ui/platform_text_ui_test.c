#include "platform/platform_text_ui.h"

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define WIDTH 420U
#define HEIGHT 64U

static void fill(uint8_t *pixels, uint8_t red, uint8_t green, uint8_t blue)
{
    for (size_t at = 0U; at < (size_t)WIDTH * HEIGHT * 4U; at += 4U) {
        pixels[at] = red;
        pixels[at + 1U] = green;
        pixels[at + 2U] = blue;
        pixels[at + 3U] = 255U;
    }
}

int main(void)
{
    static const char text[] =
        "Résumé — 日本語 — Ελληνικά — SF Pro and SF Mono 0123456789 "
        "a label beyond the old 64-byte painter limit";
    const size_t byte_count = (size_t)WIDTH * HEIGHT * 4U;
    uint8_t *first = malloc(byte_count), *second = malloc(byte_count);
    if (first == NULL || second == NULL) { free(first); free(second); return 1; }
    fill(first, 32U, 36U, 43U);
    fill(second, 32U, 36U, 43U);
    ui_rect clip = {8U, 6U, WIDTH - 16U, HEIGHT - 12U};
    if (sizeof(text) - 1U <= 64U ||
        !platform_text_ui_draw(text, sizeof(text) - 1U, UI_FONT_SYSTEM,
            UI_FONT_REGULAR, false,
            UI_THEME_DARK, 18U, UINT32_C(0xf1f4f8ff), first, WIDTH, HEIGHT,
            (size_t)WIDTH * 4U, clip) ||
        !platform_text_ui_draw(text, sizeof(text) - 1U, UI_FONT_SYSTEM,
            UI_FONT_REGULAR, false,
            UI_THEME_DARK, 18U, UINT32_C(0xf1f4f8ff), second, WIDTH, HEIGHT,
            (size_t)WIDTH * 4U, clip) || memcmp(first, second, byte_count) != 0) {
        free(first); free(second); return 2;
    }
    size_t changed = 0U;
    for (size_t at = 0U; at < byte_count; at += 4U)
        if (first[at] != 32U || first[at + 1U] != 36U || first[at + 2U] != 43U) ++changed;
    if (changed == 0U) { free(first); free(second); return 3; }
    char long_text[20001];
    memset(long_text, 'W', sizeof(long_text) - 1U);
    long_text[sizeof(long_text) - 1U] = '\0';
    fill(first, 32U, 36U, 43U);
    if (!platform_text_ui_draw(long_text, sizeof(long_text) - 1U,
            UI_FONT_SYSTEM, UI_FONT_SEMIBOLD, false,
            UI_THEME_DARK, 18U, UINT32_C(0xf1f4f8ff), first,
            WIDTH, HEIGHT, (size_t)WIDTH * 4U, clip)) {
        free(first); free(second); return 4;
    }
    size_t long_changed = 0U;
    for (size_t at = 0U; at < byte_count; at += 4U)
        if (first[at] != 32U || first[at + 1U] != 36U || first[at + 2U] != 43U)
            ++long_changed;
    if (long_changed == 0U) { free(first); free(second); return 5; }
    platform_text_ui_cache_clear();
    free(first); free(second);
    (void)printf("CoreText UI render passed: %zu mixed UTF-8 bytes, %zu changed pixels; "
        "%zu-byte label clipped to its visible width\n",
        sizeof(text) - 1U, changed, sizeof(long_text) - 1U);
    return 0;
}
