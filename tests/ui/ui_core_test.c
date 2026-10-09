#include "ui_core.h"

#include <stdint.h>
#include <stdio.h>
#include <string.h>

static bool add(ui_tree *tree, uint32_t id, uint32_t parent, uint32_t width,
                uint32_t height, uint32_t color)
{
    ui_node_spec spec = {id, parent, UI_AXIS_HORIZONTAL, width, height, 0U, 4U, color};
    return ui_tree_add(tree, &spec);
}

int main(void)
{
    ui_tree tree;
    if (!ui_tree_init(&tree, UI_THEME_DARK) || tree.count != 1U) return 1;
    tree.nodes[0].spec.axis = UI_AXIS_HORIZONTAL;
    tree.nodes[0].spec.gap = 4U;
    if (!add(&tree, 2U, UI_ROOT_ID, 100U, 20U, tree.tokens.accent) ||
        !add(&tree, 3U, UI_ROOT_ID, 0U, 20U, tree.tokens.panel_alt) ||
        add(&tree, 4U, 99U, 20U, 20U, 1U) || add(&tree, 2U, UI_ROOT_ID, 1U, 1U, 1U)) return 2;
    if (ui_tree_layout(&tree, 0U, 200U) || !ui_tree_layout(&tree, 320U, 200U)) return 3;
    const ui_node *first = ui_tree_find(&tree, 2U), *second = ui_tree_find(&tree, 3U);
    if (first == NULL || second == NULL || first->bounds.x != 0U ||
        first->bounds.width != 100U || second->bounds.x != 104U || second->bounds.width != 212U) {
        (void)fprintf(stderr, "bounds %u,%u %ux%u | %u,%u %ux%u\n", first == NULL ? 0U : first->bounds.x,
            first == NULL ? 0U : first->bounds.y, first == NULL ? 0U : first->bounds.width,
            first == NULL ? 0U : first->bounds.height, second == NULL ? 0U : second->bounds.x,
            second == NULL ? 0U : second->bounds.y, second == NULL ? 0U : second->bounds.width,
            second == NULL ? 0U : second->bounds.height);
        return 4;
    }
    if (ui_tree_add(&tree, &(ui_node_spec){4U, UI_ROOT_ID, UI_AXIS_VERTICAL, 0U, 0U, 0U, 0U, 0U})) return 5;
    raster_tile a = {0}, b = {0};
    if (!ui_tree_render(&tree, &a) || !ui_tree_render(&tree, &b) ||
        a.byte_count != b.byte_count || memcmp(a.rgba, b.rgba, a.byte_count) != 0) return 6;
    raster_tile_destroy(&a);
    raster_tile_destroy(&b);
    ui_tree overflow;
    if (!ui_tree_init(&overflow, UI_THEME_DARK)) return 10;
    overflow.nodes[0].spec.axis = UI_AXIS_HORIZONTAL;
    overflow.nodes[0].spec.gap = 4U;
    if (!add(&overflow, 2U, UI_ROOT_ID, 100U, 20U, overflow.tokens.accent) ||
        !add(&overflow, 3U, UI_ROOT_ID, 80U, 20U, overflow.tokens.panel_alt) ||
        !ui_tree_layout(&overflow, 120U, 20U)) return 11;
    first = ui_tree_find(&overflow, 2U);
    second = ui_tree_find(&overflow, 3U);
    if (first == NULL || second == NULL || first->bounds.width != 100U ||
        second->bounds.x != 104U || second->bounds.width != 12U) return 12;
    ui_tree invalid;
    if (ui_tree_init(&invalid, (ui_theme)99) || ui_tree_init(&invalid, UI_THEME_LIGHT) == false) return 7;
    for (uint32_t id = 2U; id < UI_MAX_NODES + 1U; id++) {
        if (!add(&invalid, id, UI_ROOT_ID, 0U, 1U, 0U)) return 8;
    }
    if (add(&invalid, UI_MAX_NODES + 1U, UI_ROOT_ID, 0U, 1U, 0U)) return 9;
    return 0;
}
