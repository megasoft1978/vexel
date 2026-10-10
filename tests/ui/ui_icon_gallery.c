#include "ui_core.h"
#include "ui_draw.h"
#include "ui_icons.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define ICON_COLUMNS 5U
#define ICON_CELL_WIDTH 90U
#define ICON_CELL_HEIGHT 58U
#define ICON_MARGIN 6U

static bool fill_rect(uint8_t *rgba, uint32_t width, uint32_t height,
    size_t stride, ui_rect rect, uint32_t color)
{
    if (rect.x > width || rect.y > height || rect.width > width - rect.x ||
        rect.height > height - rect.y) return false;
    uint8_t red = (uint8_t)(color >> 24U), green = (uint8_t)(color >> 16U);
    uint8_t blue = (uint8_t)(color >> 8U), alpha = (uint8_t)color;
    for (uint32_t y = rect.y; y < rect.y + rect.height; ++y) {
        uint8_t *row = rgba + (size_t)y * stride;
        for (uint32_t x = rect.x; x < rect.x + rect.width; ++x) {
            row[(size_t)x * 4U] = red; row[(size_t)x * 4U + 1U] = green;
            row[(size_t)x * 4U + 2U] = blue; row[(size_t)x * 4U + 3U] = alpha;
        }
    }
    return true;
}

int main(int argc, char **argv)
{
    if (argc != 3) {
        (void)fprintf(stderr, "usage: ui_icon_gallery <light|dark> <scale 1..2>\n");
        return 2;
    }
    ui_theme theme = strcmp(argv[1], "light") == 0 ? UI_THEME_LIGHT :
        strcmp(argv[1], "dark") == 0 ? UI_THEME_DARK : (ui_theme)99;
    if (theme > UI_THEME_LIGHT || (strcmp(argv[2], "1") != 0 && strcmp(argv[2], "2") != 0) ||
        !ui_icon_validate_all()) return 2;
    uint32_t scale = (uint32_t)(argv[2][0] - '0');
    uint32_t columns = ICON_COLUMNS;
    uint32_t rows = (uint32_t)((ui_icon_path_count() + columns - 1U) / columns);
    uint32_t logical_width = columns * ICON_CELL_WIDTH + ICON_MARGIN * 2U;
    uint32_t logical_height = rows * ICON_CELL_HEIGHT + ICON_MARGIN * 2U;
    uint32_t width = logical_width * scale, height = logical_height * scale;
    size_t stride = (size_t)width * 4U, capacity = stride * height;
    uint8_t *rgba = calloc(capacity, 1U);
    if (rgba == NULL) return 3;
    ui_tokens tokens = ui_theme_tokens(theme, false);
    if (!fill_rect(rgba, width, height, stride, (ui_rect){0U, 0U, width, height}, tokens.window)) {
        free(rgba); return 4;
    }
    for (size_t index = 0U; index < ui_icon_path_count(); ++index) {
        const char *name = NULL, *path = NULL;
        size_t path_length = 0U;
        if (!ui_icon_path_at(index, &name, &path, &path_length)) { free(rgba); return 5; }
        (void)path; (void)path_length;
        uint32_t col = (uint32_t)(index % columns), row = (uint32_t)(index / columns);
        uint32_t x = (ICON_MARGIN + col * ICON_CELL_WIDTH) * scale;
        uint32_t y = (ICON_MARGIN + row * ICON_CELL_HEIGHT) * scale;
        ui_rect card = {x, y, ICON_CELL_WIDTH * scale, ICON_CELL_HEIGHT * scale};
        (void)fill_rect(rgba, width, height, stride, card, tokens.panel);
        uint32_t icon_size = 24U * scale;
        ui_rect bounds = {x + (ICON_CELL_WIDTH * scale - icon_size) / 2U,
            y + 3U * scale, icon_size, icon_size};
        if (!ui_icon_paint_state_into((ui_icon_kind)(index + 1U), theme, false,
                (double)scale, bounds, UI_CONTROL_REST, &tokens,
                rgba, width, height, stride)) { free(rgba); return 6; }
        ui_rect label = {x + 5U * scale, y + 33U * scale,
            (ICON_CELL_WIDTH - 10U) * scale, 18U * scale};
        if (!ui_scalar_text_draw(name, strlen(name), label, 10U * scale,
                tokens.text_secondary, rgba, width, height, stride)) { free(rgba); return 7; }
        uint32_t right = x + card.width - 1U, bottom = y + card.height - 1U;
        for (uint32_t xx = x; xx < x + card.width; ++xx) {
            size_t top_offset = (size_t)y * stride + (size_t)xx * 4U;
            size_t bottom_offset = (size_t)bottom * stride + (size_t)xx * 4U;
            rgba[top_offset] = rgba[bottom_offset] = (uint8_t)(tokens.separator >> 24U);
            rgba[top_offset + 1U] = rgba[bottom_offset + 1U] = (uint8_t)(tokens.separator >> 16U);
            rgba[top_offset + 2U] = rgba[bottom_offset + 2U] = (uint8_t)(tokens.separator >> 8U);
            rgba[top_offset + 3U] = rgba[bottom_offset + 3U] = (uint8_t)tokens.separator;
        }
        for (uint32_t yy = y; yy < y + card.height; ++yy) {
            size_t left_offset = (size_t)yy * stride + (size_t)x * 4U;
            size_t right_offset = (size_t)yy * stride + (size_t)right * 4U;
            rgba[left_offset] = rgba[right_offset] = (uint8_t)(tokens.separator >> 24U);
            rgba[left_offset + 1U] = rgba[right_offset + 1U] = (uint8_t)(tokens.separator >> 16U);
            rgba[left_offset + 2U] = rgba[right_offset + 2U] = (uint8_t)(tokens.separator >> 8U);
            rgba[left_offset + 3U] = rgba[right_offset + 3U] = (uint8_t)tokens.separator;
        }
    }
    size_t written = fwrite(rgba, 1U, capacity, stdout);
    free(rgba);
    return written == capacity && fflush(stdout) == 0 ? 0 : 8;
}
