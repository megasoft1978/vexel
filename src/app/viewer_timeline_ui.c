#include "viewer_timeline.h"

#include "platform/platform_text_ui.h"
#include "ui/ui_draw.h"
#include "ui/ui_widget_paint_internal.h"
#include "ui/ui_widgets.h"
#include "viewer_chrome.h"
#include "viewer_editor.h"

#include <math.h>
#include <stdio.h>
#include <string.h>

static bool paint_widget(ui_widget_canvas *canvas, ui_widget_kind kind,
    ui_rect pixels, ui_control_state state,
    const char *label, size_t label_length, const char *value, size_t value_length,
    double numeric_value, bool checked);
static size_t utf8_prefix(const char *text, size_t length, size_t maximum);

static bool timeline_is_indefinite(const viewer_timeline *timeline)
{
    if (timeline == NULL || timeline->document == NULL) return false;
    size_t count = viewer_timeline_track_count(timeline);
    for (size_t index = 0U; index < count; ++index) {
        const svg_anim_track *track = &timeline->document->animation.tracks[index];
        if (track->duration_indefinite || track->repeat_indefinite ||
            track->repeat_duration_indefinite) return true;
    }
    return false;
}

static void scalar_ascii_text(const char *text, size_t length,
    char *out, size_t capacity, size_t *out_length)
{
    if (out_length != NULL) *out_length = 0U;
    if (text == NULL || out == NULL || out_length == NULL || capacity == 0U) return;
    size_t input = 0U, output = 0U;
    while (input < length && output < capacity - 1U) {
        const uint8_t byte = (uint8_t)text[input];
        const char *replacement = NULL;
        size_t consumed = 1U;
        if (input + 2U < length && byte == 0xe2U &&
            (uint8_t)text[input + 1U] == 0x88U &&
            (uint8_t)text[input + 2U] == 0x9eU) {
            replacement = "INF";
            consumed = 3U;
        } else if (input + 2U < length && byte == 0xe2U &&
            (uint8_t)text[input + 1U] == 0x80U &&
            ((uint8_t)text[input + 2U] == 0xa2U ||
                (uint8_t)text[input + 2U] == 0x93U)) {
            replacement = (uint8_t)text[input + 2U] == 0xa2U ? " " : "-";
            consumed = 3U;
        } else if (input + 1U < length && byte == 0xc3U &&
            (uint8_t)text[input + 1U] == 0x97U) {
            replacement = "x";
            consumed = 2U;
        }
        if (replacement != NULL) {
            size_t amount = strlen(replacement);
            if (amount > capacity - 1U - output) break;
            memcpy(out + output, replacement, amount);
            output += amount;
            input += consumed;
            continue;
        }
        if (byte < 0x80U) {
            out[output++] = (char)byte;
            input++;
            continue;
        }
        out[output++] = '?';
        input++;
        while (input < length && (((uint8_t)text[input] & 0xc0U) == 0x80U))
            input++;
    }
    out[output] = '\0';
    *out_length = output;
}

static bool scalar_timeline_text(void *context, const char *text, size_t length,
    ui_rect bounds, ui_theme theme, uint32_t font_size_px, uint32_t color,
    ui_font_family family, ui_font_weight weight, uint8_t *rgba,
    uint32_t width, uint32_t height, size_t stride)
{
    (void)context;
    (void)theme;
    (void)family;
    (void)weight;
    char ascii[256];
    size_t ascii_length = 0U;
    scalar_ascii_text(text, length, ascii, sizeof(ascii), &ascii_length);
    return ui_scalar_text_draw(ascii, ascii_length, bounds, font_size_px,
        color, rgba, width, height, stride);
}

static uint32_t widget_scale_for(double backing_scale)
{
    long scale = lround(backing_scale);
    if (scale < 1L) scale = 1L;
    if (scale > 3L) scale = 3L;
    return (uint32_t)scale;
}

static bool rect_to_widget(ui_rect pixels, uint32_t scale, ui_rect *out)
{
    if (out == NULL || scale == 0U || pixels.width == 0U || pixels.height == 0U)
        return false;
    *out = (ui_rect){pixels.x / scale, pixels.y / scale,
        pixels.width / scale + (pixels.width % scale != 0U ? 1U : 0U),
        pixels.height / scale + (pixels.height % scale != 0U ? 1U : 0U)};
    return out->width > 0U && out->height > 0U;
}

static void blend_panel_pixel(uint8_t *destination, uint32_t color)
{
    uint32_t alpha = color & 255U;
    if (alpha == 255U) {
        destination[0] = (uint8_t)(color >> 24U);
        destination[1] = (uint8_t)(color >> 16U);
        destination[2] = (uint8_t)(color >> 8U);
        destination[3] = 255U;
        return;
    }
    if (alpha == 0U) return;
    uint32_t inverse = 255U - alpha;
    uint32_t old_alpha = destination[3];
    uint32_t out_alpha = alpha + (old_alpha * inverse + 127U) / 255U;
    if (out_alpha == 0U) return;
    for (size_t channel = 0U; channel < 3U; ++channel) {
        uint32_t shift = 24U - (uint32_t)channel * 8U;
        uint32_t source = (color >> shift) & 255U;
        uint32_t numerator = source * alpha * 255U +
            (uint32_t)destination[channel] * old_alpha * inverse;
        destination[channel] = (uint8_t)((numerator + out_alpha * 127U) /
            (out_alpha * 255U));
    }
    destination[3] = (uint8_t)out_alpha;
}

static void paint_panel_background(const ui_widget_canvas *canvas, ui_rect panel)
{
    uint64_t right = (uint64_t)panel.x + panel.width;
    uint64_t bottom = (uint64_t)panel.y + panel.height;
    uint32_t x_end = right > canvas->width ? canvas->width : (uint32_t)right;
    uint32_t y_end = bottom > canvas->height ? canvas->height : (uint32_t)bottom;
    if (panel.x >= x_end || panel.y >= y_end) return;
    for (uint32_t y = panel.y; y < y_end; ++y) {
        for (uint32_t x = panel.x; x < x_end; ++x) {
            blend_panel_pixel(canvas->rgba + (size_t)y * canvas->stride +
                (size_t)x * 4U, canvas->tokens.panel);
        }
    }
}

static void paint_hline(const ui_widget_canvas *canvas, uint32_t x,
                        uint32_t y, uint32_t width, uint32_t color)
{
    if (canvas == NULL || y >= canvas->height || x >= canvas->width) return;
    uint32_t end = width > canvas->width - x ? canvas->width : x + width;
    for (uint32_t at = x; at < end; ++at)
        blend_panel_pixel(canvas->rgba + (size_t)y * canvas->stride +
            (size_t)at * 4U, color);
}

static void paint_vline(const ui_widget_canvas *canvas, uint32_t x,
                        uint32_t y, uint32_t height, uint32_t color)
{
    if (canvas == NULL || x >= canvas->width || y >= canvas->height) return;
    uint32_t end = height > canvas->height - y ? canvas->height : y + height;
    for (uint32_t at = y; at < end; ++at)
        blend_panel_pixel(canvas->rgba + (size_t)at * canvas->stride +
            (size_t)x * 4U, color);
}

static void paint_diamond(const ui_widget_canvas *canvas, int32_t center_x,
                          int32_t center_y, uint32_t radius, uint32_t color)
{
    if (canvas == NULL || radius == 0U) return;
    int32_t extent = radius > (uint32_t)INT32_MAX ? INT32_MAX : (int32_t)radius;
    for (int32_t dy = -extent; dy <= extent; ++dy) {
        int32_t remaining = extent - (dy < 0 ? -dy : dy);
        for (int32_t dx = -remaining; dx <= remaining; ++dx) {
            int64_t x = (int64_t)center_x + dx;
            int64_t y = (int64_t)center_y + dy;
            if (x < 0 || y < 0 || x >= canvas->width || y >= canvas->height) continue;
            blend_panel_pixel(canvas->rgba + (size_t)y * canvas->stride +
                (size_t)x * 4U, color);
        }
    }
}

static void paint_square(const ui_widget_canvas *canvas, int32_t center_x,
                         int32_t center_y, uint32_t radius, uint32_t color)
{
    if (canvas == NULL || radius == 0U) return;
    int32_t extent = radius > (uint32_t)INT32_MAX ? INT32_MAX : (int32_t)radius;
    for (int32_t dy = -extent; dy <= extent; ++dy) {
        for (int32_t dx = -extent; dx <= extent; ++dx) {
            int64_t x = (int64_t)center_x + dx;
            int64_t y = (int64_t)center_y + dy;
            if (x < 0 || y < 0 || x >= canvas->width || y >= canvas->height) continue;
            blend_panel_pixel(canvas->rgba + (size_t)y * canvas->stride +
                (size_t)x * 4U, color);
        }
    }
}

static void paint_dashed_hline(const ui_widget_canvas *canvas, uint32_t x,
                               uint32_t y, uint32_t width, uint32_t color)
{
    if (canvas == NULL || y >= canvas->height || x >= canvas->width) return;
    uint32_t end = width > canvas->width - x ? canvas->width : x + width;
    for (uint32_t at = x; at < end; ++at)
        if ((at - x) % 6U < 3U)
            blend_panel_pixel(canvas->rgba + (size_t)y * canvas->stride +
                (size_t)at * 4U, color);
}

static double nice_tick_step(double range, double width, double scale)
{
    if (!(range > 0.0) || !(width > 0.0) || !(scale > 0.0)) return 1.0;
    double target = range / (width / (88.0 * scale));
    if (!isfinite(target) || target <= 0.0) return 1.0;
    double magnitude = pow(10.0, floor(log10(target)));
    if (!isfinite(magnitude) || magnitude <= 0.0) return target;
    double normalized = target / magnitude;
    double multiplier = normalized <= 1.0 ? 1.0 :
        normalized <= 2.0 ? 2.0 : normalized <= 5.0 ? 5.0 : 10.0;
    return multiplier * magnitude;
}

static double track_start(const svg_anim_track *track)
{
    if (track == NULL || track->has_begin_sync || track->begin_indefinite) return 0.0;
    return isfinite(track->begin_offset) && track->begin_offset > 0.0 ?
        track->begin_offset : 0.0;
}

static double track_end(const svg_anim_track *track)
{
    if (track == NULL || !track->supported || track->begin_indefinite) return 0.0;
    double start = track_start(track);
    double end = start + track->duration_seconds;
    if (track->duration_indefinite || track->repeat_indefinite ||
        track->repeat_duration_indefinite || track->kind == SVG_ANIM_KIND_SET)
        return start + 5.0;
    double repeats = isfinite(track->repeat_count) && track->repeat_count > 0.0 ?
        track->repeat_count : 1.0;
    end = start + track->duration_seconds * repeats;
    if (!isfinite(end) || end < start) return start + 5.0;
    if (isfinite(track->repeat_duration) && track->repeat_duration >= 0.0 &&
        end - start > track->repeat_duration)
        end = start + track->repeat_duration;
    if (track->has_end && !track->end_is_sync && isfinite(track->end_seconds) &&
        track->end_seconds >= start && track->end_seconds < end)
        end = track->end_seconds;
    return end;
}

static double time_to_x(const viewer_timeline *timeline,
    const viewer_timeline_layout *layout, double seconds)
{
    double span = viewer_timeline_view_duration(timeline);
    if (!(span > 0.0)) return (double)layout->ruler.x;
    return (double)layout->ruler.x +
        ((seconds - timeline->view_start_seconds) / span) * layout->ruler.width;
}

static void paint_ruler(ui_widget_canvas *canvas, const viewer_timeline *timeline,
                        const viewer_timeline_layout *layout)
{
    ui_rect grid = layout->ruler;
    paint_hline(canvas, grid.x, grid.y + grid.height - 1U, grid.width,
        canvas->tokens.separator);
    double range = viewer_timeline_view_duration(timeline);
    double step = nice_tick_step(range, (double)grid.width, layout->backing_scale);
    if (!(step > 0.0) || !isfinite(step)) return;
    double first = ceil(timeline->view_start_seconds / step) * step;
    uint32_t ticks = 0U;
    for (double seconds = first; seconds <= timeline->view_start_seconds + range &&
            ticks < 64U; seconds += step, ++ticks) {
        double x_value = time_to_x(timeline, layout, seconds);
        if (!isfinite(x_value) || x_value < 0.0 || x_value > (double)UINT32_MAX) continue;
        uint32_t x = (uint32_t)lround(x_value);
        if (x < grid.x || x >= grid.x + grid.width) continue;
        paint_vline(canvas, x, grid.y + grid.height / 2U,
            grid.height / 2U, canvas->tokens.separator);
        char label[24];
        int amount = snprintf(label, sizeof(label), step < 1.0 ? "%.1fs" : "%.0fs", seconds);
        if (amount <= 0 || (size_t)amount >= sizeof(label)) continue;
        uint32_t label_width = (uint32_t)((size_t)amount * 7U + 8U);
        ui_rect bounds = {x + 2U, grid.y, label_width, grid.height / 2U};
        (void)paint_widget(canvas, UI_WIDGET_LABEL, bounds, UI_CONTROL_REST,
            label, (size_t)amount, NULL, 0U, 0.0, false);
    }
}

static void paint_track_lane(ui_widget_canvas *canvas,
    const viewer_timeline *timeline, const viewer_timeline_layout *layout,
    size_t row, size_t track_index)
{
    const svg_anim_track *track = &timeline->document->animation.tracks[track_index];
    ui_rect row_rect = layout->track_rows[row];
    uint32_t row_center = row_rect.y + row_rect.height / 2U;
    paint_hline(canvas, layout->track_gutter.x, row_rect.y + row_rect.height - 1U,
        layout->track_gutter.width, canvas->tokens.separator);
    paint_hline(canvas, layout->time_grid.x, row_rect.y + row_rect.height - 1U,
        layout->time_grid.width, canvas->tokens.separator);
    if (track_index > 0U) {
        const svg_anim_track *previous =
            &timeline->document->animation.tracks[track_index - 1U];
        if (previous->target_offset != track->target_offset ||
            previous->target_length != track->target_length)
            paint_hline(canvas, row_rect.x, row_rect.y, row_rect.width,
                canvas->tokens.control_edge);
    }
    char label[4096] = {0};
    size_t label_length = 0U;
    if (viewer_timeline_track_label(timeline, track_index, label,
            sizeof(label), &label_length)) {
        ui_control_state state = !track->supported ? UI_CONTROL_DISABLED :
            track_index == timeline->selected_track ? UI_CONTROL_SELECTED : UI_CONTROL_REST;
        ui_rect label_bounds = {layout->track_gutter.x, row_rect.y,
            layout->track_gutter.width, row_rect.height};
        label_length = utf8_prefix(label, label_length, UI_WIDGET_LABEL_MAX_BYTES);
        (void)paint_widget(canvas, UI_WIDGET_BUTTON_SECONDARY, label_bounds, state,
            label, label_length, NULL, 0U, 0.0, false);
    }
    if (!track->supported) {
        uint32_t indicator_radius = (uint32_t)ceil(3.0 * layout->backing_scale);
        uint32_t inset = indicator_radius * 3U;
        int32_t indicator_x = (int32_t)(layout->track_gutter.x +
            (layout->track_gutter.width > inset ?
                layout->track_gutter.width - inset : 0U));
        paint_diamond(canvas, indicator_x, (int32_t)row_center,
            indicator_radius, canvas->tokens.warning);
        return;
    }
    double start = track_start(track);
    double end = track_end(track);
    double range_end = timeline->view_start_seconds +
        viewer_timeline_view_duration(timeline);
    if (end < timeline->view_start_seconds || start > range_end) return;
    double clipped_start = start < timeline->view_start_seconds ?
        timeline->view_start_seconds : start;
    double clipped_end = end > range_end ? range_end : end;
    double bar_start = time_to_x(timeline, layout, clipped_start);
    double bar_end = time_to_x(timeline, layout, clipped_end);
    if (isfinite(bar_start) && isfinite(bar_end) && bar_end >= bar_start) {
        uint32_t x0 = bar_start < (double)layout->time_grid.x ? layout->time_grid.x :
            (uint32_t)lround(bar_start);
        uint32_t grid_end = layout->time_grid.x + layout->time_grid.width;
        uint32_t x1 = bar_end > (double)grid_end ? grid_end : (uint32_t)lround(bar_end);
        if (x1 >= x0) {
            if (track->calc_mode == SVG_ANIM_CALC_DISCRETE)
                paint_dashed_hline(canvas, x0, row_center, x1 - x0 + 1U,
                    canvas->tokens.warning);
            else paint_hline(canvas, x0, row_center, x1 - x0 + 1U,
                canvas->tokens.accent);
        }
    }
    uint32_t radius = viewer_timeline_key_hit_radius(layout->backing_scale);
    if (track->keys == NULL || !(track->duration_seconds > 0.0) ||
        !isfinite(track->duration_seconds)) return;
    size_t key_count = track->key_count > SVG_ANIM_MAX_VALUES ?
        SVG_ANIM_MAX_VALUES : track->key_count;
    for (size_t key = 0U; key < key_count; ++key) {
        double seconds = start + track->keys[key].time * track->duration_seconds;
        if (!isfinite(seconds) || seconds < timeline->view_start_seconds || seconds > range_end)
            continue;
        double key_x = time_to_x(timeline, layout, seconds);
        if (!isfinite(key_x) || key_x < (double)layout->time_grid.x ||
            key_x >= (double)(layout->time_grid.x + layout->time_grid.width) ||
            key_x > (double)INT32_MAX) continue;
        int32_t center_x = (int32_t)lround(key_x);
        uint32_t color = track->calc_mode == SVG_ANIM_CALC_DISCRETE ?
            canvas->tokens.warning : canvas->tokens.accent;
        if (track->calc_mode == SVG_ANIM_CALC_DISCRETE)
            paint_square(canvas, center_x, (int32_t)row_center, radius, color);
        else paint_diamond(canvas, center_x, (int32_t)row_center, radius, color);
    }
}

static void paint_playhead(ui_widget_canvas *canvas,
    const viewer_timeline *timeline, const viewer_timeline_layout *layout)
{
    double x_value = time_to_x(timeline, layout, timeline->time_seconds);
    if (!isfinite(x_value) || x_value < (double)layout->time_grid.x ||
        x_value > (double)(layout->time_grid.x + layout->time_grid.width) ||
        x_value > (double)UINT32_MAX) return;
    uint32_t x = (uint32_t)lround(x_value);
    uint32_t bottom = layout->panel.y + layout->panel.height;
    uint32_t y = layout->ruler.y;
    if (bottom <= y) return;
    paint_vline(canvas, x, y, bottom - y, canvas->tokens.error);
    uint32_t radius = (uint32_t)ceil(4.0 * layout->backing_scale);
    paint_diamond(canvas, (int32_t)x, (int32_t)(y + 2U * radius), radius,
        canvas->tokens.error);
}

static bool paint_widget(ui_widget_canvas *canvas, ui_widget_kind kind,
    ui_rect pixels, ui_control_state state,
    const char *label, size_t label_length, const char *value, size_t value_length,
    double numeric_value, bool checked)
{
    ui_rect bounds;
    if (!rect_to_widget(pixels, canvas->backing_scale, &bounds)) return false;
    ui_widget_draw_spec spec = {
        .kind = kind,
        .state = state,
        .bounds = bounds,
        .label = label,
        .label_length = label_length,
        .value = value,
        .value_length = value_length,
        .numeric_value = numeric_value,
        .checked = checked,
        .expanded = true
    };
    return ui_widget_paint(canvas, &spec);
}

static size_t utf8_prefix(const char *text, size_t length, size_t maximum)
{
    size_t amount = length < maximum ? length : maximum;
    while (amount > 0U && amount < length &&
        (((uint8_t)text[amount] & 0xc0U) == 0x80U)) amount--;
    return amount;
}

static void paint_fit_glyph(const ui_widget_canvas *canvas, ui_rect glyph,
    uint32_t color)
{
    uint32_t stroke = glyph.width >= 14U ? 2U : 1U;
    uint32_t inset = stroke;
    uint32_t arm = glyph.width / 3U;
    if (arm <= stroke) arm = stroke + 1U;
    uint32_t right = glyph.x + glyph.width - inset - arm;
    uint32_t bottom = glyph.y + glyph.height - inset - arm;
    ui_widget_paint_rect(canvas,
        (ui_rect){glyph.x + inset, glyph.y + inset, arm, stroke}, color);
    ui_widget_paint_rect(canvas,
        (ui_rect){glyph.x + inset, glyph.y + inset, stroke, arm}, color);
    ui_widget_paint_rect(canvas,
        (ui_rect){right, glyph.y + inset, arm, stroke}, color);
    ui_widget_paint_rect(canvas,
        (ui_rect){right + arm - stroke, glyph.y + inset, stroke, arm}, color);
    ui_widget_paint_rect(canvas,
        (ui_rect){glyph.x + inset, bottom + arm - stroke, arm, stroke}, color);
    ui_widget_paint_rect(canvas,
        (ui_rect){glyph.x + inset, bottom, stroke, arm}, color);
    ui_widget_paint_rect(canvas,
        (ui_rect){right, bottom + arm - stroke, arm, stroke}, color);
    ui_widget_paint_rect(canvas,
        (ui_rect){right + arm - stroke, bottom, stroke, arm}, color);
}

static void paint_loop_glyph(const ui_widget_canvas *canvas, ui_rect glyph,
    uint32_t color)
{
    uint32_t stroke = glyph.width >= 14U ? 2U : 1U;
    uint32_t left = glyph.x + 2U;
    uint32_t right = glyph.x + glyph.width - 2U - stroke;
    uint32_t top = glyph.y + 2U;
    uint32_t bottom = glyph.y + glyph.height - 2U - stroke;
    uint32_t middle_left = glyph.x + glyph.width / 4U;
    uint32_t middle_right = glyph.x + (glyph.width * 3U) / 4U - stroke;
    ui_widget_paint_rect(canvas,
        (ui_rect){middle_left + stroke, top, middle_right - middle_left, stroke}, color);
    ui_widget_paint_rect(canvas,
        (ui_rect){right, top + stroke, stroke, glyph.height / 4U}, color);
    ui_widget_paint_rect(canvas,
        (ui_rect){right, glyph.y + glyph.height / 2U,
            stroke, glyph.height / 4U}, color);
    ui_widget_paint_rect(canvas,
        (ui_rect){middle_left + stroke, bottom,
            middle_right - middle_left, stroke}, color);
    ui_widget_paint_rect(canvas,
        (ui_rect){left, glyph.y + glyph.height / 2U,
            stroke, glyph.height / 4U}, color);
    ui_widget_paint_rect(canvas,
        (ui_rect){left, top + stroke, stroke, glyph.height / 4U}, color);
    /* Arrowhead at the upper right marks the repeating direction. */
    ui_widget_paint_rect(canvas,
        (ui_rect){right - stroke, glyph.y + 1U, stroke * 2U + 1U, stroke}, color);
    ui_widget_paint_rect(canvas,
        (ui_rect){right + stroke, glyph.y + 1U, stroke, stroke * 2U + 1U}, color);
}

/* Project-authored media glyphs; isolated from the shared icon atlas. */
static void paint_media_control(ui_widget_canvas *canvas, ui_rect pixels,
    bool pause, viewer_timeline_mode mode, bool playback, bool fit)
{
    ui_rect button;
    if (!rect_to_widget(pixels, canvas->backing_scale, &button)) return;
    ui_widget_draw_spec spec = {.kind = UI_WIDGET_BUTTON_SECONDARY,
        .bounds = button, .state = UI_CONTROL_REST};
    (void)ui_widget_paint(canvas, &spec);
    uint32_t side = 16U;
    uint32_t extent = button.width < button.height ? button.width : button.height;
    if (extent < side + 12U) side = extent > 12U ? extent - 12U : extent;
    ui_rect glyph = {button.x + (button.width - side) / 2U,
        button.y + (button.height - side) / 2U, side, side};
    uint32_t color = canvas->tokens.text_primary;
    if (fit) { paint_fit_glyph(canvas, glyph, color); return; }
    if (!playback && mode == VIEWER_TIMELINE_MODE_LOOP) {
        paint_loop_glyph(canvas, glyph, color);
        return;
    }
    if (!playback && mode == VIEWER_TIMELINE_MODE_PING_PONG) {
        ui_widget_paint_rect(canvas, (ui_rect){glyph.x, glyph.y + side / 2U,
            side, 1U}, color);
        for (uint32_t row = 0U; row < side / 4U; ++row) {
            ui_widget_paint_rect(canvas, (ui_rect){glyph.x + row,
                glyph.y + side / 2U - row, 1U, row * 2U + 1U}, color);
            ui_widget_paint_rect(canvas, (ui_rect){glyph.x + side - row - 1U,
                glyph.y + side / 2U - row, 1U, row * 2U + 1U}, color);
        }
        return;
    }
    if (playback && pause) {
        ui_widget_paint_rect(canvas, (ui_rect){glyph.x + 2U, glyph.y, 4U, side}, color);
        ui_widget_paint_rect(canvas, (ui_rect){glyph.x + side - 6U, glyph.y, 4U, side}, color);
        return;
    }
    for (uint32_t row = 0U; row < side; ++row) {
        uint32_t distance = row < side / 2U ? row : side - row - 1U;
        ui_widget_paint_rect(canvas, (ui_rect){glyph.x + 2U, glyph.y + row,
            distance + 1U, 1U}, color);
    }
}

static bool viewer_timeline_paint_with_text(viewer_timeline *timeline,
    viewer_editor *editor, uint8_t *rgba, size_t capacity, uint32_t width,
    uint32_t height, size_t stride, double backing_scale, ui_theme theme,
    bool increased_contrast, bool reduced_motion,
    ui_widget_text_draw_fn text_draw)
{
    if (timeline == NULL || editor == NULL || rgba == NULL ||
        viewer_timeline_track_count(timeline) == 0U ||
        !isfinite(backing_scale) || backing_scale < 0.5 || backing_scale > 4.0 ||
        (theme != UI_THEME_DARK && theme != UI_THEME_LIGHT))
        return false;
    if (viewer_editor_timeline_available(editor)) {
        bool requested_open = viewer_editor_timeline_visible(editor);
        if (requested_open != timeline->dock_open)
            (void)viewer_timeline_dock_set_open(timeline, requested_open);
    } else {
        (void)viewer_editor_set_timeline_state(editor, true, timeline->dock_open,
            timeline->dock_height_points);
    }
    viewer_timeline_set_reduced_motion(timeline, reduced_motion);
    viewer_timeline_sync_selection(timeline, editor);
    viewer_timeline_layout layout;
    if (!viewer_timeline_layout_compute(timeline, width, height,
            backing_scale, &layout)) return false;
    timeline->first_track = layout.first_track;
    if (timeline->reveal_selection_pending && !layout.collapsed) {
        viewer_timeline_reveal_selected(timeline, &layout);
        if (!viewer_timeline_layout_compute(timeline, width, height,
                backing_scale, &layout)) return false;
    }
    uint32_t widget_scale = widget_scale_for(backing_scale);
    ui_widget_canvas canvas;
    if (!ui_widget_canvas_init(&canvas, rgba, capacity, width, height, stride,
            widget_scale, theme, increased_contrast, text_draw, NULL)) return false;
    paint_panel_background(&canvas, layout.panel);
    paint_hline(&canvas, layout.panel.x, layout.panel.y,
        layout.panel.width, canvas.tokens.control_edge);
    (void)paint_widget(&canvas, UI_WIDGET_SECTION_HEADER, layout.header, UI_CONTROL_REST,
        "Timeline", sizeof("Timeline") - 1U, NULL, 0U, 0.0, false);
    const char *collapse_label = layout.collapsed ?
        (timeline->dock_open ? "Close" : "Expand") : "Collapse";
    (void)paint_widget(&canvas, UI_WIDGET_BUTTON_SECONDARY, layout.collapse_toggle,
        UI_CONTROL_REST, collapse_label, strlen(collapse_label),
        NULL, 0U, 0.0, false);
    if (!layout.collapsed) {
        char time_text[64];
        double duration = viewer_timeline_duration(timeline);
        bool indefinite = timeline_is_indefinite(timeline);
        int time_amount = indefinite ? (layout.compact ?
            snprintf(time_text, sizeof(time_text), "%.2f/INF", timeline->time_seconds) :
            snprintf(time_text, sizeof(time_text), "%.2fs / INF", timeline->time_seconds)) :
            (layout.compact ? snprintf(time_text, sizeof(time_text), "%.2f/%.2fs",
                timeline->time_seconds, duration) :
                snprintf(time_text, sizeof(time_text), "%.2f / %.2fs",
                    timeline->time_seconds, duration));
        if (time_amount < 0 || (size_t)time_amount >= sizeof(time_text))
            (void)snprintf(time_text, sizeof(time_text), "-- / --s");
        char speed[16];
        int speed_amount = snprintf(speed, sizeof(speed), "%.2g×", timeline->playback_speed);
        if (speed_amount < 0 || (size_t)speed_amount >= sizeof(speed))
            (void)snprintf(speed, sizeof(speed), "1×");
        (void)paint_widget(&canvas,
            timeline->preview_failed ? UI_WIDGET_INLINE_ERROR : UI_WIDGET_HUD_PILL,
            layout.time_readout, timeline->preview_failed ? UI_CONTROL_INVALID : UI_CONTROL_REST,
            timeline->preview_failed ? "Preview unavailable" : time_text,
            timeline->preview_failed ? sizeof("Preview unavailable") - 1U : strlen(time_text),
            NULL, 0U, 0.0, false);
        paint_media_control(&canvas, layout.play_button, timeline->playing,
            timeline->mode, true, false);
        if (layout.mode_button.width > 0U)
            paint_media_control(&canvas, layout.mode_button, false,
                timeline->mode, false, false);
        if (layout.speed_button.width > 0U)
            (void)paint_widget(&canvas, UI_WIDGET_BUTTON_SECONDARY, layout.speed_button,
                UI_CONTROL_REST, speed, strlen(speed), NULL, 0U, 0.0, false);
        paint_media_control(&canvas, layout.fit_button, false,
            timeline->mode, false, true);
        paint_ruler(&canvas, timeline, &layout);
        for (size_t row = 0U; row < layout.visible_track_count; ++row) {
            paint_track_lane(&canvas, timeline, &layout, row, layout.first_track + row);
            viewer_timeline_easing_paint_key_markers(timeline, &layout,
                layout.first_track + row, row, &canvas);
        }
        paint_playhead(&canvas, timeline, &layout);
    }
    viewer_timeline_easing_layout easing_layout;
    if (viewer_timeline_easing_layout_compute(&layout, &easing_layout))
        viewer_timeline_easing_paint(timeline, &layout, &easing_layout, &canvas);
    char tooltip[4096] = {0};
    ui_rect tooltip_bounds;
    if (viewer_timeline_tooltip(timeline, &layout, width, height,
            tooltip, sizeof(tooltip), &tooltip_bounds))
        (void)paint_widget(&canvas, UI_WIDGET_TOOLTIP, tooltip_bounds,
            UI_CONTROL_REST, tooltip, strlen(tooltip), NULL, 0U, 0.0, false);
    (void)viewer_timeline_publish_accessibility(timeline, editor, &layout, width, height,
        backing_scale, theme, increased_contrast);
    return true;
}

bool viewer_timeline_paint_appearance(viewer_timeline *timeline,
    viewer_editor *editor, uint8_t *rgba, size_t capacity, uint32_t width,
    uint32_t height, size_t stride, double backing_scale, ui_theme theme,
    bool increased_contrast, bool reduced_motion)
{
    return viewer_timeline_paint_with_text(timeline, editor, rgba, capacity,
        width, height, stride, backing_scale, theme, increased_contrast,
        reduced_motion, platform_text_ui_draw_widget);
}

bool viewer_timeline_paint_scalar_appearance(viewer_timeline *timeline,
    viewer_editor *editor, uint8_t *rgba, size_t capacity, uint32_t width,
    uint32_t height, size_t stride, double backing_scale, ui_theme theme,
    bool increased_contrast, bool reduced_motion)
{
    return viewer_timeline_paint_with_text(timeline, editor, rgba, capacity,
        width, height, stride, backing_scale, theme, increased_contrast,
        reduced_motion, scalar_timeline_text);
}

bool viewer_timeline_paint(viewer_timeline *timeline, viewer_editor *editor,
    uint8_t *rgba, size_t capacity, uint32_t width, uint32_t height,
    size_t stride, double backing_scale)
{
    ui_theme theme = UI_THEME_DARK;
    bool increased_contrast = false, reduced_motion = false;
    (void)platform_text_ui_get_preferences(&theme, &increased_contrast,
        &reduced_motion);
    return viewer_timeline_paint_appearance(timeline, editor, rgba, capacity,
        width, height, stride, backing_scale, theme, increased_contrast,
        reduced_motion);
}
