#include "viewer_canvas_geometry.h"

#include <math.h>

bool viewer_canvas_navigation_resize(ui_viewer_navigation *navigation,
                                    double width, double height, bool refit)
{
    if (navigation == NULL || !isfinite(width) || !(width > 0.0) ||
        !isfinite(height) || !(height > 0.0) ||
        !isfinite(navigation->scale) || !(navigation->scale > 0.0) ||
        !isfinite(navigation->viewport_width) ||
        !(navigation->viewport_width > 0.0) ||
        !isfinite(navigation->viewport_height) ||
        !(navigation->viewport_height > 0.0)) return false;
    ui_viewer_navigation next = *navigation;
    if (refit) {
        next.viewport_width = width;
        next.viewport_height = height;
        if (!ui_viewer_navigation_fit(&next)) return false;
    } else {
        if (!isfinite(next.pan_x) || !isfinite(next.pan_y)) return false;
        double center_x = (next.viewport_width * 0.5 - next.pan_x) / next.scale;
        double center_y = (next.viewport_height * 0.5 - next.pan_y) / next.scale;
        double pan_x = width * 0.5 - center_x * next.scale;
        double pan_y = height * 0.5 - center_y * next.scale;
        if (!isfinite(center_x) || !isfinite(center_y) ||
            !isfinite(pan_x) || !isfinite(pan_y)) return false;
        next.viewport_width = width;
        next.viewport_height = height;
        next.pan_x = pan_x;
        next.pan_y = pan_y;
    }
    *navigation = next;
    return true;
}
