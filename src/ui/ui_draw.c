#include "ui_core.h"
#include "ui_draw.h"
#include "ui_icons.h"

#include <limits.h>
#include <string.h>

static const uint8_t ui_font[36][7] = {
    {14,17,19,21,25,17,14}, {4,12,4,4,4,4,14}, {14,17,1,2,4,8,31},
    {30,1,1,14,1,1,30}, {2,6,10,18,31,2,2}, {31,16,16,30,1,1,30},
    {14,16,16,30,17,17,14}, {31,1,2,4,8,8,8}, {14,17,17,14,17,17,14},
    {14,17,17,15,1,1,14}, {14,17,17,31,17,17,17}, {30,17,17,30,17,17,30},
    {14,17,16,16,16,17,14}, {30,17,17,17,17,17,30}, {31,16,16,30,16,16,31},
    {31,16,16,30,16,16,16}, {14,17,16,23,17,17,15}, {17,17,17,31,17,17,17},
    {14,4,4,4,4,4,14}, {7,2,2,2,18,18,12}, {17,18,20,24,20,18,17},
    {16,16,16,16,16,16,31}, {17,27,21,21,17,17,17}, {17,25,21,19,17,17,17},
    {14,17,17,17,17,17,14}, {30,17,17,30,16,16,16}, {14,17,17,17,21,18,13},
    {30,17,17,30,20,18,17}, {15,16,16,14,1,1,30}, {31,4,4,4,4,4,4},
    {17,17,17,17,17,17,14}, {17,17,17,17,17,10,4}, {17,17,17,21,21,21,10},
    {17,17,10,4,10,17,17}, {17,17,10,4,4,4,4}, {31,1,2,4,8,16,31}
};

static uint8_t ui_glyph_row(uint8_t character, size_t row)
{
    if (character >= 'a' && character <= 'z') character = (uint8_t)(character - 'a' + 'A');
    if (character >= '0' && character <= '9') return ui_font[character - '0'][row];
    if (character >= 'A' && character <= 'Z') return ui_font[10U + character - 'A'][row];
    switch (character) {
        case ' ': return 0U;
        case '-': return row == 3U ? 14U : 0U;
        case '_': return row == 6U ? 31U : 0U;
        case '.': return row == 6U ? 4U : 0U;
        case ',': return row == 6U ? 4U : row == 5U ? 8U : 0U;
        case ':': return row == 2U || row == 5U ? 4U : 0U;
        case ';': return row == 2U ? 4U : row == 5U ? 4U : row == 6U ? 8U : 0U;
        case '/': return (uint8_t)(1U << (row > 4U ? 0U : 4U - row));
        case '\\': return (uint8_t)(1U << (row < 5U ? row : 4U));
        case '#': return row == 2U || row == 4U ? 31U : row == 1U || row == 5U ? 10U : 0U;
        case '%': return row == 0U || row == 6U ? 17U : row == 1U || row == 5U ? 2U : row == 2U || row == 4U ? 4U : 8U;
        case '=': return row == 2U || row == 4U ? 14U : 0U;
        case '+': return row == 3U ? 14U : row == 1U || row == 2U || row == 4U || row == 5U ? 4U : 0U;
        case '[': return row == 0U || row == 6U ? 14U : 8U;
        case ']': return row == 0U || row == 6U ? 14U : 2U;
        case '<': return row == 3U ? 2U : row == 2U || row == 4U ? 4U : row == 1U || row == 5U ? 8U : 0U;
        case '>': return row == 3U ? 8U : row == 2U || row == 4U ? 4U : row == 1U || row == 5U ? 2U : 0U;
        case '(': return row == 0U || row == 6U ? 2U : row == 1U || row == 5U ? 4U : 8U;
        case ')': return row == 0U || row == 6U ? 8U : row == 1U || row == 5U ? 4U : 2U;
        case '!': return row < 5U ? 4U : row == 6U ? 4U : 0U;
        case '?': return row == 0U ? 14U : row == 1U ? 17U : row == 2U ? 2U : row == 3U ? 4U : row == 6U ? 4U : 0U;
        case '\'': return row < 2U ? 4U : 0U;
        case '"': return row < 2U ? 10U : 0U;
        case '*': return row == 1U || row == 5U ? 10U : row == 2U || row == 4U ? 4U : row == 3U ? 14U : 0U;
        case '|': return 4U;
        case '&': return row == 0U ? 12U : row == 1U || row == 2U ? 18U : row == 3U ? 12U : row == 4U || row == 5U ? 21U : 10U;
        default: return row == 0U || row == 6U ? 14U : row == 1U || row == 5U ? 17U : row == 2U || row == 4U ? 2U : 4U;
    }
}

static bool ui_pixel_valid(const ui_node *node, uint32_t x, uint32_t y,
                           uint32_t width, uint32_t height)
{
    return x >= node->bounds.x && y >= node->bounds.y &&
        x - node->bounds.x < node->bounds.width &&
        y - node->bounds.y < node->bounds.height && x < width && y < height;
}

static void ui_blend_pixel(uint8_t *destination, uint32_t color)
{
    uint32_t alpha = color & 255U;
    uint32_t inverse = 255U - alpha;
    uint32_t destination_alpha = destination[3];
    uint32_t out_alpha = alpha + (destination_alpha * inverse + 127U) / 255U;
    if (out_alpha == 0U) return;
    for (size_t channel = 0U; channel < 3U; ++channel) {
        uint32_t source = (color >> (24U - (uint32_t)(channel * 8U))) & 255U;
        uint32_t numerator = source * alpha * 255U +
            (uint32_t)destination[channel] * destination_alpha * inverse;
        destination[channel] = (uint8_t)((numerator + out_alpha * 127U) / (out_alpha * 255U));
    }
    destination[3] = (uint8_t)out_alpha;
}

static void ui_paint_rect(const ui_node *node, uint32_t color, uint8_t *rgba,
                          size_t stride, uint32_t width, uint32_t height)
{
    uint32_t x_end = node->bounds.x + node->bounds.width;
    uint32_t y_end = node->bounds.y + node->bounds.height;
    if (x_end < node->bounds.x || x_end > width) x_end = width;
    if (y_end < node->bounds.y || y_end > height) y_end = height;
    for (uint32_t y = node->bounds.y; y < y_end; ++y)
        for (uint32_t x = node->bounds.x; x < x_end; ++x)
            ui_blend_pixel(rgba + (size_t)y * stride + (size_t)x * 4U, color);
}

static bool ui_inside_rounded_corner(uint32_t x, uint32_t y, uint32_t width,
                                     uint32_t height, uint32_t radius)
{
    if (radius == 0U) return true;
    uint32_t dx = x < radius ? radius - x :
        (x >= width - radius ? x - (width - radius - 1U) : 0U);
    uint32_t dy = y < radius ? radius - y :
        (y >= height - radius ? y - (height - radius - 1U) : 0U);
    if (dx == 0U || dy == 0U) return true;
    uint64_t edge = (uint64_t)radius * radius;
    uint64_t distance = (uint64_t)dx * dx + (uint64_t)dy * dy;
    return distance <= edge;
}

static void ui_paint_rounded_rect(const ui_node *node, uint32_t color,
                                  uint32_t radius, uint8_t *rgba,
                                  size_t stride, uint32_t width, uint32_t height)
{
    if (node->bounds.x >= width || node->bounds.y >= height) return;
    uint32_t right = node->bounds.x + node->bounds.width;
    uint32_t bottom = node->bounds.y + node->bounds.height;
    if (right < node->bounds.x || right > width) right = width;
    if (bottom < node->bounds.y || bottom > height) bottom = height;
    uint32_t local_width = right - node->bounds.x;
    uint32_t local_height = bottom - node->bounds.y;
    if (radius > local_width / 2U) radius = local_width / 2U;
    if (radius > local_height / 2U) radius = local_height / 2U;
    for (uint32_t y = node->bounds.y; y < bottom; ++y) {
        for (uint32_t x = node->bounds.x; x < right; ++x) {
            if (!ui_inside_rounded_corner(x - node->bounds.x, y - node->bounds.y,
                    local_width, local_height, radius)) continue;
            ui_blend_pixel(rgba + (size_t)y * stride + (size_t)x * 4U, color);
        }
    }
}

static void ui_paint_control_outline(const ui_node *node, uint32_t color,
                                     uint32_t radius, uint8_t *rgba,
                                     size_t stride, uint32_t width,
                                     uint32_t height)
{
    uint32_t right = node->bounds.x + node->bounds.width;
    uint32_t bottom = node->bounds.y + node->bounds.height;
    if (right < node->bounds.x || right > width) right = width;
    if (bottom < node->bounds.y || bottom > height) bottom = height;
    uint32_t local_width = right - node->bounds.x;
    uint32_t local_height = bottom - node->bounds.y;
    if (local_width == 0U || local_height == 0U) return;
    if (radius > local_width / 2U) radius = local_width / 2U;
    if (radius > local_height / 2U) radius = local_height / 2U;
    for (uint32_t y = node->bounds.y; y < bottom; ++y) {
        for (uint32_t x = node->bounds.x; x < right; ++x) {
            bool edge = x == node->bounds.x || x + 1U == right ||
                y == node->bounds.y || y + 1U == bottom;
            if (!edge || !ui_inside_rounded_corner(x - node->bounds.x,
                    y - node->bounds.y, local_width, local_height, radius)) continue;
            ui_blend_pixel(rgba + (size_t)y * stride + (size_t)x * 4U, color);
        }
    }
}

static void ui_paint_text(const char *text, size_t length, uint32_t color,
                          const ui_node *node, uint32_t x, uint32_t y,
                          uint32_t scale, uint8_t *rgba, size_t stride,
                          uint32_t width, uint32_t height)
{
    uint32_t right = node->bounds.x + node->bounds.width;
    uint32_t bottom = node->bounds.y + node->bounds.height;
    uint32_t cursor = x;
    if (length > UI_TEXT_MAX_BYTES) length = UI_TEXT_MAX_BYTES;
    for (size_t index = 0U; index < length; ++index) {
        uint8_t character = (uint8_t)text[index];
        if (character >= 0x80U) {
            if ((character & 0xc0U) == 0x80U) continue;
            character = '?';
        }
        if (cursor > right || right - cursor < 5U) break;
        for (size_t row = 0U; row < 7U; ++row) {
            for (size_t column = 0U; column < 5U; ++column) {
                if ((ui_glyph_row(character, row) & (1U << (4U - column))) == 0U) continue;
                for (uint32_t sy = 0U; sy < scale; ++sy) {
                    for (uint32_t sx = 0U; sx < scale; ++sx) {
                        uint32_t px = cursor + (uint32_t)column * scale + sx;
                        uint32_t py = y + (uint32_t)row * scale + sy;
                        if (px < right && py < bottom && ui_pixel_valid(node, px, py, width, height))
                            ui_blend_pixel(rgba + (size_t)py * stride + (size_t)px * 4U, color);
                    }
                }
            }
        }
        uint32_t advance = 6U * scale;
        if (cursor > UINT32_MAX - advance) break;
        cursor += advance;
    }
}

static bool ui_paint_node(const ui_tree *tree, const ui_node *node, uint8_t *rgba,
                          size_t stride, uint32_t width, uint32_t height,
                          bool draw_text)
{
    uint32_t fill = node->spec.color;
    if ((node->spec.state & UI_STATE_PRESSED) != 0U &&
        (node->spec.role == UI_ROLE_BUTTON || node->spec.role == UI_ROLE_TEXT_FIELD ||
         node->spec.role == UI_ROLE_ROW)) fill = tree->tokens.selection_fill;
    else if ((node->spec.state & UI_STATE_HOVERED) != 0U &&
        (node->spec.role == UI_ROLE_BUTTON || node->spec.role == UI_ROLE_TEXT_FIELD ||
         node->spec.role == UI_ROLE_ROW)) fill = tree->tokens.panel_raised;
    if ((fill & 255U) != 0U) {
        if (tree->semantic_style && (node->spec.role == UI_ROLE_BUTTON ||
                node->spec.role == UI_ROLE_TEXT_FIELD))
            ui_paint_rounded_rect(node, fill, tree->tokens.radius, rgba, stride, width, height);
        else ui_paint_rect(node, fill, rgba, stride, width, height);
    }
    if (tree->semantic_style && (node->spec.role == UI_ROLE_TEXT_FIELD ||
            (node->spec.role == UI_ROLE_BUTTON &&
                node->spec.widget != UI_WIDGET_ICON_BUTTON)))
        ui_paint_control_outline(node, tree->tokens.control_edge,
            tree->tokens.radius, rgba, stride, width, height);
    if ((node->spec.state & UI_STATE_FOCUSED) != 0U &&
        node->spec.widget != UI_WIDGET_TREE_ROW &&
        node->bounds.width > 1U && node->bounds.height > 1U) {
        ui_node edge = *node;
        edge.bounds.height = 1U;
        ui_paint_rect(&edge, tree->tokens.focus_ring, rgba, stride, width, height);
        edge.bounds.y = node->bounds.y + node->bounds.height - 1U;
        ui_paint_rect(&edge, tree->tokens.focus_ring, rgba, stride, width, height);
        edge.bounds = (ui_rect){node->bounds.x, node->bounds.y + 1U, 1U,
            node->bounds.height - 2U};
        ui_paint_rect(&edge, tree->tokens.focus_ring, rgba, stride, width, height);
        edge.bounds.x = node->bounds.x + node->bounds.width - 1U;
        ui_paint_rect(&edge, tree->tokens.focus_ring, rgba, stride, width, height);
    }
    uint32_t color = (node->spec.state & UI_STATE_INVALID) != 0U ?
        tree->tokens.error : (node->spec.state & UI_STATE_DISABLED) != 0U ?
        tree->tokens.text_secondary : tree->tokens.text_primary;
    uint32_t scale = tree->tokens.font_px / 13U;
    if (tree->tokens.font_px % 13U != 0U) scale++;
    if (scale == 0U) scale = 1U;
    if (scale > 16U) scale = 16U;
    if (node->spec.role == UI_ROLE_ROW && (node->spec.state & UI_STATE_CHECKED) != 0U)
        color = tree->tokens.accent;
    if (node->spec.icon != UI_ICON_NONE) {
        if (!draw_text) return true;
        if ((node->spec.state & UI_STATE_CHECKED) != 0U) color = tree->tokens.accent;
        return ui_icon_paint_legacy_into(node->spec.icon, tree->theme,
            tree->backing_scale > 0.0 ? tree->backing_scale : 1.0,
            node->bounds, color, rgba, width, height, stride);
    }
    if (!draw_text) return true;
    uint32_t glyph_height = 7U * scale;
    uint32_t text_y = node->bounds.y + (node->bounds.height > glyph_height ?
        (node->bounds.height - glyph_height) / 2U : 0U);
    uint32_t inset = 8U * scale;
    uint32_t text_x = node->bounds.x + (node->bounds.width > inset ? inset : 0U);
    if (node->spec.role == UI_ROLE_BUTTON && node->spec.label_len <= 40U) {
        uint32_t text_width = (uint32_t)node->spec.label_len * 6U * scale;
        if (text_width < node->bounds.width) text_x = node->bounds.x + (node->bounds.width - text_width) / 2U;
    }
    ui_paint_text(node->label, node->spec.label_len, color, node, text_x, text_y,
        scale, rgba, stride, width, height);
    if (node->spec.role == UI_ROLE_TEXT_FIELD && node->spec.value_len > 0U) {
        uint32_t value_x = node->bounds.x + node->bounds.width / 2U;
        ui_paint_text(node->value, node->spec.value_len, color, node, value_x, text_y,
            scale, rgba, stride, width, height);
    }
    return true;
}

static bool ui_tree_paint_internal(const ui_tree *tree, uint8_t *rgba,
                                   size_t capacity, size_t stride,
                                   bool draw_text)
{
    if (tree == NULL || rgba == NULL || !tree->laid_out || tree->count == 0U ||
        tree->count > UI_MAX_NODES) return false;
    uint32_t width = tree->nodes[0].bounds.width;
    uint32_t height = tree->nodes[0].bounds.height;
    if (width == 0U || height == 0U || width > RASTER_TILE_MAX_AXIS ||
        height > RASTER_TILE_MAX_AXIS || (size_t)width > SIZE_MAX / 4U) return false;
    size_t row_bytes = (size_t)width * 4U;
    if (stride < row_bytes || (size_t)height > SIZE_MAX / stride ||
        capacity < stride * (size_t)height) return false;
    for (size_t index = 0U; index < tree->count; ++index) {
        if (!tree->nodes[index].occupied) return false;
        if (!ui_paint_node(tree, &tree->nodes[index], rgba, stride, width, height,
                draw_text)) return false;
    }
    return true;
}

bool ui_tree_paint_into(const ui_tree *tree, uint8_t *rgba, size_t capacity,
                        size_t stride)
{
    return ui_tree_paint_internal(tree, rgba, capacity, stride, true);
}

bool ui_tree_paint_backgrounds_into(const ui_tree *tree, uint8_t *rgba,
                                    size_t capacity, size_t stride)
{
    return ui_tree_paint_internal(tree, rgba, capacity, stride, false);
}

bool ui_scalar_text_draw(const char *text, size_t length, ui_rect clip,
    uint32_t font_size_px, uint32_t color, uint8_t *rgba,
    uint32_t width, uint32_t height, size_t stride)
{
    if ((length > 0U && text == NULL) || length > UI_TEXT_TOTAL_MAX_BYTES ||
        font_size_px == 0U || rgba == NULL || width == 0U || height == 0U ||
        (size_t)width > SIZE_MAX / 4U || stride < (size_t)width * 4U ||
        (size_t)height > SIZE_MAX / stride || clip.x > width || clip.y > height ||
        clip.width > width - clip.x || clip.height > height - clip.y) return false;
    uint32_t scale = font_size_px / 13U + (font_size_px % 13U != 0U ? 1U : 0U);
    if (scale == 0U) scale = 1U;
    if (scale > 16U) scale = 16U;
    uint32_t glyph_height = 7U * scale;
    uint32_t y_origin = clip.y + (clip.height > glyph_height ?
        (clip.height - glyph_height) / 2U : 0U);
    uint32_t cursor = clip.x;
    uint32_t right = clip.x + clip.width;
    uint32_t bottom = clip.y + clip.height;
    for (size_t index = 0U; index < length; ++index) {
        uint8_t character = (uint8_t)text[index];
        if (character >= 0x80U) {
            if ((character & 0xc0U) == 0x80U) continue;
            character = '?';
        }
        if (cursor > right || right - cursor < 5U * scale) break;
        for (size_t row = 0U; row < 7U; ++row) {
            for (size_t column = 0U; column < 5U; ++column) {
                if ((ui_glyph_row(character, row) & (1U << (4U - column))) == 0U) continue;
                for (uint32_t sy = 0U; sy < scale; ++sy) {
                    for (uint32_t sx = 0U; sx < scale; ++sx) {
                        uint32_t px = cursor + (uint32_t)column * scale + sx;
                        uint32_t py = y_origin + (uint32_t)row * scale + sy;
                        if (px >= clip.x && py >= clip.y && px < right && py < bottom &&
                            px < width && py < height)
                            ui_blend_pixel(rgba + (size_t)py * stride + (size_t)px * 4U, color);
                    }
                }
            }
        }
        uint32_t advance = 6U * scale;
        if (cursor > UINT32_MAX - advance) break;
        cursor += advance;
    }
    return true;
}
