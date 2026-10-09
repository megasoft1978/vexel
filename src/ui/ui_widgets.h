#ifndef VEXEL_UI_WIDGETS_H
#define VEXEL_UI_WIDGETS_H

#include "ui_core.h"

typedef struct ui_widget_spec {
    uint32_t id;
    uint32_t parent_id;
    ui_axis axis;
    uint32_t preferred_width;
    uint32_t preferred_height;
    uint32_t padding;
    uint32_t gap;
    uint32_t color;
    uint32_t command_id;
    uint32_t state;
    const char *label;
    size_t label_len;
    const char *value;
    size_t value_len;
} ui_widget_spec;

bool ui_widget_add_button(ui_tree *tree, const ui_widget_spec *spec);
bool ui_widget_add_checkbox(ui_tree *tree, const ui_widget_spec *spec);
bool ui_widget_add_text_field(ui_tree *tree, const ui_widget_spec *spec);
bool ui_widget_add_slider(ui_tree *tree, const ui_widget_spec *spec);
bool ui_widget_add_row(ui_tree *tree, const ui_widget_spec *spec);

#endif
