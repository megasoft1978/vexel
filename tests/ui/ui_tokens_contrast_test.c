#include "ui_core.h"
#include "ui_icons.h"

#include <math.h>
#include <stdio.h>

static double linear_channel(uint32_t color, uint32_t shift)
{
    double channel = (double)((color >> shift) & 255U) / 255.0;
    return channel <= 0.04045 ? channel / 12.92 :
        pow((channel + 0.055) / 1.055, 2.4);
}

static double luminance(uint32_t color)
{
    return 0.2126 * linear_channel(color, 24U) +
        0.7152 * linear_channel(color, 16U) +
        0.0722 * linear_channel(color, 8U);
}

static double contrast(uint32_t first, uint32_t second)
{
    double a = luminance(first), b = luminance(second);
    double lighter = a > b ? a : b;
    double darker = a > b ? b : a;
    return (lighter + 0.05) / (darker + 0.05);
}

static uint32_t color(uint8_t red, uint8_t green, uint8_t blue)
{
    return ((uint32_t)red << 24U) | ((uint32_t)green << 16U) |
        ((uint32_t)blue << 8U) | 255U;
}

int main(void)
{
    ui_tokens light = ui_theme_tokens(UI_THEME_LIGHT, false);
    ui_tokens dark = ui_theme_tokens(UI_THEME_DARK, false);
    ui_tokens light_high = ui_theme_tokens(UI_THEME_LIGHT, true);
    ui_tokens dark_high = ui_theme_tokens(UI_THEME_DARK, true);
    double light_primary = contrast(light.text_primary, light.panel);
    double light_secondary = contrast(light.text_secondary, light.panel_raised);
    double dark_primary = contrast(dark.text_primary, dark.panel);
    double dark_secondary = contrast(dark.text_secondary, dark.panel_raised);
    double light_secondary_high = contrast(light_high.text_secondary,
        light_high.panel_raised);
    double dark_secondary_high = contrast(dark_high.text_secondary,
        dark_high.panel_raised);
    uint64_t dark_icon_1x = ui_icon_asset_hash(UI_THEME_DARK, 1U);
    uint64_t light_icon_1x = ui_icon_asset_hash(UI_THEME_LIGHT, 1U);
    uint64_t dark_icon_2x = ui_icon_asset_hash(UI_THEME_DARK, 2U);
    if (light.window != color(242U, 244U, 247U) ||
        light.panel_raised != color(237U, 240U, 244U) ||
        light.canvas_well != color(231U, 235U, 240U) ||
        dark.window != color(23U, 26U, 32U) ||
        dark.panel_raised != color(41U, 46U, 55U) ||
        dark.canvas_well != color(17U, 21U, 27U) ||
        light_primary < 14.0 || light_secondary < 5.5 ||
        dark_primary < 14.0 || dark_secondary < 5.5 ||
        contrast(light_high.text_secondary, light_high.panel_raised) <= light_secondary ||
        contrast(dark_high.text_secondary, dark_high.panel_raised) <= dark_secondary ||
        contrast(light_high.control_edge, light_high.panel) <=
            contrast(light.control_edge, light.panel) ||
        contrast(dark_high.control_edge, dark_high.panel) <=
            contrast(dark.control_edge, dark.panel) || dark_icon_1x == 0U ||
        light_icon_1x != dark_icon_1x || dark_icon_2x == 0U ||
        dark_icon_2x == dark_icon_1x) return 1;
    (void)printf("{\"light_primary_panel\":%.3f,\"light_secondary_raised\":%.3f,"
        "\"dark_primary_panel\":%.3f,\"dark_secondary_raised\":%.3f,"
        "\"light_secondary_raised_high_contrast\":%.3f,"
        "\"dark_secondary_raised_high_contrast\":%.3f,"
        "\"high_contrast\":true,\"dark_icon_hash_1x\":\"%016llx\","
        "\"light_icon_hash_1x\":\"%016llx\",\"dark_icon_hash_2x\":\"%016llx\"}\n",
        light_primary, light_secondary, dark_primary, dark_secondary,
        light_secondary_high, dark_secondary_high,
        (unsigned long long)dark_icon_1x, (unsigned long long)light_icon_1x,
        (unsigned long long)dark_icon_2x);
    return 0;
}
