#include "viewer_editor_internal.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define SNAPSHOT_WIDTH 1280U
#define SNAPSHOT_HEIGHT 800U

static const uint8_t source[] =
    "<svg xmlns='http://www.w3.org/2000/svg' width='64' height='64'>"
    "<rect x='8' y='8' width='40' height='24' fill='#6b9eff'/></svg>";

int main(int argc, char **argv)
{
    if (argc != 2 || (strcmp(argv[1], "dark") != 0 &&
            strcmp(argv[1], "light") != 0)) {
        (void)fprintf(stderr, "usage: editor_snap_popover_render dark|light\n");
        return 2;
    }
    ui_theme theme = strcmp(argv[1], "light") == 0 ? UI_THEME_LIGHT : UI_THEME_DARK;
    viewer_editor *editor = viewer_editor_create_styled(
        (base_bytes){source, sizeof(source) - 1U}, theme,
        SNAPSHOT_WIDTH, SNAPSHOT_HEIGHT, true);
    size_t stride = (size_t)SNAPSHOT_WIDTH * 4U;
    size_t bytes = stride * SNAPSHOT_HEIGHT;
    uint8_t *rgba = malloc(bytes);
    if (editor == NULL || rgba == NULL) {
        viewer_editor_destroy(editor);
        free(rgba);
        return 1;
    }
    viewer_editor_resize(editor, SNAPSHOT_WIDTH, SNAPSHOT_HEIGHT, 1.0);
    ui_semantics_snapshot semantics;
    if (!viewer_editor_semantics(editor, &semantics)) goto fail;
    const ui_node *trigger = NULL;
    for (size_t index = 0U; index < editor->tree.count; ++index)
        if (editor->tree.nodes[index].spec.id == EDITOR_BUTTON_SNAPPING) {
            trigger = &editor->tree.nodes[index];
            break;
        }
    if (trigger == NULL || (viewer_editor_pointer_down(editor,
            trigger->bounds.x + trigger->bounds.width / 2U,
            trigger->bounds.y + trigger->bounds.height / 2U, 0U) &
            VIEWER_EDITOR_HANDLED) == 0U || !editor->snap_popover_open) goto fail;
    viewer_editor_pointer_up(editor);
    ui_tokens tokens = ui_theme_tokens(theme, false);
    for (size_t at = 0U; at < bytes; at += 4U) {
        rgba[at] = (uint8_t)(tokens.window >> 24U);
        rgba[at + 1U] = (uint8_t)(tokens.window >> 16U);
        rgba[at + 2U] = (uint8_t)(tokens.window >> 8U);
        rgba[at + 3U] = 255U;
    }
    viewer_editor_paint(editor, rgba, bytes, stride);
    bool okay = fwrite(rgba, 1U, bytes, stdout) == bytes;
    viewer_editor_destroy(editor);
    free(rgba);
    return okay ? 0 : 1;

fail:
    viewer_editor_destroy(editor);
    free(rgba);
    (void)fputs("snapping popover renderer failed\n", stderr);
    return 1;
}
