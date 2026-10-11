#ifndef VEXEL_APP_VIEWER_CANVAS_GEOMETRY_H
#define VEXEL_APP_VIEWER_CANVAS_GEOMETRY_H

#include "ui/viewer_controls.h"

#include <stdbool.h>

/* Resize the navigation viewport to the editor canvas. If refit is false,
 * preserve the document point at the viewport center and keep the current zoom. */
bool viewer_canvas_navigation_resize(ui_viewer_navigation *navigation,
                                    double width, double height, bool refit);

#endif
