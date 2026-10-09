#include "ui_demo.h"

#include <stdbool.h>
#include <stdint.h>

static bool ui_demo_add(ui_tree *tree, uint32_t id, uint32_t parent, ui_axis axis,
                        uint32_t width, uint32_t height, uint32_t padding,
                        uint32_t gap, uint32_t color)
{
    ui_node_spec node = {id, parent, axis, width, height, padding, gap, color};
    return ui_tree_add(tree, &node);
}

bool ui_demo_render(ui_theme theme, uint32_t width, uint32_t height, raster_tile *tile)
{
    ui_tree tree;
    if (!ui_tree_init(&tree, theme)) return false;
    const ui_tokens *t = &tree.tokens;
    bool okay = ui_demo_add(&tree, 2U, UI_ROOT_ID, UI_AXIS_HORIZONTAL, 0U, 48U,
            12U, 10U, t->panel) &&
        ui_demo_add(&tree, 3U, 2U, UI_AXIS_HORIZONTAL, 52U, 24U, 0U, 0U, t->accent) &&
        ui_demo_add(&tree, 4U, 2U, UI_AXIS_HORIZONTAL, 116U, 24U, 0U, 0U, t->panel_alt) &&
        ui_demo_add(&tree, 5U, 2U, UI_AXIS_HORIZONTAL, 92U, 24U, 0U, 0U, t->panel_alt) &&
        ui_demo_add(&tree, 6U, 2U, UI_AXIS_HORIZONTAL, 0U, 24U, 0U, 0U, 0U) &&
        ui_demo_add(&tree, 7U, 2U, UI_AXIS_HORIZONTAL, 82U, 24U, 0U, 0U, t->panel_alt) &&
        ui_demo_add(&tree, 8U, UI_ROOT_ID, UI_AXIS_HORIZONTAL, 0U, 0U, 0U, 0U, t->window) &&
        ui_demo_add(&tree, 9U, 8U, UI_AXIS_VERTICAL, 56U, 0U, 7U, 8U, t->panel) &&
        ui_demo_add(&tree, 10U, 9U, UI_AXIS_VERTICAL, 40U, 40U, 0U, 0U, t->accent) &&
        ui_demo_add(&tree, 11U, 9U, UI_AXIS_VERTICAL, 40U, 40U, 0U, 0U, t->panel_alt) &&
        ui_demo_add(&tree, 12U, 9U, UI_AXIS_VERTICAL, 40U, 40U, 0U, 0U, t->panel_alt) &&
        ui_demo_add(&tree, 13U, 9U, UI_AXIS_VERTICAL, 40U, 40U, 0U, 0U, t->panel_alt) &&
        ui_demo_add(&tree, 14U, 8U, UI_AXIS_VERTICAL, 0U, 0U, 0U, 0U, t->canvas) &&
        ui_demo_add(&tree, 15U, 14U, UI_AXIS_HORIZONTAL, 0U, 36U, 12U, 8U, t->panel) &&
        ui_demo_add(&tree, 16U, 15U, UI_AXIS_HORIZONTAL, 76U, 18U, 0U, 0U, t->panel_alt) &&
        ui_demo_add(&tree, 17U, 15U, UI_AXIS_HORIZONTAL, 76U, 18U, 0U, 0U, t->panel_alt) &&
        ui_demo_add(&tree, 18U, 15U, UI_AXIS_HORIZONTAL, 0U, 18U, 0U, 0U, 0U) &&
        ui_demo_add(&tree, 19U, 15U, UI_AXIS_HORIZONTAL, 52U, 18U, 0U, 0U, t->panel_alt) &&
        ui_demo_add(&tree, 20U, 14U, UI_AXIS_OVERLAY, 0U, 0U, 0U, 0U, t->canvas) &&
        ui_demo_add(&tree, 21U, 20U, UI_AXIS_HORIZONTAL, 180U, 92U, 0U, 0U, t->panel_alt) &&
        ui_demo_add(&tree, 22U, 8U, UI_AXIS_VERTICAL, 276U, 0U, 0U, 1U, t->border) &&
        ui_demo_add(&tree, 23U, 22U, UI_AXIS_VERTICAL, 0U, 42U, 14U, 0U, t->panel) &&
        ui_demo_add(&tree, 24U, 22U, UI_AXIS_VERTICAL, 0U, 136U, 14U, 8U, t->panel) &&
        ui_demo_add(&tree, 25U, 24U, UI_AXIS_VERTICAL, 0U, 28U, 0U, 4U, t->panel_alt) &&
        ui_demo_add(&tree, 26U, 24U, UI_AXIS_VERTICAL, 0U, 28U, 0U, 4U, t->panel_alt) &&
        ui_demo_add(&tree, 27U, 24U, UI_AXIS_VERTICAL, 0U, 28U, 0U, 4U, t->panel_alt) &&
        ui_demo_add(&tree, 28U, 22U, UI_AXIS_VERTICAL, 0U, 192U, 14U, 8U, t->panel) &&
        ui_demo_add(&tree, 29U, 28U, UI_AXIS_HORIZONTAL, 0U, 28U, 0U, 4U, t->panel_alt) &&
        ui_demo_add(&tree, 30U, 28U, UI_AXIS_HORIZONTAL, 0U, 28U, 0U, 4U, t->panel_alt) &&
        ui_demo_add(&tree, 31U, 28U, UI_AXIS_HORIZONTAL, 0U, 28U, 0U, 4U, t->panel_alt) &&
        ui_demo_add(&tree, 32U, 22U, UI_AXIS_VERTICAL, 0U, 112U, 14U, 8U, t->panel) &&
        ui_demo_add(&tree, 33U, 32U, UI_AXIS_HORIZONTAL, 0U, 28U, 0U, 8U, t->panel_alt) &&
        ui_demo_add(&tree, 34U, 33U, UI_AXIS_HORIZONTAL, 72U, 0U, 0U, 0U, t->accent) &&
        ui_demo_add(&tree, 35U, 33U, UI_AXIS_HORIZONTAL, 0U, 0U, 0U, 0U, t->panel_alt) &&
        ui_demo_add(&tree, 36U, UI_ROOT_ID, UI_AXIS_HORIZONTAL, 0U, 26U, 12U, 0U, t->panel);
    if (!okay || !ui_tree_layout(&tree, width, height)) return false;
    return ui_tree_render(&tree, tile);
}
