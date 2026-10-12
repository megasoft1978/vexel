#ifndef VEXEL_APP_VIEWER_EDITOR_INTERNAL_H
#define VEXEL_APP_VIEWER_EDITOR_INTERNAL_H

#include "viewer_editor.h"
#include "viewer_workspace.h"

#define EDITOR_OUTLINE_CAP 1024U
#define EDITOR_OUTLINE_VISIBLE 20U
#define EDITOR_OUTLINE_FILTER_CAP 64U
#define EDITOR_ATTRIBUTE_VISIBLE 10U
#define EDITOR_FIELD_CAP (64U * 1024U)
#define EDITOR_NAME_CAP 128U
#define EDITOR_STATUS_CAP 192U
#define EDITOR_LOG_CAP 64U
#define EDITOR_LOG_TEXT_CAP 128U
#define EDITOR_LOG_VISIBLE 8U
#define EDITOR_LOG_ROW_HEIGHT_POINTS 20U
#define EDITOR_DEPTH_CAP 256U
#define EDITOR_CANVAS_SELECTION_CAP 128U
#define EDITOR_SNAP_GUIDE_CAP 128U
#define EDITOR_VISIBILITY_OVERRIDE_CAP EDITOR_OUTLINE_CAP
#define EDITOR_OUTLINE_ACTION_WIDTH_POINTS 34U
#define EDITOR_OUTLINE_VISIBILITY_WIDTH_POINTS 16U
#define EDITOR_TOOLBAR_HEIGHT 40U
#define EDITOR_STATUS_HEIGHT 24U
#define EDITOR_LEFT_WIDTH 232U
#define EDITOR_RIGHT_WIDTH 320U
#define EDITOR_TOOL_RAIL_WIDTH 44U
#define EDITOR_PALETTE_QUERY_CAP 64U
#define EDITOR_PALETTE_VISIBLE_MAX 8U
#define EDITOR_HISTORY_VISIBLE_MAX 20U
#define EDITOR_SPLITTER_STRUCTURE_ID 24010U
#define EDITOR_SPLITTER_INSPECTOR_ID 24011U
#define EDITOR_NODE_FOCUS_BREADCRUMB 24012U
#define EDITOR_NODE_REVEAL_TAG 24504U
#define EDITOR_MOTION_CONTROL_CAP 4U

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
    EDITOR_NODE_CANVAS_ZOOM_MENU = 26U,
    EDITOR_NODE_TIMELINE_DOCK = 27U,
    EDITOR_BUTTON_SNAPPING = 121U,
    EDITOR_BUTTON_SNAP_GRID,
    EDITOR_BUTTON_SNAP_OBJECTS,
    EDITOR_BUTTON_SNAP_GUIDES,
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
    EDITOR_BUTTON_HISTORY = 607U,
    EDITOR_BUTTON_NODE_CORNER = 130U,
    EDITOR_BUTTON_NODE_SMOOTH,
    EDITOR_BUTTON_TOOL_SELECT = 300U,
    EDITOR_BUTTON_TOOL_PAN,
    EDITOR_BUTTON_TOOL_ZOOM,
    EDITOR_BUTTON_TOGGLE_STRUCTURE,
    EDITOR_BUTTON_TOGGLE_INSPECTOR,
    EDITOR_BUTTON_TOGGLE_TIMELINE,
    EDITOR_BUTTON_HIDE_STRUCTURE,
    EDITOR_BUTTON_HIDE_INSPECTOR,
    EDITOR_BUTTON_FOCUS_GROUP,
    EDITOR_BUTTON_RESET_FOCUS,
    EDITOR_BUTTON_TOOL_RECTANGLE = 400U,
    EDITOR_BUTTON_TOOL_ELLIPSE,
    EDITOR_BUTTON_TOOL_LINE,
    EDITOR_BUTTON_TOOL_PEN,
    EDITOR_BUTTON_TOOL_NODE,
    EDITOR_BUTTON_TOOL_TEXT,
    EDITOR_BUTTON_TOOL_POLYGON,
    EDITOR_BUTTON_TOOL_POLYLINE,
    EDITOR_BUTTON_INSPECTOR_OTHER_ATTRIBUTES = 500U,
    EDITOR_BUTTON_INSPECTOR_GEOMETRY = 501U,
    EDITOR_BUTTON_INSPECTOR_APPEARANCE = 502U,
    EDITOR_BUTTON_INSPECTOR_TAB_OBJECT = 510U,
    EDITOR_BUTTON_INSPECTOR_TAB_FILL = 511U,
    EDITOR_BUTTON_INSPECTOR_TAB_STROKE = 512U,
    EDITOR_BUTTON_INSPECTOR_TAB_DOCUMENT = 513U,
    EDITOR_BUTTON_ZOOM_HUD = 600U,
    EDITOR_BUTTON_ZOOM_PRESET_BASE = 610U,
    EDITOR_CONTEXT_EDIT_NODES = 630U,
    EDITOR_CONTEXT_FILL,
    EDITOR_CONTEXT_STROKE,
    EDITOR_CONTEXT_ALIGN,
    EDITOR_CONTEXT_GROUP,
    EDITOR_CONTEXT_DUPLICATE,
    EDITOR_CONTEXT_DELETE,
    EDITOR_OUTLINE_ROW_BASE = 1000U,
    EDITOR_ATTRIBUTE_ROW_BASE = 10000U,
    EDITOR_TEXT_FIELD_ID = 20000U,
    EDITOR_NEW_NAME_ID,
    EDITOR_NEW_VALUE_ID,
    EDITOR_OUTLINE_SEARCH_ID = 21000U,
    EDITOR_RENAME_ID,
    EDITOR_PALETTE_BACKDROP = 22000U,
    EDITOR_PALETTE_PANEL,
    EDITOR_PALETTE_SURFACE,
    EDITOR_PALETTE_TITLE,
    EDITOR_PALETTE_SEARCH,
    EDITOR_PALETTE_GROUPS,
    EDITOR_PALETTE_RESULT_BASE = 22010U,
    EDITOR_PALETTE_FOOTER = 22018U,
    EDITOR_HISTORY_BACKDROP = 22100U,
    EDITOR_HISTORY_PANEL,
    EDITOR_HISTORY_SURFACE,
    EDITOR_HISTORY_TITLE,
    EDITOR_HISTORY_CURRENT,
    EDITOR_HISTORY_SAVED,
    EDITOR_HISTORY_ROW_BASE = 22110U,
    EDITOR_HISTORY_FOOTER = 22131U,
    EDITOR_INSPECTOR_SECTION_BASE = 22200U,
    EDITOR_NODE_ID_REVEAL = 24500U,
    EDITOR_BUTTON_COPY_FULL_ID,
    EDITOR_BUTTON_FIT_CANVAS,
    EDITOR_BUTTON_ACTUAL_SIZE,
    /* Blender-style task workspaces and the consolidated status bar. */
    EDITOR_NODE_WORKSPACES = 24510U,
    EDITOR_BUTTON_WORKSPACE_BASE = 24520U,
    EDITOR_NODE_STATUS_HINT = 24530U,
    EDITOR_NODE_STATUS_REPORT = 24531U,
    EDITOR_NODE_STATUS_READOUT = 24532U,
    EDITOR_BUTTON_STATUS_CANCEL = 24533U,
    /* Properties icon tabs, as in Blender's Properties editor. */
    EDITOR_NODE_INSPECTOR_TABS = 24540U,
    EDITOR_BUTTON_INSPECTOR_TAB_BASE = 24550U,
    EDITOR_NODE_INSPECTOR_DOCUMENT_SIZE = 24560U,
    /* Operator/warning/error log pane. */
    EDITOR_NODE_LOG = 24570U,
    EDITOR_NODE_LOG_ROW_BASE = 24580U,
    EDITOR_BUTTON_TOGGLE_LOG = 530U,
    EDITOR_NODE_TOOL_OPTIONS = 24590U,
    EDITOR_NODE_ADJUST = 24600U,
    EDITOR_BUTTON_ADJUST_REPEAT = 540U
};

typedef enum editor_field_kind {
    EDITOR_FIELD_NONE,
    EDITOR_FIELD_ATTRIBUTE,
    EDITOR_FIELD_TEXT,
    EDITOR_FIELD_NEW_NAME,
    EDITOR_FIELD_NEW_VALUE,
    EDITOR_FIELD_SEARCH,
    EDITOR_FIELD_RENAME
} editor_field_kind;

typedef enum editor_canvas_gesture_kind {
    EDITOR_CANVAS_GESTURE_NONE,
    EDITOR_CANVAS_GESTURE_MOVE,
    EDITOR_CANVAS_GESTURE_RESIZE,
    EDITOR_CANVAS_GESTURE_ROTATE,
    EDITOR_CANVAS_GESTURE_DRAW,
    EDITOR_CANVAS_GESTURE_MARQUEE,
    EDITOR_CANVAS_GESTURE_PATH,
    EDITOR_CANVAS_GESTURE_PATH_NODE
} editor_canvas_gesture_kind;

typedef struct editor_outline_frame {
    document_handle node;
    document_handle child;
    size_t depth;
    bool entered;
} editor_outline_frame;

typedef struct editor_path_anchor {
    double x;
    double y;
    double handle_x;
    double handle_y;
    bool has_handle;
} editor_path_anchor;

typedef struct editor_snap_guide {
    bool vertical;
    double position;
} editor_snap_guide;

typedef struct editor_context_action {
    uint32_t id;
    const char *label;
    size_t label_length;
    ui_rect bounds;
} editor_context_action;

typedef struct editor_path_node_ref {
    size_t segment_index;
    size_t number_x;
    size_t number_y;
    size_t out_segment;
    size_t out_number_x;
    size_t out_number_y;
    size_t in_segment;
    size_t in_number_x;
    size_t in_number_y;
    double x;
    double y;
    double origin_x;
    double origin_y;
    geom_point out_control;
    geom_point in_control;
    double out_origin_x;
    double out_origin_y;
    double in_origin_x;
    double in_origin_y;
    bool has_x;
    bool has_y;
    bool has_out;
    bool has_in;
    bool relative;
    bool out_virtual;
    bool in_virtual;
    char command;
} editor_path_node_ref;

/* Density-dependent metric helpers; the chrome asks for these instead of
 * hard-coding a height so a single preference changes every list and field. */
uint32_t editor_row_height(const viewer_editor *editor);
uint32_t editor_field_height(const viewer_editor *editor);
uint32_t editor_section_height(const viewer_editor *editor);

typedef enum editor_log_severity {
    EDITOR_LOG_OPERATION = 0,
    EDITOR_LOG_INFORMATION,
    EDITOR_LOG_WARNING,
    EDITOR_LOG_ERROR
} editor_log_severity;

typedef struct editor_log_entry {
    char text[EDITOR_LOG_TEXT_CAP];
    size_t length;
    uint32_t sequence;
    uint8_t severity;
} editor_log_entry;

typedef struct editor_control_transition {
    uint32_t id;
    ui_control_state from_state, to_state;
    double start_seconds;
    uint16_t duration_ms;
    uint32_t from_color, to_color;
    bool active;
} editor_control_transition;

struct viewer_editor {
    document_edit *edit;
    viewer_workspace_kind workspace;
    viewer_editor_workspace_callback workspace_changed;
    void *workspace_context;
    char workspace_inspector_section[24];
    uint8_t inspector_tab;
    editor_log_entry log[EDITOR_LOG_CAP];
    size_t log_count;
    size_t log_start;
    uint32_t log_next_sequence;
    bool log_open;
    uint8_t density;
    viewer_editor_density_callback density_changed;
    void *density_context;
    bool focus_mode;
    bool focus_saved_structure;
    bool focus_saved_inspector;
    bool focus_saved_timeline;
    /* Numeric drag-to-scrub on an inspector field. */
    bool scrub_fine;
    bool field_scrub_armed;
    bool field_scrub_moved;
    uint32_t field_scrub_start_x;
    size_t field_scrub_attribute;
    double field_scrub_start_value;

    /* Blender's Adjust Last Operation: the parameters of the last edit, kept so
     * they can be re-applied without hunting through undo history. */
    struct {
        bool valid;
        bool dismissed;
        bool rotate;
        bool scale;
        char label[48];
        double translate_x, translate_y, scale_x, scale_y, rotate_radians;
        double pivot_x, pivot_y;
    } last_operation;
    bool status_operation_active;
    document_handle selected;
    document_handle outline[EDITOR_OUTLINE_CAP];
    uint16_t outline_depth[EDITOR_OUTLINE_CAP];
    uint16_t outline_visible[EDITOR_OUTLINE_CAP];
    document_handle collapsed_handles[EDITOR_OUTLINE_CAP];
    document_handle locked_handles[EDITOR_OUTLINE_CAP];
    document_handle visibility_handles[EDITOR_VISIBILITY_OVERRIDE_CAP];
    bool visibility_values[EDITOR_VISIBILITY_OVERRIDE_CAP];
    char outline_filter[EDITOR_OUTLINE_FILTER_CAP];
    char outline_filter_original[EDITOR_OUTLINE_FILTER_CAP];
    size_t outline_filter_length;
    size_t outline_filter_original_length;
    size_t outline_visible_count;
    size_t collapsed_count;
    size_t locked_count;
    size_t visibility_count;
    size_t outline_count;
    size_t selected_index;
    size_t outline_start;
    size_t outline_visible_start;
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
    uint32_t overlay_return_focus;
    char palette_query[EDITOR_PALETTE_QUERY_CAP];
    size_t palette_query_length;
    size_t palette_cursor;
    size_t palette_anchor;
    viewer_command_id palette_results[UI_MAX_COMMANDS];
    size_t palette_result_count;
    size_t palette_selected_index;
    size_t palette_first_visible;
    size_t palette_visible_capacity;
    size_t history_first_visible;
    size_t history_selected_cursor;
    size_t history_visible_capacity;
    viewer_command_id pending_command;
    document_handle focus_scope;
    bool focus_scope_active;
    bool inspector_geometry_expanded;
    bool inspector_appearance_expanded;
    bool selection_active;
    bool node_edit_active;
    bool inspector_other_attributes_expanded;
    ui_theme theme;
    ui_icon_kind active_tool;
    viewer_editor_draw_tool draw_tool;
    editor_canvas_gesture_kind canvas_gesture;
    editor_path_anchor path_anchors[VIEWER_EDITOR_PATH_POINT_CAP];
    size_t path_anchor_count;
    double path_current_x;
    double path_current_y;
    bool path_anchor_dragging;
    bool path_closed;
    bool path_close_preview;
    size_t path_node_segment;
    size_t path_node_number_x;
    size_t path_node_number_y;
    size_t path_selected_node_segment;
    size_t path_node_anchor_segment;
    size_t path_hovered_node_segment;
    size_t path_hovered_segment;
    uint8_t path_node_handle_kind;
    uint8_t path_hovered_handle_kind;
    double path_node_origin_x;
    double path_node_origin_y;
    double path_node_current_x;
    double path_node_current_y;
    bool path_node_has_x;
    bool path_node_has_y;
    bool path_node_relative;
    bool path_node_virtual_handle;
    bool path_node_is_initial;
    bool path_hovered_segment_valid;
    bool path_node_corner_available;
    bool path_node_smooth_available;
    bool text_draft_active;
    double text_draft_x;
    double text_draft_y;
    svg_shape_selection canvas_selection;
    size_t canvas_selection_offsets[EDITOR_CANVAS_SELECTION_CAP];
    size_t canvas_selection_count;
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
    uint32_t hovered_x;
    uint32_t hovered_y;
    uint32_t pressed_id;
    uint8_t resize_handle;
    uint32_t width;
    uint32_t height;
    double backing_scale;
    ui_tree tree;
    char status[EDITOR_STATUS_CAP];
    char display_text[EDITOR_FIELD_CAP + 1U];
    bool tree_ready;
    bool command_palette_open;
    bool history_popover_open;
    bool history_viewport_initialized;
    bool pending_command_valid;
    bool outline_truncated;
    bool modern_style;
    bool increased_contrast;
    bool reduced_motion;
    bool status_error;
    bool status_saved;
    bool status_ready;
    bool native_material_allowed;
    uint32_t system_accent_srgb;
    union {
        uint16_t ui_scale_percent;
        /* Kept as an alias for the accessibility lane's internal test API. */
        uint16_t interface_scale_percent;
    };
    uint16_t structure_width_points;
    uint16_t inspector_width_points;
    uint16_t layout_structure_width_points;
    uint16_t layout_inspector_width_points;
    uint32_t splitter_drag_id;
    uint32_t splitter_drag_start_x;
    uint16_t splitter_drag_start_width;
    bool splitter_dragging;
    double motion_time_seconds;
    double sidebar_motion_start_seconds;
    uint16_t sidebar_motion_start_structure;
    uint16_t sidebar_motion_start_inspector;
    uint16_t sidebar_motion_target_structure;
    uint16_t sidebar_motion_target_inspector;
    bool sidebar_motion_show_structure;
    bool sidebar_motion_show_inspector;
    bool sidebar_motion_active;
    editor_control_transition control_transitions[EDITOR_MOTION_CONTROL_CAP];
    document_handle reveal_handle;
    char reveal_id[EDITOR_FIELD_CAP + 1U];
    size_t reveal_id_length;
    char reveal_tag[EDITOR_FIELD_CAP + 1U];
    size_t reveal_tag_length;
    size_t reveal_scroll_codepoints;
    bool reveal_active;
    bool reveal_has_id;
    bool reveal_exceeds_limit;
    bool redraw_requested;
    bool canvas_selection_valid;
    bool marquee_preserve_selection;
    bool canvas_gesture_moved;
    bool snap_enabled;
    bool snap_bypass;
    bool navigation_overlays_hidden;
    bool zoom_menu_open;
    bool zoom_preset_pending;
    size_t zoom_preset_request;
    viewer_editor_guides guides;
    bool snap_grid_enabled;
    bool snap_objects_enabled;
    bool snap_guides_enabled;
    bool snap_popover_open;
    bool snap_popover_hovered;
    uint8_t snap_popover_hovered_row;
    bool snap_guide_x_visible;
    bool snap_guide_y_visible;
    double snap_guide_x;
    double snap_guide_y;
    editor_snap_guide snap_guides[EDITOR_SNAP_GUIDE_CAP];
    size_t snap_guide_count;
    char snap_label[32];
    bool snap_label_visible;
    bool snap_label_is_gap;
    double snap_label_delta;
    size_t context_action_count;
    editor_context_action context_actions[4];
    bool context_actions_fallback;
    uint32_t context_action_hovered;
    double context_pointer_x;
    double context_pointer_y;
    bool context_pointer_valid;
    bool want_structure;
    bool want_inspector;
    bool timeline_available;
    bool timeline_visible;
    uint16_t timeline_height_points;
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
bool editor_add_widget_node(viewer_editor *editor, uint32_t id, uint32_t parent,
                     ui_axis axis, uint32_t width, uint32_t height,
                     uint32_t padding, uint32_t gap, uint32_t color,
                     ui_role role, ui_widget_kind widget, ui_icon_kind icon,
                     const char *label, size_t label_len,
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
void editor_capture_selection_identity(viewer_editor *editor);
void editor_rebuild_visible_outline(viewer_editor *editor);
void editor_focus_scope_update(viewer_editor *editor);
void editor_focus_scope_reset(viewer_editor *editor);
bool editor_focus_scope_toggle(viewer_editor *editor);
bool editor_splitter_pointer_down(viewer_editor *editor, uint32_t x,
    uint32_t y, uint8_t click_count);
bool editor_splitter_pointer_move(viewer_editor *editor, uint32_t x, uint32_t y);
void editor_splitter_pointer_up(viewer_editor *editor);
bool editor_splitter_key(viewer_editor *editor, uint16_t key_code,
    uint32_t modifiers);
void editor_motion_control_transition(viewer_editor *editor, uint32_t id,
    ui_control_state from_state, ui_control_state to_state,
    uint16_t duration_ms);
bool editor_reveal_prepare(viewer_editor *editor, document_handle handle);
bool editor_reveal_focus(viewer_editor *editor);
bool editor_reveal_scroll(viewer_editor *editor, uint32_t x, uint32_t y,
    double delta_y);
void editor_reveal_layout_nodes(viewer_editor *editor);
void editor_reveal_paint(viewer_editor *editor, uint8_t *rgba,
    size_t capacity, size_t stride);
bool editor_outline_visible_model_index(const viewer_editor *editor, size_t row,
                                        size_t *out_index);
bool editor_outline_is_collapsed(const viewer_editor *editor, size_t index);
bool editor_outline_toggle_collapsed(viewer_editor *editor, size_t index);
bool editor_outline_toggle_visibility(viewer_editor *editor, size_t index);
bool editor_outline_is_visible(viewer_editor *editor, document_handle handle);
void editor_outline_visibility_reset(viewer_editor *editor);
bool editor_outline_toggle_lock(viewer_editor *editor, size_t index);
bool editor_outline_is_locked(const viewer_editor *editor, document_handle handle);
bool editor_canvas_selection_is_locked(viewer_editor *editor);
void editor_build_structure_panel(viewer_editor *editor, const ui_tokens *tokens);
bool editor_build_inspector_panel(viewer_editor *editor, const ui_tokens *tokens);
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
bool editor_reorder_to_edge(viewer_editor *editor, bool front);
bool editor_canvas_selection_contains(const viewer_editor *editor, size_t offset);
bool editor_selection_restore(viewer_editor *editor, const size_t *offsets,
                              size_t count);
bool editor_canvas_selection_bounds(viewer_editor *editor,
    const svg_shape_document *document, svg_shape_selection *out_selection);
bool editor_canvas_selection_toggle(viewer_editor *editor, size_t source_offset);
void editor_canvas_selection_clear(viewer_editor *editor);
bool editor_canvas_selection_marquee(viewer_editor *editor,
    const svg_shape_document *document, svg_shape_rect bounds, bool preserve);
uint32_t editor_canvas_multi_transform(viewer_editor *editor,
    double translate_x, double translate_y, double scale_x, double scale_y,
    double rotate_radians, double pivot_x, double pivot_y);
uint32_t editor_canvas_multi_translate_each(viewer_editor *editor,
    const double *translate_x, const double *translate_y, size_t count);
uint32_t editor_canvas_multi_delete(viewer_editor *editor);
uint32_t editor_canvas_multi_duplicate(viewer_editor *editor);
uint32_t editor_canvas_multi_group(viewer_editor *editor);
uint32_t editor_canvas_multi_ungroup(viewer_editor *editor);
uint32_t editor_commit_transform(viewer_editor *editor,
    double translate_x, double translate_y, double scale_x, double scale_y,
    double rotate_radians, double pivot_x, double pivot_y);
uint32_t editor_insert_drawn_shape(viewer_editor *editor,
    const svg_shape_document *document, geom_point start, geom_point end);
uint32_t editor_canvas_text_place(viewer_editor *editor,
    const svg_shape_document *document, geom_point user);
uint32_t editor_commit_text_draft(viewer_editor *editor);
bool editor_canvas_path_active(const viewer_editor *editor);
uint32_t editor_canvas_path_pointer_down(viewer_editor *editor, geom_point user,
    double screen_x, double screen_y, uint8_t click_count);
uint32_t editor_canvas_path_pointer_move(viewer_editor *editor, geom_point user,
    double screen_x, double screen_y);
uint32_t editor_canvas_path_pointer_up(viewer_editor *editor);
uint32_t editor_canvas_path_finish(viewer_editor *editor, bool closed);
void editor_canvas_path_cancel(viewer_editor *editor);
bool editor_canvas_path_node_begin(viewer_editor *editor,
    const svg_shape_document *document, geom_point user, double tolerance);
bool editor_canvas_path_node_collect(viewer_editor *editor,
    const svg_shape_document *document,
    editor_path_node_ref out_nodes[VIEWER_EDITOR_PATH_POINT_CAP],
    size_t *out_count);
bool editor_canvas_path_node_select_index(viewer_editor *editor,
    const svg_shape_document *document, size_t node_index);
uint32_t editor_canvas_path_node_hover(viewer_editor *editor,
    const svg_shape_document *document, geom_point user, double tolerance);
uint32_t editor_canvas_path_node_key(viewer_editor *editor,
    const svg_shape_document *document, uint16_t key_code, uint32_t modifiers);
uint32_t editor_canvas_path_node_nudge(viewer_editor *editor,
    const svg_shape_document *document, double dx, double dy);
uint32_t editor_canvas_path_node_convert(viewer_editor *editor,
    const svg_shape_document *document, bool smooth);
bool editor_canvas_path_node_mode_enter(viewer_editor *editor);
uint32_t editor_canvas_path_node_pointer_move(viewer_editor *editor,
    const svg_shape_document *document, geom_point user);
uint32_t editor_canvas_path_node_pointer_up(viewer_editor *editor,
    const svg_shape_document *document);
bool editor_canvas_path_nodes_overlay(viewer_editor *editor,
    const svg_shape_document *document,
    viewer_editor_canvas_overlay_state *out_overlay);
uint32_t editor_canvas_path_node_delete(viewer_editor *editor,
    const svg_shape_document *document);
uint32_t editor_canvas_path_add_node(viewer_editor *editor,
    const svg_shape_document *document, geom_point user, double tolerance);
uint32_t editor_canvas_path_curve_delete_node(viewer_editor *editor,
    const svg_shape_document *document);
uint32_t editor_canvas_path_curve_add_node(viewer_editor *editor,
    const svg_shape_document *document, geom_point user, double tolerance);
void editor_canvas_snap_translation(viewer_editor *editor,
    const svg_shape_document *document, double tolerance,
    double *translate_x, double *translate_y);
void editor_canvas_snap_clear(viewer_editor *editor);
bool editor_canvas_snap_guide_add(viewer_editor *editor, bool vertical,
                                  double position);
void editor_canvas_snap_guide_clear(viewer_editor *editor);
geom_point editor_canvas_snap_point(viewer_editor *editor,
    const svg_shape_document *document, geom_point point, double tolerance);
void editor_paint_context_actions(viewer_editor *editor,
    const svg_shape_document *document, const ui_viewer_navigation *navigation,
    ui_rect canvas, uint8_t *rgba, uint32_t width, uint32_t height,
    size_t stride);
uint32_t editor_context_action_at(viewer_editor *editor, uint32_t x, uint32_t y);
void editor_context_action_pointer_move(viewer_editor *editor,
    uint32_t x, uint32_t y);
uint32_t editor_context_action_run(viewer_editor *editor,
    const svg_shape_document *document, uint32_t action);
void editor_paint_snap_popover(viewer_editor *editor, uint8_t *rgba,
    size_t capacity, size_t stride);
bool editor_snap_popover_pointer_down(viewer_editor *editor, uint32_t x,
    uint32_t y);
void editor_snap_popover_pointer_move(viewer_editor *editor, uint32_t x,
    uint32_t y);
size_t editor_utf8_next(const char *text, size_t length, size_t cursor);
void editor_selection_range(const viewer_editor *editor, size_t *start, size_t *end);
uint32_t editor_commit_field(viewer_editor *editor);
void editor_cancel_field(viewer_editor *editor);
uint32_t editor_history(viewer_editor *editor, bool redo);
void editor_build_tree(viewer_editor *editor);
void editor_build_tree_legacy(viewer_editor *editor);
double editor_presentation_scale(const viewer_editor *editor);
bool editor_localize_slice(const char *source, size_t length,
    char *output, size_t capacity, size_t *out_length);
uint32_t editor_handle_focused_key(viewer_editor *editor, uint16_t key_code,
    uint32_t modifiers, const char *text, size_t text_length);
void editor_build_tree_modern(viewer_editor *editor);
void editor_overlay_add_nodes(viewer_editor *editor);
void editor_overlay_layout_nodes(viewer_editor *editor);
uint32_t editor_overlay_key(viewer_editor *editor, uint16_t key_code,
                            uint32_t modifiers, const char *text,
                            size_t text_len);
uint32_t editor_overlay_pointer_down(viewer_editor *editor, uint32_t id,
                                     uint32_t command_id);
bool editor_overlay_scroll(viewer_editor *editor, uint32_t x, uint32_t y,
                           double delta_y);
void editor_paint_widgets(viewer_editor *editor, uint8_t *rgba,
                          size_t capacity, size_t stride);
void viewer_editor_guides_paint(viewer_editor *editor,
    const svg_shape_document *document, const ui_viewer_navigation *navigation,
    ui_rect canvas, uint8_t *rgba, uint32_t width, uint32_t height,
    size_t stride);
void viewer_editor_guides_paint_scalar(viewer_editor *editor,
    const svg_shape_document *document, const ui_viewer_navigation *navigation,
    ui_rect canvas, uint8_t *rgba, uint32_t width, uint32_t height,
    size_t stride);

#endif
