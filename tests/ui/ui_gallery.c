#include "ui_core.h"
#include "ui_draw.h"
#include "ui_widgets.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define GALLERY_LABEL_WIDTH 160U
#define GALLERY_STATE_WIDTH 152U
#define GALLERY_STATE_GAP 4U
#define GALLERY_TOP 58U
#define GALLERY_ROW 52U
#define GALLERY_WIDTH (16U + GALLERY_LABEL_WIDTH + 7U * (GALLERY_STATE_WIDTH + GALLERY_STATE_GAP) + 16U)

static const char *const state_names[] = {
    "REST", "HOVER", "PRESSED", "FOCUS", "DISABLED", "SELECTED", "INVALID"
};
static const char *const kind_names[UI_WIDGET_KIND_COUNT] = {
    [UI_WIDGET_LABEL] = "Label",
    [UI_WIDGET_ICON_BUTTON] = "Icon button",
    [UI_WIDGET_BUTTON_PRIMARY] = "Primary button",
    [UI_WIDGET_BUTTON_SECONDARY] = "Secondary button",
    [UI_WIDGET_BUTTON_GHOST] = "Ghost button",
    [UI_WIDGET_SEGMENTED] = "Segmented control",
    [UI_WIDGET_TOGGLE] = "Toggle switch",
    [UI_WIDGET_CHECKBOX] = "Checkbox",
    [UI_WIDGET_TAB_BAR] = "Tab bar",
    [UI_WIDGET_TOOLTIP] = "Tooltip bubble",
    [UI_WIDGET_HUD_PILL] = "HUD pill",
    [UI_WIDGET_DIVIDER] = "Divider",
    [UI_WIDGET_PANEL_HEADER] = "Panel header",
    [UI_WIDGET_SECTION_HEADER] = "Section header",
    [UI_WIDGET_TREE_ROW] = "Tree row",
    [UI_WIDGET_EMPTY_STATE] = "Empty state",
    [UI_WIDGET_INLINE_ERROR] = "Inline error",
    [UI_WIDGET_TOAST] = "Toast",
    [UI_WIDGET_NUMBER_FIELD] = "Number field",
    [UI_WIDGET_SCRUB_LABEL] = "Scrub label",
    [UI_WIDGET_STEPPER] = "Stepper arrows",
    [UI_WIDGET_COLOR_FIELD] = "Swatch + hex",
    [UI_WIDGET_SLIDER] = "Slider",
    [UI_WIDGET_DROPDOWN] = "Dropdown",
    [UI_WIDGET_TEXT_FIELD] = "Text field"
};

static bool gallery_text(void *context, const char *text, size_t length,
    ui_rect bounds, ui_theme theme, uint32_t font_size_px, uint32_t color,
    ui_font_family family, ui_font_weight weight, uint8_t *rgba,
    uint32_t width, uint32_t height, size_t stride)
{
    (void)context; (void)theme; (void)family; (void)weight;
    return ui_scalar_text_draw(text, length, bounds, font_size_px, color,
        rgba, width, height, stride);
}

static bool fill_background(ui_widget_canvas *canvas)
{
    uint32_t color = canvas->tokens.window;
    uint8_t red = (uint8_t)(color >> 24U), green = (uint8_t)(color >> 16U);
    uint8_t blue = (uint8_t)(color >> 8U), alpha = (uint8_t)color;
    for (uint32_t y = 0U; y < canvas->height; ++y) {
        uint8_t *row = canvas->rgba + (size_t)y * canvas->stride;
        for (uint32_t x = 0U; x < canvas->width; ++x) {
            row[(size_t)x * 4U] = red; row[(size_t)x * 4U + 1U] = green;
            row[(size_t)x * 4U + 2U] = blue; row[(size_t)x * 4U + 3U] = alpha;
        }
    }
    return true;
}

static bool paint_label(ui_widget_canvas *canvas, const char *text, size_t length,
    ui_rect logical_bounds, uint32_t font_points, uint32_t color)
{
    uint32_t scale = canvas->backing_scale;
    if (logical_bounds.x > UINT32_MAX / scale || logical_bounds.y > UINT32_MAX / scale ||
        logical_bounds.width > UINT32_MAX / scale || logical_bounds.height > UINT32_MAX / scale)
        return false;
    ui_rect pixels = {logical_bounds.x * scale, logical_bounds.y * scale,
        logical_bounds.width * scale, logical_bounds.height * scale};
    return ui_scalar_text_draw(text, length, pixels, font_points * scale, color,
        canvas->rgba, canvas->width, canvas->height, canvas->stride);
}

static ui_widget_draw_spec sample_spec(ui_widget_kind kind, ui_control_state state,
    uint32_t x, uint32_t y, uint32_t height, uint8_t state_index)
{
    ui_widget_draw_spec spec = {.kind = kind, .state = state,
        .bounds = {x, y, GALLERY_STATE_WIDTH, height}, .segment_index = state_index % 3U,
        .segment_count = 3U, .expanded = true,
        .checked = state == UI_CONTROL_SELECTED || state == UI_CONTROL_PRESSED};
    switch (kind) {
        case UI_WIDGET_AUTO:
        case UI_WIDGET_LABEL:
            spec.label = "Selected shape"; spec.label_length = 14U; break;
        case UI_WIDGET_ICON_BUTTON:
            spec.icon = UI_ICON_SAVE; break;
        case UI_WIDGET_BUTTON_PRIMARY: spec.label = "Save"; spec.label_length = 4U; break;
        case UI_WIDGET_BUTTON_SECONDARY:
            spec.label = "Duplicate"; spec.label_length = 9U; spec.icon = UI_ICON_DUPLICATE; break;
        case UI_WIDGET_BUTTON_GHOST:
            spec.label = "Undo"; spec.label_length = 4U; spec.icon = UI_ICON_UNDO; break;
        case UI_WIDGET_SEGMENTED:
            spec.segment_labels[0] = "Move";
            spec.segment_labels[1] = "Scale";
            spec.segment_labels[2] = "Rotate";
            spec.segment_label_lengths[0] = 4U;
            spec.segment_label_lengths[1] = 5U;
            spec.segment_label_lengths[2] = 6U;
            break;
        case UI_WIDGET_TOGGLE: spec.label = "Grid"; spec.label_length = 4U; break;
        case UI_WIDGET_CHECKBOX: spec.label = "Use grid"; spec.label_length = 8U; break;
        case UI_WIDGET_TAB_BAR:
            spec.label = "Color"; spec.label_length = 5U;
            spec.secondary = "Swatches"; spec.secondary_length = 8U; break;
        case UI_WIDGET_TOOLTIP: spec.label = "Pan   Space"; spec.label_length = 11U; break;
        case UI_WIDGET_HUD_PILL: spec.label = "100%   Fit"; spec.label_length = 10U; break;
        case UI_WIDGET_DIVIDER: spec.bounds.width = 96U; spec.bounds.height = 1U; break;
        case UI_WIDGET_PANEL_HEADER: spec.label = "Inspector"; spec.label_length = 9U; break;
        case UI_WIDGET_SECTION_HEADER:
            spec.label = "Geometry"; spec.label_length = 8U; spec.expanded = state_index % 2U == 0U; break;
        case UI_WIDGET_TREE_ROW:
            spec.label = "Rectangle  #box"; spec.label_length = 15U;
            spec.icon = UI_ICON_RECTANGLE; spec.indent = 1U; break;
        case UI_WIDGET_EMPTY_STATE:
            spec.label = "No selection"; spec.label_length = 12U;
            spec.secondary = "Select a shape"; spec.secondary_length = 14U;
            spec.icon = UI_ICON_LAYERS; break;
        case UI_WIDGET_INLINE_ERROR:
            spec.label = "Invalid value"; spec.label_length = 13U;
            spec.secondary = "Enter a valid number"; spec.secondary_length = 20U;
            spec.tone = UI_WIDGET_TONE_ERROR; break;
        case UI_WIDGET_TOAST:
            spec.label = "Saved"; spec.label_length = 5U;
            spec.secondary = "Document saved"; spec.secondary_length = 14U;
            spec.tone = UI_WIDGET_TONE_SUCCESS; break;
        case UI_WIDGET_NUMBER_FIELD:
            spec.value = "64"; spec.value_length = 2U; spec.unit = "px"; spec.unit_length = 2U; break;
        case UI_WIDGET_SCRUB_LABEL:
            spec.label = "X"; spec.label_length = 1U; spec.value = "16"; spec.value_length = 2U;
            spec.unit = "px"; spec.unit_length = 2U; spec.icon = UI_ICON_RULER; break;
        case UI_WIDGET_STEPPER: spec.bounds.width = 56U; break;
        case UI_WIDGET_COLOR_FIELD:
            spec.value = "#4F7CFF"; spec.value_length = 7U;
            spec.rgba = UINT32_C(0x4f7cffff); break;
        case UI_WIDGET_SLIDER:
            spec.value = "72%"; spec.value_length = 3U; spec.numeric_value = 0.72; break;
        case UI_WIDGET_DROPDOWN:
            spec.label = "Units"; spec.label_length = 5U;
            spec.value = "Pixels"; spec.value_length = 6U;
            spec.expanded = state == UI_CONTROL_FOCUS;
            spec.bounds.width = 120U; break;
        case UI_WIDGET_TEXT_FIELD:
            spec.value = "Rectangle"; spec.value_length = 9U;
            spec.placeholder = state_index == 0U;
            spec.cursor_byte_offset = 4U;
            spec.selection_start = 0U; spec.selection_length = state == UI_CONTROL_FOCUS ? 4U : 0U;
            break;
        case UI_WIDGET_KIND_COUNT: break;
    }
    if (kind == UI_WIDGET_EMPTY_STATE) spec.bounds.height = height;
    if (kind == UI_WIDGET_TOAST || kind == UI_WIDGET_INLINE_ERROR)
        spec.bounds.height = 38U;
    if (kind == UI_WIDGET_TOOLTIP) spec.bounds.height = 28U;
    if (kind == UI_WIDGET_DROPDOWN && spec.expanded) spec.bounds.height = height;
    return spec;
}

static bool draw_sheet(ui_widget_canvas *canvas)
{
    (void)fill_background(canvas);
    uint32_t label_color = canvas->tokens.text_secondary;
    uint32_t row_y = GALLERY_TOP;
    for (size_t state = 0U; state < sizeof(state_names) / sizeof(state_names[0]); ++state) {
        uint32_t x = 16U + GALLERY_LABEL_WIDTH + (uint32_t)state *
            (GALLERY_STATE_WIDTH + GALLERY_STATE_GAP);
        if (!paint_label(canvas, state_names[state], strlen(state_names[state]),
                (ui_rect){x, 24U, GALLERY_STATE_WIDTH, 18U}, 11U, label_color)) return false;
    }
    for (uint32_t kind = UI_WIDGET_LABEL; kind < UI_WIDGET_KIND_COUNT; ++kind) {
        uint32_t row_height = GALLERY_ROW;
        if (kind == UI_WIDGET_EMPTY_STATE) row_height = 88U;
        if (kind == UI_WIDGET_DROPDOWN) row_height = 102U;
        if (!paint_label(canvas, kind_names[kind], strlen(kind_names[kind]),
                (ui_rect){16U, row_y, GALLERY_LABEL_WIDTH - 12U, 24U}, 12U,
                canvas->tokens.text_primary)) return false;
        for (uint8_t state_index = 0U; state_index < 7U; ++state_index) {
            ui_control_state state = (ui_control_state)state_index;
            uint32_t x = 16U + GALLERY_LABEL_WIDTH + (uint32_t)state_index *
                (GALLERY_STATE_WIDTH + GALLERY_STATE_GAP);
            uint32_t height = kind == UI_WIDGET_EMPTY_STATE ? 80U :
                kind == UI_WIDGET_DROPDOWN && state == UI_CONTROL_FOCUS ? 92U : 34U;
            uint32_t y = row_y + (row_height > height ? (row_height - height) / 2U : 0U);
            ui_widget_draw_spec spec = sample_spec(kind, state, x, y, height, state_index);
            if (kind != UI_WIDGET_DROPDOWN && kind != UI_WIDGET_EMPTY_STATE)
                spec.bounds.y = row_y + (row_height - spec.bounds.height) / 2U;
            if (!ui_widget_paint(canvas, &spec)) return false;
        }
        uint32_t line_y = row_y + row_height - 1U;
        if (line_y < canvas->height / canvas->backing_scale) {
            uint32_t physical_y = line_y * canvas->backing_scale;
            for (uint32_t sy = 0U; sy < canvas->backing_scale; ++sy)
                for (uint32_t x = 16U * canvas->backing_scale;
                    x < canvas->width - 16U * canvas->backing_scale; ++x) {
                    uint8_t *pixel = canvas->rgba + (size_t)(physical_y + sy) * canvas->stride +
                        (size_t)x * 4U;
                    pixel[0] = (uint8_t)(canvas->tokens.separator >> 24U);
                    pixel[1] = (uint8_t)(canvas->tokens.separator >> 16U);
                    pixel[2] = (uint8_t)(canvas->tokens.separator >> 8U);
                    pixel[3] = (uint8_t)canvas->tokens.separator;
                }
        }
        row_y += row_height;
    }
    return true;
}

int main(int argc, char **argv)
{
    if (argc != 3) {
        (void)fprintf(stderr, "usage: ui_gallery <light|dark> <scale 1..2>\n");
        return 2;
    }
    ui_theme theme = strcmp(argv[1], "light") == 0 ? UI_THEME_LIGHT :
        strcmp(argv[1], "dark") == 0 ? UI_THEME_DARK : (ui_theme)99;
    if (theme > UI_THEME_LIGHT || (strcmp(argv[2], "1") != 0 && strcmp(argv[2], "2") != 0)) return 2;
    uint32_t scale = (uint32_t)(argv[2][0] - '0');
    uint32_t component_count = UI_WIDGET_KIND_COUNT - UI_WIDGET_LABEL;
    uint32_t logical_height = GALLERY_TOP + component_count * GALLERY_ROW +
        (88U - GALLERY_ROW) + (102U - GALLERY_ROW);
    uint32_t width = GALLERY_WIDTH * scale, height = logical_height * scale;
    size_t stride = (size_t)width * 4U, capacity = stride * height;
    uint8_t *rgba = calloc(capacity, 1U);
    if (rgba == NULL) return 3;
    ui_widget_canvas canvas;
    if (!ui_widget_canvas_init(&canvas, rgba, capacity, width, height, stride,
            scale, theme, false, gallery_text, NULL) || !draw_sheet(&canvas)) {
        free(rgba);
        return 4;
    }
    size_t written = fwrite(rgba, 1U, capacity, stdout);
    free(rgba);
    return written == capacity && fflush(stdout) == 0 ? 0 : 5;
}
