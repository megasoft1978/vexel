#include "ui_widgets.h"

static bool ui_widget_add(ui_tree *tree, const ui_widget_spec *spec, ui_role role)
{
    if (spec == NULL) return false;
    ui_node_spec node = {0};
    node.id = spec->id;
    node.parent_id = spec->parent_id;
    node.axis = spec->axis;
    node.preferred_width = spec->preferred_width;
    node.preferred_height = spec->preferred_height;
    node.padding = spec->padding;
    node.gap = spec->gap;
    node.color = spec->color;
    node.role = role;
    node.command_id = spec->command_id;
    node.state = spec->state;
    node.label = spec->label;
    node.label_len = spec->label_len;
    node.value = spec->value;
    node.value_len = spec->value_len;
    return ui_tree_add(tree, &node);
}

bool ui_widget_add_button(ui_tree *tree, const ui_widget_spec *spec)
{
    return ui_widget_add(tree, spec, UI_ROLE_BUTTON);
}

bool ui_widget_add_checkbox(ui_tree *tree, const ui_widget_spec *spec)
{
    return ui_widget_add(tree, spec, UI_ROLE_CHECKBOX);
}

bool ui_widget_add_text_field(ui_tree *tree, const ui_widget_spec *spec)
{
    return ui_widget_add(tree, spec, UI_ROLE_TEXT_FIELD);
}

bool ui_widget_add_slider(ui_tree *tree, const ui_widget_spec *spec)
{
    return ui_widget_add(tree, spec, UI_ROLE_SLIDER);
}

bool ui_widget_add_row(ui_tree *tree, const ui_widget_spec *spec)
{
    return ui_widget_add(tree, spec, UI_ROLE_ROW);
}
