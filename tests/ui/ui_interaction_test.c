#include "ui_core.h"
#include "ui_widgets.h"

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static bool add_control(ui_tree *tree, uint32_t id, const char *label,
                        ui_role role, uint32_t command, uint32_t state)
{
    ui_widget_spec spec = {0};
    spec.id = id;
    spec.parent_id = UI_ROOT_ID;
    spec.axis = UI_AXIS_HORIZONTAL;
    spec.color = tree->tokens.panel;
    spec.label = label;
    spec.label_len = strlen(label);
    spec.command_id = command;
    spec.state = state;
    switch (role) {
        case UI_ROLE_BUTTON: return ui_widget_add_button(tree, &spec);
        case UI_ROLE_TEXT_FIELD: return ui_widget_add_text_field(tree, &spec);
        case UI_ROLE_SLIDER: return ui_widget_add_slider(tree, &spec);
        case UI_ROLE_ROW: return ui_widget_add_row(tree, &spec);
        case UI_ROLE_CHECKBOX: return ui_widget_add_checkbox(tree, &spec);
        default: return false;
    }
}

int main(void)
{
    ui_tree tree;
    if (!ui_tree_init(&tree, UI_THEME_DARK) ||
        !add_control(&tree, 2U, "Open", UI_ROLE_BUTTON, 10U, 0U) ||
        !add_control(&tree, 3U, "Unavailable", UI_ROLE_BUTTON, 11U, UI_STATE_DISABLED) ||
        !add_control(&tree, 4U, "Fill color", UI_ROLE_TEXT_FIELD, 0U, 0U) ||
        !add_control(&tree, 5U, "Opacity", UI_ROLE_SLIDER, 12U, 0U)) return 1;
    char borrowed_label[] = "Document name";
    char borrowed_value[] = "Untitled";
    ui_node_spec owned = {0};
    owned.id = 6U; owned.parent_id = UI_ROOT_ID; owned.axis = UI_AXIS_HORIZONTAL;
    owned.role = UI_ROLE_TEXT_FIELD; owned.label = borrowed_label;
    owned.label_len = sizeof(borrowed_label) - 1U; owned.value = borrowed_value;
    owned.value_len = sizeof(borrowed_value) - 1U;
    if (!ui_tree_add(&tree, &owned) ||
        !add_control(&tree, 7U, "Show selection", UI_ROLE_CHECKBOX, 13U, UI_STATE_CHECKED)) return 1;
    memset(borrowed_label, 'x', sizeof(borrowed_label) - 1U);
    memset(borrowed_value, 'y', sizeof(borrowed_value) - 1U);
    if (!ui_focus_next(&tree, false) || tree.focused_id != 2U ||
        !ui_focus_next(&tree, false) || tree.focused_id != 4U ||
        !ui_focus_next(&tree, true) || tree.focused_id != 2U) return 2;
    ui_command_registry registry;
    if (!ui_command_registry_init(&registry) ||
        !ui_command_register(&registry, 20U, 's', UI_MOD_COMMAND, true) ||
        ui_command_register(&registry, 20U, 'x', 0U, true) ||
        ui_command_register(&registry, 21U, 's', UI_MOD_COMMAND, true)) return 3;
    ui_command_registry full;
    if (!ui_command_registry_init(&full)) return 3;
    for (uint32_t index = 0U; index < UI_MAX_COMMANDS; index++) {
        if (!ui_command_register(&full, 100U + index, 1000U + index, 0U, true)) return 3;
    }
    if (ui_command_register(&full, 1000U, 2000U, 0U, true) ||
        ui_command_register(&registry, 22U, 'q', 0x80000000U, true)) return 3;
    uint32_t command = 0U;
    if (!ui_dispatch_key(&tree, &registry, UI_KEY_RETURN, 0U, &command) || command != 10U) {
        (void)fprintf(stderr, "button activation failed; focus=%u command=%u\n",
                      tree.focused_id, command);
        return 4;
    }
    if (!ui_focus_next(&tree, false) || tree.focused_id != 4U ||
        !ui_focus_next(&tree, true) || tree.focused_id != 2U) {
        (void)fprintf(stderr, "forward/reverse focus order ended at id=%u\n", tree.focused_id);
        return 4;
    }
    for (size_t index = 0U; index < 4U; index++) {
        if (!ui_focus_next(&tree, false)) { (void)fprintf(stderr, "focus walk failed at %zu id=%u\n", index, tree.focused_id); return 4; }
    }
    if (tree.focused_id != 7U || !ui_dispatch_key(&tree, &registry, UI_KEY_SPACE, 0U, &command) ||
        command != 13U) {
        (void)fprintf(stderr, "checkbox activation failed; focus=%u command=%u\n",
                      tree.focused_id, command);
        return 4;
    }
    if (!ui_dispatch_key(&tree, &registry, 's', UI_MOD_COMMAND, &command) || command != 20U) return 5;
    if (!ui_dispatch_key(&tree, &registry, UI_KEY_TAB, UI_MOD_SHIFT, &command) ||
        tree.focused_id != 6U) return 6;
    ui_semantics_snapshot first = {0}, second = {0};
    if (!ui_semantics_snapshot_build(&tree, &first) ||
        !ui_semantics_snapshot_build(&tree, &second) ||
        memcmp(&first, &second, sizeof(first)) != 0 || first.count != 7U) return 7;
    if (first.nodes[1].id != 2U || first.nodes[1].parent_id != UI_ROOT_ID ||
        first.nodes[1].role != UI_ROLE_BUTTON || first.nodes[1].label_len != 4U ||
        memcmp(first.nodes[1].label, "Open", 4U) != 0 ||
        (first.nodes[1].state & UI_STATE_FOCUSED) != 0U ||
        first.nodes[4].id != 5U || (first.nodes[4].state & UI_STATE_FOCUSED) != 0U ||
        first.nodes[5].id != 6U || (first.nodes[5].state & UI_STATE_FOCUSED) == 0U ||
        first.nodes[5].label_len != 13U || first.nodes[5].value_len != 8U ||
        memcmp(first.nodes[5].label, "Document name", 13U) != 0 ||
        memcmp(first.nodes[5].value, "Untitled", 8U) != 0 ||
        first.nodes[6].role != UI_ROLE_CHECKBOX ||
        (first.nodes[6].state & UI_STATE_CHECKED) != 0U ||
        (first.nodes[6].state & UI_STATE_FOCUSED) != 0U) return 8;
    const char invalid_utf8[] = {(char)0xf0, (char)0x80, (char)0x80, (char)0x80};
    ui_node_spec invalid_text = {.id = 7U, .parent_id = UI_ROOT_ID,
        .role = UI_ROLE_TEXT, .label = invalid_utf8, .label_len = sizeof(invalid_utf8)};
    if (ui_tree_add(&tree, &invalid_text)) return 9;
    char long_label[UI_TEXT_MAX_BYTES + 2U];
    memset(long_label, 'a', sizeof(long_label));
    ui_node_spec expanded = {0};
    expanded.id = 9U; expanded.parent_id = UI_ROOT_ID;
    expanded.axis = UI_AXIS_HORIZONTAL; expanded.label = long_label;
    expanded.label_len = sizeof(long_label);
    if (!ui_tree_add(&tree, &expanded)) return 9;
    size_t over_limit_length = UI_TEXT_TOTAL_MAX_BYTES + 1U;
    char *over_limit = malloc(over_limit_length);
    if (over_limit == NULL) return 9;
    memset(over_limit, 'b', over_limit_length);
    ui_node_spec too_long = {.id = 10U, .parent_id = UI_ROOT_ID,
        .axis = UI_AXIS_HORIZONTAL, .label = over_limit,
        .label_len = over_limit_length};
    bool rejected_over_limit = !ui_tree_add(&tree, &too_long);
    free(over_limit);
    if (!rejected_over_limit || tree.nodes[7].spec.label_len != UI_TEXT_MAX_BYTES - 1U ||
        tree.nodes[7].full_label_len != sizeof(long_label)) return 9;
    ui_semantics_snapshot expanded_snapshot = {0};
    const char *expanded_text = NULL;
    size_t expanded_length = 0U;
    if (!ui_semantics_snapshot_build(&tree, &expanded_snapshot) ||
        !ui_semantics_text(&expanded_snapshot, &expanded_snapshot.nodes[7], false,
            &expanded_text, &expanded_length) || expanded_length != sizeof(long_label) ||
        memcmp(expanded_text, long_label, sizeof(long_label)) != 0) return 9;
    if (!ui_tree_layout(&tree, 640U, 360U)) return 10;
    raster_tile before_focus = {0}, after_focus = {0};
    if (!ui_tree_render(&tree, &before_focus) || !ui_focus_next(&tree, false) ||
        !ui_tree_render(&tree, &after_focus) || before_focus.byte_count != after_focus.byte_count ||
        memcmp(before_focus.rgba, after_focus.rgba, before_focus.byte_count) == 0) return 11;
    raster_tile_destroy(&before_focus);
    raster_tile_destroy(&after_focus);
    return 0;
}
