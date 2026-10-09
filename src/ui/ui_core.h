#ifndef VEXEL_UI_CORE_H
#define VEXEL_UI_CORE_H

#include "raster/tile.h"

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#define UI_MAX_NODES 128U
#define UI_MAX_DEPTH 24U
#define UI_ROOT_ID 1U

typedef enum ui_theme { UI_THEME_DARK = 0, UI_THEME_LIGHT = 1 } ui_theme;
typedef enum ui_axis { UI_AXIS_OVERLAY = 0, UI_AXIS_HORIZONTAL, UI_AXIS_VERTICAL } ui_axis;
typedef struct ui_rect { uint32_t x, y, width, height; } ui_rect;
typedef struct ui_tokens {
    uint32_t window, panel, panel_alt, canvas, text, muted, accent, border;
    uint32_t spacing, radius, font_px;
} ui_tokens;
typedef struct ui_node_spec {
    uint32_t id;
    uint32_t parent_id;
    ui_axis axis;
    uint32_t preferred_width;
    uint32_t preferred_height;
    uint32_t padding;
    uint32_t gap;
    uint32_t color;
} ui_node_spec;
typedef struct ui_node {
    ui_node_spec spec;
    ui_rect bounds;
    uint16_t first_child;
    uint16_t next_sibling;
    uint16_t child_count;
    bool occupied;
} ui_node;
typedef struct ui_tree {
    ui_node nodes[UI_MAX_NODES];
    size_t count;
    ui_tokens tokens;
    bool laid_out;
} ui_tree;

/* Initializes an empty tree with stable, opaque dark or light tokens. */
bool ui_tree_init(ui_tree *tree, ui_theme theme);
/* Adds a node. Zero preferred extent means flexible allocation on that axis. */
bool ui_tree_add(ui_tree *tree, const ui_node_spec *spec);
/* Runs bounded deterministic layout. The root occupies the full pixel viewport. */
bool ui_tree_layout(ui_tree *tree, uint32_t width, uint32_t height);
/* Renders opaque node backgrounds via the existing scalar SVG raster API. */
bool ui_tree_render(const ui_tree *tree, raster_tile *out_tile);
const ui_node *ui_tree_find(const ui_tree *tree, uint32_t id);

#endif
