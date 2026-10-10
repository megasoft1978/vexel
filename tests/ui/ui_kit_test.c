#include "ui_core.h"
#include "ui_icons.h"
#include "ui_widgets.h"

#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static double linear_channel(uint32_t color, uint32_t shift)
{
    double value = (double)((color >> shift) & 255U) / 255.0;
    return value <= 0.04045 ? value / 12.92 : pow((value + 0.055) / 1.055, 2.4);
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
    return (fmax(a, b) + 0.05) / (fmin(a, b) + 0.05);
}

static uint32_t color(uint8_t red, uint8_t green, uint8_t blue)
{
    return ((uint32_t)red << 24U) | ((uint32_t)green << 16U) |
        ((uint32_t)blue << 8U) | 255U;
}

static void print_contrast_values(const char *name, uint32_t foreground,
    const uint32_t *surfaces, const char *const *surface_names, size_t count)
{
    (void)printf("\"%s\":{", name);
    for (size_t index = 0U; index < count; ++index) {
        if (index > 0U) (void)putchar(',');
        (void)printf("\"%s\":%.3f", surface_names[index],
            contrast(foreground, surfaces[index]));
    }
    (void)putchar('}');
}

static void print_contrast_profile(const char *theme_name, ui_theme theme,
    bool increased_contrast)
{
    static const char *const surface_names[] = {
        "panel", "panel_raised", "panel_hover", "selection_fill", "canvas_well"
    };
    ui_tokens tokens = ui_theme_tokens(theme, increased_contrast);
    uint32_t text_surfaces[] = {tokens.panel, tokens.panel_raised, tokens.panel_hover,
        tokens.selection_fill, tokens.canvas_well};
    (void)printf("{\"theme\":\"%s\",\"increased_contrast\":%s,\"text\":{",
        theme_name, increased_contrast ? "true" : "false");
    print_contrast_values("primary", tokens.text_primary, text_surfaces,
        surface_names, sizeof(surface_names) / sizeof(surface_names[0]));
    (void)putchar(',');
    print_contrast_values("secondary", tokens.text_secondary, text_surfaces,
        surface_names, sizeof(surface_names) / sizeof(surface_names[0]));
    (void)printf("},\"controls\":{\"edge_panel\":%.3f,\"edge_panel_raised\":%.3f,"
        "\"edge_panel_hover\":%.3f,\"selection_edge_fill\":%.3f,"
        "\"focus_ring_panel\":%.3f},\"semantic\":{\"success_panel\":%.3f,"
        "\"warning_panel\":%.3f,\"error_panel\":%.3f}}",
        contrast(tokens.control_edge, tokens.panel),
        contrast(tokens.control_edge, tokens.panel_raised),
        contrast(tokens.control_edge, tokens.panel_hover),
        contrast(tokens.selection_edge, tokens.selection_fill),
        contrast(tokens.focus_ring, tokens.panel), contrast(tokens.success, tokens.panel),
        contrast(tokens.warning, tokens.panel), contrast(tokens.error, tokens.panel));
}

static int print_contrast_report(void)
{
    (void)printf("{\"version\":1,\"method\":\"WCAG 2.1 relative luminance, sRGB\","
        "\"minimums\":{\"text\":4.5,\"controls\":3.0},\"profiles\":[");
    print_contrast_profile("light", UI_THEME_LIGHT, false);
    (void)putchar(',');
    print_contrast_profile("light", UI_THEME_LIGHT, true);
    (void)putchar(',');
    print_contrast_profile("dark", UI_THEME_DARK, false);
    (void)putchar(',');
    print_contrast_profile("dark", UI_THEME_DARK, true);
    (void)puts("]}");
    return 0;
}

static int test_tokens(bool quiet)
{
    const uint32_t backgrounds[] = {
        0U /* initialized below */, 0U, 0U, 0U, 0U
    };
    (void)backgrounds;
    for (uint32_t theme_index = 0U; theme_index < 2U; ++theme_index) {
        ui_theme theme = (ui_theme)theme_index;
        for (unsigned contrast_index = 0U; contrast_index < 2U; ++contrast_index) {
            ui_tokens tokens = ui_theme_tokens(theme, contrast_index != 0U);
            uint32_t text_surfaces[] = {tokens.panel, tokens.panel_raised,
                tokens.panel_hover, tokens.selection_fill, tokens.canvas_well};
            for (size_t index = 0U; index < sizeof(text_surfaces) / sizeof(text_surfaces[0]); ++index) {
                if (contrast(tokens.text_primary, text_surfaces[index]) < 4.5 ||
                    contrast(tokens.text_secondary, text_surfaces[index]) < 4.5) {
                    (void)fprintf(stderr, "text contrast failed: theme=%u high=%u surface=%zu\n",
                        theme_index, contrast_index, index);
                    return 1;
                }
            }
            uint32_t edge_surfaces[] = {tokens.panel, tokens.panel_raised, tokens.panel_hover};
            for (size_t index = 0U; index < sizeof(edge_surfaces) / sizeof(edge_surfaces[0]); ++index)
                if (contrast(tokens.control_edge, edge_surfaces[index]) < 3.0) return 2;
            if (contrast(tokens.selection_edge, tokens.selection_fill) < 3.0 ||
                contrast(tokens.focus_ring, tokens.panel) < 3.0 ||
                contrast(tokens.error, tokens.panel) < 4.5 ||
                contrast(tokens.success, tokens.panel) < 4.5 ||
                contrast(tokens.warning, tokens.panel) < 4.5) return 3;
        }
    }
    ui_tokens light = ui_theme_tokens(UI_THEME_LIGHT, false);
    ui_tokens dark = ui_theme_tokens(UI_THEME_DARK, false);
    ui_tokens light_high = ui_theme_tokens(UI_THEME_LIGHT, true);
    ui_tokens dark_high = ui_theme_tokens(UI_THEME_DARK, true);
    if (light.window != color(242U, 244U, 247U) || light.panel != color(255U, 255U, 255U) ||
        light.panel_raised != color(237U, 240U, 244U) ||
        light.panel_hover != color(245U, 247U, 250U) ||
        light.canvas_well != color(231U, 235U, 240U) ||
        light.text_primary != color(31U, 39U, 50U) ||
        light.text_secondary != color(82U, 97U, 113U) ||
        light.text_tertiary != color(116U, 128U, 146U) ||
        light.disabled_text != color(137U, 147U, 162U) ||
        light.selection_fill != color(220U, 234U, 255U) ||
        light.checker_a != color(224U, 228U, 234U) ||
        dark.window != color(23U, 26U, 32U) || dark.panel != color(32U, 36U, 43U) ||
        dark.panel_raised != color(41U, 46U, 55U) ||
        dark.panel_hover != color(48U, 54U, 64U) ||
        dark.canvas_well != color(17U, 21U, 27U) ||
        dark.text_secondary != color(185U, 194U, 207U) ||
        dark.text_tertiary != color(144U, 153U, 166U) ||
        dark.disabled_text != color(124U, 134U, 148U) ||
        dark.selection_fill != color(38U, 60U, 88U) ||
        light_high.text_secondary == light.text_secondary ||
        dark_high.control_edge == dark.control_edge) return 4;
    ui_type_scale type = ui_theme_type_scale();
    ui_metrics metrics = ui_design_metrics();
    if (type.document_title_pt != 14U || type.panel_heading_pt != 13U ||
        type.section_heading_pt != 12U || type.body_pt != 13U ||
        type.secondary_pt != 12U || type.metadata_pt != 11U ||
        type.geometry_pt != 12U || type.empty_heading_pt != 22U ||
        type.geometry_family != UI_FONT_MONOSPACED || type.title_weight != UI_FONT_SEMIBOLD ||
        metrics.toolbar_height != 40U || metrics.context_height != 32U ||
        metrics.tool_rail_width != 44U || metrics.structure_width != 232U ||
        metrics.inspector_width != 320U || metrics.status_height != 24U ||
        metrics.tree_row_height != 28U || metrics.section_header_height != 24U ||
        metrics.field_height != 30U || metrics.icon_button_target != 32U ||
        metrics.icon_button_glyph != 22U || metrics.control_radius != 6U) return 5;
    if (!quiet)
        (void)printf("token contrast matrix passed: light/dark x normal/increased; text >= 4.5, controls >= 3\n");
    return 0;
}

static int test_icons(void)
{
    if (ui_icon_path_count() != 45U || !ui_icon_validate_all()) return 10;
    for (size_t index = 0U; index < 45U; ++index) {
        const char *name = NULL, *path = NULL;
        size_t length = 0U;
        ui_icon_bounds bounds = {0};
        if (!ui_icon_path_at(index, &name, &path, &length) || name == NULL || path == NULL ||
            !ui_icon_path_validate(path, length, &bounds) || bounds.left < 0.0 ||
            bounds.top < 0.0 || bounds.right > 24.0 || bounds.bottom > 24.0) return 11;
        for (size_t character = 0U; character < length; ++character) {
            char c = path[character];
            if (!((c >= 'A' && c <= 'Z') || (c >= 'a' && c <= 'z') ||
                    (c >= '0' && c <= '9') || c == '+' || c == '-' || c == '.' ||
                    c == ',' || c == ' ')) return 12;
            if ((c >= 'A' && c <= 'Z') && c != 'M' && c != 'L' && c != 'C' &&
                c != 'Q' && c != 'A' && c != 'Z') return 13;
            if (c >= 'a' && c <= 'z') return 14;
        }
        for (uint32_t scale = 1U; scale <= 3U; ++scale) {
            uint8_t *rgba = calloc(72U * 72U, 4U);
            if (rgba == NULL) return 15;
            ui_icon_kind icon = (ui_icon_kind)(index + 1U);
            bool drawn = ui_icon_paint_into(icon, UI_THEME_LIGHT, (double)scale,
                (ui_rect){0U, 0U, 24U * scale, 24U * scale},
                UINT32_C(0x1f2732ff), rgba, 72U, 72U, 72U * 4U);
            size_t visible = 0U;
            for (size_t byte = 3U; byte < 72U * 72U * 4U; byte += 4U)
                if (rgba[byte] != 0U) visible++;
            free(rgba);
            if (!drawn || visible == 0U) return 16;
        }
    }
    static const char arc_path[] = "M0 12 A12 12 0 0 1 24 12";
    static const char bad_arc_flags[] = "M0 12 A12 12 0 2 1 24 12";
    static const char outside_arc[] = "M1 1 A24 24 0 1 1 2 1";
    if (ui_icon_path_validate("M0 0 L30 30", 11U, NULL) ||
        ui_icon_path_validate("M0 0 H10", 8U, NULL) ||
        ui_icon_path_validate("M0 0 m1 1", 9U, NULL) ||
        !ui_icon_path_validate(arc_path, sizeof(arc_path) - 1U, NULL) ||
        ui_icon_path_validate(bad_arc_flags, sizeof(bad_arc_flags) - 1U, NULL) ||
        ui_icon_path_validate(outside_arc, sizeof(outside_arc) - 1U, NULL) ||
        !ui_icon_path_validate("M0 0 C1 1 2 2 3 3", 17U, NULL)) return 17;
    return 0;
}

static int test_emphasized_icon_masks(void)
{
    static const ui_icon_kind icons[] = {UI_ICON_PEN, UI_ICON_NODE_EDIT,
        UI_ICON_STROKE, UI_ICON_UNGROUP, UI_ICON_MORE};
    uint64_t hashes[sizeof(icons) / sizeof(icons[0])] = {0};
    for (size_t index = 0U; index < sizeof(icons) / sizeof(icons[0]); ++index) {
        uint8_t rgba[48U * 48U * 4U] = {0};
        if (!ui_icon_paint_into(icons[index], UI_THEME_LIGHT, 2.0,
                (ui_rect){0U, 0U, 48U, 48U}, UINT32_C(0x1f2732ff),
                rgba, 48U, 48U, 48U * 4U)) return 18;
        uint64_t hash = UINT64_C(1469598103934665603);
        size_t painted = 0U;
        for (size_t at = 3U; at < sizeof(rgba); at += 4U) {
            if (rgba[at] == 0U) continue;
            ++painted;
            hash = (hash ^ (uint64_t)(at / 4U)) * UINT64_C(1099511628211);
        }
        if (painted < 16U) return 19;
        hashes[index] = hash;
        for (size_t prior = 0U; prior < index; ++prior)
            if (hashes[prior] == hashes[index]) return 20;
    }
    return 0;
}

static int test_interactions(void)
{
    ui_widget_pointer_state pointer = {.enabled = true};
    if (ui_widget_pointer_visual_state(&pointer) != UI_CONTROL_REST ||
        ui_widget_pointer_transition(&pointer, UI_WIDGET_POINTER_ENTER) ||
        ui_widget_pointer_visual_state(&pointer) != UI_CONTROL_HOVER ||
        ui_widget_pointer_transition(&pointer, UI_WIDGET_POINTER_DOWN) ||
        ui_widget_pointer_visual_state(&pointer) != UI_CONTROL_PRESSED ||
        !ui_widget_pointer_transition(&pointer, UI_WIDGET_POINTER_UP_INSIDE)) return 20;
    pointer.enabled = false;
    if (ui_widget_pointer_transition(&pointer, UI_WIDGET_POINTER_ENTER) ||
        ui_widget_pointer_visual_state(&pointer) != UI_CONTROL_DISABLED) return 21;

    ui_widget_text_edit text = {0};
    if (!ui_widget_text_edit_init(&text, "Layer name", 10U, "box", 3U) ||
        !ui_widget_text_edit_begin(&text) || !ui_widget_text_edit_pointer(&text, 0U, false) ||
        ui_widget_text_edit_key(&text, 0x1f642U, 0U) != UI_WIDGET_INPUT_CHANGED ||
        text.length != 7U || !ui_widget_text_edit_commit(&text)) return 22;
    if (!ui_widget_text_edit_begin(&text) ||
        ui_widget_text_edit_key(&text, 'x', 0U) != UI_WIDGET_INPUT_CHANGED ||
        ui_widget_text_edit_key(&text, UI_KEY_ESCAPE, 0U) != UI_WIDGET_INPUT_CANCELLED ||
        text.length != 7U) return 23;
    char full[UI_WIDGET_EDIT_MAX_BYTES];
    memset(full, 'a', sizeof(full));
    if (!ui_widget_text_edit_init(&text, "Full value", 10U, full, sizeof(full)) ||
        !ui_widget_text_edit_begin(&text) ||
        ui_widget_text_edit_key(&text, 'b', 0U) != UI_WIDGET_INPUT_INVALID ||
        text.length != UI_WIDGET_EDIT_MAX_BYTES) return 24;

    double parsed = 0.0;
    if (!ui_widget_number_parse(" -12.5e+2 ", 10U, &parsed) || parsed != -1250.0 ||
        ui_widget_number_parse("1px", 3U, &parsed) ||
        ui_widget_number_parse("1e309", 5U, &parsed) ||
        ui_widget_number_parse("nan", 3U, &parsed)) return 25;
    ui_widget_number_field number = {0};
    if (!ui_widget_number_field_init(&number, "Width", 5U, 2.0, 0.0, 20.0,
            1.0, "px", 2U) ||
        ui_widget_number_field_key(&number, UI_KEY_UP, 0U) != UI_WIDGET_INPUT_CHANGED ||
        ui_widget_number_field_key(&number, UI_KEY_UP, UI_MOD_OPTION) != UI_WIDGET_INPUT_CHANGED ||
        ui_widget_number_field_key(&number, UI_KEY_UP, UI_MOD_SHIFT) != UI_WIDGET_INPUT_CHANGED ||
        fabs(number.value - 13.1) > 1e-9 ||
        ui_widget_number_field_key(&number, UI_KEY_ESCAPE, 0U) != UI_WIDGET_INPUT_CANCELLED ||
        number.value != 2.0 || !ui_widget_number_field_set_draft(&number, "5.5", 3U) ||
        ui_widget_number_field_key(&number, UI_KEY_RETURN, 0U) != UI_WIDGET_INPUT_COMMITTED ||
        number.value != 5.5 || !ui_widget_number_field_set_draft(&number, "1e", 2U) ||
        ui_widget_number_field_key(&number, UI_KEY_RETURN, 0U) != UI_WIDGET_INPUT_INVALID ||
        ui_widget_number_field_key(&number, UI_KEY_ESCAPE, 0U) != UI_WIDGET_INPUT_CANCELLED ||
        number.value != 5.5) return 26;
    if (ui_widget_number_field_pointer(&number, UI_WIDGET_POINTER_DOWN, 100, 0U) !=
            UI_WIDGET_INPUT_CHANGED ||
        ui_widget_number_field_pointer(&number, UI_WIDGET_POINTER_MOVE, 108, 0U) !=
            UI_WIDGET_INPUT_CHANGED || fabs(number.value - 7.5) > 1e-9 ||
        ui_widget_number_field_pointer(&number, UI_WIDGET_POINTER_UP_INSIDE, 108, 0U) !=
            UI_WIDGET_INPUT_COMMITTED) return 27;

    uint32_t rgba = 0U;
    if (!ui_widget_hex_parse("#4F7", 4U, &rgba) || rgba != UINT32_C(0x44ff77ff) ||
        !ui_widget_hex_parse("#4f7cff80", 9U, &rgba) || rgba != UINT32_C(0x4f7cff80) ||
        ui_widget_hex_parse("#xyz", 4U, &rgba)) return 28;
    char hex[10]; size_t hex_length = 0U;
    if (!ui_widget_hex_format(rgba, hex, sizeof(hex), &hex_length) ||
        hex_length != 9U || memcmp(hex, "#4F7CFF80", 9U) != 0 ||
        !ui_widget_hex_parse(hex, hex_length, &rgba) || rgba != UINT32_C(0x4f7cff80)) return 29;
    ui_widget_color_field color_field = {0};
    if (!ui_widget_color_field_init(&color_field, "Fill", 4U,
            UINT32_C(0x4f7cffff)) ||
        !ui_widget_color_field_set_draft(&color_field, "#12", 3U) ||
        ui_widget_color_field_key(&color_field, UI_KEY_RETURN, 0U) != UI_WIDGET_INPUT_INVALID ||
        ui_widget_color_field_key(&color_field, UI_KEY_ESCAPE, 0U) != UI_WIDGET_INPUT_CANCELLED ||
        color_field.rgba != UINT32_C(0x4f7cffff) ||
        !ui_widget_color_field_set_draft(&color_field, "#00FF80", 7U) ||
        ui_widget_color_field_key(&color_field, UI_KEY_RETURN, 0U) != UI_WIDGET_INPUT_COMMITTED ||
        color_field.rgba != UINT32_C(0x00ff80ff)) return 30;

    ui_widget_slider slider = {0};
    if (!ui_widget_slider_init(&slider, "Opacity", 7U, 0.5, 0.0, 1.0, 0.1) ||
        ui_widget_slider_key(&slider, UI_KEY_RIGHT, UI_MOD_SHIFT) != UI_WIDGET_INPUT_CHANGED ||
        slider.value != 1.0 || !ui_widget_slider_pointer(&slider, UI_WIDGET_POINTER_DOWN,
            51, 0, 100) || fabs(slider.value - 0.5) > 1e-9 ||
        !ui_widget_slider_pointer(&slider, UI_WIDGET_POINTER_UP_INSIDE, 51, 0, 100)) return 31;
    ui_widget_option options[] = {{"Normal", 6U}, {"Multiply", 8U}, {"Screen", 6U}};
    ui_widget_dropdown dropdown = {0};
    if (!ui_widget_dropdown_init(&dropdown, "Blend mode", 10U, options, 3U, 0U) ||
        ui_widget_dropdown_key(&dropdown, UI_KEY_DOWN, 0U) != UI_WIDGET_INPUT_CHANGED ||
        ui_widget_dropdown_key(&dropdown, UI_KEY_DOWN, 0U) != UI_WIDGET_INPUT_CHANGED ||
        dropdown.active_index != 1U ||
        ui_widget_dropdown_key(&dropdown, UI_KEY_RETURN, 0U) != UI_WIDGET_INPUT_COMMITTED ||
        dropdown.selected_index != 1U ||
        ui_widget_dropdown_key(&dropdown, UI_KEY_SPACE, 0U) != UI_WIDGET_INPUT_CHANGED ||
        ui_widget_dropdown_key(&dropdown, UI_KEY_ESCAPE, 0U) != UI_WIDGET_INPUT_CANCELLED ||
        dropdown.selected_index != 1U) return 32;
    return 0;
}

typedef struct text_capture {
    char labels[3][24];
    size_t count;
    ui_font_family family[3];
    ui_font_weight weight[3];
    uint32_t size[3];
} text_capture;

static bool capture_text(void *context, const char *text, size_t length,
    ui_rect bounds, ui_theme theme, uint32_t font_size, uint32_t color_value,
    ui_font_family family, ui_font_weight weight, uint8_t *pixels,
    uint32_t width, uint32_t height, size_t stride)
{
    (void)bounds; (void)theme; (void)color_value; (void)pixels;
    (void)width; (void)height; (void)stride;
    text_capture *probe = context;
    if (probe->count >= 3U || length >= sizeof(probe->labels[0])) return false;
    memcpy(probe->labels[probe->count], text, length);
    probe->labels[probe->count][length] = '\0';
    probe->family[probe->count] = family;
    probe->weight[probe->count] = weight;
    probe->size[probe->count] = font_size;
    ++probe->count;
    return true;
}

static int test_paint_bounds(void)
{
    const uint32_t small_width = 64U, small_height = 64U;
    const uint32_t large_width = 2048U, large_height = 1024U;
    size_t small_bytes = (size_t)small_width * small_height * 4U;
    size_t large_bytes = (size_t)large_width * large_height * 4U;
    uint8_t *small = calloc(small_bytes, 1U);
    uint8_t *large = calloc(large_bytes, 1U);
    if (small == NULL || large == NULL) { free(small); free(large); return 40; }
    ui_widget_canvas small_canvas, large_canvas;
    if (!ui_widget_canvas_init(&small_canvas, small, small_bytes, small_width, small_height,
            small_width * 4U, 1U, UI_THEME_LIGHT, false, NULL, NULL) ||
        !ui_widget_canvas_init(&large_canvas, large, large_bytes, large_width, large_height,
            large_width * 4U, 1U, UI_THEME_LIGHT, false, NULL, NULL)) {
        free(small); free(large); return 41;
    }
    ui_widget_draw_spec spec = {.kind = UI_WIDGET_ICON_BUTTON, .state = UI_CONTROL_FOCUS,
        .bounds = {12U, 12U, 28U, 28U}, .icon = UI_ICON_SELECT};
    if (!ui_widget_paint(&small_canvas, &spec) || !ui_widget_paint(&large_canvas, &spec)) {
        free(small); free(large); return 42;
    }
    for (uint32_t y = 0U; y < small_height; ++y) {
        if (memcmp(small + (size_t)y * small_width * 4U,
                large + (size_t)y * large_width * 4U, small_width * 4U) != 0) {
            free(small); free(large); return 43;
        }
    }
    ui_widget_draw_spec empty_state = {.kind = UI_WIDGET_EMPTY_STATE,
        .state = UI_CONTROL_REST, .bounds = {96U, 96U, 280U, 120U},
        .icon = UI_ICON_LAYERS, .label = "No selection", .label_length = 12U,
        .secondary = "Select a shape", .secondary_length = 14U};
    if (!ui_widget_paint(&large_canvas, &empty_state)) {
        free(small); free(large); return 45;
    }
    text_capture capture = {0};
    ui_widget_canvas text_canvas;
    if (!ui_widget_canvas_init(&text_canvas, large, large_bytes, large_width,
            large_height, large_width * 4U, 1U, UI_THEME_LIGHT, false,
            capture_text, &capture)) {
        free(small); free(large); return 46;
    }
    ui_widget_draw_spec segmented = {.kind = UI_WIDGET_SEGMENTED,
        .state = UI_CONTROL_REST, .bounds = {64U, 256U, 300U, 36U},
        .segment_index = 1U, .segment_count = 3U,
        .segment_labels = {"Select", "Pan", "Zoom"},
        .segment_label_lengths = {6U, 3U, 4U}};
    if (!ui_widget_paint(&text_canvas, &segmented) || capture.count != 3U ||
        strcmp(capture.labels[0], "Select") != 0 ||
        strcmp(capture.labels[1], "Pan") != 0 || strcmp(capture.labels[2], "Zoom") != 0 ||
        capture.family[0] != UI_FONT_SYSTEM || capture.family[1] != UI_FONT_SYSTEM ||
        capture.family[2] != UI_FONT_SYSTEM || capture.weight[0] != UI_FONT_REGULAR ||
        capture.weight[1] != UI_FONT_SEMIBOLD || capture.weight[2] != UI_FONT_REGULAR ||
        capture.size[0] != 12U || capture.size[1] != 12U || capture.size[2] != 12U) {
        free(small); free(large); return 47;
    }
    capture.count = 0U;
    ui_widget_draw_spec number_field = {.kind = UI_WIDGET_NUMBER_FIELD,
        .state = UI_CONTROL_REST, .bounds = {64U, 304U, 180U, 30U},
        .value = "64", .value_length = 2U, .unit = "px", .unit_length = 2U};
    if (!ui_widget_paint(&text_canvas, &number_field) || capture.count != 2U ||
        strcmp(capture.labels[0], "64") != 0 || strcmp(capture.labels[1], "px") != 0 ||
        capture.family[0] != UI_FONT_MONOSPACED ||
        capture.family[1] != UI_FONT_SYSTEM || capture.size[0] != 12U ||
        capture.size[1] != 11U) {
        free(small); free(large); return 48;
    }
    if (ui_widget_paint_scratch_limit() != 0U ||
        ui_widget_canvas_init(&small_canvas, small, small_bytes - 1U,
            small_width, small_height, small_width * 4U, 1U, UI_THEME_LIGHT, false,
            NULL, NULL)) { free(small); free(large); return 44; }
    free(small); free(large);
    return 0;
}

int main(int argc, char **argv)
{
    if (argc == 2 && strcmp(argv[1], "--contrast-report") == 0) {
        int result = test_tokens(true);
        if (result != 0) return result;
        return print_contrast_report();
    }
    if (argc != 1) return 2;
    int result = test_tokens(false);
    if (result != 0) return result;
    result = test_icons();
    if (result != 0) return result;
    result = test_emphasized_icon_masks();
    if (result != 0) return result;
    result = test_interactions();
    if (result != 0) return result;
    result = test_paint_bounds();
    if (result != 0) return result;
    (void)puts("UI kit contracts passed: tokens, 45 icons, input state, parser edges, bounded painting");
    return 0;
}
