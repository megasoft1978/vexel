#include "ui_widgets.h"

#include "ui_icons.h"

#include <math.h>
#include <stdlib.h>
#include <string.h>

#define UI_WIDGET_MAX_CANVAS_AXIS 16384U
#define UI_PAINT_SAMPLES 4U
#define UI_POINT_TEXT 13U

static bool canvas_valid(const ui_widget_canvas *canvas)
{
    return canvas != NULL && canvas->rgba != NULL && canvas->width > 0U &&
        canvas->height > 0U && canvas->width <= UI_WIDGET_MAX_CANVAS_AXIS &&
        canvas->height <= UI_WIDGET_MAX_CANVAS_AXIS && canvas->backing_scale >= 1U &&
        canvas->backing_scale <= 3U && canvas->theme >= UI_THEME_DARK &&
        canvas->theme <= UI_THEME_LIGHT &&
        (size_t)canvas->width <= SIZE_MAX / 4U &&
        canvas->stride >= (size_t)canvas->width * 4U &&
        (size_t)canvas->height <= SIZE_MAX / canvas->stride &&
        canvas->capacity >= (size_t)canvas->height * canvas->stride;
}

bool ui_widget_canvas_init(ui_widget_canvas *canvas, uint8_t *rgba,
    size_t capacity, uint32_t width, uint32_t height, size_t stride,
    uint32_t backing_scale, ui_theme theme, bool increased_contrast,
    ui_widget_text_draw_fn draw_text, void *text_context)
{
    if (canvas == NULL || rgba == NULL || width == 0U || height == 0U ||
        width > UI_WIDGET_MAX_CANVAS_AXIS || height > UI_WIDGET_MAX_CANVAS_AXIS ||
        backing_scale == 0U || backing_scale > 3U || theme < UI_THEME_DARK ||
        theme > UI_THEME_LIGHT ||
        (size_t)width > SIZE_MAX / 4U || stride < (size_t)width * 4U ||
        (size_t)height > SIZE_MAX / stride || capacity < (size_t)height * stride) return false;
    *canvas = (ui_widget_canvas){.rgba = rgba, .capacity = capacity,
        .stride = stride, .width = width, .height = height,
        .backing_scale = backing_scale, .theme = theme,
        .increased_contrast = increased_contrast,
        .tokens = ui_theme_tokens(theme, increased_contrast),
        .draw_text = draw_text, .text_context = text_context};
    return true;
}

static bool scaled_rect(const ui_widget_canvas *canvas, ui_rect logical,
                        ui_rect *out_pixels)
{
    if (canvas == NULL || out_pixels == NULL || logical.width == 0U || logical.height == 0U)
        return false;
    uint64_t scale = canvas->backing_scale;
    uint64_t x = (uint64_t)logical.x * scale, y = (uint64_t)logical.y * scale;
    uint64_t width = (uint64_t)logical.width * scale, height = (uint64_t)logical.height * scale;
    if (x > UINT32_MAX || y > UINT32_MAX || width > UINT32_MAX || height > UINT32_MAX ||
        x + width > UINT32_MAX || y + height > UINT32_MAX) return false;
    *out_pixels = (ui_rect){(uint32_t)x, (uint32_t)y, (uint32_t)width, (uint32_t)height};
    return true;
}

static void blend_pixel(uint8_t *destination, uint32_t color, uint8_t coverage)
{
    uint32_t alpha = (((color & 255U) * (uint32_t)coverage) + 127U) / 255U;
    uint32_t inverse = 255U - alpha;
    uint32_t old_alpha = destination[3];
    uint32_t out_alpha = alpha + (old_alpha * inverse + 127U) / 255U;
    if (out_alpha == 0U) return;
    for (size_t channel = 0U; channel < 3U; ++channel) {
        uint32_t source = (color >> (24U - (uint32_t)(channel * 8U))) & 255U;
        uint32_t numerator = source * alpha * 255U +
            (uint32_t)destination[channel] * old_alpha * inverse;
        destination[channel] = (uint8_t)((numerator + out_alpha * 127U) /
            (out_alpha * 255U));
    }
    destination[3] = (uint8_t)out_alpha;
}

static bool rounded_contains(double x, double y, double left, double top,
    double right, double bottom, double radius)
{
    if (x < left || x >= right || y < top || y >= bottom) return false;
    double cx = x < left + radius ? left + radius :
        x >= right - radius ? right - radius : x;
    double cy = y < top + radius ? top + radius :
        y >= bottom - radius ? bottom - radius : y;
    double dx = x - cx, dy = y - cy;
    return dx * dx + dy * dy <= radius * radius;
}

static void paint_round(const ui_widget_canvas *canvas, ui_rect logical,
    uint32_t radius_points, uint32_t fill, uint32_t edge, uint32_t edge_points)
{
    ui_rect rect = {0};
    if (!scaled_rect(canvas, logical, &rect) || rect.x >= canvas->width ||
        rect.y >= canvas->height) return;
    uint32_t right_limit = rect.x + rect.width;
    uint32_t bottom_limit = rect.y + rect.height;
    if (right_limit < rect.x || right_limit > canvas->width) right_limit = canvas->width;
    if (bottom_limit < rect.y || bottom_limit > canvas->height) bottom_limit = canvas->height;
    double radius = (double)(radius_points * canvas->backing_scale);
    if (radius > (double)rect.width / 2.0) radius = (double)rect.width / 2.0;
    if (radius > (double)rect.height / 2.0) radius = (double)rect.height / 2.0;
    double inset = (double)(edge_points * canvas->backing_scale);
    double outer_left = rect.x, outer_top = rect.y;
    double outer_right = (double)rect.x + rect.width;
    double outer_bottom = (double)rect.y + rect.height;
    double inner_left = outer_left + inset, inner_top = outer_top + inset;
    double inner_right = outer_right - inset, inner_bottom = outer_bottom - inset;
    double inner_radius = radius > inset ? radius - inset : 0.0;
    for (uint32_t y = rect.y; y < bottom_limit; ++y) {
        for (uint32_t x = rect.x; x < right_limit; ++x) {
            unsigned fill_coverage = 0U, edge_coverage = 0U;
            for (unsigned sy = 0U; sy < UI_PAINT_SAMPLES; ++sy) {
                for (unsigned sx = 0U; sx < UI_PAINT_SAMPLES; ++sx) {
                    double px = (double)x + ((double)sx + 0.5) / UI_PAINT_SAMPLES;
                    double py = (double)y + ((double)sy + 0.5) / UI_PAINT_SAMPLES;
                    bool outer = rounded_contains(px, py, outer_left, outer_top,
                        outer_right, outer_bottom, radius);
                    if (!outer) continue;
                    bool inner = edge_points > 0U && inner_left < inner_right &&
                        inner_top < inner_bottom && rounded_contains(px, py,
                        inner_left, inner_top, inner_right, inner_bottom, inner_radius);
                    if (edge_points > 0U && !inner) edge_coverage++;
                    else if (fill != 0U) fill_coverage++;
                }
            }
            uint8_t *pixel = canvas->rgba + (size_t)y * canvas->stride + (size_t)x * 4U;
            if (fill_coverage > 0U && fill != 0U)
                blend_pixel(pixel, fill, (uint8_t)((fill_coverage * 255U + 8U) / 16U));
            if (edge_coverage > 0U && edge != 0U)
                blend_pixel(pixel, edge, (uint8_t)((edge_coverage * 255U + 8U) / 16U));
        }
    }
}

static void paint_rect(const ui_widget_canvas *canvas, ui_rect logical,
                       uint32_t color)
{
    ui_rect rect = {0};
    if (!scaled_rect(canvas, logical, &rect) || rect.x >= canvas->width ||
        rect.y >= canvas->height) return;
    uint32_t right = rect.x + rect.width, bottom = rect.y + rect.height;
    if (right < rect.x || right > canvas->width) right = canvas->width;
    if (bottom < rect.y || bottom > canvas->height) bottom = canvas->height;
    for (uint32_t y = rect.y; y < bottom; ++y)
        for (uint32_t x = rect.x; x < right; ++x)
            blend_pixel(canvas->rgba + (size_t)y * canvas->stride + (size_t)x * 4U,
                color, 255U);
}

static void paint_focus(const ui_widget_canvas *canvas, ui_rect bounds)
{
    uint32_t left = bounds.x > 3U ? bounds.x - 3U : 0U;
    uint32_t top = bounds.y > 3U ? bounds.y - 3U : 0U;
    uint64_t right = (uint64_t)bounds.x + bounds.width + 3U;
    uint64_t bottom = (uint64_t)bounds.y + bounds.height + 3U;
    if (right > UINT32_MAX || bottom > UINT32_MAX) return;
    ui_rect ring = {left, top, (uint32_t)right - left, (uint32_t)bottom - top};
    paint_round(canvas, ring, 9U, 0U, canvas->tokens.focus_ring, 2U);
}

static uint32_t state_foreground(const ui_widget_canvas *canvas, ui_control_state state)
{
    if (state == UI_CONTROL_DISABLED) return canvas->tokens.disabled_text;
    if (state == UI_CONTROL_INVALID) return canvas->tokens.error;
    return canvas->tokens.text_primary;
}

static uint32_t state_surface(const ui_widget_canvas *canvas, ui_control_state state,
                              uint32_t ordinary)
{
    switch (state) {
        case UI_CONTROL_HOVER: return canvas->tokens.panel_hover;
        case UI_CONTROL_PRESSED: return canvas->tokens.pressed_fill;
        case UI_CONTROL_SELECTED: return canvas->tokens.selection_fill;
        case UI_CONTROL_DISABLED: return canvas->tokens.panel_raised;
        case UI_CONTROL_REST:
        case UI_CONTROL_FOCUS:
        case UI_CONTROL_INVALID: return ordinary;
    }
    return ordinary;
}

static bool draw_text(const ui_widget_canvas *canvas, const char *text, size_t length,
    ui_rect bounds, uint32_t color, ui_font_family family, ui_font_weight weight,
    uint32_t font_points)
{
    if (text == NULL || length == 0U || canvas->draw_text == NULL) return true;
    ui_rect pixels = {0};
    if (!scaled_rect(canvas, bounds, &pixels)) return false;
    uint32_t font_px = font_points * canvas->backing_scale;
    if (font_px == 0U || font_px > 256U) return false;
    return canvas->draw_text(canvas->text_context, text, length, pixels,
        canvas->theme, font_px, color, family, weight, canvas->rgba,
        canvas->width, canvas->height, canvas->stride);
}

static bool draw_icon(const ui_widget_canvas *canvas, ui_icon_kind icon,
    ui_rect bounds, ui_control_state state)
{
    ui_rect pixels = {0};
    if (!scaled_rect(canvas, bounds, &pixels)) return false;
    return ui_icon_paint_state_into(icon, canvas->theme, canvas->increased_contrast,
        (double)canvas->backing_scale, pixels, state, &canvas->tokens,
        canvas->rgba, canvas->width, canvas->height, canvas->stride);
}

static ui_rect inset(ui_rect rect, uint32_t left, uint32_t top,
                     uint32_t right, uint32_t bottom)
{
    if (left + right >= rect.width || top + bottom >= rect.height) return (ui_rect){0};
    return (ui_rect){rect.x + left, rect.y + top,
        rect.width - left - right, rect.height - top - bottom};
}

static bool draw_button(const ui_widget_canvas *canvas, const ui_widget_draw_spec *spec,
    uint32_t surface, uint32_t foreground, bool primary)
{
    bool icon_only = spec->kind == UI_WIDGET_ICON_BUTTON;
    if (primary && spec->state == UI_CONTROL_PRESSED)
        surface = canvas->theme == UI_THEME_LIGHT ? UINT32_C(0x124b97ff) :
            canvas->tokens.selection_fill;
    if (primary && spec->state != UI_CONTROL_DISABLED) {
        foreground = canvas->theme == UI_THEME_LIGHT ? UINT32_C(0xffffffff) :
            (spec->state == UI_CONTROL_PRESSED ? canvas->tokens.text_primary :
                canvas->tokens.window);
    }
    if (surface != 0U) paint_round(canvas, spec->bounds, 6U, surface, 0U, 0U);
    if (icon_only) {
        if (spec->state == UI_CONTROL_SELECTED) {
            ui_rect marker = {spec->bounds.x, spec->bounds.y + spec->bounds.height - 3U,
                spec->bounds.width, 2U};
            paint_round(canvas, marker, 1U, canvas->tokens.accent, 0U, 0U);
        }
        uint32_t icon_size = spec->bounds.height < 18U ? spec->bounds.height : 18U;
        ui_rect icon_bounds = {spec->bounds.x + (spec->bounds.width - icon_size) / 2U,
            spec->bounds.y + (spec->bounds.height - icon_size) / 2U, icon_size, icon_size};
        return draw_icon(canvas, spec->icon, icon_bounds, spec->state);
    }
    if (spec->icon != UI_ICON_NONE) {
        ui_rect icon_bounds = {spec->bounds.x + 8U, spec->bounds.y +
            (spec->bounds.height > 16U ? (spec->bounds.height - 16U) / 2U : 0U), 16U, 16U};
        if (!draw_icon(canvas, spec->icon, icon_bounds, spec->state)) return false;
    }
    uint32_t left = spec->icon == UI_ICON_NONE ? 8U : 28U;
    ui_rect text_bounds = inset(spec->bounds, left, 0U, 8U, 0U);
    return draw_text(canvas, spec->label, spec->label_length, text_bounds,
        foreground, UI_FONT_SYSTEM, UI_FONT_REGULAR, UI_POINT_TEXT);
}

static bool draw_toggle(const ui_widget_canvas *canvas, const ui_widget_draw_spec *spec)
{
    uint32_t track_width = 28U, track_height = 16U;
    uint32_t x = spec->bounds.x + 8U;
    uint32_t y = spec->bounds.y + (spec->bounds.height > track_height ?
        (spec->bounds.height - track_height) / 2U : 0U);
    uint32_t track = spec->checked ? canvas->tokens.accent : canvas->tokens.control_edge;
    if (spec->state == UI_CONTROL_DISABLED) track = canvas->tokens.panel_raised;
    paint_round(canvas, (ui_rect){x, y, track_width, track_height}, 8U, track,
        spec->checked ? 0U : canvas->tokens.control_edge, spec->checked ? 0U : 1U);
    uint32_t knob_x = spec->checked ? x + 14U : x + 2U;
    uint32_t knob_color = spec->state == UI_CONTROL_DISABLED ? canvas->tokens.disabled_text :
        canvas->theme == UI_THEME_LIGHT ? UINT32_C(0xffffffff) : canvas->tokens.text_primary;
    paint_round(canvas, (ui_rect){knob_x, y + 2U, 12U, 12U}, 6U, knob_color, 0U, 0U);
    ui_rect text_bounds = {x + 36U, spec->bounds.y, spec->bounds.width > 44U ?
        spec->bounds.width - 44U : 0U, spec->bounds.height};
    return draw_text(canvas, spec->label, spec->label_length, text_bounds,
        state_foreground(canvas, spec->state), UI_FONT_SYSTEM, UI_FONT_REGULAR, UI_POINT_TEXT);
}

static void draw_check(const ui_widget_canvas *canvas, ui_rect bounds, uint32_t color)
{
    ui_rect px = {0};
    if (!scaled_rect(canvas, bounds, &px)) return;
    uint32_t start_x = px.x + px.width / 4U, mid_x = px.x + px.width * 2U / 5U;
    uint32_t end_x = px.x + px.width * 3U / 4U;
    uint32_t mid_y = px.y + px.height * 3U / 5U, start_y = px.y + px.height / 2U;
    uint32_t end_y = px.y + px.height / 4U;
    uint32_t steps = end_x - start_x;
    for (uint32_t i = 0U; i <= steps; ++i) {
        double t = steps == 0U ? 0.0 : (double)i / (double)steps;
        double x = (double)start_x + t * (double)(mid_x - start_x);
        double y = (double)start_y + t * (double)(mid_y - start_y);
        for (int32_t dy = -1; dy <= 1; ++dy) for (int32_t dx = -1; dx <= 1; ++dx) {
            int64_t px_x = (int64_t)llround(x) + dx, px_y = (int64_t)llround(y) + dy;
            if (px_x >= 0 && px_y >= 0 && (uint64_t)px_x < canvas->width &&
                (uint64_t)px_y < canvas->height)
                blend_pixel(canvas->rgba + (size_t)px_y * canvas->stride +
                    (size_t)px_x * 4U, color, 255U);
        }
    }
    steps = end_x - mid_x;
    for (uint32_t i = 0U; i <= steps; ++i) {
        double t = steps == 0U ? 0.0 : (double)i / (double)steps;
        double x = (double)mid_x + t * (double)(end_x - mid_x);
        double y = (double)mid_y + t * (double)((int32_t)end_y - (int32_t)mid_y);
        for (int32_t dy = -1; dy <= 1; ++dy) for (int32_t dx = -1; dx <= 1; ++dx) {
            int64_t px_x = (int64_t)llround(x) + dx, px_y = (int64_t)llround(y) + dy;
            if (px_x >= 0 && px_y >= 0 && (uint64_t)px_x < canvas->width &&
                (uint64_t)px_y < canvas->height)
                blend_pixel(canvas->rgba + (size_t)px_y * canvas->stride +
                    (size_t)px_x * 4U, color, 255U);
        }
    }
}

static void draw_chevron_segment(const ui_widget_canvas *canvas, int32_t x0,
    int32_t y0, int32_t x1, int32_t y1, uint32_t thickness, uint32_t color)
{
    int32_t dx = x1 - x0, dy = y1 - y0;
    uint32_t steps = (uint32_t)(abs(dx) > abs(dy) ? abs(dx) : abs(dy));
    for (uint32_t i = 0U; i <= steps; ++i) {
        double t = steps == 0U ? 0.0 : (double)i / (double)steps;
        int32_t x = (int32_t)llround((double)x0 + (double)dx * t);
        int32_t y = (int32_t)llround((double)y0 + (double)dy * t);
        int32_t radius = (int32_t)(thickness / 2U);
        for (int32_t oy = -radius; oy <= radius; ++oy) {
            for (int32_t ox = -radius; ox <= radius; ++ox) {
                int64_t px = (int64_t)x + ox, py = (int64_t)y + oy;
                if (px >= 0 && py >= 0 && (uint64_t)px < canvas->width &&
                    (uint64_t)py < canvas->height)
                    blend_pixel(canvas->rgba + (size_t)py * canvas->stride +
                        (size_t)px * 4U, color, 255U);
            }
        }
    }
}

static void draw_chevron(const ui_widget_canvas *canvas, ui_rect bounds,
    bool points_up, uint32_t color)
{
    ui_rect px = {0};
    if (!scaled_rect(canvas, bounds, &px)) return;
    int32_t middle_x = (int32_t)(px.x + px.width / 2U);
    int32_t middle_y = (int32_t)(px.y + px.height / 2U);
    int32_t wing = (int32_t)(px.width / 3U);
    int32_t rise = (int32_t)(px.height / 4U);
    int32_t left_y = points_up ? middle_y + rise : middle_y - rise;
    int32_t right_y = left_y;
    int32_t tip_y = points_up ? middle_y - rise : middle_y + rise;
    uint32_t thickness = 2U * canvas->backing_scale;
    draw_chevron_segment(canvas, middle_x - wing, left_y, middle_x, tip_y,
        thickness, color);
    draw_chevron_segment(canvas, middle_x, tip_y, middle_x + wing, right_y,
        thickness, color);
}

static bool draw_checkbox(const ui_widget_canvas *canvas, const ui_widget_draw_spec *spec)
{
    ui_rect box = {spec->bounds.x + 6U, spec->bounds.y +
        (spec->bounds.height > 16U ? (spec->bounds.height - 16U) / 2U : 0U), 16U, 16U};
    bool active = spec->checked || spec->state == UI_CONTROL_SELECTED ||
        spec->state == UI_CONTROL_PRESSED;
    uint32_t fill = active ? canvas->tokens.accent : canvas->tokens.panel;
    if (spec->state == UI_CONTROL_DISABLED) fill = canvas->tokens.panel_raised;
    paint_round(canvas, box, 3U, fill, active ? 0U : canvas->tokens.control_edge,
        active ? 0U : 1U);
    if (active) draw_check(canvas, inset(box, 2U, 2U, 2U, 2U),
        canvas->theme == UI_THEME_LIGHT ? UINT32_C(0xffffffff) : canvas->tokens.window);
    ui_rect label_bounds = {box.x + 24U, spec->bounds.y,
        spec->bounds.width > 36U ? spec->bounds.width - 36U : 0U,
        spec->bounds.height};
    return draw_text(canvas, spec->label, spec->label_length, label_bounds,
        state_foreground(canvas, spec->state), UI_FONT_SYSTEM, UI_FONT_REGULAR, UI_POINT_TEXT);
}

static bool draw_segmented(const ui_widget_canvas *canvas, const ui_widget_draw_spec *spec)
{
    uint32_t count = spec->segment_count < 2U ? 3U : spec->segment_count;
    if (count > 8U) count = 8U;
    paint_round(canvas, spec->bounds, 6U, canvas->tokens.panel_raised,
        canvas->tokens.separator, 1U);
    uint32_t segment_width = spec->bounds.width / count;
    uint32_t selected = spec->segment_index < count ? spec->segment_index : 0U;
    ui_rect active = {spec->bounds.x + segment_width * selected + 1U,
        spec->bounds.y + 1U, segment_width > 2U ? segment_width - 2U : segment_width,
        spec->bounds.height > 2U ? spec->bounds.height - 2U : spec->bounds.height};
    paint_round(canvas, active, 5U, canvas->tokens.panel, 0U, 0U);
    for (uint32_t index = 1U; index < count; ++index) {
        ui_rect divider = {spec->bounds.x + segment_width * index, spec->bounds.y + 6U,
            1U, spec->bounds.height > 12U ? spec->bounds.height - 12U : 1U};
        paint_rect(canvas, divider, canvas->tokens.separator);
    }
    return draw_text(canvas, spec->label, spec->label_length, inset(spec->bounds, 8U, 0U, 8U, 0U),
        state_foreground(canvas, spec->state), UI_FONT_SYSTEM, UI_FONT_REGULAR, UI_POINT_TEXT);
}

static bool draw_tab_bar(const ui_widget_canvas *canvas, const ui_widget_draw_spec *spec)
{
    uint32_t count = 2U;
    uint32_t segment_width = spec->bounds.width / count;
    uint32_t selected = spec->segment_index < count ? spec->segment_index : 0U;
    const char *label = selected == 0U ? spec->label : spec->secondary;
    size_t length = selected == 0U ? spec->label_length : spec->secondary_length;
    ui_rect tab = {spec->bounds.x + selected * segment_width, spec->bounds.y,
        segment_width, spec->bounds.height};
    ui_rect underline = {tab.x + 8U, tab.y + tab.height - 2U,
        tab.width > 16U ? tab.width - 16U : tab.width, 2U};
    paint_rect(canvas, underline, canvas->tokens.accent);
    return draw_text(canvas, label, length, tab, state_foreground(canvas, spec->state),
        UI_FONT_SYSTEM, selected == spec->segment_index ? UI_FONT_SEMIBOLD : UI_FONT_REGULAR,
        UI_POINT_TEXT);
}

static void draw_shadow(const ui_widget_canvas *canvas, ui_rect bounds)
{
    uint32_t shadow = canvas->tokens.popover_shadow;
    static const uint8_t weights[] = {45U, 70U, 90U};
    for (size_t layer = 0U; layer < sizeof(weights); ++layer) {
        ui_rect spread = {bounds.x, bounds.y + (uint32_t)layer + 1U,
            bounds.width, bounds.height};
        uint32_t color = (shadow & UINT32_C(0xffffff00)) |
            (((shadow & 255U) * weights[layer]) / 255U);
        paint_round(canvas, spread, 6U + (uint32_t)layer, color, 0U, 0U);
    }
}

static bool draw_tree_row(const ui_widget_canvas *canvas, const ui_widget_draw_spec *spec)
{
    uint32_t bg = spec->state == UI_CONTROL_SELECTED ? canvas->tokens.selection_fill :
        spec->state == UI_CONTROL_HOVER || spec->state == UI_CONTROL_PRESSED ?
            canvas->tokens.panel_hover : 0U;
    if (bg != 0U) paint_round(canvas, spec->bounds, 4U, bg, 0U, 0U);
    if (spec->state == UI_CONTROL_SELECTED)
        paint_round(canvas, (ui_rect){spec->bounds.x, spec->bounds.y + 4U, 2U,
            spec->bounds.height > 8U ? spec->bounds.height - 8U : 1U},
            1U, canvas->tokens.accent, 0U, 0U);
    uint32_t guide_x = spec->bounds.x + 12U;
    for (uint32_t depth = 0U; depth < spec->indent && depth < 6U; ++depth) {
        ui_rect guide = {guide_x + depth * 16U, spec->bounds.y, 1U, spec->bounds.height};
        paint_rect(canvas, guide, canvas->tokens.separator);
    }
    uint32_t start = spec->bounds.x + 4U + (uint32_t)spec->indent * 16U;
    ui_icon_kind disclosure = spec->expanded ? UI_ICON_CHEVRON_DOWN : UI_ICON_CHEVRON_RIGHT;
    if (!draw_icon(canvas, disclosure, (ui_rect){start, spec->bounds.y + 5U, 16U, 16U},
            spec->state)) return false;
    start += 18U;
    if (spec->icon != UI_ICON_NONE) {
        if (!draw_icon(canvas, spec->icon, (ui_rect){start, spec->bounds.y + 5U, 16U, 16U},
                spec->state)) return false;
    }
    start += 20U;
    uint32_t trailing_width = 36U;
    ui_rect text = {start, spec->bounds.y, spec->bounds.x + spec->bounds.width > start + trailing_width ?
        spec->bounds.x + spec->bounds.width - start - trailing_width : 0U, spec->bounds.height};
    if (!draw_text(canvas, spec->label, spec->label_length, text,
            state_foreground(canvas, spec->state), UI_FONT_SYSTEM, UI_FONT_REGULAR, UI_POINT_TEXT))
        return false;
    uint32_t trail_x = spec->bounds.x + (spec->bounds.width > 34U ? spec->bounds.width - 34U : 0U);
    if (!draw_icon(canvas, UI_ICON_EYE, (ui_rect){trail_x, spec->bounds.y + 5U, 14U, 14U},
            UI_CONTROL_HOVER)) return false;
    return draw_icon(canvas, UI_ICON_LOCK,
        (ui_rect){trail_x + 16U, spec->bounds.y + 5U, 14U, 14U}, UI_CONTROL_HOVER);
}

static bool draw_input(const ui_widget_canvas *canvas, const ui_widget_draw_spec *spec)
{
    uint32_t edge = spec->invalid || spec->state == UI_CONTROL_INVALID ?
        canvas->tokens.error : canvas->tokens.control_edge;
    ui_rect input_bounds = spec->bounds;
    if (spec->kind == UI_WIDGET_DROPDOWN && spec->expanded) input_bounds.height = 30U;
    paint_round(canvas, input_bounds, 6U,
        state_surface(canvas, spec->state, canvas->tokens.panel_raised), edge, 1U);
    if (spec->kind == UI_WIDGET_NUMBER_FIELD || spec->kind == UI_WIDGET_SCRUB_LABEL) {
        uint32_t unit_width = spec->unit_length > 0U ? 36U : 8U;
        uint32_t value_start = spec->kind == UI_WIDGET_SCRUB_LABEL ? 60U : 8U;
        uint32_t trailing_reserve = unit_width;
        if (spec->kind == UI_WIDGET_NUMBER_FIELD) trailing_reserve += 20U;
        if (spec->kind == UI_WIDGET_SCRUB_LABEL && spec->icon != UI_ICON_NONE)
            (void)draw_icon(canvas, spec->icon,
                (ui_rect){spec->bounds.x + 6U, spec->bounds.y + 7U, 14U, 14U}, spec->state);
        ui_rect value = {spec->bounds.x + value_start, spec->bounds.y,
            spec->bounds.width > value_start + 8U + trailing_reserve ?
                spec->bounds.width - value_start - 8U - trailing_reserve : 0U,
            spec->bounds.height};
        if (spec->label_length > 0U && spec->kind == UI_WIDGET_SCRUB_LABEL)
            (void)draw_text(canvas, spec->label, spec->label_length,
                (ui_rect){spec->bounds.x + 24U, spec->bounds.y, 24U, spec->bounds.height},
                canvas->tokens.text_secondary, UI_FONT_SYSTEM, UI_FONT_REGULAR, 12U);
        if (!draw_text(canvas, spec->value, spec->value_length, value,
                state_foreground(canvas, spec->state), UI_FONT_MONOSPACED,
                UI_FONT_REGULAR, 12U)) return false;
        uint32_t unit_x = spec->bounds.x + spec->bounds.width -
            (spec->kind == UI_WIDGET_NUMBER_FIELD ? 54U : 34U);
        if (spec->unit_length > 0U && !draw_text(canvas, spec->unit,
                spec->unit_length, (ui_rect){unit_x, spec->bounds.y, 28U, spec->bounds.height}, canvas->tokens.text_secondary,
                UI_FONT_SYSTEM, UI_FONT_REGULAR, 11U)) return false;
        if (spec->kind == UI_WIDGET_NUMBER_FIELD) {
            uint32_t stepper_x = spec->bounds.x + spec->bounds.width - 16U;
            uint32_t color = state_foreground(canvas, spec->state);
            draw_chevron(canvas, (ui_rect){stepper_x, spec->bounds.y + 4U, 10U, 8U},
                true, color);
            draw_chevron(canvas, (ui_rect){stepper_x, spec->bounds.y + 15U, 10U, 8U},
                false, color);
        }
        return true;
    }
    if (spec->kind == UI_WIDGET_STEPPER) {
        uint32_t button_width = spec->bounds.width / 2U;
        ui_rect first = {spec->bounds.x, spec->bounds.y, button_width, spec->bounds.height};
        ui_rect second = {spec->bounds.x + button_width, spec->bounds.y,
            spec->bounds.width - button_width, spec->bounds.height};
        paint_round(canvas, first, 5U, state_surface(canvas, spec->state,
            canvas->tokens.panel_raised), canvas->tokens.control_edge, 1U);
        paint_round(canvas, second, 5U, state_surface(canvas, spec->state,
            canvas->tokens.panel_raised), canvas->tokens.control_edge, 1U);
        ui_rect first_arrow = {first.x + (first.width > 16U ? (first.width - 16U) / 2U : 0U),
            first.y + (first.height > 16U ? (first.height - 16U) / 2U : 0U), 16U, 16U};
        ui_rect second_arrow = {second.x + (second.width > 16U ? (second.width - 16U) / 2U : 0U),
            second.y + (second.height > 16U ? (second.height - 16U) / 2U : 0U), 16U, 16U};
        uint32_t color = state_foreground(canvas, spec->state);
        draw_chevron(canvas, first_arrow, true, color);
        draw_chevron(canvas, second_arrow, false, color);
        return true;
    }
    if (spec->kind == UI_WIDGET_COLOR_FIELD) {
        ui_rect swatch = {spec->bounds.x + 4U, spec->bounds.y + 4U, 22U, 22U};
        paint_round(canvas, swatch, 4U, canvas->tokens.checker_a, 0U, 0U);
        ui_rect check_half = {swatch.x + 3U, swatch.y + 3U, 8U, 19U};
        paint_rect(canvas, check_half, canvas->tokens.checker_b);
        ui_rect color_half = {swatch.x + 11U, swatch.y + 3U, 8U, 19U};
        paint_round(canvas, color_half, 2U, spec->rgba, canvas->tokens.separator, 1U);
        (void)draw_icon(canvas, UI_ICON_EYEDROPPER,
            (ui_rect){spec->bounds.x + spec->bounds.width - 24U,
                spec->bounds.y + 7U, 16U, 16U}, spec->state);
        return draw_text(canvas, spec->value, spec->value_length,
            inset(spec->bounds, 34U, 0U, 28U, 0U), state_foreground(canvas, spec->state),
            UI_FONT_MONOSPACED, UI_FONT_REGULAR, 12U);
    }
    if (spec->kind == UI_WIDGET_SLIDER) {
        uint32_t track_width = spec->bounds.width > 64U ? spec->bounds.width - 64U : 1U;
        ui_rect track = {spec->bounds.x + 8U, spec->bounds.y + spec->bounds.height / 2U - 1U,
            track_width, 3U};
        paint_round(canvas, track, 2U, canvas->tokens.separator, 0U, 0U);
        double normalized = isfinite(spec->numeric_value) ? spec->numeric_value : 0.0;
        if (normalized < 0.0) normalized = 0.0;
        if (normalized > 1.0) normalized = 1.0;
        ui_rect filled = track;
        filled.width = (uint32_t)llround((double)track.width * normalized);
        if (filled.width > 0U) paint_round(canvas, filled, 2U, canvas->tokens.accent, 0U, 0U);
        uint32_t thumb_x = track.x + (uint32_t)llround((double)track.width * normalized);
        paint_round(canvas, (ui_rect){thumb_x > 6U ? thumb_x - 6U : 0U,
            spec->bounds.y + spec->bounds.height / 2U - 6U, 12U, 12U}, 6U,
            spec->state == UI_CONTROL_DISABLED ? canvas->tokens.disabled_text : canvas->tokens.panel,
            canvas->tokens.accent, 1U);
        return draw_text(canvas, spec->value, spec->value_length,
            inset(spec->bounds, spec->bounds.width > 56U ? spec->bounds.width - 48U : 8U,
                0U, 8U, 0U), state_foreground(canvas, spec->state),
            UI_FONT_MONOSPACED, UI_FONT_REGULAR, 11U);
    }
    if (spec->kind == UI_WIDGET_DROPDOWN) {
        ui_rect control = spec->bounds;
        if (spec->expanded) control.height = 30U;
        ui_rect label = inset(control, 8U, 0U,
            spec->value_length > 0U ? spec->bounds.width / 2U : 24U, 0U);
        if (!draw_text(canvas, spec->label, spec->label_length, label,
                canvas->tokens.text_secondary, UI_FONT_SYSTEM, UI_FONT_REGULAR, 12U)) return false;
        ui_rect value = inset(control, spec->bounds.width / 2U, 0U, 25U, 0U);
        if (!draw_text(canvas, spec->value, spec->value_length, value,
                state_foreground(canvas, spec->state), UI_FONT_SYSTEM, UI_FONT_REGULAR, 12U)) return false;
        (void)draw_icon(canvas, UI_ICON_CHEVRON_DOWN,
            (ui_rect){spec->bounds.x + spec->bounds.width - 20U,
                spec->bounds.y + (control.height - 16U) / 2U, 16U, 16U}, spec->state);
        if (spec->expanded && spec->bounds.height > 34U) {
            ui_rect popup = {spec->bounds.x, spec->bounds.y + 32U,
                spec->bounds.width, spec->bounds.height - 32U};
            draw_shadow(canvas, popup);
            paint_round(canvas, popup, 6U, canvas->tokens.panel,
                canvas->tokens.separator, 1U);
            const char *items[] = {"Auto", "Pixels", "Percent"};
            size_t lengths[] = {4U, 6U, 7U};
            uint32_t item_height = popup.height / 3U;
            for (size_t index = 0U; index < 3U; ++index) {
                ui_rect item = {popup.x + 2U, popup.y + (uint32_t)index * item_height + 1U,
                    popup.width - 4U, item_height};
                if (index == spec->segment_index)
                    paint_round(canvas, item, 4U, canvas->tokens.selection_fill, 0U, 0U);
                if (!draw_text(canvas, items[index], lengths[index],
                        inset(item, 8U, 0U, 8U, 0U), canvas->tokens.text_primary,
                        UI_FONT_SYSTEM, index == spec->segment_index ?
                            UI_FONT_SEMIBOLD : UI_FONT_REGULAR, 12U)) return false;
            }
        }
        return true;
    }
    uint32_t foreground = spec->placeholder ? canvas->tokens.text_tertiary :
        state_foreground(canvas, spec->state);
    ui_rect text = inset(spec->bounds, 8U, 0U, 8U, 0U);
    if (spec->selection_length > 0U) {
        uint32_t start = spec->bounds.x + 8U +
            (uint32_t)spec->selection_start * 7U;
        ui_rect selection = {start, spec->bounds.y + 5U,
            (uint32_t)spec->selection_length * 7U,
            spec->bounds.height > 10U ? spec->bounds.height - 10U : 1U};
        paint_round(canvas, selection, 2U, canvas->tokens.selection_fill, 0U, 0U);
    }
    if (!draw_text(canvas, spec->value, spec->value_length, text, foreground,
            UI_FONT_SYSTEM, UI_FONT_REGULAR, UI_POINT_TEXT)) return false;
    if (spec->state == UI_CONTROL_FOCUS && spec->cursor_byte_offset <= spec->value_length) {
        uint32_t cursor_x = spec->bounds.x + 8U + (uint32_t)spec->cursor_byte_offset * 7U;
        paint_rect(canvas, (ui_rect){cursor_x, spec->bounds.y + 7U, 1U,
            spec->bounds.height > 14U ? spec->bounds.height - 14U : 1U},
            canvas->tokens.text_primary);
    }
    return true;
}

static bool paint_component(ui_widget_canvas *canvas, const ui_widget_draw_spec *spec)
{
    uint32_t foreground = state_foreground(canvas, spec->state);
    switch (spec->kind) {
        case UI_WIDGET_ICON_BUTTON:
        case UI_WIDGET_BUTTON_GHOST:
            return draw_button(canvas, spec,
                spec->kind == UI_WIDGET_ICON_BUTTON ? state_surface(canvas, spec->state, 0U) :
                    state_surface(canvas, spec->state, 0U), foreground, false);
        case UI_WIDGET_BUTTON_PRIMARY: {
            uint32_t surface = state_surface(canvas, spec->state, canvas->tokens.accent);
            if (spec->state == UI_CONTROL_HOVER || spec->state == UI_CONTROL_SELECTED)
                surface = canvas->tokens.accent;
            return draw_button(canvas, spec, surface, foreground, true);
        }
        case UI_WIDGET_BUTTON_SECONDARY:
            return draw_button(canvas, spec, state_surface(canvas, spec->state,
                canvas->tokens.panel_raised), foreground, false);
        case UI_WIDGET_SEGMENTED: return draw_segmented(canvas, spec);
        case UI_WIDGET_TOGGLE: return draw_toggle(canvas, spec);
        case UI_WIDGET_CHECKBOX: return draw_checkbox(canvas, spec);
        case UI_WIDGET_TAB_BAR: return draw_tab_bar(canvas, spec);
        case UI_WIDGET_TOOLTIP: {
            draw_shadow(canvas, spec->bounds);
            paint_round(canvas, spec->bounds, 6U, canvas->tokens.panel,
                canvas->tokens.separator, 1U);
            return draw_text(canvas, spec->label, spec->label_length,
                inset(spec->bounds, 10U, 0U, 10U, 0U), canvas->tokens.text_primary,
                UI_FONT_SYSTEM, UI_FONT_REGULAR, 12U);
        }
        case UI_WIDGET_HUD_PILL:
            paint_round(canvas, spec->bounds, 6U, canvas->tokens.panel_raised,
                canvas->tokens.separator, 1U);
            return draw_text(canvas, spec->label, spec->label_length,
                inset(spec->bounds, 10U, 0U, 10U, 0U), foreground,
                UI_FONT_MONOSPACED, UI_FONT_REGULAR, 12U);
        case UI_WIDGET_DIVIDER: {
            ui_rect line = spec->bounds;
            if (line.height > line.width) line.width = 1U;
            else line.height = 1U;
            paint_rect(canvas, line, canvas->tokens.separator);
            return true;
        }
        case UI_WIDGET_PANEL_HEADER:
            return draw_text(canvas, spec->label, spec->label_length,
                inset(spec->bounds, 12U, 0U, 32U, 0U), canvas->tokens.text_primary,
                UI_FONT_SYSTEM, UI_FONT_SEMIBOLD, 14U) &&
                draw_icon(canvas, UI_ICON_MORE,
                    (ui_rect){spec->bounds.x + spec->bounds.width - 24U,
                        spec->bounds.y + 8U, 16U, 16U}, spec->state);
        case UI_WIDGET_SECTION_HEADER: {
            ui_icon_kind disclosure = spec->expanded ? UI_ICON_CHEVRON_DOWN : UI_ICON_CHEVRON_RIGHT;
            return draw_icon(canvas, disclosure,
                    (ui_rect){spec->bounds.x + 8U, spec->bounds.y + 8U, 16U, 16U}, spec->state) &&
                draw_text(canvas, spec->label, spec->label_length,
                    inset(spec->bounds, 30U, 0U, 8U, 0U), foreground,
                    UI_FONT_SYSTEM, UI_FONT_SEMIBOLD, 13U);
        }
        case UI_WIDGET_TREE_ROW: return draw_tree_row(canvas, spec);
        case UI_WIDGET_EMPTY_STATE: {
            bool roomy = spec->bounds.width >= 240U && spec->bounds.height >= 112U;
            uint32_t icon_size = roomy ? 40U : 24U;
            uint32_t icon_top = roomy ? 8U : 3U;
            uint32_t heading_top = roomy ? 54U : 29U;
            uint32_t heading_size = roomy ? ui_theme_type_scale().empty_heading_pt : 16U;
            uint32_t message_top = roomy ? 86U : 53U;
            ui_rect icon_rect = {spec->bounds.x + (spec->bounds.width - icon_size) / 2U,
                spec->bounds.y + icon_top, icon_size, icon_size};
            if (!draw_icon(canvas, spec->icon == UI_ICON_NONE ? UI_ICON_LAYERS : spec->icon,
                    icon_rect, UI_CONTROL_REST)) return false;
            ui_rect heading = {spec->bounds.x + 6U, spec->bounds.y + heading_top,
                spec->bounds.width > 12U ? spec->bounds.width - 12U : 0U,
                heading_size + 6U};
            if (!draw_text(canvas, spec->label, spec->label_length, heading,
                    canvas->tokens.text_primary, UI_FONT_SYSTEM, UI_FONT_SEMIBOLD,
                    heading_size)) return false;
            ui_rect message = {spec->bounds.x + 8U, spec->bounds.y + message_top,
                spec->bounds.width > 16U ? spec->bounds.width - 16U : 0U,
                roomy ? 24U : 20U};
            return draw_text(canvas, spec->secondary, spec->secondary_length, message,
                canvas->tokens.text_secondary, UI_FONT_SYSTEM, UI_FONT_REGULAR,
                roomy ? 12U : 11U);
        }
        case UI_WIDGET_INLINE_ERROR:
        case UI_WIDGET_TOAST: {
            if (spec->kind == UI_WIDGET_TOAST) draw_shadow(canvas, spec->bounds);
            uint32_t surface = spec->state == UI_CONTROL_DISABLED ? canvas->tokens.panel_raised :
                canvas->tokens.panel;
            uint32_t tone_color = spec->tone == UI_WIDGET_TONE_SUCCESS ? canvas->tokens.success :
                spec->tone == UI_WIDGET_TONE_WARNING ? canvas->tokens.warning :
                spec->tone == UI_WIDGET_TONE_NEUTRAL ? canvas->tokens.accent : canvas->tokens.error;
            if (spec->kind == UI_WIDGET_INLINE_ERROR && spec->tone == UI_WIDGET_TONE_NEUTRAL)
                tone_color = canvas->tokens.error;
            paint_round(canvas, spec->bounds, 6U, surface, tone_color, 1U);
            if (!draw_icon(canvas, UI_ICON_CLOSE,
                    (ui_rect){spec->bounds.x + 8U, spec->bounds.y + 8U, 14U, 14U},
                    spec->kind == UI_WIDGET_TOAST ? UI_CONTROL_REST : UI_CONTROL_INVALID)) return false;
            ui_rect label = {spec->bounds.x + 28U, spec->bounds.y + 1U,
                spec->bounds.width > 36U ? spec->bounds.width - 36U : 0U, 17U};
            ui_rect detail = {label.x, spec->bounds.y + 18U, label.width, 17U};
            return draw_text(canvas, spec->label, spec->label_length, label,
                    tone_color, UI_FONT_SYSTEM, UI_FONT_SEMIBOLD, 11U) &&
                draw_text(canvas, spec->secondary, spec->secondary_length,
                    detail, canvas->tokens.text_secondary, UI_FONT_SYSTEM,
                    UI_FONT_REGULAR, 10U);
        }
        case UI_WIDGET_NUMBER_FIELD:
        case UI_WIDGET_SCRUB_LABEL:
        case UI_WIDGET_STEPPER:
        case UI_WIDGET_COLOR_FIELD:
        case UI_WIDGET_SLIDER:
        case UI_WIDGET_DROPDOWN:
        case UI_WIDGET_TEXT_FIELD:
            return draw_input(canvas, spec);
        case UI_WIDGET_KIND_COUNT: return false;
    }
    return false;
}

bool ui_widget_paint(ui_widget_canvas *canvas, const ui_widget_draw_spec *spec)
{
    if (!canvas_valid(canvas) || spec == NULL || spec->kind < UI_WIDGET_ICON_BUTTON ||
        spec->kind >= UI_WIDGET_KIND_COUNT || spec->state < UI_CONTROL_REST ||
        spec->state > UI_CONTROL_INVALID || spec->bounds.width == 0U ||
        spec->bounds.height == 0U || spec->bounds.width > UI_WIDGET_MAX_CANVAS_AXIS ||
        spec->bounds.height > UI_WIDGET_MAX_CANVAS_AXIS ||
        spec->label_length > UI_TEXT_TOTAL_MAX_BYTES ||
        spec->value_length > UI_TEXT_TOTAL_MAX_BYTES ||
        spec->secondary_length > UI_TEXT_TOTAL_MAX_BYTES ||
        spec->unit_length > UI_WIDGET_UNIT_MAX_BYTES ||
        spec->tone < UI_WIDGET_TONE_NEUTRAL || spec->tone > UI_WIDGET_TONE_ERROR ||
        (spec->label_length > 0U && spec->label == NULL) ||
        (spec->value_length > 0U && spec->value == NULL) ||
        (spec->secondary_length > 0U && spec->secondary == NULL) ||
        (spec->unit_length > 0U && spec->unit == NULL)) return false;
    ui_widget_draw_spec adjusted = *spec;
    if (adjusted.kind == UI_WIDGET_TOAST && adjusted.bounds.width > 360U)
        adjusted.bounds.width = 360U;
    bool result = paint_component(canvas, &adjusted);
    ui_rect focus_bounds = adjusted.bounds;
    if (adjusted.kind == UI_WIDGET_DROPDOWN && adjusted.expanded) focus_bounds.height = 30U;
    if (result && adjusted.state == UI_CONTROL_FOCUS) paint_focus(canvas, focus_bounds);
    return result;
}

size_t ui_widget_paint_scratch_limit(void)
{
    return 0U;
}
