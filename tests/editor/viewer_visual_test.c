#include "viewer_editor_internal.h"
#include "viewer_editor.h"
#include "viewer_chrome.h"
#include "platform/platform_window.h"

#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static const ui_semantics_node *find_label(const ui_semantics_snapshot *snapshot,
                                           const char *label)
{
    size_t length = strlen(label);
    for (size_t index = 0U; index < snapshot->count; ++index) {
        const ui_semantics_node *node = &snapshot->nodes[index];
        const char *full = NULL;
        size_t full_length = 0U;
        if (ui_semantics_text(snapshot, node, false, &full, &full_length) &&
            full_length == length && memcmp(full, label, length) == 0) return node;
    }
    return NULL;
}

static const ui_semantics_node *find_role_label(const ui_semantics_snapshot *snapshot,
    ui_role role, const char *label)
{
    size_t length = strlen(label);
    for (size_t index = 0U; index < snapshot->count; ++index) {
        const ui_semantics_node *node = &snapshot->nodes[index];
        const char *full = NULL;
        size_t full_length = 0U;
        if (node->role == role && ui_semantics_text(snapshot, node, false,
                &full, &full_length) && full_length == length &&
            memcmp(full, label, length) == 0) return node;
    }
    return NULL;
}

static bool click_node(viewer_editor *editor, const ui_semantics_node *node,
                       uint32_t offset_x)
{
    if (editor == NULL || node == NULL || offset_x >= node->frame.width) return false;
    uint32_t x = node->frame.x + offset_x;
    uint32_t y = node->frame.y + node->frame.height / 2U;
    (void)viewer_editor_pointer_move(editor, x, y);
    bool handled = (viewer_editor_pointer_down(editor, x, y, 0U) &
        VIEWER_EDITOR_HANDLED) != 0U;
    viewer_editor_pointer_up(editor);
    return handled;
}

static const uint8_t visual_source[] =
    "<svg xmlns='http://www.w3.org/2000/svg' width='64' height='64'>"
    "<rect id='shape' x='8' y='8' width='24' height='24'/></svg>";

static bool test_timeline_dock_and_reduce_motion(void)
{
    viewer_editor *editor = viewer_editor_create_styled(
        (base_bytes){visual_source, sizeof(visual_source) - 1U}, UI_THEME_DARK,
        1280U, 800U, true);
    ui_semantics_snapshot *snapshot = malloc(sizeof(*snapshot));
    if (editor == NULL || snapshot == NULL) {
        viewer_editor_destroy(editor);
        free(snapshot);
        return false;
    }
    viewer_chrome_layout layout;
    bool okay = viewer_editor_set_timeline_state(editor, true, true, 220U) &&
        viewer_editor_get_chrome_layout(editor, &layout) &&
        layout.timeline.height == 220U &&
        layout.timeline.y + layout.timeline.height <= layout.status.y &&
        layout.canvas.y + layout.canvas.height <= layout.timeline.y &&
        viewer_editor_semantics(editor, snapshot);
    const ui_semantics_node *toggle = okay ? find_label(snapshot,
        "Animation timeline") : NULL;
    okay = okay && toggle != NULL && click_node(editor, toggle,
        toggle->frame.width / 2U) && !viewer_editor_timeline_visible(editor) &&
        viewer_editor_get_chrome_layout(editor, &layout) &&
        layout.timeline.height == 32U;
    (void)viewer_editor_take_redraw_request(editor);
    okay = okay && viewer_editor_set_appearance(editor, UI_THEME_LIGHT, true, true) &&
        viewer_editor_reduced_motion(editor) &&
        viewer_editor_take_redraw_request(editor) &&
        viewer_editor_set_timeline_state(editor, true, true, 220U) &&
        viewer_editor_get_chrome_layout(editor, &layout) &&
        layout.timeline.height == 220U;
    free(snapshot);
    viewer_editor_destroy(editor);
    return okay;
}

static int test_scale_splitters_and_motion(viewer_editor *editor,
                                           ui_semantics_snapshot *snapshot)
{
    viewer_editor_set_motion_time(editor, 100.0);
    (void)viewer_editor_motion_tick(editor, 100.0);
    if (!viewer_editor_set_ui_scale(editor, 110U) ||
        viewer_editor_ui_scale_percent(editor) != 110U ||
        !viewer_editor_get_chrome_layout(editor, &(viewer_chrome_layout){0})) return 70;
    viewer_chrome_layout layout;
    if (!viewer_editor_get_chrome_layout(editor, &layout) || layout.toolbar.height != 44U ||
        viewer_editor_set_ui_scale(editor, 105U)) return 71;
    if (!viewer_editor_set_ui_scale(editor, 100U) ||
        !viewer_editor_set_sidebar_visibility(editor, false, true) ||
        !viewer_editor_motion_active(editor) ||
        !viewer_editor_get_chrome_layout(editor, &layout) ||
        !layout.structure_visible || layout.structure.width != EDITOR_LEFT_WIDTH) return 72;
    if (!viewer_editor_motion_tick(editor, 100.08) ||
        !viewer_editor_get_chrome_layout(editor, &layout) ||
        !layout.structure_visible || layout.structure.width != EDITOR_LEFT_WIDTH / 2U) return 73;
    if (!viewer_editor_set_sidebar_visibility(editor, true, true) ||
        !viewer_editor_get_chrome_layout(editor, &layout) ||
        layout.structure.width != EDITOR_LEFT_WIDTH / 2U ||
        !viewer_editor_motion_tick(editor, 100.24) ||
        !viewer_editor_get_chrome_layout(editor, &layout) ||
        layout.structure.width != EDITOR_LEFT_WIDTH) return 74;

    if (!viewer_editor_set_appearance(editor, UI_THEME_DARK, false, true) ||
        !viewer_editor_semantics(editor, snapshot)) return 75;
    const ui_semantics_node *structure_slider = find_role_label(snapshot,
        UI_ROLE_SLIDER, "Layers panel width");
    if (structure_slider == NULL || structure_slider->frame.width !=
            VIEWER_CHROME_SPLITTER_POINTS) return 76;
    uint32_t x = layout.structure_splitter.x + layout.structure_splitter.width / 2U;
    uint32_t y = layout.structure_splitter.y + layout.structure_splitter.height / 2U;
    if ((viewer_editor_pointer_down_click_count(editor, x, y, 0U, 1U) &
            VIEWER_EDITOR_HANDLED) == 0U) return 77;
    viewer_editor_pointer_move(editor, x + 32U, y);
    viewer_editor_pointer_up(editor);
    if (!viewer_editor_get_chrome_layout(editor, &layout) ||
        layout.structure.width != EDITOR_LEFT_WIDTH + 32U) return 78;
    if ((viewer_editor_key(editor, 124U, 0U, NULL, 0U) & VIEWER_EDITOR_HANDLED) == 0U ||
        !viewer_editor_get_chrome_layout(editor, &layout) ||
        layout.structure.width != EDITOR_LEFT_WIDTH + 40U) return 79;
    x = layout.structure_splitter.x + layout.structure_splitter.width / 2U;
    y = layout.structure_splitter.y + layout.structure_splitter.height / 2U;
    if ((viewer_editor_pointer_down_click_count(editor, x, y, 0U, 2U) &
            VIEWER_EDITOR_HANDLED) == 0U ||
        !viewer_editor_get_chrome_layout(editor, &layout) ||
        layout.structure.width != EDITOR_LEFT_WIDTH ||
        editor->focus_id != EDITOR_SPLITTER_STRUCTURE_ID ||
        !viewer_editor_semantics(editor, snapshot)) return 80;
    structure_slider = find_role_label(snapshot, UI_ROLE_SLIDER, "Layers panel width");
    if (structure_slider == NULL ||
        (structure_slider->state & UI_STATE_FOCUSED) == 0U) return 86;

    x = layout.inspector_splitter.x + layout.inspector_splitter.width / 2U;
    y = layout.inspector_splitter.y + layout.inspector_splitter.height / 2U;
    if ((viewer_editor_pointer_down_click_count(editor, x, y, 0U, 1U) &
            VIEWER_EDITOR_HANDLED) == 0U) return 81;
    viewer_editor_pointer_move(editor, x - 600U, y);
    viewer_editor_pointer_up(editor);
    if (!viewer_editor_get_chrome_layout(editor, &layout) ||
        layout.inspector.width != 440U) return 82;
    x = layout.inspector_splitter.x + layout.inspector_splitter.width / 2U;
    y = layout.inspector_splitter.y + layout.inspector_splitter.height / 2U;
    if ((viewer_editor_pointer_down_click_count(editor, x, y, 0U, 1U) &
            VIEWER_EDITOR_HANDLED) == 0U) return 83;
    viewer_editor_pointer_move(editor, x + 600U, y);
    viewer_editor_pointer_up(editor);
    if (!viewer_editor_get_chrome_layout(editor, &layout) ||
        layout.inspector.width != 240U) return 84;
    if (!viewer_editor_set_panel_widths(editor, EDITOR_LEFT_WIDTH,
            EDITOR_RIGHT_WIDTH)) return 85;
    return 0;
}

int main(void)
{
    if (!test_timeline_dock_and_reduce_motion()) return 1;
    viewer_editor *editor = viewer_editor_create_styled(
        (base_bytes){visual_source, sizeof(visual_source) - 1U},
        UI_THEME_DARK, 1280U, 800U, true);
    ui_semantics_snapshot *snapshot = malloc(sizeof(*snapshot));
    if (editor == NULL || snapshot == NULL) {
        viewer_editor_destroy(editor);
        free(snapshot);
        return 2;
    }
    int result = test_scale_splitters_and_motion(editor, snapshot);
    free(snapshot);
    viewer_editor_destroy(editor);
    if (result != 0) return result;
    (void)puts("visual editor tests passed: timeline, scale, splitters, motion");
    return 0;
}
