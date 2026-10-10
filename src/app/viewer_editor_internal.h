#ifndef VEXEL_APP_VIEWER_EDITOR_INTERNAL_H
#define VEXEL_APP_VIEWER_EDITOR_INTERNAL_H

#include "viewer_editor.h"

#define EDITOR_OUTLINE_CAP 1024U
#define EDITOR_OUTLINE_VISIBLE 20U
#define EDITOR_ATTRIBUTE_VISIBLE 10U
#define EDITOR_FIELD_CAP (64U * 1024U)
#define EDITOR_NAME_CAP 128U
#define EDITOR_STATUS_CAP 192U
#define EDITOR_DEPTH_CAP 256U
#define EDITOR_TOOLBAR_HEIGHT 40U
#define EDITOR_STATUS_HEIGHT 24U
#define EDITOR_LEFT_WIDTH 232U
#define EDITOR_RIGHT_WIDTH 320U
#define EDITOR_TOOL_RAIL_WIDTH 44U

enum {
    EDITOR_NODE_TOOLBAR = 2U,
    EDITOR_NODE_WORKSPACE,
    EDITOR_NODE_OUTLINE,
    EDITOR_NODE_OUTLINE_TITLE,
    EDITOR_NODE_CANVAS,
    EDITOR_NODE_INSPECTOR,
    EDITOR_NODE_INSPECTOR_TITLE,
    EDITOR_NODE_SELECTED_TAG,
    EDITOR_NODE_STATUS,
    EDITOR_NODE_OPTIONS = 20U,
    EDITOR_NODE_TOOL_RAIL,
    EDITOR_NODE_CANVAS_HUD,
    EDITOR_NODE_CANVAS_TILE_STATUS,
    EDITOR_NODE_OUTLINE_HEADER,
    EDITOR_NODE_INSPECTOR_HEADER,
    EDITOR_BUTTON_SAVE = 100U,
    EDITOR_BUTTON_UNDO,
    EDITOR_BUTTON_REDO,
    EDITOR_BUTTON_DELETE,
    EDITOR_BUTTON_DUPLICATE,
    EDITOR_BUTTON_GROUP,
    EDITOR_BUTTON_UNGROUP,
    EDITOR_BUTTON_SAVE_AS,
    EDITOR_BUTTON_ADD_ATTRIBUTE,
    EDITOR_BUTTON_REMOVE_ATTRIBUTE,
    EDITOR_BUTTON_TOOL_SELECT = 300U,
    EDITOR_BUTTON_TOOL_PAN,
    EDITOR_BUTTON_TOOL_ZOOM,
    EDITOR_BUTTON_TOGGLE_STRUCTURE,
    EDITOR_BUTTON_TOGGLE_INSPECTOR,
    EDITOR_BUTTON_HIDE_STRUCTURE,
    EDITOR_BUTTON_HIDE_INSPECTOR,
    EDITOR_OUTLINE_ROW_BASE = 1000U,
    EDITOR_ATTRIBUTE_ROW_BASE = 10000U,
    EDITOR_TEXT_FIELD_ID = 20000U,
    EDITOR_NEW_NAME_ID,
    EDITOR_NEW_VALUE_ID
};

typedef enum editor_field_kind {
    EDITOR_FIELD_NONE,
    EDITOR_FIELD_ATTRIBUTE,
    EDITOR_FIELD_TEXT,
    EDITOR_FIELD_NEW_NAME,
    EDITOR_FIELD_NEW_VALUE
} editor_field_kind;

typedef enum editor_canvas_gesture_kind {
    EDITOR_CANVAS_GESTURE_NONE,
    EDITOR_CANVAS_GESTURE_MOVE,
    EDITOR_CANVAS_GESTURE_RESIZE,
    EDITOR_CANVAS_GESTURE_ROTATE,
    EDITOR_CANVAS_GESTURE_DRAW
} editor_canvas_gesture_kind;

typedef struct editor_outline_frame {
    document_handle node;
    document_handle child;
    size_t depth;
    bool entered;
} editor_outline_frame;

struct viewer_editor {
    document_edit *edit;
    document_handle selected;
    document_handle outline[EDITOR_OUTLINE_CAP];
    uint16_t outline_depth[EDITOR_OUTLINE_CAP];
    size_t outline_count;
    size_t selected_index;
    size_t outline_start;
    size_t attribute_start;
    size_t selected_attribute;
    size_t selected_offset;
    char selected_id[EDITOR_NAME_CAP];
    bool selected_has_id;
    char selected_attribute_name[EDITOR_NAME_CAP];
    char field[EDITOR_FIELD_CAP];
    size_t field_length;
    size_t cursor;
    size_t anchor;
    editor_field_kind field_kind;
    size_t field_attribute;
    char new_name[EDITOR_NAME_CAP];
    size_t new_name_length;
    char new_value[EDITOR_FIELD_CAP];
    size_t new_value_length;
    uint32_t focus_id;
    ui_theme theme;
    ui_icon_kind active_tool;
    viewer_editor_draw_tool draw_tool;
    editor_canvas_gesture_kind canvas_gesture;
    svg_shape_selection canvas_selection;
    svg_shape_rect canvas_preview_bounds;
    double gesture_start_x;
    double gesture_start_y;
    double gesture_start_device_x;
    double gesture_start_device_y;
    double gesture_current_x;
    double gesture_current_y;
    double gesture_scale_x;
    double gesture_scale_y;
    double gesture_rotation;
    double gesture_pivot_x;
    double gesture_pivot_y;
    uint32_t hovered_id;
    uint32_t pressed_id;
    uint8_t resize_handle;
    uint32_t width;
    uint32_t height;
    double backing_scale;
    ui_tree tree;
    char status[EDITOR_STATUS_CAP];
    char display_text[EDITOR_FIELD_CAP + 1U];
    bool tree_ready;
    bool outline_truncated;
    bool modern_style;
    bool increased_contrast;
    bool reduced_motion;
    bool redraw_requested;
    bool canvas_selection_valid;
    bool canvas_gesture_moved;
    bool want_structure;
    bool want_inspector;
    bool effective_structure;
    bool effective_inspector;
    double canvas_zoom;
    double canvas_pointer_x;
    double canvas_pointer_y;
    uint32_t canvas_document_width;
    uint32_t canvas_document_height;
    size_t canvas_tiles_done;
    size_t canvas_tiles_total;
    bool canvas_pointer_valid;
    bool canvas_tiles_active;
};

void editor_set_text(char *target, size_t capacity, const char *source);
size_t editor_copy_slice(char *destination, size_t capacity, base_bytes bytes);
bool editor_add_node(viewer_editor *editor, uint32_t id, uint32_t parent,
                     ui_axis axis, uint32_t width, uint32_t height,
                     uint32_t padding, uint32_t gap, uint32_t color,
                     ui_role role, const char *label, size_t label_len,
                     const char *value, size_t value_len, uint32_t state,
                     uint32_t command_id);
size_t editor_format_outline(const viewer_editor *editor, size_t index,
                             char out[UI_TEXT_MAX_BYTES]);
size_t editor_utf8_prefix(const uint8_t *bytes, size_t length, size_t maximum);
bool editor_same_handle(document_handle left, document_handle right);
const document_model *editor_model(const viewer_editor *editor);
void editor_status_error(viewer_editor *editor, document_edit_error error);
void editor_status_ok(viewer_editor *editor, const char *message);
void editor_refresh_outline(viewer_editor *editor);
void editor_select_handle(viewer_editor *editor, document_handle selected);
void editor_rebind_after_change(viewer_editor *editor, size_t old_offset,
                                const char *old_id, document_handle preferred);
bool editor_select_outline_index(viewer_editor *editor, size_t index);
bool editor_select_parent(viewer_editor *editor);
bool editor_select_child(viewer_editor *editor);
bool editor_select_delta(viewer_editor *editor, int direction);
bool editor_begin_field(viewer_editor *editor, editor_field_kind kind,
                        size_t attribute_index);
bool editor_store_pending_field(viewer_editor *editor);
uint32_t editor_apply_result(viewer_editor *editor, document_edit_error result,
                             size_t old_offset, const char *old_id,
                             document_handle preferred);
uint32_t editor_apply_button(viewer_editor *editor, uint32_t id);
bool editor_reorder(viewer_editor *editor, bool forward);
size_t editor_utf8_next(const char *text, size_t length, size_t cursor);
void editor_selection_range(const viewer_editor *editor, size_t *start, size_t *end);
uint32_t editor_commit_field(viewer_editor *editor);
void editor_cancel_field(viewer_editor *editor);
uint32_t editor_history(viewer_editor *editor, bool redo);
void editor_build_tree(viewer_editor *editor);
void editor_build_tree_modern(viewer_editor *editor);

#endif
