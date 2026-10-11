#include "viewer_home.h"

#include <math.h>
#include <string.h>

#define HOME_MAX_AXIS 8192U
#define HOME_MAX_BYTES (128U * 1024U * 1024U)
#define HOME_RECENT_ROW_HEIGHT 30U
#define HOME_MIN_SCALE 0.5
#define HOME_MAX_SCALE 4.0

static const uint8_t home_glyphs[36][7] = {
    {14,17,17,31,17,17,17}, {30,17,17,30,17,17,30}, {14,17,16,16,16,17,14},
    {30,17,17,17,17,17,30}, {31,16,16,30,16,16,31}, {31,16,16,30,16,16,16},
    {14,17,16,23,17,17,15}, {17,17,17,31,17,17,17}, {14,4,4,4,4,4,14},
    {7,2,2,2,18,18,12}, {17,18,20,24,20,18,17}, {16,16,16,16,16,16,31},
    {17,27,21,21,17,17,17}, {17,25,21,19,17,17,17}, {14,17,17,17,17,17,14},
    {30,17,17,30,16,16,16}, {14,17,17,17,21,18,13}, {30,17,17,30,20,18,17},
    {15,16,16,14,1,1,30}, {31,4,4,4,4,4,4}, {17,17,17,17,17,17,14},
    {17,17,17,17,17,10,4}, {17,17,17,21,21,21,10}, {17,17,10,4,10,17,17},
    {17,17,10,4,4,4,4}, {31,1,2,4,8,16,31}, {14,17,19,21,25,17,14},
    {4,12,4,4,4,4,14}, {14,17,1,2,4,8,31}, {30,1,1,14,1,1,30},
    {2,6,10,18,31,2,2}, {31,16,16,30,1,1,30}, {14,16,16,30,17,17,14},
    {31,1,2,4,8,8,8}, {14,17,17,14,17,17,14}, {14,17,17,15,1,1,14}
};

static bool frame_valid(uint8_t *rgba, size_t capacity, size_t stride,
                        uint32_t width, uint32_t height)
{
    if (rgba == NULL || width == 0U || height == 0U || width > HOME_MAX_AXIS ||
        height > HOME_MAX_AXIS || (size_t)width > SIZE_MAX / 4U) return false;
    size_t row_bytes = (size_t)width * 4U;
    if (stride < row_bytes || (size_t)(height - 1U) > (SIZE_MAX - row_bytes) / stride)
        return false;
    size_t required = (size_t)(height - 1U) * stride + row_bytes;
    return required <= capacity && required <= HOME_MAX_BYTES;
}

static uint32_t scale_value(double scale)
{
    if (!isfinite(scale) || scale < HOME_MIN_SCALE || scale > HOME_MAX_SCALE) return 0U;
    double rounded = floor(scale + 0.5);
    if (rounded < 1.0) rounded = 1.0;
    if (rounded > 4.0) rounded = 4.0;
    return (uint32_t)rounded;
}

static ui_rect scaled_rect(double x, double y, double width, double height, uint32_t scale)
{
    ui_rect rect = {0U};
    if (scale == 0U || !(width > 0.0) || !(height > 0.0)) return rect;
    double sx = floor(x * (double)scale);
    double sy = floor(y * (double)scale);
    double sw = floor(width * (double)scale);
    double sh = floor(height * (double)scale);
    if (!isfinite(sx) || !isfinite(sy) || !isfinite(sw) || !isfinite(sh) ||
        sx < 0.0 || sy < 0.0 || sw < 1.0 || sh < 1.0 ||
        sx > (double)UINT32_MAX || sy > (double)UINT32_MAX ||
        sw > (double)UINT32_MAX || sh > (double)UINT32_MAX) return rect;
    rect.x = (uint32_t)sx;
    rect.y = (uint32_t)sy;
    rect.width = (uint32_t)sw;
    rect.height = (uint32_t)sh;
    return rect;
}

static bool rect_inside(ui_rect rect, uint32_t width, uint32_t height)
{
    return rect.width > 0U && rect.height > 0U && rect.x <= width && rect.y <= height &&
        rect.width <= width - rect.x && rect.height <= height - rect.y;
}

static bool rect_contains(ui_rect rect, uint32_t x, uint32_t y)
{
    return x >= rect.x && y >= rect.y && x - rect.x < rect.width &&
        y - rect.y < rect.height;
}

static size_t bounded_text_length(const char *text, size_t cap)
{
    size_t length = 0U;
    if (text == NULL) return 0U;
    while (length < cap && text[length] != '\0') length++;
    return length;
}

static bool layout_for_empty(const viewer_home_state *state, uint32_t width,
                             uint32_t height, uint32_t scale,
                             viewer_home_layout *out)
{
    double point_width = (double)width / scale;
    double point_height = (double)height / scale;
    double inset = point_width < 360.0 ? 16.0 : 28.0;
    double card_width = fmin(560.0, point_width - inset * 2.0);
    if (card_width < 160.0) return false;
    size_t recent_count = state->recent_count;
    if (recent_count > VIEWER_HOME_RECENT_MAX) recent_count = VIEWER_HOME_RECENT_MAX;
    if (state->loading || state->load_failed) recent_count = 0U;
    double maximum_height = point_height - 32.0;
    while (recent_count > 0U &&
        250.0 + (double)(recent_count - 1U) * HOME_RECENT_ROW_HEIGHT > maximum_height)
        recent_count--;
    double card_height = state->loading ? 220.0 : state->load_failed ?
        (card_width < 420.0 ? 270.0 : 230.0) :
        (recent_count == 0U ? 300.0 : 250.0 + (double)(recent_count - 1U) * HOME_RECENT_ROW_HEIGHT);
    if (card_height > maximum_height) card_height = maximum_height;
    if (card_height < 152.0) return false;
    double left = (point_width - card_width) * 0.5;
    double top = (point_height - card_height) * 0.5;
    double button_width = fmin(184.0, card_width - 40.0);
    if (state->load_failed) {
        if (card_width < 420.0) {
            button_width = fmin(184.0, card_width - 32.0);
            double button_left = left + (card_width - button_width) * 0.5;
            out->open_button = scaled_rect(button_left, top + 112.0,
                button_width, 40.0, scale);
            out->retry_button = scaled_rect(button_left, top + 160.0,
                button_width, 40.0, scale);
        } else {
            out->open_button = scaled_rect(left + card_width * 0.5 - button_width - 6.0,
                top + 126.0, button_width, 40.0, scale);
            out->retry_button = scaled_rect(left + card_width * 0.5 + 6.0,
                top + 126.0, fmin(112.0, button_width), 40.0, scale);
        }
    } else if (!state->loading) {
        out->open_button = scaled_rect(left + (card_width - button_width) * 0.5,
            top + 102.0, button_width, 40.0, scale);
    }
    double recent_top = top + 168.0;
    for (size_t index = 0U; index < recent_count; ++index) {
        out->recent_rows[index] = scaled_rect(left + 28.0,
            recent_top + (double)index * HOME_RECENT_ROW_HEIGHT,
            card_width - 56.0, 28.0, scale);
    }
    out->recent_count = recent_count;
    out->drop_zone = scaled_rect(left, top, card_width, card_height, scale);
    return rect_inside(out->drop_zone, width, height) &&
        (state->loading || rect_inside(out->open_button, width, height)) &&
        (!state->load_failed || rect_inside(out->retry_button, width, height));
}

bool viewer_home_layout_get(const viewer_home_state *state, uint32_t width,
                            uint32_t height, double backing_scale,
                            viewer_home_layout *out_layout)
{
    if (state == NULL || out_layout == NULL || width == 0U || height == 0U ||
        width > HOME_MAX_AXIS || height > HOME_MAX_AXIS ||
        (size_t)width > SIZE_MAX / 4U || !isfinite(backing_scale) ||
        backing_scale < HOME_MIN_SCALE || backing_scale > HOME_MAX_SCALE) return false;
    uint32_t scale = scale_value(backing_scale);
    if (scale == 0U) return false;
    *out_layout = (viewer_home_layout){0};
    if (state->has_document) {
        ui_rect canvas = state->canvas;
        if (!rect_inside(canvas, width, height)) canvas = (ui_rect){0U, 0U, width, height};
        if (state->drop_hover) out_layout->drop_zone = canvas;
        if (state->load_failed) {
            uint32_t inset = 16U * scale;
            uint32_t button_height = 36U * scale;
            uint32_t button_width = 136U * scale;
            if (canvas.width <= inset * 2U || canvas.height <= inset * 2U + button_height)
                return false;
            uint32_t card_x = canvas.x + 12U * scale;
            uint32_t card_y = canvas.y + 12U * scale;
            uint32_t card_width = canvas.width - 24U * scale;
            uint32_t maximum_card_width = 400U * scale;
            if (card_width > maximum_card_width) card_width = maximum_card_width;
            uint32_t banner_height = button_height + 58U * scale;
            if (banner_height > canvas.height - inset * 2U) return false;
            out_layout->drop_zone = state->drop_hover ? canvas : (ui_rect){0U};
            out_layout->notice = (ui_rect){card_x, card_y, card_width, banner_height};
            uint32_t left = card_x + 12U * scale;
            uint32_t top = card_y;
            uint32_t width_available = card_width - 24U * scale;
            out_layout->open_button = (ui_rect){left, top + 48U * scale,
                button_width, button_height};
            out_layout->retry_button = (ui_rect){left + button_width + 8U * scale,
                top + 48U * scale, button_width, button_height};
            if (out_layout->open_button.width + out_layout->retry_button.width + 8U * scale >
                width_available) {
                out_layout->open_button.width = width_available / 2U - 4U * scale;
                out_layout->retry_button.x = left + out_layout->open_button.width + 8U * scale;
                out_layout->retry_button.width = width_available -
                    out_layout->open_button.width - 8U * scale;
            }
            if (!rect_inside(out_layout->open_button, width, height) ||
                !rect_inside(out_layout->retry_button, width, height)) return false;
        } else if (state->loading) {
            uint32_t inset = 12U * scale;
            uint32_t available_width = canvas.width > inset * 2U ?
                canvas.width - inset * 2U : 0U;
            uint32_t notice_width = available_width < 304U * scale ?
                available_width : 304U * scale;
            if (notice_width == 0U || canvas.height <= inset * 2U + 38U * scale)
                return false;
            out_layout->notice = (ui_rect){canvas.x + inset, canvas.y + inset,
                notice_width, 38U * scale};
        }
        return true;
    }
    out_layout->drop_zone = (ui_rect){0U, 0U, width, height};
    return layout_for_empty(state, width, height, scale, out_layout);
}

viewer_home_action viewer_home_pointer_down(const viewer_home_state *state,
    uint32_t width, uint32_t height, double backing_scale, uint32_t x, uint32_t y)
{
    viewer_home_layout layout = {0};
    if (!viewer_home_layout_get(state, width, height, backing_scale, &layout))
        return (viewer_home_action){VIEWER_HOME_ACTION_NONE, 0U};
    if (!state->loading && rect_contains(layout.open_button, x, y))
        return (viewer_home_action){VIEWER_HOME_ACTION_OPEN, 0U};
    if (state->load_failed && rect_contains(layout.retry_button, x, y))
        return (viewer_home_action){VIEWER_HOME_ACTION_RETRY, 0U};
    for (size_t index = 0U; index < layout.recent_count; ++index) {
        if (rect_contains(layout.recent_rows[index], x, y))
            return (viewer_home_action){VIEWER_HOME_ACTION_RECENT, index};
    }
    return (viewer_home_action){VIEWER_HOME_ACTION_NONE, 0U};
}

static void color_components(uint32_t packed, uint8_t *red, uint8_t *green,
                             uint8_t *blue)
{
    *red = (uint8_t)(packed >> 24U);
    *green = (uint8_t)(packed >> 16U);
    *blue = (uint8_t)(packed >> 8U);
}

static void fill_rect(uint8_t *rgba, size_t stride, uint32_t width, uint32_t height,
                      ui_rect rect, uint32_t color)
{
    if (!rect_inside(rect, width, height)) return;
    uint8_t red, green, blue;
    color_components(color, &red, &green, &blue);
    uint32_t alpha = color & 0xffU;
    for (uint32_t y = rect.y; y < rect.y + rect.height; ++y) {
        for (uint32_t x = rect.x; x < rect.x + rect.width; ++x) {
            uint8_t *pixel = rgba + (size_t)y * stride + (size_t)x * 4U;
            pixel[0] = (uint8_t)(((uint32_t)red * alpha +
                (uint32_t)pixel[0] * (255U - alpha)) / 255U);
            pixel[1] = (uint8_t)(((uint32_t)green * alpha +
                (uint32_t)pixel[1] * (255U - alpha)) / 255U);
            pixel[2] = (uint8_t)(((uint32_t)blue * alpha +
                (uint32_t)pixel[2] * (255U - alpha)) / 255U);
            pixel[3] = 255U;
        }
    }
}

static const uint8_t *glyph_for(char character)
{
    if (character >= 'a' && character <= 'z') character = (char)(character - 'a' + 'A');
    if (character >= 'A' && character <= 'Z')
        return home_glyphs[(size_t)(character - 'A')];
    if (character >= '0' && character <= '9')
        return home_glyphs[26U + (size_t)(character - '0')];
    return NULL;
}

static void draw_text(uint8_t *rgba, size_t stride, uint32_t width, uint32_t height,
                      int64_t x, int64_t y, const char *text, uint32_t packed,
                      uint32_t glyph_scale, size_t text_limit)
{
    if (text == NULL || glyph_scale == 0U || glyph_scale > 8U) return;
    uint8_t red, green, blue;
    color_components(packed, &red, &green, &blue);
    size_t length = bounded_text_length(text, text_limit);
    int64_t cursor = x;
    for (size_t index = 0U; index < length; ++index) {
        char character = text[index];
        const uint8_t *glyph = glyph_for(character);
        static const uint8_t dash[7] = {0,0,0,31,0,0,0};
        static const uint8_t dot[7] = {0,0,0,0,0,6,6};
        static const uint8_t colon[7] = {0,4,4,0,4,4,0};
        if (character == '-') glyph = dash;
        else if (character == '.' || character == ',') glyph = dot;
        else if (character == ':') glyph = colon;
        if (glyph != NULL) {
            for (uint32_t row = 0U; row < 7U; ++row) {
                for (uint32_t column = 0U; column < 5U; ++column) {
                    if ((glyph[row] & (uint8_t)(1U << (4U - column))) == 0U) continue;
                    for (uint32_t sy = 0U; sy < glyph_scale; ++sy) {
                        for (uint32_t sx = 0U; sx < glyph_scale; ++sx) {
                            int64_t px = cursor + (int64_t)(column * glyph_scale + sx);
                            int64_t py = y + (int64_t)(row * glyph_scale + sy);
                            if (px < 0 || py < 0 || (uint64_t)px >= width ||
                                (uint64_t)py >= height) continue;
                            uint8_t *pixel = rgba + (size_t)py * stride + (size_t)px * 4U;
                            pixel[0] = red;
                            pixel[1] = green;
                            pixel[2] = blue;
                            pixel[3] = 255U;
                        }
                    }
                }
            }
        }
        cursor += (int64_t)(6U * glyph_scale);
        if (cursor >= (int64_t)width) break;
    }
}

static void paint_button(uint8_t *rgba, size_t stride, uint32_t width, uint32_t height,
                         ui_rect rect, uint32_t fill, uint32_t label_color,
                         const char *label, uint32_t scale)
{
    fill_rect(rgba, stride, width, height, rect, fill);
    size_t length = bounded_text_length(label, 32U);
    int64_t text_width = (int64_t)length * (int64_t)(6U * scale);
    int64_t x = (int64_t)rect.x + ((int64_t)rect.width - text_width) / 2;
    int64_t y = (int64_t)rect.y + ((int64_t)rect.height - (int64_t)(7U * scale)) / 2;
    draw_text(rgba, stride, width, height, x, y, label, label_color, scale, 32U);
}

static void paint_empty(uint8_t *rgba, size_t stride, uint32_t width, uint32_t height,
                        const viewer_home_state *state, const viewer_home_layout *layout,
                        const ui_tokens *tokens, uint32_t scale)
{
    fill_rect(rgba, stride, width, height, (ui_rect){0U, 0U, width, height}, tokens->window);
    ui_rect card = layout->drop_zone;
    ui_rect shadow = card;
    shadow.x += 5U * scale;
    shadow.y += 7U * scale;
    fill_rect(rgba, stride, width, height, shadow, tokens->popover_shadow);
    fill_rect(rgba, stride, width, height, card, tokens->panel);
    ui_rect top_rule = {card.x, card.y, card.width, 4U * scale};
    fill_rect(rgba, stride, width, height, top_rule, tokens->accent);
    int64_t center_x = (int64_t)card.x + (int64_t)card.width / 2;
    uint32_t glyph_scale = scale * 2U;
    if (state->loading) {
        draw_text(rgba, stride, width, height, center_x - 66 * scale,
            (int64_t)card.y + 66 * scale, "LOADING SVG", tokens->text_primary,
            glyph_scale, 48U);
        draw_text(rgba, stride, width, height, center_x - 105 * scale,
            (int64_t)card.y + 98 * scale, "PREPARING YOUR PREVIEW",
            tokens->text_secondary, scale, 52U);
        ui_rect track = {card.x + 32U * scale, card.y + 148U * scale,
            card.width - 64U * scale, 6U * scale};
        fill_rect(rgba, stride, width, height, track, tokens->separator);
        ui_rect progress = track;
        progress.width = track.width / 3U;
        fill_rect(rgba, stride, width, height, progress, tokens->accent);
    } else if (state->load_failed) {
        draw_text(rgba, stride, width, height, center_x - 126 * scale,
            (int64_t)card.y + 26 * scale, "COULD NOT OPEN SVG", tokens->error,
            glyph_scale, 48U);
        const char *error = state->load_error == NULL || state->load_error[0] == '\0' ?
            "SVG COULD NOT BE READ" : state->load_error;
        draw_text(rgba, stride, width, height, (int64_t)card.x + 28 * scale,
            (int64_t)card.y + 68 * scale, error, tokens->text_secondary,
            scale, 64U);
        paint_button(rgba, stride, width, height, layout->open_button, tokens->accent,
            UINT32_C(0xffffffff), "OPEN SVG...", scale);
        paint_button(rgba, stride, width, height, layout->retry_button,
            tokens->panel_raised, tokens->text_primary, "RETRY", scale);
    } else {
        draw_text(rgba, stride, width, height, center_x - 30 * scale,
            (int64_t)card.y + 24 * scale, "VEXEL", tokens->accent,
            glyph_scale, 16U);
        draw_text(rgba, stride, width, height, center_x - 93 * scale,
            (int64_t)card.y + 56 * scale, "START WITH AN SVG", tokens->text_primary,
            glyph_scale, 48U);
        draw_text(rgba, stride, width, height, center_x - 105 * scale,
            (int64_t)card.y + 88 * scale, "EDIT SVG FILES STORED ON THIS MAC",
            tokens->text_secondary, scale, 56U);
        paint_button(rgba, stride, width, height, layout->open_button, tokens->accent,
            UINT32_C(0xffffffff), "OPEN SVG...", scale);
        if (layout->recent_count > 0U) {
            draw_text(rgba, stride, width, height, (int64_t)card.x + 28 * scale,
                (int64_t)card.y + 145 * scale, "RECENT FILES", tokens->text_tertiary,
                scale, 32U);
            for (size_t index = 0U; index < layout->recent_count; ++index) {
                ui_rect row = layout->recent_rows[index];
                fill_rect(rgba, stride, width, height, row,
                    (index & 1U) == 0U ? tokens->panel : tokens->panel_raised);
                draw_text(rgba, stride, width, height, (int64_t)row.x + 10 * scale,
                    (int64_t)row.y + 9 * scale, state->recent_labels[index],
                    tokens->text_primary, scale, VIEWER_HOME_RECENT_LABEL_CAP - 1U);
            }
        }
        draw_text(rgba, stride, width, height, center_x - 93 * scale,
            (int64_t)card.y + (int64_t)card.height - 31 * scale,
            "DROP AN SVG FILE HERE", tokens->text_tertiary, scale, 40U);
    }
}

static void paint_drop_highlight(uint8_t *rgba, size_t stride, uint32_t width,
                                 uint32_t height, ui_rect target, uint32_t scale,
                                 const ui_tokens *tokens)
{
    if (!rect_inside(target, width, height)) return;
    uint32_t thickness = 3U * scale;
    fill_rect(rgba, stride, width, height,
        (ui_rect){target.x, target.y, target.width, thickness}, tokens->accent);
    fill_rect(rgba, stride, width, height,
        (ui_rect){target.x, target.y + target.height - thickness, target.width, thickness},
        tokens->accent);
    fill_rect(rgba, stride, width, height,
        (ui_rect){target.x, target.y, thickness, target.height}, tokens->accent);
    fill_rect(rgba, stride, width, height,
        (ui_rect){target.x + target.width - thickness, target.y, thickness, target.height},
        tokens->accent);
    const char *label = "DROP SVG TO OPEN IN A NEW WINDOW";
    size_t length = bounded_text_length(label, 48U);
    int64_t text_width = (int64_t)length * 6 * (int64_t)scale;
    int64_t x = (int64_t)target.x + ((int64_t)target.width - text_width) / 2;
    int64_t y = (int64_t)target.y + 16 * (int64_t)scale;
    ui_rect backing = {(uint32_t)(x - 8 * (int64_t)scale),
        (uint32_t)(y - 5 * (int64_t)scale),
        (uint32_t)(text_width + 16 * (int64_t)scale), 20U * scale};
    fill_rect(rgba, stride, width, height, backing, tokens->panel);
    draw_text(rgba, stride, width, height, x, y, label, tokens->text_primary,
        scale, 48U);
}

static void paint_document_notice(uint8_t *rgba, size_t stride, uint32_t width,
                                  uint32_t height, const viewer_home_state *state,
                                  const viewer_home_layout *layout,
                                  const ui_tokens *tokens, uint32_t scale)
{
    if (state->load_failed) {
        ui_rect card = layout->notice;
        fill_rect(rgba, stride, width, height, card, tokens->panel);
        draw_text(rgba, stride, width, height, (int64_t)card.x + 14 * scale,
            (int64_t)card.y + 12 * scale, "COULD NOT OPEN SVG - PREVIEW RETAINED",
            tokens->error, scale, 56U);
        const char *message = state->load_error == NULL ? "OPEN FAILED" : state->load_error;
        draw_text(rgba, stride, width, height, (int64_t)card.x + 14 * scale,
            (int64_t)card.y + 32 * scale, message, tokens->text_secondary, scale, 56U);
        paint_button(rgba, stride, width, height, layout->open_button, tokens->accent,
            UINT32_C(0xffffffff), "OPEN SVG...", scale);
        paint_button(rgba, stride, width, height, layout->retry_button,
            tokens->panel_raised, tokens->text_primary, "RETRY", scale);
    } else if (state->loading) {
        ui_rect card = layout->notice;
        fill_rect(rgba, stride, width, height, card, tokens->panel);
        draw_text(rgba, stride, width, height, (int64_t)card.x + 12 * scale,
            (int64_t)card.y + 12 * scale, "LOADING SVG - PREVIEW RETAINED",
            tokens->text_primary, scale, 48U);
    }
}

void viewer_home_paint(const viewer_home_state *state, ui_theme theme,
    uint8_t *rgba, size_t capacity, size_t stride, uint32_t width, uint32_t height,
    double backing_scale)
{
    if (state == NULL || (theme != UI_THEME_DARK && theme != UI_THEME_LIGHT) ||
        !frame_valid(rgba, capacity, stride, width, height)) return;
    viewer_home_layout layout = {0};
    if (!viewer_home_layout_get(state, width, height, backing_scale, &layout)) return;
    uint32_t scale = scale_value(backing_scale);
    ui_tokens tokens = ui_theme_tokens(theme, false);
    if (!state->has_document) paint_empty(rgba, stride, width, height, state,
        &layout, &tokens, scale);
    else if (state->loading || state->load_failed) paint_document_notice(rgba, stride,
        width, height, state, &layout, &tokens, scale);
    if (state->drop_hover) paint_drop_highlight(rgba, stride, width, height,
        layout.drop_zone, scale, &tokens);
}
