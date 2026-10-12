/* WO-UI-01: task workspaces and the consolidated status bar. */

#include "viewer_editor.h"
#include "viewer_editor_internal.h"
#include "viewer_workspace.h"
#include "ui/ui_core.h"
#include "platform/platform_settings.h"
#include "platform/platform_window.h"
#include "platform/platform_recents.h"

#define _GNU_SOURCE
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
    /* Draw: Object tab. Nodes: Object tab with Geometry open. Animate: keeps
     * the current tab and section, shows the timeline. Inspect: Document tab. */
    static const struct {
        const char *name, *label, *tab, *section;
        bool structure, inspector, timeline;
    } expected[4] = {
        {"draw", "Draw", "Object", NULL, true, true, false},
        {"nodes", "Nodes", "Object", "Geometry", true, true, false},
        {"animate", "Animate", NULL, NULL, true, true, true},
        {"inspect", "Inspect", "Document", NULL, true, true, false}
    };
    for (size_t index = 0U; index < 4U; ++index) {
        const viewer_workspace_spec *spec =
            viewer_workspace_get((viewer_workspace_kind)index);
        if (strcmp(spec->name, expected[index].name) != 0 ||
            strcmp(spec->label, expected[index].label) != 0 ||
            (spec->inspector_tab == NULL) != (expected[index].tab == NULL) ||
            (spec->inspector_tab != NULL &&
                strcmp(spec->inspector_tab, expected[index].tab) != 0) ||
            (spec->inspector_section == NULL) != (expected[index].section == NULL) ||
            (spec->inspector_section != NULL &&
                strcmp(spec->inspector_section, expected[index].section) != 0) ||
            spec->structure != expected[index].structure ||
            spec->inspector != expected[index].inspector ||
            spec->timeline != expected[index].timeline) return 3;
        if (strcmp(viewer_workspace_name((viewer_workspace_kind)index),
                expected[index].name) != 0) return 4;
    }
    /* An out-of-range kind degrades to the default rather than reading out of bounds. */
    if (viewer_workspace_get((viewer_workspace_kind)99) !=
        viewer_workspace_get(VIEWER_WORKSPACE_DRAW)) return 5;
    if (strcmp(viewer_workspace_shortcut_hint(), "Control-PageDown") != 0) return 6;
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
    /* Every entry point tolerates NULL rather than crashing. */
    if (viewer_editor_set_workspace(NULL, VIEWER_WORKSPACE_NODES) ||
        viewer_editor_cycle_workspace(NULL, false) ||
        viewer_editor_apply_workspace_name(NULL, "draw", 4U) ||
        viewer_editor_workspace(NULL) != VIEWER_WORKSPACE_DRAW) { result = 11; goto done; }
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
        /* Draw and Nodes use the Object tab, Inspect the Document tab. */
        if (spec->inspector_tab != NULL) {
            uint8_t tab = strcmp(spec->inspector_tab, "Document") == 0 ?
                (uint8_t)(EDITOR_BUTTON_INSPECTOR_TAB_DOCUMENT -
                    EDITOR_BUTTON_INSPECTOR_TAB_OBJECT) : 0U;
            if (editor->inspector_tab != tab) { result = 12; goto done; }
        }
        if (kind == VIEWER_WORKSPACE_NODES && !editor->inspector_geometry_expanded) {
            result = 13;
            goto done;
        }
    }
    /* Nodes opens a collapsed geometry section. */
    editor->inspector_geometry_expanded = false;
    (void)viewer_editor_set_workspace(editor, VIEWER_WORKSPACE_NODES);
    if (!editor->inspector_geometry_expanded) { result = 18; goto done; }
    /* Animate keeps whatever tab and section the user had open. */
    (void)viewer_editor_set_workspace(editor, VIEWER_WORKSPACE_DRAW);
    editor->inspector_tab = (uint8_t)(EDITOR_BUTTON_INSPECTOR_TAB_STROKE -
        EDITOR_BUTTON_INSPECTOR_TAB_OBJECT);
    editor->inspector_geometry_expanded = false;
    (void)viewer_editor_set_workspace(editor, VIEWER_WORKSPACE_ANIMATE);
    if (editor->inspector_tab != (uint8_t)(EDITOR_BUTTON_INSPECTOR_TAB_STROKE -
            EDITOR_BUTTON_INSPECTOR_TAB_OBJECT) || editor->inspector_geometry_expanded) {
        result = 17;
        goto done;
    }
    /* The timeline follows the workspace once a timeline is available. */
    if (!viewer_editor_set_timeline_state(editor, true, false, 220U)) { result = 14; goto done; }
    (void)viewer_editor_set_workspace(editor, VIEWER_WORKSPACE_ANIMATE);
    if (!editor->timeline_visible) { result = 15; goto done; }
    (void)viewer_editor_set_workspace(editor, VIEWER_WORKSPACE_INSPECT);
    if (editor->timeline_visible) { result = 16; goto done; }
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

/* The workspace strip and the three status regions must be visible to the
 * accessibility tree, not only painted. */
static int check_semantics(void)
{
    viewer_editor *editor = viewer_editor_create_styled((base_bytes){source,
        sizeof(source) - 1U}, UI_THEME_DARK, 1280U, 800U, true);
    if (editor == NULL) return 1;
    ui_semantics_snapshot *snapshot = malloc(sizeof(*snapshot));
    size_t pixels = 0U;
    uint8_t *frame = malloc(1280U * 800U * 4U);
    int result = 0;
    if (snapshot == NULL || frame == NULL ||
        !viewer_editor_semantics(editor, snapshot)) { result = 2; goto done; }
    (void)pixels; (void)frame;
    size_t tabs = 0U, hint = 0U, report = 0U, readout = 0U;
    bool draw_checked = false, hint_value = false;
    for (size_t index = 0U; index < snapshot->count; ++index) {
        const ui_semantics_node *node = &snapshot->nodes[index];
        if (node->id >= 24520U && node->id < 24524U) {
            tabs++;
            const char *full = NULL; size_t length = 0U;
            const char *value = NULL; size_t value_length = 0U;
            if (!ui_semantics_text(snapshot, node, false, &full, &length)) continue;
            (void)ui_semantics_text(snapshot, node, true, &value, &value_length);
            if (node->id == 24520U) {
                draw_checked = (node->state & UI_STATE_CHECKED) != 0U &&
                    length == 4U && memcmp(full, "Draw", 4U) == 0;
                hint_value = value != NULL && value_length == 16U &&
                    memcmp(value, "Control-PageDown", 16U) == 0;
            }
        }
        if (node->id == 24530U) hint++;
        if (node->id == 24531U) report++;
        if (node->id == 24532U) readout++;
        if (node->id == 24533U) result = 5; /* the cancel affordance is hidden */
    }
    if (tabs != 4U) result = 3;
    else if (!draw_checked) result = 4;
    else if (!hint_value) result = 6;
    else if (hint != 1U || report != 1U || readout != 1U) result = 7;
    /* The report region carries the operation message and its cancel button. */
    if (result == 0) {
        viewer_editor_set_operation_progress(editor, true, "Indexing 42%");
        if (!viewer_editor_semantics(editor, snapshot)) { result = 8; goto done; }
        bool cancel = false, message = false;
        for (size_t index = 0U; index < snapshot->count; ++index) {
            const ui_semantics_node *node = &snapshot->nodes[index];
            const char *full = NULL; size_t length = 0U;
            if (!ui_semantics_text(snapshot, node, false, &full, &length)) continue;
            if (node->id == 24533U) cancel = true;
            if (node->id == 24531U && length == 12U &&
                memcmp(full, "Indexing 42%", 12U) == 0) message = true;
        }
        if (!cancel) result = 9;
        else if (!message) result = 10;
    }
done:
    free(frame);
    free(snapshot);
    viewer_editor_destroy(editor);
    return result;
}

/* platform_settings.m dispatches menu actions to the window; the test has none. */
void platform_window_dispatch_action(platform_window *window,
    platform_viewer_action action)
{
    (void)window;
    (void)action;
}

/* Only the Settings window's "clear recents" button reaches this. */
void platform_recents_clear(void)
{
}

/* The stored workspace round-trips through the same store/load functions the
 * app uses, against a throwaway defaults domain (never the user's preferences). */
static void settings_persist(void *context, const char *name)
{
    (void)context;
    platform_settings_store_workspace(name);
}

static viewer_workspace_kind load_stored_workspace(viewer_editor *editor)
{
    platform_settings_values values = {0};
    platform_settings_get(&values);
    (void)viewer_editor_apply_workspace_name(editor, values.workspace,
        strlen(values.workspace));
    return viewer_editor_workspace(editor);
}

static int check_settings(void)
{
    platform_settings_use_test_domain("com.vexel.test.workspace-settings");
    int result = 0;
    viewer_editor *first = viewer_editor_create_styled((base_bytes){source,
        sizeof(source) - 1U}, UI_THEME_DARK, 1280U, 800U, true);
    viewer_editor *second = viewer_editor_create_styled((base_bytes){source,
        sizeof(source) - 1U}, UI_THEME_DARK, 1280U, 800U, true);
    viewer_editor *third = viewer_editor_create_styled((base_bytes){source,
        sizeof(source) - 1U}, UI_THEME_DARK, 1280U, 800U, true);
    if (first == NULL || second == NULL || third == NULL) { result = 1; goto done; }
    /* Nothing stored: the default workspace, and loading writes nothing beyond
     * the one-time defaults seed. */
    platform_settings_values values = {0};
    platform_settings_get(&values);
    size_t seeded = platform_settings_write_count();
    if (load_stored_workspace(second) != VIEWER_WORKSPACE_DRAW ||
        platform_settings_write_count() != seeded) { result = 2; goto done; }
    /* Switching in the first editor stores the name; a fresh editor restores it. */
    viewer_editor_set_workspace_callback(first, settings_persist, NULL);
    (void)viewer_editor_set_workspace(first, VIEWER_WORKSPACE_INSPECT);
    if (load_stored_workspace(second) != VIEWER_WORKSPACE_INSPECT) { result = 3; goto done; }
    (void)viewer_editor_set_workspace(first, VIEWER_WORKSPACE_ANIMATE);
    if (load_stored_workspace(second) != VIEWER_WORKSPACE_ANIMATE) { result = 4; goto done; }
    /* A stored unknown name loads as Draw and performs zero settings writes. */
    platform_settings_store_workspace("sculpt");
    platform_settings_get(&values);
    if (strcmp(values.workspace, "sculpt") != 0) { result = 5; goto done; }
    size_t before_writes = platform_settings_write_count();
    if (load_stored_workspace(third) != VIEWER_WORKSPACE_DRAW) { result = 6; goto done; }
    if (platform_settings_write_count() != before_writes) { result = 7; goto done; }
    /* Storing the value that is already stored is not a write either. */
    platform_settings_store_workspace("sculpt");
    if (platform_settings_write_count() != before_writes) { result = 8; goto done; }
done:
    viewer_editor_destroy(first);
    viewer_editor_destroy(second);
    viewer_editor_destroy(third);
    platform_settings_use_test_domain(NULL);
    return result;
}

/* The right region of the status bar reports document size with a unit. */
static int check_status_readout(void)
{
    char text[32];
    editor_format_byte_size(0U, text, sizeof(text));
    if (strcmp(text, "0 B") != 0) return 1;
    editor_format_byte_size(1023U, text, sizeof(text));
    if (strcmp(text, "1023 B") != 0) return 2;
    editor_format_byte_size(1024U, text, sizeof(text));
    if (strcmp(text, "1.0 KB") != 0) return 3;
    editor_format_byte_size(1258291U, text, sizeof(text));
    if (strcmp(text, "1.2 MB") != 0) return 4;
    viewer_editor *editor = viewer_editor_create_styled((base_bytes){source,
        sizeof(source) - 1U}, UI_THEME_DARK, 1280U, 800U, true);
    ui_semantics_snapshot *snapshot = malloc(sizeof(*snapshot));
    int result = 0;
    if (editor != NULL) {
        editor->canvas_zoom = 1.5;
        editor->tree_ready = false;
    }
    if (editor == NULL || snapshot == NULL ||
        !viewer_editor_semantics(editor, snapshot)) { result = 5; goto done; }
    char expected[32];
    editor_format_byte_size(viewer_editor_byte_length(editor), expected,
        sizeof(expected));
    if (viewer_editor_byte_length(editor) != sizeof(source) - 1U) { result = 6; goto done; }
    bool found = false;
    for (size_t index = 0U; index < snapshot->count; ++index) {
        const ui_semantics_node *node = &snapshot->nodes[index];
        const char *full = NULL; size_t length = 0U;
        if (node->id != 24532U ||
            !ui_semantics_text(snapshot, node, false, &full, &length)) continue;
        /* Selection count, zoom and size all appear in the readout. */
        char readout[256] = {0};
        if (length >= sizeof(readout)) { result = 7; goto done; }
        memcpy(readout, full, length);
        found = strstr(readout, "No selection") != NULL &&
            strstr(readout, "150%") != NULL && strstr(readout, expected) != NULL;
    }
    if (!found) result = 8;
done:
    free(snapshot);
    viewer_editor_destroy(editor);
    return result;
}

/* Ending an operation clears its own progress text but keeps a later result. */
static int check_progress_lifecycle(void)
{
    viewer_editor *editor = viewer_editor_create_styled((base_bytes){source,
        sizeof(source) - 1U}, UI_THEME_DARK, 1280U, 800U, true);
    if (editor == NULL) return 1;
    int result = 0;
    viewer_editor_set_operation_progress(editor, true, "Rendering preview");
    if (!editor->status_operation_active ||
        strcmp(editor->status, "Rendering preview") != 0) { result = 2; goto done; }
    viewer_editor_set_operation_progress(editor, false, NULL);
    if (editor->status_operation_active || editor->status[0] != '\0') { result = 3; goto done; }
    viewer_editor_set_operation_progress(editor, true, "Exporting PNG");
    viewer_editor_set_status(editor, "PNG exported");
    viewer_editor_set_operation_progress(editor, false, NULL);
    if (editor->status_operation_active ||
        strcmp(editor->status, "PNG exported") != 0) result = 4;
done:
    viewer_editor_destroy(editor);
    return result;
}

/* WO-UI-03: the bounded operator/warning/error log. */
static int check_log(void)
{
    viewer_editor *editor = viewer_editor_create_styled((base_bytes){source,
        sizeof(source) - 1U}, UI_THEME_DARK, 1280U, 800U, true);
    if (editor == NULL) return 1;
    int result = 0;
    if (viewer_editor_log_count(editor) != 0U || viewer_editor_log_open(editor)) {
        result = 2;
        goto done;
    }
    viewer_editor_log(editor, VIEWER_EDITOR_LOG_OPERATION, "Set Fill", 8U);
    viewer_editor_log(editor, VIEWER_EDITOR_LOG_WARNING, "Unsaved changes", 15U);
    viewer_editor_log(editor, VIEWER_EDITOR_LOG_ERROR, "Invalid argument", 16U);
    if (viewer_editor_log_count(editor) != 3U) { result = 3; goto done; }
    uint8_t severity = 0U;
    char text[128] = {0};
    size_t length = 0U;
    if (!viewer_editor_log_at(editor, 2U, &severity, text, sizeof(text), &length) ||
        severity != VIEWER_EDITOR_LOG_ERROR || length != 16U ||
        memcmp(text, "Invalid argument", 16U) != 0) { result = 4; goto done; }
    /* The bound is real: overflow keeps the newest entries and drops the oldest. */
    for (size_t index = 0U; index < 200U; ++index)
        viewer_editor_log(editor, VIEWER_EDITOR_LOG_INFORMATION, "Chunk", 5U);
    if (viewer_editor_log_count(editor) != 64U) { result = 5; goto done; }
    if (!viewer_editor_log_at(editor, 63U, &severity, text, sizeof(text), &length) ||
        length != 5U || memcmp(text, "Chunk", 5U) != 0) { result = 6; goto done; }
    /* Toggling shows and hides the pane without touching the document. */
    uint8_t *before = malloc(sizeof(source));
    uint8_t *after = malloc(sizeof(source));
    size_t before_length = 0U, after_length = 0U;
    if (before == NULL || after == NULL ||
        !viewer_editor_materialize(editor, before, sizeof(source), &before_length)) {
        result = 7;
        free(before); free(after);
        goto done;
    }
    viewer_editor_set_log_open(editor, true);
    ui_semantics_snapshot *snapshot = malloc(sizeof(*snapshot));
    const ui_semantics_node *node = NULL;
    if (snapshot == NULL || !viewer_editor_semantics(editor, snapshot)) {
        result = 8;
    } else {
        for (size_t index = 0U; index < snapshot->count; ++index)
            if (snapshot->nodes[index].id == 24570U) node = &snapshot->nodes[index];
        if (node == NULL || node->frame.height == 0U) result = 9;
    }
    free(snapshot);
    if (!viewer_editor_materialize(editor, after, sizeof(source), &after_length) ||
        after_length != before_length || memcmp(before, after, before_length) != 0)
        result = 10;
    free(before); free(after);
    if (result == 0 && (viewer_editor_set_log_open(editor, false) == false)) result = 11;
done:
    viewer_editor_destroy(editor);
    return result;
}

/* A canvas gesture whose hit test missed leaves the selection on the document
 * root. Transforming the root used to rewrite the whole document, dropping the
 * shapes; it must now be refused with the document left untouched. */
static int check_root_transform_is_refused(void)
{
    viewer_editor *editor = viewer_editor_create_styled((base_bytes){source,
        sizeof(source) - 1U}, UI_THEME_DARK, 1280U, 800U, true);
    if (editor == NULL) return 1;
    int result = 0;
    uint8_t *before = malloc(sizeof(source));
    uint8_t *after = malloc(sizeof(source));
    if (before == NULL || after == NULL) { result = 2; goto cleanup; }
    size_t before_length = 0U, after_length = 0U;
    if (!viewer_editor_materialize(editor, before, sizeof(source), &before_length)) {
        result = 3;
        goto cleanup;
    }
    /* Nothing was selected, so the selection is the document root, which is
     * exactly the state a missed canvas hit test leaves behind. */
    (void)editor_commit_transform(editor, 2.0, 2.0, 1.0, 1.0, 0.0, 0.0, 0.0);
    if (!viewer_editor_materialize(editor, after, sizeof(source), &after_length) ||
        after_length != before_length || memcmp(before, after, before_length) != 0) {
        result = 4;   /* the document was rewritten: the defect */
        goto cleanup;
    }
    /* The shape survives and the root carries no matrix. */
    if (memmem(after, after_length, "<rect", 5U) == NULL) { result = 5; goto cleanup; }
    if (memmem(after, after_length, "transform=\"matrix(", 17U) != NULL) {
        result = 6;
        goto cleanup;
    }
cleanup:
    free(before); free(after);
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
    result = check_semantics();
    if (result != 0) { fprintf(stderr, "workspace semantics failed: %d\n", result); return 1; }
    result = check_root_transform_is_refused();
    if (result != 0) { fprintf(stderr, "root transform guard failed: %d\n", result); return 1; }
    result = check_log();
    if (result != 0) { fprintf(stderr, "log pane failed: %d\n", result); return 1; }
    result = check_editor();
    if (result != 0) { fprintf(stderr, "workspace editor failed: %d\n", result); return 1; }
    result = check_settings();
    if (result != 0) { fprintf(stderr, "workspace settings failed: %d\n", result); return 1; }
    result = check_status_readout();
    if (result != 0) { fprintf(stderr, "status readout failed: %d\n", result); return 1; }
    result = check_progress_lifecycle();
    if (result != 0) { fprintf(stderr, "progress lifecycle failed: %d\n", result); return 1; }
    printf("workspace tests passed: table, names, cycling, semantics, log, root-guard, panes, document invariance, settings, readout, progress\n");
    return 0;
}
