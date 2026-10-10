#include "ui_icons.h"

#include <math.h>
#include <string.h>

#define UI_ICON_MAX_SCALE 4U
#define UI_ICON_VIEWBOX 24U
#define UI_ICON_MAX_AXIS (UI_ICON_VIEWBOX * UI_ICON_MAX_SCALE)
#define UI_ICON_THEME_COUNT 2U
#define UI_ICON_COUNT 3U
#define UI_ICON_MASK_BYTES (UI_ICON_MAX_AXIS * UI_ICON_MAX_AXIS)

typedef struct ui_icon_segment { uint8_t x1, y1, x2, y2; } ui_icon_segment;

static const ui_icon_segment select_geometry[] = {
    {4U, 2U, 19U, 17U}, {19U, 17U, 13U, 17U},
    {13U, 17U, 10U, 23U}, {10U, 23U, 4U, 2U}, {4U, 2U, 4U, 2U}
};
static const ui_icon_segment pan_geometry[] = {
    {12U, 3U, 12U, 21U}, {3U, 12U, 21U, 12U},
    {12U, 3U, 9U, 7U}, {12U, 3U, 15U, 7U},
    {12U, 21U, 9U, 17U}, {12U, 21U, 15U, 17U},
    {3U, 12U, 7U, 9U}, {3U, 12U, 7U, 15U},
    {21U, 12U, 17U, 9U}, {21U, 12U, 17U, 15U}
};
static const ui_icon_segment zoom_geometry[] = {
    {5U, 12U, 5U, 10U}, {5U, 10U, 6U, 7U}, {6U, 7U, 8U, 5U},
    {8U, 5U, 11U, 4U}, {11U, 4U, 14U, 5U}, {14U, 5U, 16U, 7U},
    {16U, 7U, 17U, 10U}, {17U, 10U, 17U, 12U}, {17U, 12U, 16U, 15U},
    {16U, 15U, 14U, 17U}, {14U, 17U, 11U, 18U}, {11U, 18U, 8U, 17U},
    {8U, 17U, 6U, 15U}, {6U, 15U, 5U, 12U},
    {15U, 15U, 22U, 22U}, {8U, 11U, 14U, 11U}, {11U, 8U, 11U, 14U}
};

static uint8_t ui_icon_masks[UI_ICON_THEME_COUNT][UI_ICON_MAX_SCALE + 1U]
    [UI_ICON_COUNT][UI_ICON_MASK_BYTES];
static bool ui_icon_ready[UI_ICON_THEME_COUNT][UI_ICON_MAX_SCALE + 1U][UI_ICON_COUNT];

static void set_mask_pixel(uint8_t *mask, uint32_t axis, int32_t x, int32_t y,
                          uint32_t stroke)
{
    int32_t radius = (int32_t)(stroke / 2U);
    for (int32_t dy = -radius; dy <= radius; ++dy) {
        for (int32_t dx = -radius; dx <= radius; ++dx) {
            int32_t px = x + dx, py = y + dy;
            if (px >= 0 && py >= 0 && (uint32_t)px < axis && (uint32_t)py < axis)
                mask[(size_t)(uint32_t)py * axis + (uint32_t)px] = 255U;
        }
    }
}

static void draw_segment(uint8_t *mask, uint32_t axis, const ui_icon_segment *segment,
                         uint32_t scale, uint32_t stroke)
{
    int32_t x0 = (int32_t)segment->x1 * (int32_t)scale;
    int32_t y0 = (int32_t)segment->y1 * (int32_t)scale;
    int32_t x1 = (int32_t)segment->x2 * (int32_t)scale;
    int32_t y1 = (int32_t)segment->y2 * (int32_t)scale;
    int32_t dx = x1 >= x0 ? x1 - x0 : x0 - x1;
    int32_t sx = x0 < x1 ? 1 : -1;
    int32_t dy = y1 >= y0 ? y0 - y1 : y1 - y0;
    int32_t sy = y0 < y1 ? 1 : -1;
    int32_t error = dx + dy;
    for (;;) {
        set_mask_pixel(mask, axis, x0, y0, stroke);
        if (x0 == x1 && y0 == y1) break;
        int32_t twice = 2 * error;
        if (twice >= dy) { error += dy; x0 += sx; }
        if (twice <= dx) { error += dx; y0 += sy; }
    }
}

static const ui_icon_segment *icon_segments(ui_icon_kind icon, size_t *out_count)
{
    switch (icon) {
        case UI_ICON_SELECT:
            *out_count = sizeof(select_geometry) / sizeof(select_geometry[0]);
            return select_geometry;
        case UI_ICON_PAN:
            *out_count = sizeof(pan_geometry) / sizeof(pan_geometry[0]);
            return pan_geometry;
        case UI_ICON_ZOOM:
            *out_count = sizeof(zoom_geometry) / sizeof(zoom_geometry[0]);
            return zoom_geometry;
        case UI_ICON_NONE: default: *out_count = 0U; return NULL;
    }
}

static uint8_t *mask_get(ui_icon_kind icon, ui_theme theme, uint32_t scale,
                         uint32_t *out_axis)
{
    size_t count = 0U;
    const ui_icon_segment *segments = icon_segments(icon, &count);
    if (segments == NULL || theme > UI_THEME_LIGHT || scale == 0U ||
        scale > UI_ICON_MAX_SCALE) return NULL;
    uint32_t theme_index = (uint32_t)theme;
    uint32_t icon_index = (uint32_t)icon - 1U;
    uint32_t axis = UI_ICON_VIEWBOX * scale;
    if (!ui_icon_ready[theme_index][scale][icon_index]) {
        uint8_t *mask = ui_icon_masks[theme_index][scale][icon_index];
        memset(mask, 0, UI_ICON_MASK_BYTES);
        uint32_t stroke = (uint32_t)ceil(1.6 * (double)scale);
        if (stroke == 0U) stroke = 1U;
        for (size_t index = 0U; index < count; ++index)
            draw_segment(mask, axis, &segments[index], scale, stroke);
        ui_icon_ready[theme_index][scale][icon_index] = true;
    }
    *out_axis = axis;
    return ui_icon_masks[theme_index][scale][icon_index];
}

static void blend_mask_pixel(uint8_t *destination, uint32_t color, uint8_t coverage)
{
    uint32_t alpha = ((color & 255U) * coverage + 127U) / 255U;
    uint32_t inverse = 255U - alpha;
    uint32_t destination_alpha = destination[3];
    uint32_t out_alpha = alpha + (destination_alpha * inverse + 127U) / 255U;
    if (out_alpha == 0U) return;
    for (size_t channel = 0U; channel < 3U; ++channel) {
        uint32_t source = (color >> (24U - (uint32_t)(channel * 8U))) & 255U;
        uint32_t numerator = source * alpha * 255U +
            (uint32_t)destination[channel] * destination_alpha * inverse;
        destination[channel] = (uint8_t)((numerator + out_alpha * 127U) /
            (out_alpha * 255U));
    }
    destination[3] = (uint8_t)out_alpha;
}

bool ui_icon_paint_into(ui_icon_kind icon, ui_theme theme, double backing_scale,
                        ui_rect bounds, uint32_t color, uint8_t *rgba,
                        uint32_t width, uint32_t height, size_t stride)
{
    if (icon == UI_ICON_NONE) return true;
    if (rgba == NULL || !isfinite(backing_scale) || backing_scale < 0.5 ||
        backing_scale > (double)UI_ICON_MAX_SCALE || width == 0U || height == 0U ||
        (size_t)width > SIZE_MAX / 4U || stride < (size_t)width * 4U ||
        (size_t)height > SIZE_MAX / stride) return false;
    uint32_t scale = (uint32_t)ceil(backing_scale);
    uint32_t icon_axis = 0U;
    uint8_t *mask = mask_get(icon, theme, scale, &icon_axis);
    if (mask == NULL) return false;
    uint32_t origin_x = bounds.x + (bounds.width > icon_axis ?
        (bounds.width - icon_axis) / 2U : 0U);
    uint32_t origin_y = bounds.y + (bounds.height > icon_axis ?
        (bounds.height - icon_axis) / 2U : 0U);
    uint32_t draw_width = bounds.width < icon_axis ? bounds.width : icon_axis;
    uint32_t draw_height = bounds.height < icon_axis ? bounds.height : icon_axis;
    if (origin_x > width || draw_width > width - origin_x ||
        origin_y > height || draw_height > height - origin_y) return false;
    for (uint32_t y = 0U; y < draw_height; ++y) {
        for (uint32_t x = 0U; x < draw_width; ++x) {
            uint8_t coverage = mask[(size_t)y * icon_axis + x];
            if (coverage == 0U) continue;
            blend_mask_pixel(rgba + (size_t)(origin_y + y) * stride +
                (size_t)(origin_x + x) * 4U, color, coverage);
        }
    }
    return true;
}

uint64_t ui_icon_asset_hash(ui_theme theme, uint32_t integer_scale)
{
    if (theme > UI_THEME_LIGHT || integer_scale == 0U || integer_scale > UI_ICON_MAX_SCALE)
        return 0U;
    uint64_t hash = UINT64_C(1469598103934665603);
    for (uint32_t icon = UI_ICON_SELECT; icon <= UI_ICON_ZOOM; ++icon) {
        uint32_t axis = 0U;
        uint8_t *mask = mask_get((ui_icon_kind)icon, theme, integer_scale, &axis);
        if (mask == NULL) return 0U;
        size_t length = (size_t)axis * axis;
        for (size_t index = 0U; index < length; ++index) {
            hash ^= mask[index];
            hash *= UINT64_C(1099511628211);
        }
    }
    return hash;
}
