#include "viewer_internal.h"
#include "viewer_sessions.h"

#include <stdio.h>
#include <string.h>

static const uint8_t source_a[] =
    "<svg width='32' height='32'><rect id='first' x='1' y='1' width='8' height='8'/>"
    "</svg>";
static const uint8_t source_b[] =
    "<svg width='48' height='48'><circle id='second' cx='12' cy='12' r='6'/>"
    "</svg>";

static int fail(const char *message)
{
    (void)fprintf(stderr, "viewer sessions test: %s\n", message);
    return 1;
}

static size_t find_offset(const uint8_t *source, size_t length, const char *text)
{
    size_t text_length = strlen(text);
    if (text_length > length) return SIZE_MAX;
    for (size_t index = 0U; index <= length - text_length; ++index)
        if (memcmp(source + index, text, text_length) == 0) return index;
    return SIZE_MAX;
}

static bool source_matches(viewer_editor *editor, const uint8_t *expected,
                           size_t expected_length)
{
    uint8_t bytes[512];
    size_t length = 0U;
    return viewer_editor_materialize(editor, bytes, sizeof(bytes), &length) &&
        length == expected_length && memcmp(bytes, expected, length) == 0;
}

int main(void)
{
    viewer_app first = {0}, second = {0};
    first.editor = viewer_editor_create((base_bytes){source_a, sizeof(source_a) - 1U},
        UI_THEME_DARK, 1280U, 800U);
    second.editor = viewer_editor_create((base_bytes){source_b, sizeof(source_b) - 1U},
        UI_THEME_LIGHT, 1280U, 800U);
    if (first.editor == NULL || second.editor == NULL) return fail("create independent editors");
    memcpy(first.path, "/tmp/first.svg", sizeof("/tmp/first.svg"));
    memcpy(second.path, "/tmp/second.svg", sizeof("/tmp/second.svg"));
    first.navigation = (ui_viewer_navigation){.scale = 2.5, .pan_x = 14.0,
        .pan_y = -8.0, .viewport_width = 640.0, .viewport_height = 480.0};
    second.navigation = (ui_viewer_navigation){.scale = 0.75, .pan_x = -2.0,
        .pan_y = 6.0, .viewport_width = 720.0, .viewport_height = 520.0};
    size_t first_shape = find_offset(source_a, sizeof(source_a) - 1U, "<rect");
    size_t second_shape = find_offset(source_b, sizeof(source_b) - 1U, "<circle");
    if (first_shape == SIZE_MAX || second_shape == SIZE_MAX ||
        !viewer_editor_select_at_source_offset(first.editor, first_shape) ||
        !viewer_editor_select_at_source_offset(second.editor, second_shape))
        return fail("select independently in both documents");
    (void)viewer_editor_key(first.editor, 120U, 0U, NULL, 0U);
    (void)viewer_editor_key(first.editor, 0U, UI_MOD_COMMAND, "a", 1U);
    (void)viewer_editor_key(first.editor, 0U, 0U, "renamed", 7U);
    uint32_t changed = viewer_editor_key(first.editor, 36U, 0U, NULL, 0U);
    if ((changed & VIEWER_EDITOR_MUTATED) == 0U || !viewer_editor_is_dirty(first.editor) ||
        viewer_editor_is_dirty(second.editor)) return fail("dirty marker must stay session-local");
    uint32_t history = viewer_editor_key(first.editor, 0U,
        PLATFORM_MODIFIER_COMMAND, "z", 1U);
    if ((history & VIEWER_EDITOR_MUTATED) == 0U || viewer_editor_is_dirty(first.editor) ||
        viewer_editor_is_dirty(second.editor) ||
        !source_matches(first.editor, source_a, sizeof(source_a) - 1U) ||
        !source_matches(second.editor, source_b, sizeof(source_b) - 1U))
        return fail("undo must restore only the active document's independent history");
    history = viewer_editor_key(first.editor, 0U,
        PLATFORM_MODIFIER_COMMAND | PLATFORM_MODIFIER_SHIFT, "z", 1U);
    if ((history & VIEWER_EDITOR_MUTATED) == 0U || !viewer_editor_is_dirty(first.editor) ||
        viewer_editor_is_dirty(second.editor))
        return fail("redo state must stay isolated to its document session");

    viewer_session_collection sessions;
    viewer_sessions_init(&sessions);
    if (!viewer_sessions_add(&sessions, &first) || !viewer_sessions_add(&sessions, &second) ||
        sessions.active != &second || !viewer_sessions_activate(&sessions, &first) ||
        sessions.active != &first) return fail("activate a different native document session");
    if (!viewer_editor_is_dirty(first.editor) || viewer_editor_is_dirty(second.editor) ||
        first.navigation.scale != 2.5 || first.navigation.pan_x != 14.0 ||
        second.navigation.scale != 0.75 || second.navigation.pan_y != 6.0)
        return fail("tab activation must preserve each editor and viewport");
    document_source_span first_selection = {0}, second_selection = {0};
    if (!viewer_editor_selection_span(first.editor, &first_selection) ||
        !viewer_editor_selection_span(second.editor, &second_selection) ||
        first_selection.offset != first_shape || second_selection.offset != second_shape)
        return fail("tab activation must preserve each selection");
    uint8_t bytes[512];
    size_t byte_count = 0U;
    if (!viewer_editor_materialize(first.editor, bytes, sizeof(bytes), &byte_count) ||
        byte_count <= sizeof(source_a) - 1U ||
        memcmp(source_b, source_a, sizeof(source_a) - 1U) == 0)
        return fail("each tab must retain its own editable source and history");

    if (!viewer_sessions_remove(&sessions, &second) || sessions.count != 1U ||
        sessions.items[0] != &first || sessions.active != &first ||
        viewer_sessions_remove(&sessions, &second))
        return fail("closing one tab must remove only that stable session");
    viewer_editor_destroy(first.editor);
    viewer_editor_destroy(second.editor);
    return 0;
}
