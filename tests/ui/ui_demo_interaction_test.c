#include "ui_demo.h"

#include <stdint.h>
#include <stdio.h>
#include <string.h>

int main(void)
{
    ui_tree tree;
    ui_command_registry commands;
    if (!ui_demo_tree_build(&tree, UI_THEME_DARK, 1280U, 800U) ||
        !ui_demo_commands_init(&commands)) return 1;
    ui_semantics_snapshot snapshot;
    if (!ui_semantics_snapshot_build(&tree, &snapshot)) return 2;
    uint32_t focusable_ids[32];
    size_t focusable_count = 0U;
    for (size_t index = 0U; index < snapshot.count; index++) {
        const ui_semantics_node *node = &snapshot.nodes[index];
        if (node->role == UI_ROLE_BUTTON || node->role == UI_ROLE_TEXT_FIELD ||
            node->role == UI_ROLE_SLIDER || node->role == UI_ROLE_ROW) {
            if (node->label_len == 0U || focusable_count >= 32U) return 3;
            focusable_ids[focusable_count++] = node->id;
        }
    }
    if (focusable_count < 10U) return 4;
    for (size_t index = 0U; index < focusable_count; index++) {
        if (!ui_focus_next(&tree, false) || tree.focused_id != focusable_ids[index]) { (void)fprintf(stderr, "focus index %zu expected %u got %u\n", index, focusable_ids[index], tree.focused_id); return 5; }
    }
    if (!ui_focus_next(&tree, false) || tree.focused_id != focusable_ids[0]) return 6;
    uint32_t command = 0U;
    if (!ui_dispatch_key(&tree, &commands, UI_KEY_RETURN, 0U, &command) ||
        command != UI_DEMO_COMMAND_NEW) return 7;
    if (!ui_dispatch_key(&tree, &commands, 's', UI_MOD_COMMAND, &command) ||
        command != UI_DEMO_COMMAND_SAVE) return 8;
    if (!ui_dispatch_key(&tree, &commands, UI_KEY_TAB, UI_MOD_SHIFT, &command) ||
        tree.focused_id != focusable_ids[focusable_count - 1U]) return 9;
    if (ui_tree_add(&tree, &(ui_node_spec){.id = 1000U, .parent_id = UI_ROOT_ID,
            .label = "ignored", .label_len = 7U})) return 10;
    ui_semantics_snapshot repeated;
    if (!ui_semantics_snapshot_build(&tree, &repeated) ||
        memcmp(&snapshot, &repeated, sizeof(snapshot)) == 0) {
        /* Focus is represented in tree state; this verifies the snapshot changed. */
        return 11;
    }
    return 0;
}
