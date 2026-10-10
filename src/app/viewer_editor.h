#ifndef VEXEL_APP_VIEWER_EDITOR_H
#define VEXEL_APP_VIEWER_EDITOR_H

#include "document_edit.h"
#include "platform/platform_window.h"
#include "svg/shape/shape.h"
#include "ui/ui_core.h"
#include "ui/viewer_controls.h"

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

typedef struct viewer_editor viewer_editor;
struct viewer_chrome_layout;

typedef enum viewer_editor_draw_tool {
    VIEWER_EDITOR_TOOL_SELECT,
    VIEWER_EDITOR_TOOL_RECT,
    VIEWER_EDITOR_TOOL_ELLIPSE,
    VIEWER_EDITOR_TOOL_LINE
} viewer_editor_draw_tool;

typedef struct viewer_editor_canvas_overlay_state {
    svg_shape_rect bounds; /* decoded document tile pixels */
    double rotation_radians;
    bool visible;
    bool drawing_preview;
} viewer_editor_canvas_overlay_state;

enum {
    VIEWER_EDITOR_HANDLED = 1U << 0U,
    VIEWER_EDITOR_MUTATED = 1U << 1U,
    VIEWER_EDITOR_SAVE = 1U << 2U,
    VIEWER_EDITOR_SAVE_AS = 1U << 3U,
    VIEWER_EDITOR_OPEN = 1U << 4U,
    VIEWER_EDITOR_QUIT = 1U << 5U,
    VIEWER_EDITOR_RELOAD = 1U << 6U
};

viewer_editor *viewer_editor_create(base_bytes source, ui_theme theme,
                                    uint32_t width, uint32_t height);
/* Explicit style constructor used by deterministic and preference-injection tests. */
viewer_editor *viewer_editor_create_styled(base_bytes source, ui_theme theme,
                                           uint32_t width, uint32_t height,
                                           bool modern_style);
void viewer_editor_destroy(viewer_editor *editor);
bool viewer_editor_is_dirty(const viewer_editor *editor);
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
bool viewer_editor_scroll(viewer_editor *editor, uint32_t x, double delta_y);

size_t viewer_editor_byte_length(const viewer_editor *editor);
bool viewer_editor_materialize(viewer_editor *editor, uint8_t *buffer,
                               size_t capacity, size_t *out_length);
document_edit_save_result viewer_editor_save(viewer_editor *editor,
                                             base_bytes path);
bool viewer_editor_select_at_source_offset(viewer_editor *editor, size_t offset);
bool viewer_editor_selection_span(viewer_editor *editor,
                                  document_source_span *out_span);
bool viewer_editor_semantics(viewer_editor *editor,
                             ui_semantics_snapshot *out_snapshot);
bool viewer_editor_set_appearance(viewer_editor *editor, ui_theme theme,
                                  bool increased_contrast, bool reduced_motion);
bool viewer_editor_set_sidebar_visibility(viewer_editor *editor,
                                          bool structure_visible,
                                          bool inspector_visible);
bool viewer_editor_set_tool(viewer_editor *editor, ui_icon_kind tool);
ui_icon_kind viewer_editor_get_tool(const viewer_editor *editor);
bool viewer_editor_text_input_active(const viewer_editor *editor);
bool viewer_editor_set_draw_tool(viewer_editor *editor,
                                 viewer_editor_draw_tool tool);
viewer_editor_draw_tool viewer_editor_get_draw_tool(const viewer_editor *editor);
/* Pointer coordinates and navigation use backing pixels relative to canvas origin. */
uint32_t viewer_editor_canvas_pointer_down(viewer_editor *editor,
    const svg_shape_document *document, const ui_viewer_navigation *navigation,
    double x, double y, uint32_t modifiers);
uint32_t viewer_editor_canvas_pointer_move(viewer_editor *editor,
    const svg_shape_document *document, const ui_viewer_navigation *navigation,
    double x, double y);
uint32_t viewer_editor_canvas_pointer_up(viewer_editor *editor,
    const svg_shape_document *document);
uint32_t viewer_editor_cancel_canvas_gesture(viewer_editor *editor);
uint32_t viewer_editor_nudge_selection(viewer_editor *editor,
                                       double dx, double dy);
bool viewer_editor_get_canvas_overlay(viewer_editor *editor,
    const svg_shape_document *document,
    viewer_editor_canvas_overlay_state *out_overlay);
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
bool viewer_editor_get_chrome_layout(viewer_editor *editor,
                                     struct viewer_chrome_layout *out_layout);

#endif
