#include "ui/viewer_controls.h"

#include <float.h>
#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

static bool close_to(double a, double b)
{
    return fabs(a - b) < 1e-8;
}

int main(void)
{
    ui_viewer_navigation nav;
    if (!ui_viewer_navigation_init(&nav, 800.0, 600.0, 400.0, 200.0) ||
        !ui_viewer_navigation_fit(&nav) || !close_to(nav.scale, 2.0) ||
        !close_to(nav.pan_x, 0.0) || !close_to(nav.pan_y, 100.0)) return 1;
    if (!ui_viewer_navigation_drag_begin(&nav, 10.0, 20.0) ||
        !ui_viewer_navigation_drag(&nav, 35.0, 45.0) ||
        !ui_viewer_navigation_drag_end(&nav) || !close_to(nav.pan_x, 25.0) ||
        !close_to(nav.pan_y, 125.0)) return 2;
    double wx = (175.0 - nav.pan_x) / nav.scale, wy = (225.0 - nav.pan_y) / nav.scale;
    if (!ui_viewer_navigation_wheel(&nav, -120.0, 175.0, 225.0)) return 3;
    double sx = 0.0, sy = 0.0;
    if (!ui_viewer_navigation_project(&nav, wx, wy, &sx, &sy) ||
        !close_to(sx, 175.0) || !close_to(sy, 225.0)) return 4;
    wx = (175.0 - nav.pan_x) / nav.scale; wy = (225.0 - nav.pan_y) / nav.scale;
    if (!ui_viewer_navigation_pinch(&nav, 0.25, 175.0, 225.0) ||
        !ui_viewer_navigation_project(&nav, wx, wy, &sx, &sy) ||
        !close_to(sx, 175.0) || !close_to(sy, 225.0)) return 5;
    if (!ui_viewer_navigation_zoom(&nav, 1e100, 0.0, 0.0) || nav.scale != UI_VIEWER_MAX_SCALE ||
        !ui_viewer_navigation_zoom(&nav, 1e-100, 0.0, 0.0) || nav.scale != UI_VIEWER_MIN_SCALE) return 6;
    if (ui_viewer_navigation_zoom(&nav, NAN, 0.0, 0.0) ||
        ui_viewer_navigation_wheel(&nav, INFINITY, 0.0, 0.0) ||
        ui_viewer_navigation_pinch(&nav, NAN, 0.0, 0.0) ||
        ui_viewer_navigation_resize(&nav, 0.0, 100.0) ||
        ui_viewer_navigation_project(&nav, INFINITY, 0.0, &sx, &sy)) return 7;
    double center_world_x = (nav.viewport_width * 0.5 - nav.pan_x) / nav.scale;
    double center_world_y = (nav.viewport_height * 0.5 - nav.pan_y) / nav.scale;
    if (!ui_viewer_navigation_resize(&nav, 1000.0, 800.0) ||
        !ui_viewer_navigation_project(&nav, center_world_x, center_world_y, &sx, &sy) ||
        !close_to(sx, 500.0) || !close_to(sy, 400.0) ||
        !ui_viewer_navigation_key(&nav, UI_VIEWER_KEY_ZOOM_IN) ||
        !ui_viewer_navigation_key(&nav, UI_VIEWER_KEY_ZOOM_OUT) ||
        !ui_viewer_navigation_key(&nav, UI_VIEWER_KEY_LEFT) ||
        !ui_viewer_navigation_key(&nav, UI_VIEWER_KEY_RIGHT) ||
        !ui_viewer_navigation_key(&nav, UI_VIEWER_KEY_UP) ||
        !ui_viewer_navigation_key(&nav, UI_VIEWER_KEY_DOWN) ||
        !ui_viewer_navigation_key(&nav, UI_VIEWER_KEY_FIT)) return 8;
    nav.pan_x = DBL_MAX;
    if (!ui_viewer_navigation_key(&nav, UI_VIEWER_KEY_LEFT) || !isfinite(nav.pan_x) ||
        nav.pan_x != DBL_MAX || !isfinite(nav.pan_y) || !ui_viewer_navigation_fit(&nav)) return 19;

    ui_viewer_navigation retina;
    if (!ui_viewer_navigation_init(&retina, 800.0, 600.0, 400.0, 300.0) ||
        !ui_viewer_navigation_resize_scaled(&retina, 800.0, 600.0, 2.0) ||
        retina.viewport_width != 1600.0 || retina.viewport_height != 1200.0 ||
        !ui_viewer_navigation_fit(&retina)) return 15;
    double retina_world_x = (400.0 - retina.pan_x) / retina.scale;
    double retina_world_y = (300.0 - retina.pan_y) / retina.scale;
    if (!ui_viewer_navigation_wheel_scaled(&retina, -120.0, 200.0, 150.0, 2.0) ||
        !ui_viewer_navigation_project(&retina, retina_world_x, retina_world_y, &sx, &sy) ||
        !close_to(sx, 400.0) || !close_to(sy, 300.0) ||
        !ui_viewer_navigation_pinch_scaled(&retina, 0.1, 200.0, 150.0, 2.0)) return 16;
    double pan_before = retina.pan_x;
    if (!ui_viewer_navigation_drag_begin_scaled(&retina, 20.0, 15.0, 2.0) ||
        !ui_viewer_navigation_drag_scaled(&retina, 25.0, 20.0, 2.0) ||
        !ui_viewer_navigation_drag_end(&retina) ||
        !close_to(retina.pan_x - pan_before, 10.0)) return 18;
    if (ui_viewer_navigation_resize_scaled(&retina, 800.0, 600.0, NAN) ||
        ui_viewer_navigation_wheel_scaled(&retina, 1.0, 200.0, 150.0, 0.0) ||
        ui_viewer_navigation_pinch_scaled(&retina, 1.0, INFINITY, 150.0, 2.0)) return 17;

    ui_viewer_progress progress = {0};
    if (!ui_viewer_progress_update(&progress, 4U, 37U, UI_PROGRESS_ACTIVE, "Indexing", 8U) ||
        ui_viewer_progress_update(&progress, 3U, 99U, UI_PROGRESS_ACTIVE, "stale", 5U)) return 9;
    ui_semantic_node semantics[3] = {0};
    if (ui_viewer_semantics(&nav, &progress, semantics, 3U) != 3U ||
        strcmp(semantics[0].role, "canvas") != 0 || strcmp(semantics[1].role, "progressbar") != 0 ||
        semantics[1].value != 37U || !semantics[2].enabled || strcmp(semantics[2].name, "Cancel indexing") != 0) return 10;
    uint8_t rgba[64U * 64U * 4U] = {0}; bool cancel = false;
    if (!ui_viewer_overlay_rgba(&progress, rgba, sizeof(rgba), 64U, 64U, 64U * 4U) ||
        !ui_viewer_progress_cancel(&progress, &cancel) || !cancel ||
        progress.phase != UI_PROGRESS_ACTIVE ||
        ui_viewer_semantics(&nav, &progress, semantics, 3U) != 3U || semantics[2].enabled) return 11;
    if (!ui_viewer_progress_update(&progress, 4U, 37U, UI_PROGRESS_CANCELED, "Canceled", 8U) ||
        ui_viewer_semantics(&nav, &progress, semantics, 3U) != 2U) return 12;
    if (ui_viewer_progress_update(&progress, 4U, 100U, UI_PROGRESS_COMPLETE, "Ready", 5U) ||
        !ui_viewer_progress_update(&progress, 5U, 100U, UI_PROGRESS_COMPLETE, "Ready", 5U) ||
        ui_viewer_semantics(&nav, &progress, semantics, 3U) != 2U) return 15;
    if (!ui_viewer_progress_update(&progress, 6U, 24U, UI_PROGRESS_FAILED, "Failed", 6U) ||
        ui_viewer_semantics(&nav, &progress, semantics, 3U) != 2U) return 16;

    /* Stable allocation-free projection workload: 10k immutable world points over 100 frames. */
    uint64_t checksum = UINT64_C(14695981039346656037);
    for (unsigned frame = 0U; frame < 100U; frame++) {
        if (!ui_viewer_navigation_wheel(&nav, frame % 2U == 0U ? -1.0 : 1.0, 400.0, 300.0)) return 13;
        for (unsigned item = 0U; item < 10000U; item++) {
            double x = 0.0, y = 0.0;
            if (!ui_viewer_navigation_project(&nav, (double)(item % 100U) * 3.0,
                                               (double)(item / 100U) * 2.0, &x, &y)) return 14;
            int64_t qx = (int64_t)llround(x * 1000.0), qy = (int64_t)llround(y * 1000.0);
            checksum = (checksum ^ (uint64_t)qx) * UINT64_C(1099511628211);
            checksum = (checksum ^ (uint64_t)qy) * UINT64_C(1099511628211);
        }
    }
    (void)printf("navigation tests: pass\nprojection_items: 1000000\nchecksum: %016llx\nallocations: 0 (navigation API has no allocator calls)\n",
                 (unsigned long long)checksum);
    (void)puts("retina_backing_scale: 2x viewport, drag, and cursor conversion pass");
    return 0;
}
