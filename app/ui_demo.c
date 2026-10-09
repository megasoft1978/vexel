#include "ui_demo.h"
#include "ui_widgets.h"

#include <stdbool.h>
#include <stdint.h>

static bool ui_demo_add(ui_tree *tree, uint32_t id, uint32_t parent, ui_axis axis,
                        uint32_t width, uint32_t height, uint32_t padding,
                        uint32_t gap, uint32_t color)
{
    ui_node_spec node = {0};
    node.id = id; node.parent_id = parent; node.axis = axis;
    node.preferred_width = width; node.preferred_height = height;
    node.padding = padding; node.gap = gap; node.color = color;
    switch (id) {
        case 3U: node.role = UI_ROLE_BUTTON; node.label = "New"; node.label_len = 3U; node.command_id = UI_DEMO_COMMAND_NEW; break;
        case 4U: node.role = UI_ROLE_BUTTON; node.label = "Open"; node.label_len = 4U; node.command_id = UI_DEMO_COMMAND_OPEN; break;
        case 5U: node.role = UI_ROLE_BUTTON; node.label = "Save"; node.label_len = 4U; node.command_id = UI_DEMO_COMMAND_SAVE; break;
        case 7U: node.role = UI_ROLE_BUTTON; node.label = "Undo"; node.label_len = 4U; node.command_id = UI_DEMO_COMMAND_UNDO; break;
        case 10U: node.role = UI_ROLE_BUTTON; node.label = "Select tool"; node.label_len = 11U; node.command_id = UI_DEMO_COMMAND_SELECT_TOOL; node.state = UI_STATE_CHECKED; break;
        case 11U: node.role = UI_ROLE_BUTTON; node.label = "Node tool"; node.label_len = 9U; node.command_id = UI_DEMO_COMMAND_NODE_TOOL; break;
        case 12U: node.role = UI_ROLE_BUTTON; node.label = "Rectangle tool"; node.label_len = 14U; node.command_id = UI_DEMO_COMMAND_RECT_TOOL; break;
        case 13U: node.role = UI_ROLE_BUTTON; node.label = "Text tool"; node.label_len = 9U; node.command_id = UI_DEMO_COMMAND_TEXT_TOOL; break;
        case 16U: node.role = UI_ROLE_BUTTON; node.label = "Zoom out"; node.label_len = 8U; node.command_id = UI_DEMO_COMMAND_ZOOM_OUT; break;
        case 17U: node.role = UI_ROLE_BUTTON; node.label = "Fit page"; node.label_len = 8U; node.command_id = UI_DEMO_COMMAND_FIT; break;
        case 19U: node.role = UI_ROLE_BUTTON; node.label = "Zoom in"; node.label_len = 7U; node.command_id = UI_DEMO_COMMAND_ZOOM_IN; break;
        case 25U: node.role = UI_ROLE_TEXT_FIELD; node.label = "Fill color"; node.label_len = 10U; node.value = "#6b9eff"; node.value_len = 7U; break;
        case 26U: node.role = UI_ROLE_TEXT_FIELD; node.label = "Stroke color"; node.label_len = 12U; node.value = "#283044"; node.value_len = 7U; break;
        case 27U: node.role = UI_ROLE_SLIDER; node.label = "Opacity"; node.label_len = 7U; node.value = "100%"; node.value_len = 4U; node.command_id = UI_DEMO_COMMAND_SET_OPACITY; break;
        case 29U: node.role = UI_ROLE_ROW; node.label = "Layer 1"; node.label_len = 7U; node.command_id = UI_DEMO_COMMAND_SELECT_LAYER_1; break;
        case 30U: node.role = UI_ROLE_ROW; node.label = "Layer 2"; node.label_len = 7U; node.command_id = UI_DEMO_COMMAND_SELECT_LAYER_2; break;
        case 31U: node.role = UI_ROLE_ROW; node.label = "Layer 3"; node.label_len = 7U; node.command_id = UI_DEMO_COMMAND_SELECT_LAYER_3; break;
        default: break;
    }
    ui_widget_spec widget = {
        .id = node.id, .parent_id = node.parent_id, .axis = node.axis,
        .preferred_width = node.preferred_width, .preferred_height = node.preferred_height,
        .padding = node.padding, .gap = node.gap, .color = node.color,
        .command_id = node.command_id, .state = node.state,
        .label = node.label, .label_len = node.label_len,
        .value = node.value, .value_len = node.value_len};
    switch (node.role) {
        case UI_ROLE_BUTTON: return ui_widget_add_button(tree, &widget);
        case UI_ROLE_CHECKBOX: return ui_widget_add_checkbox(tree, &widget);
        case UI_ROLE_TEXT_FIELD: return ui_widget_add_text_field(tree, &widget);
        case UI_ROLE_SLIDER: return ui_widget_add_slider(tree, &widget);
        case UI_ROLE_ROW: return ui_widget_add_row(tree, &widget);
        default: return ui_tree_add(tree, &node);
    }
}

bool ui_demo_tree_build(ui_tree *tree, ui_theme theme, uint32_t width, uint32_t height)
{
    if (tree == NULL || !ui_tree_init(tree, theme)) return false;
    const ui_tokens *t = &tree->tokens;
    bool okay = ui_demo_add(tree, 2U, UI_ROOT_ID, UI_AXIS_HORIZONTAL, 0U, 48U,
            12U, 10U, t->panel) &&
        ui_demo_add(tree, 3U, 2U, UI_AXIS_HORIZONTAL, 52U, 24U, 0U, 0U, t->accent) &&
        ui_demo_add(tree, 4U, 2U, UI_AXIS_HORIZONTAL, 116U, 24U, 0U, 0U, t->panel_alt) &&
        ui_demo_add(tree, 5U, 2U, UI_AXIS_HORIZONTAL, 92U, 24U, 0U, 0U, t->panel_alt) &&
        ui_demo_add(tree, 6U, 2U, UI_AXIS_HORIZONTAL, 0U, 24U, 0U, 0U, 0U) &&
        ui_demo_add(tree, 7U, 2U, UI_AXIS_HORIZONTAL, 82U, 24U, 0U, 0U, t->panel_alt) &&
        ui_demo_add(tree, 8U, UI_ROOT_ID, UI_AXIS_HORIZONTAL, 0U, 0U, 0U, 0U, t->window) &&
        ui_demo_add(tree, 9U, 8U, UI_AXIS_VERTICAL, 56U, 0U, 7U, 8U, t->panel) &&
        ui_demo_add(tree, 10U, 9U, UI_AXIS_VERTICAL, 40U, 40U, 0U, 0U, t->accent) &&
        ui_demo_add(tree, 11U, 9U, UI_AXIS_VERTICAL, 40U, 40U, 0U, 0U, t->panel_alt) &&
        ui_demo_add(tree, 12U, 9U, UI_AXIS_VERTICAL, 40U, 40U, 0U, 0U, t->panel_alt) &&
        ui_demo_add(tree, 13U, 9U, UI_AXIS_VERTICAL, 40U, 40U, 0U, 0U, t->panel_alt) &&
        ui_demo_add(tree, 14U, 8U, UI_AXIS_VERTICAL, 0U, 0U, 0U, 0U, t->canvas) &&
        ui_demo_add(tree, 15U, 14U, UI_AXIS_HORIZONTAL, 0U, 36U, 12U, 8U, t->panel) &&
        ui_demo_add(tree, 16U, 15U, UI_AXIS_HORIZONTAL, 76U, 18U, 0U, 0U, t->panel_alt) &&
        ui_demo_add(tree, 17U, 15U, UI_AXIS_HORIZONTAL, 76U, 18U, 0U, 0U, t->panel_alt) &&
        ui_demo_add(tree, 18U, 15U, UI_AXIS_HORIZONTAL, 0U, 18U, 0U, 0U, 0U) &&
        ui_demo_add(tree, 19U, 15U, UI_AXIS_HORIZONTAL, 52U, 18U, 0U, 0U, t->panel_alt) &&
        ui_demo_add(tree, 20U, 14U, UI_AXIS_OVERLAY, 0U, 0U, 0U, 0U, t->canvas) &&
        ui_demo_add(tree, 21U, 20U, UI_AXIS_HORIZONTAL, 180U, 92U, 0U, 0U, t->panel_alt) &&
        ui_demo_add(tree, 22U, 8U, UI_AXIS_VERTICAL, 276U, 0U, 0U, 1U, t->border) &&
        ui_demo_add(tree, 23U, 22U, UI_AXIS_VERTICAL, 0U, 42U, 14U, 0U, t->panel) &&
        ui_demo_add(tree, 24U, 22U, UI_AXIS_VERTICAL, 0U, 136U, 14U, 8U, t->panel) &&
        ui_demo_add(tree, 25U, 24U, UI_AXIS_VERTICAL, 0U, 28U, 0U, 4U, t->panel_alt) &&
        ui_demo_add(tree, 26U, 24U, UI_AXIS_VERTICAL, 0U, 28U, 0U, 4U, t->panel_alt) &&
        ui_demo_add(tree, 27U, 24U, UI_AXIS_VERTICAL, 0U, 28U, 0U, 4U, t->panel_alt) &&
        ui_demo_add(tree, 28U, 22U, UI_AXIS_VERTICAL, 0U, 192U, 14U, 8U, t->panel) &&
        ui_demo_add(tree, 29U, 28U, UI_AXIS_HORIZONTAL, 0U, 28U, 0U, 4U, t->panel_alt) &&
        ui_demo_add(tree, 30U, 28U, UI_AXIS_HORIZONTAL, 0U, 28U, 0U, 4U, t->panel_alt) &&
        ui_demo_add(tree, 31U, 28U, UI_AXIS_HORIZONTAL, 0U, 28U, 0U, 4U, t->panel_alt) &&
        ui_demo_add(tree, 32U, 22U, UI_AXIS_VERTICAL, 0U, 112U, 14U, 8U, t->panel) &&
        ui_demo_add(tree, 33U, 32U, UI_AXIS_HORIZONTAL, 0U, 28U, 0U, 8U, t->panel_alt) &&
        ui_demo_add(tree, 34U, 33U, UI_AXIS_HORIZONTAL, 72U, 0U, 0U, 0U, t->accent) &&
        ui_demo_add(tree, 35U, 33U, UI_AXIS_HORIZONTAL, 0U, 0U, 0U, 0U, t->panel_alt) &&
        ui_demo_add(tree, 36U, UI_ROOT_ID, UI_AXIS_HORIZONTAL, 0U, 26U, 12U, 0U, t->panel);
    return okay && ui_tree_layout(tree, width, height);
}

bool ui_demo_commands_init(ui_command_registry *registry)
{
    if (!ui_command_registry_init(registry)) return false;
    return ui_command_register(registry, UI_DEMO_COMMAND_NEW, 'n', UI_MOD_COMMAND, true) &&
        ui_command_register(registry, UI_DEMO_COMMAND_OPEN, 'o', UI_MOD_COMMAND, true) &&
        ui_command_register(registry, UI_DEMO_COMMAND_SAVE, 's', UI_MOD_COMMAND, true) &&
        ui_command_register(registry, UI_DEMO_COMMAND_UNDO, 'z', UI_MOD_COMMAND, true) &&
        ui_command_register(registry, UI_DEMO_COMMAND_REDO, 'z', UI_MOD_COMMAND | UI_MOD_SHIFT, true);
}

bool ui_demo_render(ui_theme theme, uint32_t width, uint32_t height, raster_tile *tile)
{
    ui_tree tree;
    return tile != NULL && ui_demo_tree_build(&tree, theme, width, height) &&
        ui_tree_render(&tree, tile);
}
