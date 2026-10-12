#ifndef VEXEL_APP_VIEWER_EDITOR_H
#define VEXEL_APP_VIEWER_EDITOR_H

#include "document_edit.h"
#include "viewer_commands.h"
#include "platform/platform_window.h"
#include "svg/shape/shape.h"
#include "ui/ui_core.h"
#include "viewer_workspace.h"
#include "ui/viewer_controls.h"
#include "viewer_editor_guides.h"

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

typedef struct viewer_editor viewer_editor;
struct viewer_chrome_layout;

typedef enum viewer_editor_draw_tool {
    VIEWER_EDITOR_TOOL_SELECT,
    VIEWER_EDITOR_TOOL_RECT,
    VIEWER_EDITOR_TOOL_ELLIPSE,
    VIEWER_EDITOR_TOOL_LINE,
    VIEWER_EDITOR_TOOL_POLYGON,
    VIEWER_EDITOR_TOOL_POLYLINE,
    VIEWER_EDITOR_TOOL_PEN,
    VIEWER_EDITOR_TOOL_TEXT
} viewer_editor_draw_tool;

typedef enum viewer_editor_alignment {
    VIEWER_EDITOR_ALIGN_LEFT = 0,
    VIEWER_EDITOR_ALIGN_HORIZONTAL_CENTER,
    VIEWER_EDITOR_ALIGN_RIGHT,
    VIEWER_EDITOR_ALIGN_TOP,
    VIEWER_EDITOR_ALIGN_VERTICAL_CENTER,
    VIEWER_EDITOR_ALIGN_BOTTOM
} viewer_editor_alignment;

typedef enum viewer_editor_distribution {
    VIEWER_EDITOR_DISTRIBUTE_HORIZONTAL = 0,
    VIEWER_EDITOR_DISTRIBUTE_VERTICAL
} viewer_editor_distribution;

typedef enum viewer_editor_guide_pointer_result {
    VIEWER_EDITOR_GUIDE_POINTER_NONE = 0,
    VIEWER_EDITOR_GUIDE_POINTER_HANDLED,
    VIEWER_EDITOR_GUIDE_POINTER_CONTEXT_HORIZONTAL,
    VIEWER_EDITOR_GUIDE_POINTER_CONTEXT_VERTICAL
} viewer_editor_guide_pointer_result;

#define VIEWER_EDITOR_PATH_POINT_CAP 128U
#define VIEWER_EDITOR_PREVIEW_EXTRA_BYTES (1024U * 32U)

typedef struct viewer_editor_canvas_overlay_state {
    svg_shape_rect bounds; /* decoded document tile pixels */
    double rotation_radians;
    bool visible;
    bool drawing_preview;
    size_t path_point_count;
    geom_point path_points[VIEWER_EDITOR_PATH_POINT_CAP];
    geom_point path_handles[VIEWER_EDITOR_PATH_POINT_CAP];
    geom_point path_in_handles[VIEWER_EDITOR_PATH_POINT_CAP];
    uint8_t path_point_states[VIEWER_EDITOR_PATH_POINT_CAP];
    bool path_has_handles[VIEWER_EDITOR_PATH_POINT_CAP];
    bool path_has_in_handles[VIEWER_EDITOR_PATH_POINT_CAP];
    bool path_closed_preview;
    bool path_nodes_only;
    bool snap_guide_x_visible;
    bool snap_guide_y_visible;
    double snap_guide_x;
    double snap_guide_y;
    bool snap_label_visible;
    char snap_label[32];
} viewer_editor_canvas_overlay_state;

enum {
    VIEWER_EDITOR_HANDLED = 1U << 0U,
    VIEWER_EDITOR_MUTATED = 1U << 1U,
    VIEWER_EDITOR_SAVE = 1U << 2U,
    VIEWER_EDITOR_SAVE_AS = 1U << 3U,
    VIEWER_EDITOR_OPEN = 1U << 4U,
    VIEWER_EDITOR_QUIT = 1U << 5U,
    VIEWER_EDITOR_RELOAD = 1U << 6U,
    VIEWER_EDITOR_SELECT_ALL = 1U << 7U,
    VIEWER_EDITOR_DELETE_PATH_NODE = 1U << 8U,
    VIEWER_EDITOR_COPY = 1U << 9U,
    VIEWER_EDITOR_CUT = 1U << 10U,
    VIEWER_EDITOR_PASTE = 1U << 11U,
    VIEWER_EDITOR_PASTE_IN_PLACE = 1U << 12U,
    VIEWER_EDITOR_EXPORT_PNG = 1U << 13U,
    VIEWER_EDITOR_COPY_PNG = 1U << 14U,
    VIEWER_EDITOR_PREVIEW_CHANGED = 1U << 15U,
    VIEWER_EDITOR_ZOOM_PRESET_REQUEST = 1U << 16U,
    VIEWER_EDITOR_COMMAND_SELECTED = 1U << 17U,
    VIEWER_EDITOR_NODE_CORNER = 1U << 18U,
    VIEWER_EDITOR_NODE_SMOOTH = 1U << 19U,
    VIEWER_EDITOR_FIT_CANVAS = 1U << 20U,
    VIEWER_EDITOR_ACTUAL_SIZE = 1U << 21U,
    VIEWER_EDITOR_COPY_FULL_ID = 1U << 22U
};

viewer_editor *viewer_editor_create(base_bytes source, ui_theme theme,
                                    uint32_t width, uint32_t height);
/* Explicit style constructor used by deterministic and preference-injection tests. */
viewer_editor *viewer_editor_create_styled(base_bytes source, ui_theme theme,
                                           uint32_t width, uint32_t height,
                                           bool modern_style);
void viewer_editor_destroy(viewer_editor *editor);
bool viewer_editor_is_dirty(const viewer_editor *editor);
bool viewer_editor_can_undo(const viewer_editor *editor);
bool viewer_editor_can_redo(const viewer_editor *editor);
bool viewer_editor_has_selection(const viewer_editor *editor);
bool viewer_editor_sidebar_visibility(const viewer_editor *editor,
                                     bool *out_structure, bool *out_inspector);
const char *viewer_editor_status(const viewer_editor *editor);
void viewer_editor_set_status(viewer_editor *editor, const char *message);
void viewer_editor_resize(viewer_editor *editor, uint32_t width, uint32_t height,
                          double backing_scale);
void viewer_editor_paint(viewer_editor *editor, uint8_t *rgba, size_t capacity,
                         size_t stride);
/* Stable scalar text path for hash-pinned artwork; app display uses CoreText. */
void viewer_editor_paint_scalar(viewer_editor *editor, uint8_t *rgba,
                                size_t capacity, size_t stride);

/* Events are plain values so the edit state machine can be tested headlessly. */
uint32_t viewer_editor_key(viewer_editor *editor, uint16_t key_code,
                           uint32_t modifiers, const char *text, size_t text_len);
uint32_t viewer_editor_pointer_down(viewer_editor *editor, uint32_t x, uint32_t y,
                                    uint8_t button);
uint32_t viewer_editor_pointer_down_click_count(viewer_editor *editor,
    uint32_t x, uint32_t y, uint8_t button, uint8_t click_count);
bool viewer_editor_scroll(viewer_editor *editor, uint32_t x, double delta_y);
bool viewer_editor_scroll_at(viewer_editor *editor, uint32_t x, uint32_t y,
                             double delta_y);

size_t viewer_editor_byte_length(const viewer_editor *editor);
bool viewer_editor_materialize(viewer_editor *editor, uint8_t *buffer,
                               size_t capacity, size_t *out_length);
/* Materializes a render-only source snapshot with UI visibility overrides.
 * It never changes the editable source or dirty state. */
bool viewer_editor_materialize_preview(viewer_editor *editor, uint8_t *buffer,
                                       size_t capacity, size_t *out_length);
document_edit_save_result viewer_editor_save(viewer_editor *editor,
                                             base_bytes path);
bool viewer_editor_select_at_source_offset(viewer_editor *editor, size_t offset);
bool viewer_editor_select_all(viewer_editor *editor,
                              const svg_shape_document *document);
bool viewer_editor_selection_span(viewer_editor *editor,
                                  document_source_span *out_span);
bool viewer_editor_semantics(viewer_editor *editor,
                             ui_semantics_snapshot *out_snapshot);
bool viewer_editor_set_appearance(viewer_editor *editor, ui_theme theme,
                                  bool increased_contrast, bool reduced_motion);
bool viewer_editor_set_system_accent(viewer_editor *editor, uint32_t accent_srgb,
                                     bool material_allowed);
bool viewer_editor_set_ui_scale(viewer_editor *editor, uint16_t percent);
bool viewer_editor_set_interface_scale(viewer_editor *editor,
                                       uint8_t scale_percent);
bool viewer_editor_set_panel_widths(viewer_editor *editor,
    uint16_t structure_width_points, uint16_t inspector_width_points);
bool viewer_editor_native_material_enabled(const viewer_editor *editor);
uint16_t viewer_editor_ui_scale_percent(const viewer_editor *editor);
bool viewer_editor_copy_full_id(const viewer_editor *editor, uint8_t *buffer,
    size_t capacity, size_t *out_length);
void viewer_editor_set_motion_time(viewer_editor *editor, double seconds);
bool viewer_editor_motion_tick(viewer_editor *editor, double seconds);
bool viewer_editor_motion_active(const viewer_editor *editor);
/* Drives the status bar's progress region and its cancel affordance. */
void viewer_editor_set_operation_progress(viewer_editor *editor, bool active,
    const char *message);

/* Notified after a workspace change so the platform layer can persist it. */
typedef void (*viewer_editor_workspace_callback)(void *context, const char *name);
void viewer_editor_set_workspace_callback(viewer_editor *editor,
    viewer_editor_workspace_callback callback, void *context);
/* Applies a remembered workspace name; unknown names fall back to the default. */
bool viewer_editor_apply_workspace_name(viewer_editor *editor,
    const char *name, size_t length);
/* Applies a task workspace's pane visibility. */
bool viewer_editor_set_workspace(viewer_editor *editor, viewer_workspace_kind workspace);
bool viewer_editor_cycle_workspace(viewer_editor *editor, bool backwards);
viewer_workspace_kind viewer_editor_workspace(const viewer_editor *editor);
bool viewer_editor_set_sidebar_visibility(viewer_editor *editor,
                                          bool structure_visible,
                                          bool inspector_visible);
bool viewer_editor_set_rulers_visible(viewer_editor *editor, bool visible);
bool viewer_editor_rulers_visible(const viewer_editor *editor);
bool viewer_editor_set_guides_visible(viewer_editor *editor, bool visible);
bool viewer_editor_guides_visible(const viewer_editor *editor);
bool viewer_editor_set_guides_locked(viewer_editor *editor, bool locked);
bool viewer_editor_guides_locked(const viewer_editor *editor);
bool viewer_editor_clear_guides(viewer_editor *editor,
                                viewer_editor_guide_axis_kind axis);
bool viewer_editor_get_navigation_canvas_rect(viewer_editor *editor,
                                               struct viewer_chrome_layout *out_layout,
                                               ui_rect *out_canvas);
viewer_editor_guide_pointer_result viewer_editor_guide_pointer_down(
    viewer_editor *editor, const svg_shape_document *document,
    const ui_viewer_navigation *navigation, ui_rect canvas,
    double screen_x, double screen_y, uint8_t button);
bool viewer_editor_guide_pointer_move(viewer_editor *editor,
    const svg_shape_document *document, const ui_viewer_navigation *navigation,
    ui_rect canvas, double screen_x, double screen_y);
bool viewer_editor_guide_pointer_up(viewer_editor *editor);
uint32_t viewer_editor_delete_selected_guide(viewer_editor *editor);
void viewer_editor_set_navigation_overlays_hidden(viewer_editor *editor,
                                                   bool hidden);
void viewer_editor_set_snap_bypass(viewer_editor *editor, bool bypass);
bool viewer_editor_take_zoom_preset(viewer_editor *editor, double *out_scale);
double viewer_editor_zoom_preset_scale(size_t index);
bool viewer_editor_set_timeline_state(viewer_editor *editor, bool available,
                                      bool visible, uint32_t height_points);
bool viewer_editor_timeline_visible(const viewer_editor *editor);
bool viewer_editor_timeline_available(const viewer_editor *editor);
bool viewer_editor_reduced_motion(const viewer_editor *editor);
bool viewer_editor_set_tool(viewer_editor *editor, ui_icon_kind tool);
ui_icon_kind viewer_editor_get_tool(const viewer_editor *editor);
bool viewer_editor_text_input_active(const viewer_editor *editor);
bool viewer_editor_node_edit_active(const viewer_editor *editor);
uint32_t viewer_editor_node_edit_key(viewer_editor *editor,
    const svg_shape_document *document, uint16_t key_code, uint32_t modifiers);
uint32_t viewer_editor_node_edit_hover(viewer_editor *editor,
    const svg_shape_document *document, geom_point user, double tolerance);
uint32_t viewer_editor_convert_selected_path_node(viewer_editor *editor,
    const svg_shape_document *document, bool smooth);
bool viewer_editor_set_draw_tool(viewer_editor *editor,
                                 viewer_editor_draw_tool tool);
viewer_editor_draw_tool viewer_editor_get_draw_tool(const viewer_editor *editor);
bool viewer_editor_open_command_palette(viewer_editor *editor);
bool viewer_editor_open_history(viewer_editor *editor);
bool viewer_editor_command_palette_is_open(const viewer_editor *editor);
bool viewer_editor_history_is_open(const viewer_editor *editor);
bool viewer_editor_take_pending_command(viewer_editor *editor,
                                        viewer_command_id *out_command);
bool viewer_editor_toggle_snapping(viewer_editor *editor);
/* Pointer coordinates and navigation use backing pixels relative to canvas origin. */
uint32_t viewer_editor_canvas_pointer_down(viewer_editor *editor,
    const svg_shape_document *document, const ui_viewer_navigation *navigation,
    double x, double y, uint32_t modifiers);
uint32_t viewer_editor_canvas_pointer_down_click_count(viewer_editor *editor,
    const svg_shape_document *document, const ui_viewer_navigation *navigation,
    double x, double y, uint32_t modifiers, uint8_t click_count);
uint32_t viewer_editor_canvas_pointer_move(viewer_editor *editor,
    const svg_shape_document *document, const ui_viewer_navigation *navigation,
    double x, double y);
uint32_t viewer_editor_canvas_pointer_move_modifiers(viewer_editor *editor,
    const svg_shape_document *document, const ui_viewer_navigation *navigation,
    double x, double y, uint32_t modifiers);
platform_cursor_kind viewer_editor_canvas_cursor(viewer_editor *editor,
    const svg_shape_document *document, const ui_viewer_navigation *navigation,
    double x, double y);
uint32_t viewer_editor_canvas_pointer_up(viewer_editor *editor,
    const svg_shape_document *document);
uint32_t viewer_editor_cancel_canvas_gesture(viewer_editor *editor);
uint32_t viewer_editor_nudge_selection(viewer_editor *editor,
                                       double dx, double dy);
uint32_t viewer_editor_align_selection(viewer_editor *editor,
    const svg_shape_document *document, viewer_editor_alignment alignment);
uint32_t viewer_editor_distribute_selection(viewer_editor *editor,
    const svg_shape_document *document,
    viewer_editor_distribution distribution);
uint32_t viewer_editor_delete_selected_path_node(viewer_editor *editor,
    const svg_shape_document *document);
bool viewer_editor_copy_selection(viewer_editor *editor, uint8_t *buffer,
    size_t capacity, size_t *out_length);
uint32_t viewer_editor_cut_selection(viewer_editor *editor);
uint32_t viewer_editor_paste_svg(viewer_editor *editor,
    const uint8_t *bytes, size_t length, bool paste_in_place);
/* System clipboard paste makes colliding element IDs unique before insertion. */
uint32_t viewer_editor_paste_svg_from_clipboard(viewer_editor *editor,
    const uint8_t *bytes, size_t length, bool paste_in_place);
bool viewer_editor_get_canvas_overlay(viewer_editor *editor,
    const svg_shape_document *document,
    viewer_editor_canvas_overlay_state *out_overlay);
bool viewer_editor_get_selection_bounds(viewer_editor *editor,
    const svg_shape_document *document, svg_shape_rect *out_bounds);
void viewer_editor_paint_canvas_overlay(viewer_editor *editor,
    const svg_shape_document *document, const ui_viewer_navigation *navigation,
    ui_rect canvas, uint8_t *rgba, uint32_t width, uint32_t height,
    size_t stride);
void viewer_editor_set_canvas_status(viewer_editor *editor, double zoom,
    bool pointer_valid, double pointer_x, double pointer_y,
    uint32_t document_width, uint32_t document_height,
    size_t tiles_done, size_t tiles_total, bool tiles_active);
bool viewer_editor_take_redraw_request(viewer_editor *editor);
void viewer_editor_pointer_move(viewer_editor *editor, uint32_t x, uint32_t y);
void viewer_editor_pointer_up(viewer_editor *editor);
uint32_t viewer_editor_context_action_at(viewer_editor *editor,
    uint32_t x, uint32_t y);
uint32_t viewer_editor_context_action_run(viewer_editor *editor,
    const svg_shape_document *document, uint32_t action);
bool viewer_editor_get_chrome_layout(viewer_editor *editor,
                                     struct viewer_chrome_layout *out_layout);

#endif
