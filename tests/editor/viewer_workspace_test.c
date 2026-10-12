/* WO-UI-01: task workspaces and the consolidated status bar. */

#include "viewer_editor.h"
#include "viewer_editor_internal.h"
#include "viewer_workspace.h"

#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static const uint8_t source[] =
    "<svg xmlns='http://www.w3.org/2000/svg' width='64' height='64' viewBox='0 0 64 64'>"
    "<g id='layer'><rect id='shape' x='4' y='6' width='20' height='12' fill='#336699'/>"
    "<circle id='dot' cx='40' cy='40' r='6'/></g></svg>";

static int check_table(void)
{
    if (viewer_workspace_count() != 4U) return 1;
    if (viewer_workspace_default() != VIEWER_WORKSPACE_DRAW) return 2;
    static const struct {
        const char *name, *label, *group;
        bool structure, inspector, timeline;
    } expected[4] = {
        {"draw", "Draw", "Object", true, true, false},
        {"nodes", "Nodes", "Geometry", true, true, false},
        {"animate", "Animate", "Object", true, true, true},
        {"inspect", "Inspect", "Document", true, true, false}
    };
    for (size_t index = 0U; index < 4U; ++index) {
        const viewer_workspace_spec *spec =
            viewer_workspace_get((viewer_workspace_kind)index);
        if (strcmp(spec->name, expected[index].name) != 0 ||
            strcmp(spec->label, expected[index].label) != 0 ||
            strcmp(spec->inspector_group, expected[index].group) != 0 ||
            spec->structure != expected[index].structure ||
            spec->inspector != expected[index].inspector ||
            spec->timeline != expected[index].timeline) return 3;
        if (strcmp(viewer_workspace_name((viewer_workspace_kind)index),
                expected[index].name) != 0) return 4;
    }
    /* An out-of-range kind degrades to the default rather than reading out of bounds. */
    if (viewer_workspace_get((viewer_workspace_kind)99) !=
        viewer_workspace_get(VIEWER_WORKSPACE_DRAW)) return 5;
    return 0;
}

static int check_names(void)
{
    if (viewer_workspace_from_name("nodes", 5U) != VIEWER_WORKSPACE_NODES) return 1;
    if (viewer_workspace_from_name("animate", 7U) != VIEWER_WORKSPACE_ANIMATE) return 2;
    if (viewer_workspace_from_name("inspect", 7U) != VIEWER_WORKSPACE_INSPECT) return 3;
    /* Unknown, empty and NULL all fall back to the default workspace. */
    if (viewer_workspace_from_name("sculpt", 6U) != VIEWER_WORKSPACE_DRAW) return 4;
    if (viewer_workspace_from_name("", 0U) != VIEWER_WORKSPACE_DRAW) return 5;
    if (viewer_workspace_from_name(NULL, 0U) != VIEWER_WORKSPACE_DRAW) return 6;
    /* A prefix is not a match. */
    if (viewer_workspace_from_name("node", 4U) != VIEWER_WORKSPACE_DRAW) return 7;
    return 0;
}

static int check_cycle(void)
{
    if (viewer_workspace_cycle(VIEWER_WORKSPACE_DRAW, false) != VIEWER_WORKSPACE_NODES)
        return 1;
    if (viewer_workspace_cycle(VIEWER_WORKSPACE_INSPECT, false) != VIEWER_WORKSPACE_DRAW)
        return 2;
    if (viewer_workspace_cycle(VIEWER_WORKSPACE_DRAW, true) != VIEWER_WORKSPACE_INSPECT)
        return 3;
    if (viewer_workspace_cycle(VIEWER_WORKSPACE_NODES, true) != VIEWER_WORKSPACE_DRAW)
        return 4;
    /* An out-of-range kind behaves as the default. */
    if (viewer_workspace_cycle((viewer_workspace_kind)42, false) != VIEWER_WORKSPACE_NODES)
        return 5;
    /* Four steps forward return to the start. */
    viewer_workspace_kind kind = VIEWER_WORKSPACE_DRAW;
    for (size_t step = 0U; step < 4U; ++step)
        kind = viewer_workspace_cycle(kind, false);
    if (kind != VIEWER_WORKSPACE_DRAW) return 6;
    return 0;
}

static int check_editor(void)
{
    viewer_editor *editor = viewer_editor_create_styled((base_bytes){source,
        sizeof(source) - 1U}, UI_THEME_DARK, 1280U, 800U, true);
    if (editor == NULL) return 1;
    int result = 0;
    uint8_t *before = malloc(sizeof(source));
    uint8_t *after = malloc(sizeof(source));
    size_t before_length = 0U, after_length = 0U;
    if (before == NULL || after == NULL ||
        !viewer_editor_materialize(editor, before, sizeof(source), &before_length)) {
        result = 2;
        goto done;
    }
    if (viewer_editor_workspace(editor) != VIEWER_WORKSPACE_DRAW) { result = 3; goto done; }
    /* Each workspace applies exactly the panes it documents. */
    for (size_t index = 0U; index < viewer_workspace_count(); ++index) {
        viewer_workspace_kind kind = (viewer_workspace_kind)index;
        const viewer_workspace_spec *spec = viewer_workspace_get(kind);
        (void)viewer_editor_set_workspace(editor, kind);
        if (viewer_editor_workspace(editor) != kind) { result = 4; goto done; }
        if (editor->want_structure != spec->structure ||
            editor->want_inspector != spec->inspector) { result = 5; goto done; }
        if (editor->timeline_available && editor->timeline_visible != spec->timeline) {
            result = 6;
            goto done;
        }
    }
    /* Cycle forward through every workspace and wrap back to the start. */
    (void)viewer_editor_set_workspace(editor, VIEWER_WORKSPACE_DRAW);
    for (size_t step = 0U; step < viewer_workspace_count(); ++step)
        (void)viewer_editor_cycle_workspace(editor, false);
    if (viewer_editor_workspace(editor) != VIEWER_WORKSPACE_DRAW) { result = 7; goto done; }
    (void)viewer_editor_cycle_workspace(editor, true);
    if (viewer_editor_workspace(editor) != VIEWER_WORKSPACE_INSPECT) { result = 8; goto done; }
    /* An unknown remembered name falls back without disturbing the document. */
    if (!viewer_editor_apply_workspace_name(editor, "sculpt", 6U) ||
        viewer_editor_workspace(editor) != VIEWER_WORKSPACE_DRAW) { result = 9; goto done; }
    if (!viewer_editor_materialize(editor, after, sizeof(source), &after_length) ||
        after_length != before_length ||
        memcmp(before, after, before_length) != 0) { result = 10; goto done; }
done:
    free(before);
    free(after);
    viewer_editor_destroy(editor);
    return result;
}

int main(void)
{
    int result = check_table();
    if (result != 0) { fprintf(stderr, "workspace table failed: %d\n", result); return 1; }
    result = check_names();
    if (result != 0) { fprintf(stderr, "workspace names failed: %d\n", result); return 1; }
    result = check_cycle();
    if (result != 0) { fprintf(stderr, "workspace cycling failed: %d\n", result); return 1; }
    result = check_editor();
    if (result != 0) { fprintf(stderr, "workspace editor failed: %d\n", result); return 1; }
    printf("workspace tests passed: table, names, cycling, panes, document invariance\n");
    return 0;
}
