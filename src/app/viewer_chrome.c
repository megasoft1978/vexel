#include "viewer_chrome.h"

#include <math.h>
#include <string.h>

#define VIEWER_CHROME_MAX_AXIS 8192U

static bool scaled_points(uint32_t points, double scale, uint32_t *out_value)
{
    if (out_value == NULL || !isfinite(scale) || scale < 0.5 || scale > 4.0)
        return false;
    double value = ceil((double)points * scale);
    if (!isfinite(value) || value < 0.0 || value > (double)VIEWER_CHROME_MAX_AXIS)
        return false;
    *out_value = (uint32_t)value;
    return true;
}

static uint32_t min_u32(uint32_t left, uint32_t right)
{
    return left < right ? left : right;
}

bool viewer_chrome_layout_compute(uint32_t width, uint32_t height,
                                  double backing_scale,
                                  bool want_structure, bool want_inspector,
                                  viewer_chrome_layout *out_layout)
{
    return viewer_chrome_layout_compute_docked(width, height, backing_scale,
        want_structure, want_inspector, false, 0U, out_layout);
}

bool viewer_chrome_layout_compute_docked(uint32_t width, uint32_t height,
    double backing_scale, bool want_structure, bool want_inspector,
    bool timeline_visible, uint32_t timeline_height_points,
    viewer_chrome_layout *out_layout)
{
    if (out_layout == NULL || width == 0U || height == 0U ||
        width > VIEWER_CHROME_MAX_AXIS || height > VIEWER_CHROME_MAX_AXIS ||
        !isfinite(backing_scale) || backing_scale < 0.5 || backing_scale > 4.0)
        return false;
    uint32_t toolbar_height, options_height, status_height, rail_width;
    uint32_t structure_width, inspector_width, canvas_min;
    if (!scaled_points(VIEWER_CHROME_TOOLBAR_POINTS, backing_scale, &toolbar_height) ||
        !scaled_points(VIEWER_CHROME_OPTIONS_POINTS, backing_scale, &options_height) ||
        !scaled_points(VIEWER_CHROME_STATUS_POINTS, backing_scale, &status_height) ||
        !scaled_points(VIEWER_CHROME_RAIL_POINTS, backing_scale, &rail_width) ||
        !scaled_points(VIEWER_CHROME_STRUCTURE_POINTS, backing_scale, &structure_width) ||
        !scaled_points(VIEWER_CHROME_INSPECTOR_POINTS, backing_scale, &inspector_width) ||
        !scaled_points(VIEWER_CHROME_CANVAS_MIN_POINTS, backing_scale, &canvas_min))
        return false;
    memset(out_layout, 0, sizeof(*out_layout));

    toolbar_height = min_u32(toolbar_height, height);
    uint32_t remaining_height = height - toolbar_height;
    options_height = min_u32(options_height, remaining_height);
    remaining_height -= options_height;
    status_height = min_u32(status_height, remaining_height);
    remaining_height -= status_height;
    uint32_t dock_height = 0U;
    if (timeline_visible && timeline_height_points > 0U &&
        !scaled_points(timeline_height_points, backing_scale, &dock_height)) return false;
    uint32_t workspace_height = remaining_height;
    uint32_t minimum_workspace = 120U;
    if (timeline_visible && dock_height > 0U &&
        workspace_height < dock_height + minimum_workspace) {
        uint32_t collapsed = 0U;
        if (!scaled_points(32U, backing_scale, &collapsed)) return false;
        dock_height = workspace_height > minimum_workspace + collapsed ? collapsed :
            (workspace_height > minimum_workspace ? workspace_height - minimum_workspace : 0U);
    }
    if (dock_height > workspace_height) dock_height = workspace_height;
    workspace_height -= dock_height;
    uint32_t timeline_y = toolbar_height + options_height + workspace_height;
    uint32_t status_y = timeline_y + dock_height;
    out_layout->toolbar = (ui_rect){0U, 0U, width, toolbar_height};
    out_layout->options = (ui_rect){0U, toolbar_height, width, options_height};
    out_layout->workspace = (ui_rect){0U, toolbar_height + options_height,
        width, workspace_height};
    out_layout->timeline = (ui_rect){0U, timeline_y, width, dock_height};
    out_layout->status = (ui_rect){0U, status_y, width, status_height};

    uint32_t safe_rail = width > 1U ? min_u32(rail_width, width - 1U) : width;
    uint64_t requested_width = (uint64_t)safe_rail + canvas_min;
    if (want_structure) requested_width += structure_width;
    if (want_inspector) requested_width += inspector_width;
    bool show_inspector = want_inspector;
    bool show_structure = want_structure;
    if (requested_width > width && show_inspector) {
        requested_width -= inspector_width;
        show_inspector = false;
    }
    if (requested_width > width && show_structure) {
        requested_width -= structure_width;
        show_structure = false;
    }

    uint32_t left = safe_rail;
    uint32_t right = width;
    out_layout->rail = (ui_rect){0U, out_layout->workspace.y,
        safe_rail, workspace_height};
    out_layout->structure_visible = show_structure;
    out_layout->inspector_visible = show_inspector;
    if (show_structure) {
        uint32_t actual = min_u32(structure_width, right - left);
        out_layout->structure = (ui_rect){left, out_layout->workspace.y,
            actual, workspace_height};
        left += actual;
    }
    if (show_inspector) {
        uint32_t actual = min_u32(inspector_width, right - left);
        out_layout->inspector = (ui_rect){right - actual, out_layout->workspace.y,
            actual, workspace_height};
        right -= actual;
    }
    if (right < left) return false;
    out_layout->canvas = (ui_rect){left, out_layout->workspace.y,
        right - left, workspace_height};
    return true;
}
