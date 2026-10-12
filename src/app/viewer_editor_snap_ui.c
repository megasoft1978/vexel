#include "viewer_editor_internal.h"

#include "ui/ui_widgets.h"
#include "platform/platform_text_ui.h"

#include <math.h>
#include <string.h>

#define SNAP_POPOVER_WIDTH_POINTS 220U
#define SNAP_POPOVER_TITLE_POINTS 28U
#define SNAP_POPOVER_ROW_POINTS 30U
#define SNAP_POPOVER_FOOTER_POINTS 22U
#define SNAP_POPOVER_GAP_POINTS 4U

typedef struct snap_popover_geometry {
    ui_rect panel;
    ui_rect trigger;
    uint32_t scale;
} snap_popover_geometry;

static bool snap_popover_geometry_get(viewer_editor *editor,
    snap_popover_geometry *out)
{
    if (editor == NULL || out == NULL) return false;
    if (!editor->tree_ready) editor_build_tree(editor);
    if (!editor->tree_ready) return false;
    const ui_node *trigger = NULL;
    for (size_t index = 0U; index < editor->tree.count; ++index)
        if (editor->tree.nodes[index].spec.id == EDITOR_BUTTON_SNAPPING) {
            trigger = &editor->tree.nodes[index];
            break;
        }
    if (trigger == NULL) return false;
    double raw_scale = editor->backing_scale;
    if (!isfinite(raw_scale) || raw_scale < 0.5) raw_scale = 1.0;
    uint32_t scale = (uint32_t)llround(raw_scale);
    if (scale == 0U) scale = 1U;
    if (scale > 8U) scale = 8U;
    uint32_t width = SNAP_POPOVER_WIDTH_POINTS * scale;
    uint32_t height = (SNAP_POPOVER_TITLE_POINTS +
        SNAP_POPOVER_ROW_POINTS * 3U + SNAP_POPOVER_FOOTER_POINTS + 8U) * scale;
    uint32_t x = trigger->bounds.x;
    if ((uint64_t)x + width > editor->width)
        x = editor->width > width ? editor->width - width : 0U;
    uint32_t y = trigger->bounds.y + trigger->bounds.height +
        SNAP_POPOVER_GAP_POINTS * scale;
    if ((uint64_t)y + height > editor->height)
        y = trigger->bounds.y > height + SNAP_POPOVER_GAP_POINTS * scale ?
            trigger->bounds.y - height - SNAP_POPOVER_GAP_POINTS * scale : 0U;
    if ((uint64_t)y + height > editor->height)
        y = editor->height > height ? editor->height - height : 0U;
    *out = (snap_popover_geometry){
        .panel = {x, y, width, height},
        .trigger = trigger->bounds,
        .scale = scale
    };
    return true;
}

/* Exposed so tests can ask where the popover really is instead of recomputing
 * the placement rules, which include a flip when it would not fit below. */
bool editor_snap_popover_panel(viewer_editor *editor, ui_rect *out_panel)
{
    snap_popover_geometry geometry;
    if (out_panel == NULL || !snap_popover_geometry_get(editor, &geometry))
        return false;
    *out_panel = geometry.panel;
    return true;
}

static bool snap_popover_contains(ui_rect bounds, uint32_t x, uint32_t y)
{
    return x >= bounds.x && y >= bounds.y &&
        x - bounds.x < bounds.width && y - bounds.y < bounds.height;
}

static void snap_popover_close(viewer_editor *editor)
{
    editor->snap_popover_open = false;
    editor->snap_popover_hovered = false;
    editor->snap_popover_hovered_row = UINT8_MAX;
    editor->tree_ready = false;
    editor->redraw_requested = true;
}

bool editor_snap_popover_pointer_down(viewer_editor *editor, uint32_t x,
                                      uint32_t y)
{
    if (editor == NULL || !editor->snap_popover_open) return false;
    snap_popover_geometry geometry;
    if (!snap_popover_geometry_get(editor, &geometry)) {
        snap_popover_close(editor);
        return false;
    }
    if (snap_popover_contains(geometry.panel, x, y)) {
        uint32_t title = SNAP_POPOVER_TITLE_POINTS * geometry.scale;
        uint32_t row_height = SNAP_POPOVER_ROW_POINTS * geometry.scale;
        if (y < geometry.panel.y + title) return true;
        size_t row = (y - geometry.panel.y - title) / row_height;
        if (row > 2U) return true;
        bool *value = row == 0U ? &editor->snap_grid_enabled :
            row == 1U ? &editor->snap_objects_enabled :
                &editor->snap_guides_enabled;
        *value = !*value;
        editor->tree_ready = false;
        editor->redraw_requested = true;
        editor_status_ok(editor, row == 0U ?
            (*value ? "Grid snapping on" : "Grid snapping off") :
            row == 1U ? (*value ? "Object snapping on" : "Object snapping off") :
                (*value ? "Guide snapping on" : "Guide snapping off"));
        return true;
    }
    if (!snap_popover_contains(geometry.trigger, x, y)) snap_popover_close(editor);
    return false;
}

void editor_snap_popover_pointer_move(viewer_editor *editor, uint32_t x,
                                      uint32_t y)
{
    if (editor == NULL || !editor->snap_popover_open) return;
    snap_popover_geometry geometry;
    if (!snap_popover_geometry_get(editor, &geometry)) return;
    uint32_t row_start = geometry.panel.y +
        SNAP_POPOVER_TITLE_POINTS * geometry.scale;
    uint32_t rows_height = SNAP_POPOVER_ROW_POINTS * 3U * geometry.scale;
    uint8_t hovered_row = UINT8_MAX;
    if (snap_popover_contains(geometry.panel, x, y) && y >= row_start &&
        y - row_start < rows_height)
        hovered_row = (uint8_t)((y - row_start) /
            (SNAP_POPOVER_ROW_POINTS * geometry.scale));
    if (hovered_row == editor->snap_popover_hovered_row) return;
    editor->snap_popover_hovered_row = hovered_row;
    editor->snap_popover_hovered = hovered_row < 3U;
    editor->redraw_requested = true;
}

static ui_rect snap_popover_points(viewer_editor *editor, ui_rect pixels)
{
    double scale = editor->backing_scale > 0.0 ? editor->backing_scale : 1.0;
    return (ui_rect){
        (uint32_t)llround((double)pixels.x / scale),
        (uint32_t)llround((double)pixels.y / scale),
        (uint32_t)llround((double)pixels.width / scale),
        (uint32_t)llround((double)pixels.height / scale)};
}

void editor_paint_snap_popover(viewer_editor *editor, uint8_t *rgba,
    size_t capacity, size_t stride)
{
    if (editor == NULL || rgba == NULL || !editor->snap_popover_open ||
        !editor->tree_ready) return;
    snap_popover_geometry geometry;
    if (!snap_popover_geometry_get(editor, &geometry)) return;
    uint32_t scale = geometry.scale;
    ui_widget_canvas canvas;
    if (!ui_widget_canvas_init(&canvas, rgba, capacity, editor->width,
            editor->height, stride, scale, editor->theme,
            editor->increased_contrast, platform_text_ui_draw_widget, NULL)) return;
    ui_rect panel = snap_popover_points(editor, geometry.panel);
    const char *title = editor->snap_enabled ? "Snapping on" : "Snapping off";
    ui_widget_draw_spec title_spec = {.kind = UI_WIDGET_TOOLTIP,
        .state = UI_CONTROL_REST, .bounds = {panel.x, panel.y, panel.width,
            SNAP_POPOVER_TITLE_POINTS}, .label = title,
        .label_length = editor->snap_enabled ? 11U : 12U};
    if (!ui_widget_paint(&canvas, &title_spec)) return;
    const char *labels[3] = {"Grid · whole user units",
        "Objects · edges and centers", "Guides · alignment lines"};
    const bool checked[3] = {editor->snap_grid_enabled,
        editor->snap_objects_enabled, editor->snap_guides_enabled};
    uint32_t title_height = (uint32_t)llround(
        (double)(SNAP_POPOVER_TITLE_POINTS * scale) / editor->backing_scale);
    for (size_t row = 0U; row < 3U; ++row) {
        uint32_t row_y = panel.y + title_height +
            (uint32_t)row * SNAP_POPOVER_ROW_POINTS;
        ui_widget_draw_spec row_spec = {
            .kind = UI_WIDGET_CHECKBOX,
            .state = editor->snap_popover_hovered_row == row ?
                UI_CONTROL_HOVER : UI_CONTROL_REST,
            .bounds = {panel.x + 8U, row_y, panel.width - 16U,
                SNAP_POPOVER_ROW_POINTS},
            .label = labels[row], .label_length = strlen(labels[row]),
            .checked = checked[row], .font_size_pt = 12U};
        if (!ui_widget_paint(&canvas, &row_spec)) return;
    }
    uint32_t footer_y = panel.y + SNAP_POPOVER_TITLE_POINTS +
        SNAP_POPOVER_ROW_POINTS * 3U;
    ui_widget_draw_spec footer = {.kind = UI_WIDGET_LABEL,
        .state = UI_CONTROL_REST,
        .bounds = {panel.x + 10U, footer_y, panel.width - 20U,
            SNAP_POPOVER_FOOTER_POINTS},
        .label = "6 pt tolerance · hold Shift to bypass",
        .label_length = sizeof("6 pt tolerance · hold Shift to bypass") - 1U,
        .font_size_pt = 10U,
        .text_color = ui_theme_tokens(editor->theme,
            editor->increased_contrast).text_secondary};
    (void)ui_widget_paint(&canvas, &footer);
}
