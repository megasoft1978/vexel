#include "viewer_editor.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define EDITOR_SNAPSHOT_WIDTH 1280U
#define EDITOR_SNAPSHOT_HEIGHT 800U

int main(int argc, char **argv)
{
    if (argc != 2 || (strcmp(argv[1], "dark") != 0 && strcmp(argv[1], "light") != 0)) {
        (void)fprintf(stderr, "usage: editor_chrome_render dark|light\n");
        return 2;
    }
    static const uint8_t source[] =
        "<svg xmlns='http://www.w3.org/2000/svg' width='64' height='64'>"
        "<g id='layer'><rect id='selected' x='8' y='8' width='40' height='24' fill='#6b9eff'/>"
        "<circle id='dot' cx='32' cy='48' r='8' fill='#ef8354'/></g></svg>";
    ui_theme theme = strcmp(argv[1], "light") == 0 ? UI_THEME_LIGHT : UI_THEME_DARK;
    viewer_editor *editor = viewer_editor_create((base_bytes){source, sizeof(source) - 1U},
        theme, EDITOR_SNAPSHOT_WIDTH, EDITOR_SNAPSHOT_HEIGHT);
    size_t bytes = (size_t)EDITOR_SNAPSHOT_WIDTH * EDITOR_SNAPSHOT_HEIGHT * 4U;
    uint8_t *rgba = malloc(bytes);
    if (editor == NULL || rgba == NULL) {
        viewer_editor_destroy(editor);
        free(rgba);
        return 1;
    }
    uint8_t red = theme == UI_THEME_DARK ? 21U : 250U;
    uint8_t green = theme == UI_THEME_DARK ? 24U : 251U;
    uint8_t blue = theme == UI_THEME_DARK ? 30U : 253U;
    for (size_t at = 0U; at < bytes; at += 4U) {
        rgba[at] = red;
        rgba[at + 1U] = green;
        rgba[at + 2U] = blue;
        rgba[at + 3U] = 255U;
    }
    (void)viewer_editor_key(editor, 124U, 0U, NULL, 0U);
    viewer_editor_paint(editor, rgba, bytes, (size_t)EDITOR_SNAPSHOT_WIDTH * 4U);
    bool okay = fwrite(rgba, 1U, bytes, stdout) == bytes;
    viewer_editor_destroy(editor);
    free(rgba);
    return okay ? 0 : 1;
}
