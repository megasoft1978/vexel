#ifndef VEXEL_UI_CORE_H
#define VEXEL_UI_CORE_H

#include "raster/tile.h"

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#define UI_MAX_NODES 128U
#define UI_MAX_DEPTH 24U
#define UI_ROOT_ID 1U
#define UI_TEXT_MAX_BYTES 64U
#define UI_TEXT_TOTAL_MAX_BYTES (1024U * 1024U)
#define UI_MAX_COMMANDS 64U
#define UI_MOD_SHIFT 1U
#define UI_MOD_COMMAND 2U
#define UI_MOD_OPTION 4U
#define UI_MOD_CONTROL 8U

typedef enum ui_theme { UI_THEME_DARK = 0, UI_THEME_LIGHT = 1 } ui_theme;
typedef enum ui_axis { UI_AXIS_OVERLAY = 0, UI_AXIS_HORIZONTAL, UI_AXIS_VERTICAL } ui_axis;
typedef enum ui_role {
    UI_ROLE_NONE = 0, UI_ROLE_WINDOW, UI_ROLE_GROUP, UI_ROLE_BUTTON, UI_ROLE_CHECKBOX,
    UI_ROLE_TEXT, UI_ROLE_TEXT_FIELD, UI_ROLE_SLIDER, UI_ROLE_LIST, UI_ROLE_ROW, UI_ROLE_STATUS
} ui_role;
typedef enum ui_icon_kind {
    UI_ICON_NONE = 0,
    UI_ICON_SELECT,
    UI_ICON_PAN,
    UI_ICON_ZOOM_IN,
    UI_ICON_ZOOM_OUT,
    UI_ICON_RECTANGLE,
    UI_ICON_ELLIPSE,
    UI_ICON_LINE,
    UI_ICON_PEN,
    UI_ICON_NODE_EDIT,
    UI_ICON_PENCIL,
    UI_ICON_TEXT,
    UI_ICON_EYEDROPPER,
    UI_ICON_GRADIENT,
    UI_ICON_FILL,
    UI_ICON_STROKE,
    UI_ICON_LAYERS,
    UI_ICON_EYE,
    UI_ICON_EYE_OFF,
    UI_ICON_LOCK,
    UI_ICON_UNLOCK,
    UI_ICON_PLUS,
    UI_ICON_MINUS,
    UI_ICON_CLOSE,
    UI_ICON_CHEVRON_RIGHT,
    UI_ICON_CHEVRON_DOWN,
    UI_ICON_MORE,
    UI_ICON_SEARCH,
    UI_ICON_SAVE,
    UI_ICON_UNDO,
    UI_ICON_REDO,
    UI_ICON_DUPLICATE,
    UI_ICON_TRASH,
    UI_ICON_GROUP,
    UI_ICON_UNGROUP,
    UI_ICON_FIT,
    UI_ICON_ACTUAL_SIZE,
    UI_ICON_RULER,
    UI_ICON_GRID,
    UI_ICON_SNAP,
    UI_ICON_ALIGN_LEFT,
    UI_ICON_ALIGN_CENTER,
    UI_ICON_ALIGN_RIGHT,
    UI_ICON_ALIGN_TOP,
    UI_ICON_ALIGN_MIDDLE,
    UI_ICON_ALIGN_BOTTOM,
    UI_ICON_COUNT,
    /* Kept as the editor tool name for the original zoom icon. */
    UI_ICON_ZOOM = UI_ICON_ZOOM_IN
} ui_icon_kind;
typedef enum ui_key {
    UI_KEY_TAB = 0x110000U,
    UI_KEY_RETURN,
    UI_KEY_SPACE,
    UI_KEY_ESCAPE,
    UI_KEY_LEFT,
    UI_KEY_RIGHT,
    UI_KEY_UP,
    UI_KEY_DOWN,
    UI_KEY_HOME,
    UI_KEY_END,
    UI_KEY_BACKSPACE,
    UI_KEY_DELETE
} ui_key;
typedef enum ui_control_state {
    UI_CONTROL_REST = 0,
    UI_CONTROL_HOVER,
    UI_CONTROL_PRESSED,
    UI_CONTROL_FOCUS,
    UI_CONTROL_DISABLED,
    UI_CONTROL_SELECTED,
    UI_CONTROL_INVALID
} ui_control_state;
/* Presentation intent for retained nodes; AUTO preserves legacy role mapping. */
typedef enum ui_widget_kind {
    UI_WIDGET_AUTO = 0,
    UI_WIDGET_LABEL,
    UI_WIDGET_ICON_BUTTON,
    UI_WIDGET_BUTTON_PRIMARY,
    UI_WIDGET_BUTTON_SECONDARY,
    UI_WIDGET_BUTTON_GHOST,
    UI_WIDGET_SEGMENTED,
    UI_WIDGET_TOGGLE,
    UI_WIDGET_CHECKBOX,
    UI_WIDGET_TAB_BAR,
    UI_WIDGET_TOOLTIP,
    UI_WIDGET_HUD_PILL,
    UI_WIDGET_DIVIDER,
    UI_WIDGET_PANEL_HEADER,
    UI_WIDGET_SECTION_HEADER,
    UI_WIDGET_TREE_ROW,
    UI_WIDGET_EMPTY_STATE,
    UI_WIDGET_INLINE_ERROR,
    UI_WIDGET_TOAST,
    UI_WIDGET_NUMBER_FIELD,
    UI_WIDGET_SCRUB_LABEL,
    UI_WIDGET_STEPPER,
    UI_WIDGET_COLOR_FIELD,
    UI_WIDGET_SLIDER,
    UI_WIDGET_DROPDOWN,
    UI_WIDGET_TEXT_FIELD,
    UI_WIDGET_KIND_COUNT
} ui_widget_kind;
typedef enum ui_font_family { UI_FONT_SYSTEM = 0, UI_FONT_MONOSPACED } ui_font_family;
typedef enum ui_font_weight { UI_FONT_REGULAR = 0, UI_FONT_SEMIBOLD } ui_font_weight;
enum {
    UI_STATE_DISABLED = 1U << 0U,
    UI_STATE_CHECKED = 1U << 1U,
    UI_STATE_FOCUSED = 1U << 2U,
    UI_STATE_EXPANDED = 1U << 3U,
    UI_STATE_HOVERED = 1U << 4U,
    UI_STATE_PRESSED = 1U << 5U,
    UI_STATE_INVALID = 1U << 6U
};
typedef struct ui_rect { uint32_t x, y, width, height; } ui_rect;
typedef struct ui_tokens {
    /* Semantic style tokens, packed as sRGB RRGGBBAA. */
    uint32_t window, panel, panel_raised, panel_hover, pressed_fill, canvas_well;
    uint32_t text_primary, text_secondary, text_tertiary, disabled_text;
    uint32_t separator, control_edge;
    uint32_t accent, selection_fill, selection_edge, focus_ring;
    uint32_t checker_a, checker_b, guide, success, warning, error;
    uint32_t popover_shadow;
    /* Compatibility aliases for older demo clients. */
    uint32_t panel_alt, canvas, text, muted, border;
    uint32_t spacing, radius, font_px;
    uint32_t spacing_4, spacing_8, spacing_12, spacing_16, spacing_24, spacing_32;
    uint32_t radius_small, radius_control, radius_large;
} ui_tokens;
typedef struct ui_type_scale {
    uint16_t document_title_pt, panel_heading_pt, section_heading_pt;
    uint16_t body_pt, secondary_pt, metadata_pt, geometry_pt, empty_heading_pt;
    ui_font_family geometry_family;
    ui_font_weight title_weight, section_weight, empty_heading_weight;
} ui_type_scale;
typedef struct ui_metrics {
    uint16_t toolbar_height, context_height, tool_rail_width;
    uint16_t structure_width, inspector_width, status_height;
    uint16_t tree_row_height, section_header_height, field_height;
    uint16_t icon_button_target, icon_button_glyph, control_radius;
} ui_metrics;
typedef struct ui_node_spec {
    uint32_t id;
    uint32_t parent_id;
    ui_axis axis;
    uint32_t preferred_width;
    uint32_t preferred_height;
    uint32_t padding;
    uint32_t gap;
    uint32_t color;
    ui_role role;
    const char *label;
    size_t label_len;
    const char *value;
    size_t value_len;
    uint32_t state;
    uint32_t command_id;
    ui_icon_kind icon;
    ui_widget_kind widget;
} ui_node_spec;
typedef struct ui_node {
    ui_node_spec spec;
    ui_rect bounds;
    uint16_t first_child;
    uint16_t next_sibling;
    uint16_t child_count;
    bool occupied;
    size_t full_label_len;
    size_t full_value_len;
    size_t label_offset;
    size_t value_offset;
    char label[UI_TEXT_MAX_BYTES];
    char value[UI_TEXT_MAX_BYTES];
} ui_node;
typedef struct ui_tree {
    ui_node nodes[UI_MAX_NODES];
    size_t count;
    ui_tokens tokens;
    char text_storage[UI_TEXT_TOTAL_MAX_BYTES];
    size_t text_bytes_used;
    bool laid_out;
    uint32_t focused_id;
    ui_theme theme;
    bool increased_contrast;
    bool semantic_style;
    double backing_scale;
} ui_tree;

typedef struct ui_semantics_node {
    uint32_t id;
    uint32_t parent_id;
    ui_role role;
    uint32_t state;
    uint32_t command_id;
    size_t label_len;
    size_t value_len;
    size_t label_full_len;
    size_t value_full_len;
    size_t label_offset;
    size_t value_offset;
    ui_rect frame;
    char label[UI_TEXT_MAX_BYTES];
    char value[UI_TEXT_MAX_BYTES];
} ui_semantics_node;
typedef struct ui_semantics_snapshot {
    size_t count;
    size_t text_bytes_used;
    ui_semantics_node nodes[UI_MAX_NODES];
    char text_storage[UI_TEXT_TOTAL_MAX_BYTES];
} ui_semantics_snapshot;
typedef struct ui_command_binding {
    uint32_t command_id;
    uint32_t key;
    uint32_t modifiers;
    bool enabled;
} ui_command_binding;
typedef struct ui_command_registry {
    size_t count;
    ui_command_binding bindings[UI_MAX_COMMANDS];
} ui_command_registry;

/* Initializes an empty tree with stable, opaque dark or light tokens. */
bool ui_tree_init(ui_tree *tree, ui_theme theme);
/* Initializes the design-system palette. Tests may inject both preferences. */
bool ui_tree_init_accessible(ui_tree *tree, ui_theme theme,
                             bool increased_contrast);
ui_tokens ui_theme_tokens(ui_theme theme, bool increased_contrast);
ui_type_scale ui_theme_type_scale(void);
ui_metrics ui_design_metrics(void);
/* Adds a node. Labels/values are copied as bounded valid UTF-8 slices. */
/* Zero preferred extent means flexible allocation on the parent axis. */
bool ui_tree_add(ui_tree *tree, const ui_node_spec *spec);
/* Runs bounded deterministic layout. The root occupies the full pixel viewport. */
bool ui_tree_layout(ui_tree *tree, uint32_t width, uint32_t height);
/* Renders node fills and focused-control rings via the public scalar SVG raster API. */
bool ui_tree_render(const ui_tree *tree, raster_tile *out_tile);
/* Paints fills, labels, and values directly into caller storage without a viewport-sized scratch tile. */
bool ui_tree_paint_into(const ui_tree *tree, uint8_t *rgba, size_t capacity,
                        size_t stride);
/* Fills, focus rings, and icons only; native text is painted by platform/. */
bool ui_tree_paint_backgrounds_into(const ui_tree *tree, uint8_t *rgba,
                                    size_t capacity, size_t stride);
const ui_node *ui_tree_find(const ui_tree *tree, uint32_t id);
/* Snapshots are caller-owned, zero-filled, and ordered by retained-tree insertion. */
bool ui_semantics_snapshot_build(const ui_tree *tree, ui_semantics_snapshot *out_snapshot);
/* Returns a borrowed full UTF-8 slice backed by the semantics snapshot. */
bool ui_semantics_text(const ui_semantics_snapshot *snapshot,
                       const ui_semantics_node *node, bool value,
                       const char **out_text, size_t *out_length);
/* Focus traversal wraps and skips nodes that are disabled or not interactive roles. */
bool ui_focus_next(ui_tree *tree, bool reverse);
bool ui_command_registry_init(ui_command_registry *registry);
bool ui_command_register(ui_command_registry *registry, uint32_t command_id,
                         uint32_t key, uint32_t modifiers, bool enabled);
/* Handles Tab/Shift-Tab, Enter/Space activation, then registered key commands. */
bool ui_dispatch_key(ui_tree *tree, const ui_command_registry *registry,
                     uint32_t key, uint32_t modifiers, uint32_t *out_command_id);

#endif
