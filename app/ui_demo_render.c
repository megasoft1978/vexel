#include "ui_demo.h"

#include <stdio.h>
#include <string.h>

int main(int argc, char **argv)
{
    if (argc != 2 || (strcmp(argv[1], "dark") != 0 && strcmp(argv[1], "light") != 0)) {
        (void)fprintf(stderr, "usage: ui_demo_render dark|light\n");
        return 2;
    }
    ui_theme theme = strcmp(argv[1], "light") == 0 ? UI_THEME_LIGHT : UI_THEME_DARK;
    raster_tile tile = {0};
    if (!ui_demo_render(theme, 1280U, 800U, &tile)) return 1;
    bool okay = fwrite(tile.rgba, 1U, tile.byte_count, stdout) == tile.byte_count;
    raster_tile_destroy(&tile);
    return okay ? 0 : 1;
}
