#include "viewer_canvas_geometry.h"

#include <math.h>
#include <stdio.h>

int main(void)
{
    ui_viewer_navigation navigation = {0};
    if (!ui_viewer_navigation_init(&navigation, 1280.0, 800.0, 512.0, 256.0) ||
        !ui_viewer_navigation_fit(&navigation)) return 1;
    if (!viewer_canvas_navigation_resize(&navigation, 900.0, 700.0, true) ||
        fabs(navigation.scale - (900.0 / 512.0)) > 1e-12 ||
        navigation.viewport_width != 900.0 || navigation.viewport_height != 700.0)
        return 2;

    if (!ui_viewer_navigation_zoom(&navigation, 2.0, 470.0, 330.0)) return 3;
    double center_x = (navigation.viewport_width * 0.5 - navigation.pan_x) /
        navigation.scale;
    double center_y = (navigation.viewport_height * 0.5 - navigation.pan_y) /
        navigation.scale;
    double scale = navigation.scale;
    if (!viewer_canvas_navigation_resize(&navigation, 1100.0, 760.0, false)) return 4;
    double resized_center_x = (navigation.viewport_width * 0.5 - navigation.pan_x) /
        navigation.scale;
    double resized_center_y = (navigation.viewport_height * 0.5 - navigation.pan_y) /
        navigation.scale;
    if (fabs(navigation.scale - scale) > 1e-12 ||
        fabs(resized_center_x - center_x) > 1e-9 ||
        fabs(resized_center_y - center_y) > 1e-9) return 5;

    ui_viewer_navigation unchanged = navigation;
    if (viewer_canvas_navigation_resize(&navigation, 0.0, 760.0, true) ||
        navigation.viewport_width != unchanged.viewport_width ||
        navigation.viewport_height != unchanged.viewport_height ||
        navigation.scale != unchanged.scale || navigation.pan_x != unchanged.pan_x ||
        navigation.pan_y != unchanged.pan_y) return 6;
    (void)puts("canvas geometry: initial fit, pre-zoom refit, and post-zoom center preserved");
    return 0;
}
