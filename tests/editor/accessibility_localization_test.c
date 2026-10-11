#include "viewer_editor_internal.h"
#include "viewer_chrome.h"
#include "platform/platform_localization.h"

#include <stdio.h>
#include <string.h>

#define CHECK(condition, message) do { \
    if (!(condition)) { \
        (void)fprintf(stderr, "accessibility/localization: %s\n", message); \
        viewer_editor_destroy(editor); \
        return 1; \
    } \
} while (0)

static bool slice_contains(const char *bytes, size_t length, const char *needle)
{
    size_t needle_length = strlen(needle);
    if (needle_length == 0U || needle_length > length) return false;
    for (size_t offset = 0U; offset <= length - needle_length; ++offset)
        if (memcmp(bytes + offset, needle, needle_length) == 0) return true;
    return false;
}

static const ui_node *find_long_layer_row(const viewer_editor *editor)
{
    for (size_t index = 0U; index < editor->tree.count; ++index) {
        const ui_node *node = &editor->tree.nodes[index];
        if (node->spec.role == UI_ROLE_ROW && node->full_label_len > UI_TEXT_MAX_BYTES)
            return node;
    }
    return NULL;
}

int main(void)
{
    static const uint8_t source[] =
        "<svg xmlns='http://www.w3.org/2000/svg' width='128' height='96'>"
        "<rect id='rect-with-a-deliberately-long-accessible-identifier-"
        "0123456789-abcdefghijklmnopqrstuvwxyz-0123456789-abcdefghijk' "
        "x='12.5' y='8' width='40' height='24' fill='#336699'/></svg>";
    viewer_editor *editor = viewer_editor_create_styled(
        (base_bytes){source, sizeof(source) - 1U}, UI_THEME_DARK, 1280U, 800U, false);
    CHECK(editor != NULL, "editor creation failed");
    editor->modern_style = true;

    char localized[PLATFORM_LOCALIZATION_TEXT_CAP] = {0};
    size_t localized_length = 0U;
    CHECK(platform_localization_copy_default("Layers", 6U, localized,
        sizeof(localized), &localized_length), "default string lookup failed");
    CHECK(localized_length == 6U && memcmp(localized, "Layers", 6U) == 0,
        "English resource lookup changed the fallback");
    char count[128] = {0};
    CHECK(platform_localization_format_count_message(12U,
        "editor.status.one_layer_selected", "1 layer selected",
        "editor.status.layers_selected", "%@ layers selected",
        "Layer selection count.", count, sizeof(count), &localized_length),
        "count message formatting failed");
    CHECK(strstr(count, "12 layers selected") != NULL,
        "count message did not retain its localized format");
    double measurement = 1234.5;
    char formatted_number[128] = {0};
    CHECK(platform_localization_format_number(measurement, 1U, formatted_number,
        sizeof(formatted_number), &localized_length), "number formatting failed");
    CHECK(localized_length > 0U && measurement == 1234.5,
        "measurement formatting changed its source value");

    editor_build_tree(editor);
    CHECK(editor->tree_ready, "modern semantic tree did not build");
    const ui_node *long_row = find_long_layer_row(editor);
    CHECK(long_row != NULL, "long layer label was not retained in the tree");
    const char *full_label = editor->tree.text_storage + long_row->label_offset;
    CHECK(slice_contains(full_label, long_row->full_label_len,
        "abcdefghijklmnopqrstuvwxyz"), "full VoiceOver label was truncated");
    const char *rectangle_tag = strstr((const char *)source, "<rect");
    CHECK(rectangle_tag != NULL && editor_canvas_selection_toggle(editor,
        (size_t)(rectangle_tag - (const char *)source)),
        "canvas selection setup failed");
    editor->tree_ready = false;
    editor_build_tree(editor);
    bool announced_canvas_selection = false;
    for (size_t index = 0U; index < editor->tree.count; ++index) {
        const ui_node *node = &editor->tree.nodes[index];
        if (node->spec.role != UI_ROLE_ROW ||
            (node->spec.state & UI_STATE_SELECTED) == 0U) continue;
        const char *value = editor->tree.text_storage + node->value_offset;
        announced_canvas_selection = node->full_value_len ==
            sizeof("Selected on canvas") - 1U &&
            memcmp(value, "Selected on canvas", node->full_value_len) == 0;
    }
    CHECK(announced_canvas_selection,
        "canvas selection state/value is missing from the accessible layer row");

    uint8_t materialized[sizeof(source) + 32U] = {0};
    size_t materialized_length = 0U;
    CHECK(viewer_editor_materialize(editor, materialized, sizeof(materialized),
        &materialized_length), "source materialization failed");
    CHECK(materialized_length == sizeof(source) - 1U &&
        memcmp(materialized, source, materialized_length) == 0,
        "initial source changed during UI construction");

    platform_localization_test_set_pseudo_locale(true);
    viewer_editor_set_status(editor, "Ready");
    editor_build_tree(editor);
    const ui_node *layers_title = ui_tree_find(&editor->tree, EDITOR_NODE_OUTLINE_TITLE);
    CHECK(layers_title != NULL && layers_title->full_label_len > 0U,
        "localized Layers title is missing");
    const char *localized_title = editor->tree.text_storage + layers_title->label_offset;
    CHECK(slice_contains(localized_title, layers_title->full_label_len, "⟦"),
        "pseudo-locale was not applied to an editor label");
    CHECK(slice_contains(localized_title, layers_title->full_label_len,
        "\xe2\x80\xab"), "pseudo-locale did not exercise RTL ordering");
    CHECK(platform_localization_format_count_message(12U,
        "editor.status.one_layer_selected", "1 layer selected",
        "editor.status.layers_selected", "%@ layers selected",
        "Layer selection count.", count, sizeof(count), &localized_length),
        "pseudo-locale count formatting failed");
    CHECK(slice_contains(count, localized_length, "12 layers selected"),
        "pseudo-locale corrupted the count placeholder");
    platform_localization_test_set_pseudo_locale(false);
    CHECK(viewer_editor_materialize(editor, materialized, sizeof(materialized),
        &materialized_length), "post-localization materialization failed");
    CHECK(materialized_length == sizeof(source) - 1U &&
        memcmp(materialized, source, materialized_length) == 0,
        "localization changed SVG source bytes");

    editor->focus_id = EDITOR_OUTLINE_ROW_BASE;
    editor->tree_ready = false;
    (void)viewer_editor_key(editor, 125U, 0U, NULL, 0U);
    CHECK(editor->selected_index == 1U && editor->focus_id == EDITOR_OUTLINE_ROW_BASE + 1U,
        "Down did not move focus and selection in the focused layer list");

    editor->focus_id = EDITOR_RENAME_ID;
    editor->tree_ready = false;
    size_t old_field_length = strlen(editor->selected_id);
    (void)viewer_editor_key(editor, 0U, 0U, "x", 1U);
    CHECK(editor->field_kind == EDITOR_FIELD_RENAME &&
        editor->field_length == old_field_length + 1U,
        "focused inspector text field did not accept ordinary text");
    uint32_t tab_result = viewer_editor_key(editor, 48U, 0U, NULL, 0U);
    CHECK((tab_result & VIEWER_EDITOR_HANDLED) != 0U &&
        editor->field_kind == EDITOR_FIELD_NONE && editor->focus_id != 0U,
        "Tab did not commit the focused field and continue keyboard navigation");

    editor->canvas_zoom = 1.375;
    viewer_chrome_layout before = {0}, after = {0};
    CHECK(viewer_editor_get_chrome_layout(editor, &before),
        "100 percent layout did not compute");
    CHECK(viewer_editor_set_interface_scale(editor, 200U),
        "200 percent interface text size was rejected");
    CHECK(viewer_editor_get_chrome_layout(editor, &after),
        "200 percent layout did not compute");
    CHECK(after.toolbar.height > before.toolbar.height && editor->interface_scale_percent == 200U,
        "larger interface text did not reflow chrome metrics");
    CHECK(editor->canvas_zoom == 1.375,
        "interface text size changed canvas zoom");

    (void)viewer_editor_set_tool(editor, UI_ICON_PAN);
    editor->tree_ready = false;
    editor_build_tree(editor);
    const ui_node *pan_tool = ui_tree_find(&editor->tree, EDITOR_BUTTON_TOOL_PAN);
    CHECK(pan_tool != NULL && (pan_tool->spec.state & UI_STATE_CHECKED) != 0U,
        "active tool state is missing from the semantic tree");

    viewer_editor_destroy(editor);
    (void)puts("editor accessibility/localization regressions passed");
    return 0;
}
