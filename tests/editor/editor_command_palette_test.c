#include "viewer_editor.h"
#include "viewer_editor_internal.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

#define CHECK(condition) do { if (!(condition)) { \
    (void)fprintf(stderr, "FAIL %s:%d: %s\n", __FILE__, __LINE__, #condition); \
    goto done; } } while (0)

static const uint8_t source[] =
    "<svg width='32' height='32'><rect id='target' x='0' y='2' "
    "width='8' height='8'/></svg>";

static bool text_has(const char *text, const char *needle)
{
    return text != NULL && needle != NULL && strstr(text, needle) != NULL;
}

static bool set_coordinate(viewer_editor *editor, const char *name, unsigned value)
{
    char encoded[16] = {0};
    int amount = snprintf(encoded, sizeof(encoded), "%u", value);
    if (amount < 0 || (size_t)amount >= sizeof(encoded)) return false;
    document_handle target = {0};
    document_source_span span = {0};
    if (!document_edit_find_id(editor->edit,
            (base_bytes){(const uint8_t *)"target", 6U}, &target) ||
        !document_edit_source_span(editor->edit, target, &span)) return false;
    if (document_edit_set_attribute(editor->edit, target,
            (base_bytes){(const uint8_t *)name, strlen(name)},
            (base_bytes){(const uint8_t *)encoded, (size_t)amount}) != DOCUMENT_EDIT_OK)
        return false;
    editor_rebind_after_change(editor, span.offset, "target", (document_handle){0});
    return true;
}

static bool latest_history_label(viewer_editor *editor, const char *expected)
{
    size_t count = document_edit_history_count(editor->edit);
    document_edit_history_entry entry = {{0}, 0U, false, false};
    return count > 0U && document_edit_history_entry_at(editor->edit,
        count - 1U, &entry) && strcmp(entry.label, expected) == 0;
}

static bool test_named_selection_history(void)
{
    static const uint8_t two_shapes[] =
        "<svg width='32' height='32'><rect id='a' x='1' y='2' width='4' height='4'/>"
        "<rect id='b' x='10' y='2' width='4' height='4'/></svg>";
    viewer_editor *editor = viewer_editor_create_styled(
        (base_bytes){two_shapes, sizeof(two_shapes) - 1U}, UI_THEME_DARK,
        1280U, 800U, true);
    if (editor == NULL) return false;
    document_handle first = {0U}, second = {0U};
    document_source_span first_span = {0U, 0U}, second_span = {0U, 0U};
    bool okay = document_edit_find_id(editor->edit,
            (base_bytes){(const uint8_t *)"a", 1U}, &first) &&
        document_edit_find_id(editor->edit,
            (base_bytes){(const uint8_t *)"b", 1U}, &second) &&
        document_edit_source_span(editor->edit, first, &first_span) &&
        document_edit_source_span(editor->edit, second, &second_span) &&
        editor_canvas_selection_toggle(editor, first_span.offset) &&
        editor_canvas_selection_toggle(editor, second_span.offset);
    uint32_t result = okay ? editor_canvas_multi_transform(editor,
        2.0, 0.0, 1.0, 1.0, 0.0, 0.0, 0.0) : VIEWER_EDITOR_HANDLED;
    okay = okay && (result & VIEWER_EDITOR_MUTATED) != 0U &&
        latest_history_label(editor, "Move");
    result = okay ? editor_canvas_multi_transform(editor,
        0.0, 0.0, 1.5, 1.5, 0.0, 0.0, 0.0) : VIEWER_EDITOR_HANDLED;
    okay = okay && (result & VIEWER_EDITOR_MUTATED) != 0U &&
        latest_history_label(editor, "Resize");
    result = okay ? editor_canvas_multi_transform(editor,
        0.0, 0.0, 1.0, 1.0, 0.25, 0.0, 0.0) : VIEWER_EDITOR_HANDLED;
    okay = okay && (result & VIEWER_EDITOR_MUTATED) != 0U &&
        latest_history_label(editor, "Rotate");
    result = okay ? editor_canvas_multi_duplicate(editor) : VIEWER_EDITOR_HANDLED;
    okay = okay && (result & VIEWER_EDITOR_MUTATED) != 0U &&
        latest_history_label(editor, "Duplicate");
    result = okay ? editor_canvas_multi_delete(editor) : VIEWER_EDITOR_HANDLED;
    okay = okay && (result & VIEWER_EDITOR_MUTATED) != 0U &&
        latest_history_label(editor, "Delete");
    viewer_editor_destroy(editor);
    return okay;
}

static bool test_command_catalog_shortcut_contract(void)
{
    const viewer_command_descriptor *select =
        viewer_command_find(VIEWER_COMMAND_TOOL_SELECT);
    const viewer_command_descriptor *fit_width =
        viewer_command_find(VIEWER_COMMAND_FIT_WIDTH);
    const viewer_command_descriptor *pixel_grid =
        viewer_command_find(VIEWER_COMMAND_TOGGLE_PIXEL_GRID);
    const viewer_command_descriptor *search =
        viewer_command_find(VIEWER_COMMAND_SEARCH);
    return viewer_command_count() <= 64U && select != NULL && fit_width != NULL &&
        pixel_grid != NULL && search != NULL &&
        strcmp(select->shortcut, "V") == 0 && select->menu_key == NULL &&
        strcmp(fit_width->shortcut, "W") == 0 && fit_width->menu_key == NULL &&
        strcmp(pixel_grid->shortcut, "G / ⌘G") == 0 &&
        pixel_grid->menu_key != NULL && strcmp(pixel_grid->menu_key, "g") == 0 &&
        pixel_grid->menu_modifiers == VIEWER_COMMAND_MENU_COMMAND &&
        strcmp(search->shortcut, "⌘K") == 0 && search->menu_key != NULL &&
        strcmp(search->menu_key, "k") == 0 &&
        search->menu_modifiers == VIEWER_COMMAND_MENU_COMMAND;
}

static bool test_command_palette_and_history_popover(void)
{
    viewer_editor *editor = NULL;
    char save_path[] = "/tmp/vexel-history-ui.XXXXXX";
    int save_fd = -1;
    uint8_t materialized[1024];
    size_t materialized_length = 0U;
    int result = 1;
    editor = viewer_editor_create_styled((base_bytes){source, sizeof(source) - 1U},
        UI_THEME_DARK, 1280U, 800U, true);
    CHECK(editor != NULL);
    ui_semantics_snapshot semantics = {0};

    uint32_t key_result = viewer_editor_key(editor, 40U,
        PLATFORM_MODIFIER_COMMAND, "k", 1U);
    CHECK((key_result & VIEWER_EDITOR_HANDLED) != 0U);
    CHECK(viewer_editor_command_palette_is_open(editor));
    CHECK(viewer_editor_semantics(editor, &semantics));
    const ui_node *search_field = ui_tree_find(&editor->tree,
        EDITOR_PALETTE_SEARCH);
    CHECK(search_field != NULL &&
        (search_field->spec.state & UI_STATE_FOCUSED) != 0U);
    uint32_t search_x = search_field->bounds.x + search_field->bounds.width / 2U;
    uint32_t search_y = search_field->bounds.y + search_field->bounds.height / 2U;
    CHECK((viewer_editor_pointer_down(editor, search_x, search_y, 0U) &
        VIEWER_EDITOR_HANDLED) != 0U);
    CHECK(editor->focus_id == EDITOR_PALETTE_SEARCH);
    CHECK((viewer_editor_key(editor, 0U, 0U, "undo", 4U) &
        VIEWER_EDITOR_HANDLED) != 0U);
    CHECK(editor->palette_result_count == 2U);
    CHECK(editor->palette_results[0] == VIEWER_COMMAND_UNDO);
    CHECK(editor->palette_results[1] == VIEWER_COMMAND_UNDO_HISTORY);
    CHECK(viewer_editor_semantics(editor, &semantics));
    CHECK(semantics.count <= UI_MAX_NODES);
    const ui_node *undo_row = ui_tree_find(&editor->tree, EDITOR_PALETTE_RESULT_BASE);
    CHECK(undo_row != NULL && text_has(undo_row->label, "Undo"));
    CHECK((viewer_editor_key(editor, 125U, 0U, NULL, 0U) &
        VIEWER_EDITOR_HANDLED) != 0U);
    CHECK(editor->focus_id == EDITOR_PALETTE_RESULT_BASE + 1U);
    CHECK(viewer_editor_semantics(editor, &semantics));
    const ui_node *focused_row = ui_tree_find(&editor->tree,
        EDITOR_PALETTE_RESULT_BASE + 1U);
    CHECK(focused_row != NULL &&
        (focused_row->spec.state & UI_STATE_FOCUSED) != 0U);
    key_result = viewer_editor_key(editor, 36U, 0U, NULL, 0U);
    CHECK((key_result & VIEWER_EDITOR_COMMAND_SELECTED) != 0U);
    viewer_command_id selected = VIEWER_COMMAND_NONE;
    CHECK(viewer_editor_take_pending_command(editor, &selected));
    CHECK(selected == VIEWER_COMMAND_UNDO_HISTORY);
    CHECK(!viewer_editor_command_palette_is_open(editor));

    CHECK(viewer_editor_open_command_palette(editor));
    CHECK((viewer_editor_key(editor, 0U, 0U, "group", 5U) &
        VIEWER_EDITOR_HANDLED) != 0U);
    CHECK(viewer_editor_semantics(editor, &semantics));
    const ui_node *group_row = ui_tree_find(&editor->tree, EDITOR_PALETTE_RESULT_BASE);
    CHECK(group_row != NULL && group_row->spec.command_id == VIEWER_COMMAND_GROUP);
    CHECK((group_row->spec.state & UI_STATE_DISABLED) != 0U);
    CHECK(text_has(group_row->label, "Select 2+ layers"));
    CHECK(text_has(group_row->label, "⌘G") && strlen(group_row->label) < 64U);
    key_result = viewer_editor_key(editor, 36U, 0U, NULL, 0U);
    CHECK((key_result & VIEWER_EDITOR_HANDLED) != 0U);
    CHECK((key_result & VIEWER_EDITOR_COMMAND_SELECTED) == 0U);
    CHECK(!viewer_editor_take_pending_command(editor, &selected));
    CHECK(viewer_editor_key(editor, 53U, 0U, NULL, 0U) == VIEWER_EDITOR_HANDLED);

    save_fd = mkstemp(save_path);
    CHECK(save_fd >= 0);
    CHECK(close(save_fd) == 0);
    save_fd = -1;
    for (unsigned index = 1U; index <= 25U; ++index) {
        CHECK(set_coordinate(editor, (index % 2U) == 1U ? "x" : "y",
            (index % 2U) == 1U ? index : index + 10U));
        if (index == 10U) {
            document_edit_save_result saved = viewer_editor_save(editor,
                (base_bytes){(const uint8_t *)save_path, strlen(save_path)});
            CHECK(saved.error == DOCUMENT_EDIT_OK);
        }
    }
    CHECK(document_edit_history_count(editor->edit) == 25U);
    CHECK(document_edit_history_cursor(editor->edit) == 25U);
    CHECK(viewer_editor_semantics(editor, &semantics));
    const ui_node *history_button = ui_tree_find(&editor->tree, EDITOR_BUTTON_HISTORY);
    CHECK(history_button != NULL && history_button->bounds.width > 0U);
    uint32_t click_x = history_button->bounds.x + history_button->bounds.width / 2U;
    uint32_t click_y = history_button->bounds.y + history_button->bounds.height / 2U;
    CHECK((viewer_editor_pointer_down(editor, click_x, click_y, 0U) &
        VIEWER_EDITOR_HANDLED) != 0U);
    CHECK(viewer_editor_history_is_open(editor));
    CHECK(viewer_editor_semantics(editor, &semantics));
    CHECK(semantics.count <= UI_MAX_NODES);
    CHECK(editor->history_first_visible == 6U);
    const ui_node *first_row = ui_tree_find(&editor->tree, EDITOR_HISTORY_ROW_BASE);
    const ui_node *saved_row = ui_tree_find(&editor->tree, EDITOR_HISTORY_ROW_BASE + 4U);
    const ui_node *current_row = ui_tree_find(&editor->tree,
        EDITOR_HISTORY_ROW_BASE + 19U);
    const ui_node *saved_summary = ui_tree_find(&editor->tree, EDITOR_HISTORY_SAVED);
    CHECK(first_row != NULL && text_has(first_row->label, "Edit attribute"));
    CHECK(saved_row != NULL && text_has(saved_row->label, "Saved"));
    CHECK(current_row != NULL && text_has(current_row->label, "Current"));
    CHECK((current_row->spec.state & UI_STATE_FOCUSED) != 0U);
    CHECK(saved_summary != NULL && text_has(saved_summary->label, "Saved:"));
    CHECK(first_row->bounds.width > 0U && first_row->bounds.height > 0U);
    click_x = first_row->bounds.x + first_row->bounds.width / 2U;
    click_y = first_row->bounds.y + first_row->bounds.height / 2U;
    key_result = viewer_editor_pointer_down(editor, click_x, click_y, 0U);
    CHECK((key_result & VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(!viewer_editor_history_is_open(editor));
    CHECK(document_edit_history_cursor(editor->edit) == 6U);
    CHECK(viewer_editor_materialize(editor, materialized, sizeof(materialized),
        &materialized_length));
    static const char expected[] =
        "<svg width='32' height='32'><rect id='target' x='5' y='16' "
        "width='8' height='8'/></svg>";
    CHECK(materialized_length == sizeof(expected) - 1U &&
        memcmp(materialized, expected, sizeof(expected) - 1U) == 0);

    CHECK(set_coordinate(editor, "x", 99U));
    CHECK(document_edit_history_count(editor->edit) == 7U);
    CHECK(document_edit_history_cursor(editor->edit) == 7U);
    result = 0;
done:
    if (save_fd >= 0) (void)close(save_fd);
    (void)unlink(save_path);
    viewer_editor_destroy(editor);
    return result == 0;
}

int main(void)
{
    if (!test_command_catalog_shortcut_contract()) {
        (void)fputs("command catalog shortcut contract failed\n", stderr);
        return 1;
    }
    if (!test_named_selection_history()) {
        (void)fputs("named selection history labels failed\n", stderr);
        return 1;
    }
    if (!test_command_palette_and_history_popover()) return 1;
    (void)puts("editor command palette and history tests passed");
    return 0;
}
