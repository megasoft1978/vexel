#include "ui_core.h"

#include "raster/shape_render.h"
#include "shape.h"

#include <stdio.h>
#include <string.h>

#define UI_NO_NODE UINT16_MAX
#define UI_SVG_CAPACITY 256U

static uint32_t ui_color(uint8_t red, uint8_t green, uint8_t blue)
{
    return ((uint32_t)red << 24U) | ((uint32_t)green << 16U) |
           ((uint32_t)blue << 8U) | 255U;
}

static ui_tokens ui_theme_tokens(ui_theme theme)
{
    if (theme == UI_THEME_LIGHT) {
        return (ui_tokens){ui_color(242, 244, 247), ui_color(255, 255, 255),
            ui_color(232, 236, 242), ui_color(250, 251, 253), ui_color(30, 36, 45),
            ui_color(105, 115, 128), ui_color(45, 111, 219), ui_color(211, 217, 225),
            8U, 6U, 13U};
    }
    return (ui_tokens){ui_color(25, 28, 34), ui_color(35, 39, 47),
        ui_color(44, 49, 59), ui_color(21, 24, 30), ui_color(233, 237, 244),
        ui_color(148, 157, 171), ui_color(105, 159, 255), ui_color(55, 61, 72),
        8U, 6U, 13U};
}

static uint16_t ui_index(const ui_tree *tree, uint32_t id)
{
    if (tree == NULL || id == 0U) return UI_NO_NODE;
    for (size_t index = 0; index < tree->count; index++) {
        if (tree->nodes[index].occupied && tree->nodes[index].spec.id == id) {
            return (uint16_t)index;
        }
    }
    return UI_NO_NODE;
}

const ui_node *ui_tree_find(const ui_tree *tree, uint32_t id)
{
    uint16_t index = ui_index(tree, id);
    return index == UI_NO_NODE ? NULL : &tree->nodes[index];
}

bool ui_tree_init(ui_tree *tree, ui_theme theme)
{
    if (tree == NULL || (theme != UI_THEME_DARK && theme != UI_THEME_LIGHT)) return false;
    memset(tree, 0, sizeof(*tree));
    tree->tokens = ui_theme_tokens(theme);
    return ui_tree_add(tree, &(ui_node_spec){UI_ROOT_ID, 0U, UI_AXIS_VERTICAL,
        0U, 0U, 0U, 0U, tree->tokens.window});
}

bool ui_tree_add(ui_tree *tree, const ui_node_spec *spec)
{
    if (tree == NULL || spec == NULL || tree->laid_out || tree->count >= UI_MAX_NODES ||
        spec->id == 0U || spec->axis > UI_AXIS_VERTICAL ||
        spec->preferred_width > RASTER_TILE_MAX_AXIS ||
        spec->preferred_height > RASTER_TILE_MAX_AXIS ||
        spec->padding > RASTER_TILE_MAX_AXIS || spec->gap > RASTER_TILE_MAX_AXIS ||
        ui_index(tree, spec->id) != UI_NO_NODE) {
        return false;
    }
    uint16_t parent = UI_NO_NODE;
    if (tree->count == 0U) {
        if (spec->id != UI_ROOT_ID || spec->parent_id != 0U) return false;
    } else {
        parent = ui_index(tree, spec->parent_id);
        if (parent == UI_NO_NODE || tree->nodes[parent].child_count == UINT16_MAX) return false;
    }
    uint16_t added = (uint16_t)tree->count;
    tree->nodes[added] = (ui_node){.spec = *spec, .first_child = UI_NO_NODE,
        .next_sibling = UI_NO_NODE, .occupied = true};
    tree->count++;
    if (parent != UI_NO_NODE) {
        ui_node *owner = &tree->nodes[parent];
        if (owner->first_child == UI_NO_NODE) owner->first_child = added;
        else {
            uint16_t last = owner->first_child;
            while (tree->nodes[last].next_sibling != UI_NO_NODE) last = tree->nodes[last].next_sibling;
            tree->nodes[last].next_sibling = added;
        }
        owner->child_count++;
    }
    return true;
}

static bool ui_layout_node(ui_tree *tree, uint16_t index, ui_rect box, uint32_t depth)
{
    if (depth > UI_MAX_DEPTH) return false;
    ui_node *node = &tree->nodes[index];
    node->bounds = box;
    if (node->child_count == 0U) return true;
    uint32_t twice_padding = node->spec.padding * 2U;
    uint32_t inner_x = box.x + node->spec.padding;
    uint32_t inner_y = box.y + node->spec.padding;
    uint32_t inner_width = box.width > twice_padding ? box.width - twice_padding : 0U;
    uint32_t inner_height = box.height > twice_padding ? box.height - twice_padding : 0U;
    if (node->spec.axis == UI_AXIS_OVERLAY) {
        for (uint16_t child = node->first_child; child != UI_NO_NODE;
             child = tree->nodes[child].next_sibling) {
            ui_node *item = &tree->nodes[child];
            uint32_t width = item->spec.preferred_width == 0U ? inner_width : item->spec.preferred_width;
            uint32_t height = item->spec.preferred_height == 0U ? inner_height : item->spec.preferred_height;
            if (width > inner_width) width = inner_width;
            if (height > inner_height) height = inner_height;
            if (!ui_layout_node(tree, child, (ui_rect){inner_x, inner_y, width, height}, depth + 1U)) {
                return false;
            }
        }
        return true;
    }
    uint32_t main_extent = node->spec.axis == UI_AXIS_HORIZONTAL ? inner_width : inner_height;
    uint32_t cross_extent = node->spec.axis == UI_AXIS_HORIZONTAL ? inner_height : inner_width;
    uint32_t gaps = node->spec.gap * (uint32_t)(node->child_count - 1U);
    uint32_t fixed = 0U, flexible = 0U;
    for (uint16_t child = node->first_child; child != UI_NO_NODE; child = tree->nodes[child].next_sibling) {
        ui_node_spec *spec = &tree->nodes[child].spec;
        uint32_t wanted = node->spec.axis == UI_AXIS_HORIZONTAL ? spec->preferred_width : spec->preferred_height;
        if (wanted == 0U) flexible++;
        else if (UINT32_MAX - fixed < wanted) return false;
        else fixed += wanted;
    }
    uint32_t bounded_gaps = gaps > main_extent ? main_extent : gaps;
    uint32_t available = main_extent - bounded_gaps;
    uint32_t flex_extent = available > fixed ? available - fixed : 0U;
    uint32_t remaining = available;
    uint32_t cursor = node->spec.axis == UI_AXIS_HORIZONTAL ? inner_x : inner_y;
    uint16_t child = node->first_child;
    while (child != UI_NO_NODE) {
        if (child != node->first_child && remaining > 0U) {
            uint32_t gap = node->spec.gap < remaining ? node->spec.gap : remaining;
            cursor += gap;
            remaining -= gap;
        }
        ui_node *item = &tree->nodes[child];
        uint32_t preferred = node->spec.axis == UI_AXIS_HORIZONTAL ? item->spec.preferred_width : item->spec.preferred_height;
        uint32_t extent = preferred == 0U ? (flexible == 0U ? 0U : flex_extent / flexible) : preferred;
        if (extent > remaining) extent = remaining;
        if (preferred == 0U && flexible > 0U) {
            flex_extent = flex_extent > extent ? flex_extent - extent : 0U;
            flexible--;
        }
        uint32_t cross = node->spec.axis == UI_AXIS_HORIZONTAL ? item->spec.preferred_height : item->spec.preferred_width;
        if (cross == 0U || cross > cross_extent) cross = cross_extent;
        ui_rect child_box = node->spec.axis == UI_AXIS_HORIZONTAL
            ? (ui_rect){cursor, inner_y, extent, cross}
            : (ui_rect){inner_x, cursor, cross, extent};
        if (!ui_layout_node(tree, child, child_box, depth + 1U)) return false;
        cursor += extent;
        remaining -= extent;
        child = item->next_sibling;
    }
    return true;
}

bool ui_tree_layout(ui_tree *tree, uint32_t width, uint32_t height)
{
    if (tree == NULL || tree->count == 0U || tree->laid_out || width == 0U || height == 0U ||
        width > RASTER_TILE_MAX_AXIS || height > RASTER_TILE_MAX_AXIS) return false;
    if (!ui_layout_node(tree, 0U, (ui_rect){0U, 0U, width, height}, 0U)) return false;
    tree->laid_out = true;
    return true;
}

static bool ui_render_node(const ui_node *node, raster_tile *tile)
{
    char svg[UI_SVG_CAPACITY];
    uint32_t rgba = node->spec.color;
    int length = snprintf(svg, sizeof(svg),
        "<svg width=\"4\" height=\"4\"><rect width=\"4\" height=\"4\" fill=\"#%06x\"/></svg>",
        (rgba >> 8U) & 0xffffffU);
    if (length < 0 || (size_t)length >= sizeof(svg)) return false;
    svg_shape_document document = {0};
    svg_shape_diagnostic diagnostic = {0};
    base_bytes bytes = {(const uint8_t *)svg, (size_t)length};
    if (svg_shape_decode(bytes, &document, &diagnostic) != SVG_SHAPE_ERROR_NONE) return false;
    raster_tile sample = {0};
    bool okay = raster_shape_render(&document, &sample, &diagnostic);
    svg_shape_document_destroy(&document);
    if (!okay) return false;
    size_t sample_at = ((size_t)(sample.height / 2U) * sample.width + sample.width / 2U) * 4U;
    const uint8_t *paint = sample.rgba + sample_at;
    uint32_t x_end = node->bounds.x + node->bounds.width;
    uint32_t y_end = node->bounds.y + node->bounds.height;
    if (x_end > tile->width) x_end = tile->width;
    if (y_end > tile->height) y_end = tile->height;
    for (uint32_t y = node->bounds.y; y < y_end; y++) {
        for (uint32_t x = node->bounds.x; x < x_end; x++) {
            size_t at = ((size_t)y * tile->width + x) * 4U;
            uint32_t alpha = paint[3];
            uint32_t inverse = 255U - alpha;
            for (size_t channel = 0U; channel < 3U; channel++) {
                uint32_t value = (uint32_t)paint[channel] * alpha +
                    (uint32_t)tile->rgba[at + channel] * inverse;
                tile->rgba[at + channel] = (uint8_t)((value + 127U) / 255U);
            }
            tile->rgba[at + 3U] = 255U;
        }
    }
    raster_tile_destroy(&sample);
    return true;
}

bool ui_tree_render(const ui_tree *tree, raster_tile *out_tile)
{
    if (tree == NULL || out_tile == NULL || !tree->laid_out || tree->count == 0U) return false;
    *out_tile = (raster_tile){0};
    const ui_rect root = tree->nodes[0].bounds;
    if (!raster_tile_create(root.width, root.height, out_tile)) return false;
    for (size_t index = 0U; index < tree->count; index++) {
        const ui_node *node = &tree->nodes[index];
        if ((node->spec.color & 255U) == 0U) continue;
        if (!ui_render_node(node, out_tile)) {
            raster_tile_destroy(out_tile);
            return false;
        }
    }
    return true;
}
