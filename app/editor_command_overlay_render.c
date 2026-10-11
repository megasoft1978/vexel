#include "viewer_editor.h"
#include "viewer_editor_internal.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

#define RENDER_WIDTH 1280U
#define RENDER_HEIGHT 800U

static const uint8_t source[] =
    "<svg xmlns='http://www.w3.org/2000/svg' width='64' height='64'>"
    "<rect id='target' x='0' y='2' width='18' height='12' fill='#4f7cff'/>"
    "</svg>";

static bool set_coordinate(viewer_editor *editor, const char *name, unsigned value)
{
    char encoded[16] = {0};
    int amount = snprintf(encoded, sizeof(encoded), "%u", value);
    if (amount < 0 || (size_t)amount >= sizeof(encoded)) return false;
    document_handle target = {0};
    document_source_span span = {0};
    if (!document_edit_find_id(editor->edit,
            (base_bytes){(const uint8_t *)"target", 6U}, &target) ||
        !document_edit_source_span(editor->edit, target, &span) ||
        document_edit_set_attribute(editor->edit, target,
            (base_bytes){(const uint8_t *)name, strlen(name)},
            (base_bytes){(const uint8_t *)encoded, (size_t)amount}) != DOCUMENT_EDIT_OK)
        return false;
    editor_rebind_after_change(editor, span.offset, "target", (document_handle){0});
    return true;
}

static void fill_background(uint8_t *rgba, size_t bytes, ui_theme theme)
{
    uint8_t red = theme == UI_THEME_DARK ? 21U : 250U;
    uint8_t green = theme == UI_THEME_DARK ? 24U : 251U;
    uint8_t blue = theme == UI_THEME_DARK ? 30U : 253U;
    for (size_t at = 0U; at < bytes; at += 4U) {
        rgba[at] = red;
        rgba[at + 1U] = green;
        rgba[at + 2U] = blue;
        rgba[at + 3U] = 255U;
    }
}

static bool make_history(viewer_editor *editor)
{
    char path[] = "/tmp/vexel-overlay-render.XXXXXX";
    int descriptor = mkstemp(path);
    if (descriptor < 0) return false;
    bool okay = close(descriptor) == 0;
    for (unsigned index = 1U; okay && index <= 18U; ++index) {
        okay = set_coordinate(editor, (index % 2U) == 1U ? "x" : "y",
            (index % 2U) == 1U ? index : index + 10U);
        if (okay && index == 9U) {
            document_edit_save_result saved = viewer_editor_save(editor,
                (base_bytes){(const uint8_t *)path, strlen(path)});
            okay = saved.error == DOCUMENT_EDIT_OK;
        }
    }
    (void)unlink(path);
    return okay;
}

int main(int argc, char **argv)
{
    if (argc != 3 || (strcmp(argv[1], "dark") != 0 &&
            strcmp(argv[1], "light") != 0) ||
        (strcmp(argv[2], "palette") != 0 && strcmp(argv[2], "history") != 0)) {
        (void)fprintf(stderr, "usage: editor_command_overlay_render dark|light palette|history\n");
        return 2;
    }
    ui_theme theme = strcmp(argv[1], "light") == 0 ? UI_THEME_LIGHT : UI_THEME_DARK;
    viewer_editor *editor = viewer_editor_create_styled(
        (base_bytes){source, sizeof(source) - 1U}, theme,
        RENDER_WIDTH, RENDER_HEIGHT, true);
    size_t bytes = (size_t)RENDER_WIDTH * RENDER_HEIGHT * 4U;
    uint8_t *rgba = malloc(bytes);
    if (editor == NULL || rgba == NULL) {
        viewer_editor_destroy(editor);
        free(rgba);
        return 1;
    }
    fill_background(rgba, bytes, theme);
    bool okay = true;
    if (strcmp(argv[2], "palette") == 0) {
        okay = viewer_editor_open_command_palette(editor) &&
            (viewer_editor_key(editor, 0U, 0U, "group", 5U) &
                VIEWER_EDITOR_HANDLED) != 0U;
    } else {
        okay = make_history(editor) && viewer_editor_open_history(editor);
    }
    if (okay) viewer_editor_paint(editor, rgba, bytes,
        (size_t)RENDER_WIDTH * 4U);
    if (okay) okay = fwrite(rgba, 1U, bytes, stdout) == bytes;
    viewer_editor_destroy(editor);
    free(rgba);
    return okay ? 0 : 1;
}
