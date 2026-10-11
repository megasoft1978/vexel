#ifndef VEXEL_UI_WIDGETS_H
#define VEXEL_UI_WIDGETS_H

#include "ui_core.h"

#define UI_WIDGET_EDIT_MAX_BYTES 127U
#define UI_WIDGET_LABEL_MAX_BYTES 63U
#define UI_WIDGET_UNIT_MAX_BYTES 8U
#define UI_WIDGET_MAX_OPTIONS 16U
#define UI_WIDGET_MAX_SEGMENTS 8U

typedef enum ui_widget_input_result {
    UI_WIDGET_INPUT_UNHANDLED = 0,
    UI_WIDGET_INPUT_CHANGED,
    UI_WIDGET_INPUT_COMMITTED,
    UI_WIDGET_INPUT_CANCELLED,
    UI_WIDGET_INPUT_INVALID
} ui_widget_input_result;

typedef enum ui_widget_tone {
    UI_WIDGET_TONE_NEUTRAL = 0,
    UI_WIDGET_TONE_SUCCESS,
    UI_WIDGET_TONE_WARNING,
    UI_WIDGET_TONE_ERROR
} ui_widget_tone;

typedef enum ui_widget_pointer_event {
    UI_WIDGET_POINTER_ENTER = 0,
    UI_WIDGET_POINTER_LEAVE,
    UI_WIDGET_POINTER_MOVE,
    UI_WIDGET_POINTER_DOWN,
    UI_WIDGET_POINTER_UP_INSIDE,
    UI_WIDGET_POINTER_UP_OUTSIDE,
    UI_WIDGET_POINTER_CANCEL
} ui_widget_pointer_event;

typedef struct ui_widget_pointer_state {
    bool enabled, hovered, pressed, focused, selected;
} ui_widget_pointer_state;

typedef struct ui_widget_text_edit {
    char semantic_label[UI_WIDGET_LABEL_MAX_BYTES + 1U];
    char value[UI_WIDGET_EDIT_MAX_BYTES + 1U];
    char original[UI_WIDGET_EDIT_MAX_BYTES + 1U];
    size_t semantic_label_length, length, original_length;
    size_t cursor, anchor;
    bool editing, invalid;
} ui_widget_text_edit;

typedef struct ui_widget_number_field {
    ui_widget_text_edit edit;
    char unit[UI_WIDGET_UNIT_MAX_BYTES + 1U];
    size_t unit_length;
    double value, original, minimum, maximum, step;
    double scrub_origin_value;
    int32_t scrub_origin_x;
    bool scrubbing;
} ui_widget_number_field;

typedef struct ui_widget_color_field {
    ui_widget_text_edit edit;
    uint32_t rgba, original_rgba;
} ui_widget_color_field;

typedef struct ui_widget_slider {
    char semantic_label[UI_WIDGET_LABEL_MAX_BYTES + 1U];
    size_t semantic_label_length;
    double value, minimum, maximum, step;
    bool dragging;
} ui_widget_slider;

typedef struct ui_widget_option { const char *text; size_t length; } ui_widget_option;
typedef struct ui_widget_dropdown {
    char semantic_label[UI_WIDGET_LABEL_MAX_BYTES + 1U];
    size_t semantic_label_length;
    char options[UI_WIDGET_MAX_OPTIONS][UI_WIDGET_LABEL_MAX_BYTES + 1U];
    uint8_t option_lengths[UI_WIDGET_MAX_OPTIONS];
    size_t option_count;
    size_t selected_index, active_index;
    bool expanded;
} ui_widget_dropdown;

typedef struct ui_widget_spec {
    uint32_t id;
    uint32_t parent_id;
    ui_axis axis;
    uint32_t preferred_width;
    uint32_t preferred_height;
    uint32_t padding;
    uint32_t gap;
    uint32_t color;
    uint32_t command_id;
    uint32_t state;
    ui_widget_kind widget;
    const char *label;
    size_t label_len;
    const char *value;
    size_t value_len;
    ui_icon_kind icon;
} ui_widget_spec;

bool ui_widget_add_button(ui_tree *tree, const ui_widget_spec *spec);
bool ui_widget_add_checkbox(ui_tree *tree, const ui_widget_spec *spec);
bool ui_widget_add_text_field(ui_tree *tree, const ui_widget_spec *spec);
bool ui_widget_add_slider(ui_tree *tree, const ui_widget_spec *spec);
bool ui_widget_add_row(ui_tree *tree, const ui_widget_spec *spec);

ui_control_state ui_widget_pointer_visual_state(const ui_widget_pointer_state *state);
bool ui_widget_pointer_transition(ui_widget_pointer_state *state,
                                  ui_widget_pointer_event event);

bool ui_widget_text_edit_init(ui_widget_text_edit *edit, const char *semantic_label,
    size_t label_length, const char *initial_value, size_t value_length);
bool ui_widget_text_edit_begin(ui_widget_text_edit *edit);
ui_widget_input_result ui_widget_text_edit_key(ui_widget_text_edit *edit,
    uint32_t key, uint32_t modifiers);
bool ui_widget_text_edit_pointer(ui_widget_text_edit *edit, size_t byte_offset,
                                 bool extend_selection);
bool ui_widget_text_edit_commit(ui_widget_text_edit *edit);
bool ui_widget_text_edit_cancel(ui_widget_text_edit *edit);

bool ui_widget_number_parse(const char *text, size_t length, double *out_value);
bool ui_widget_number_format(double value, char *buffer, size_t capacity,
                             size_t *out_length);
bool ui_widget_number_field_init(ui_widget_number_field *field,
    const char *semantic_label, size_t label_length, double value,
    double minimum, double maximum, double step,
    const char *unit, size_t unit_length);
bool ui_widget_number_field_set_draft(ui_widget_number_field *field,
                                     const char *text, size_t length);
ui_widget_input_result ui_widget_number_field_key(ui_widget_number_field *field,
    uint32_t key, uint32_t modifiers);
ui_widget_input_result ui_widget_number_field_pointer(ui_widget_number_field *field,
    ui_widget_pointer_event event, int32_t x, uint32_t modifiers);

bool ui_widget_hex_parse(const char *text, size_t length, uint32_t *out_rgba);
bool ui_widget_hex_format(uint32_t rgba, char *buffer, size_t capacity,
                          size_t *out_length);
bool ui_widget_color_field_init(ui_widget_color_field *field,
    const char *semantic_label, size_t label_length, uint32_t rgba);
bool ui_widget_color_field_set_draft(ui_widget_color_field *field,
                                     const char *text, size_t length);
ui_widget_input_result ui_widget_color_field_key(ui_widget_color_field *field,
    uint32_t key, uint32_t modifiers);

bool ui_widget_slider_init(ui_widget_slider *slider, const char *semantic_label,
    size_t label_length, double value, double minimum, double maximum, double step);
ui_widget_input_result ui_widget_slider_key(ui_widget_slider *slider,
    uint32_t key, uint32_t modifiers);
bool ui_widget_slider_pointer(ui_widget_slider *slider, ui_widget_pointer_event event,
    int32_t x, int32_t track_x, uint32_t track_width);

bool ui_widget_dropdown_init(ui_widget_dropdown *dropdown,
    const char *semantic_label, size_t label_length,
    const ui_widget_option *options, size_t option_count, size_t selected_index);
ui_widget_input_result ui_widget_dropdown_key(ui_widget_dropdown *dropdown,
    uint32_t key, uint32_t modifiers);

typedef bool (*ui_widget_text_draw_fn)(void *context, const char *text,
    size_t length, ui_rect bounds, ui_theme theme, uint32_t font_size_px,
    uint32_t color, ui_font_family family, ui_font_weight weight,
    uint8_t *rgba, uint32_t width, uint32_t height, size_t stride);

typedef struct ui_widget_canvas {
    uint8_t *rgba;
    size_t capacity, stride;
    uint32_t width, height, backing_scale;
    double ui_scale;
    uint8_t text_scale_percent;
    ui_theme theme;
    bool increased_contrast;
    ui_tokens tokens;
    ui_widget_text_draw_fn draw_text;
    void *text_context;
} ui_widget_canvas;

typedef struct ui_widget_draw_spec {
    ui_widget_kind kind;
    ui_control_state state;
    ui_rect bounds;
    ui_icon_kind icon;
    const char *label;
    size_t label_length;
    const char *value;
    size_t value_length;
    const char *secondary;
    size_t secondary_length;
    const char *unit;
    size_t unit_length;
    double numeric_value;
    uint32_t rgba;
    uint8_t font_size_pt;
    ui_font_family font_family;
    ui_font_weight font_weight;
    uint8_t segment_index, segment_count;
    const char *segment_labels[UI_WIDGET_MAX_SEGMENTS];
    size_t segment_label_lengths[UI_WIDGET_MAX_SEGMENTS];
    uint8_t indent;
    uint16_t selection_start, selection_length, cursor_byte_offset;
    ui_widget_tone tone;
    uint32_t text_color;
    bool checked, expanded, invalid, placeholder, focus_visible;
    bool has_children, visible, locked, show_row_actions;
    bool has_transition;
    ui_control_state transition_from, transition_to;
    uint16_t transition_progress_permille;
    uint32_t transition_from_color, transition_to_color;
} ui_widget_draw_spec;

bool ui_widget_canvas_init(ui_widget_canvas *canvas, uint8_t *rgba,
    size_t capacity, uint32_t width, uint32_t height, size_t stride,
    uint32_t backing_scale, ui_theme theme, bool increased_contrast,
    ui_widget_text_draw_fn draw_text, void *text_context);
bool ui_widget_canvas_init_scaled(ui_widget_canvas *canvas, uint8_t *rgba,
    size_t capacity, uint32_t width, uint32_t height, size_t stride,
    uint32_t backing_scale, double ui_scale, ui_theme theme,
    bool increased_contrast, ui_widget_text_draw_fn draw_text, void *text_context);
bool ui_widget_canvas_set_text_scale(ui_widget_canvas *canvas,
    uint8_t scale_percent);
bool ui_widget_paint(ui_widget_canvas *canvas, const ui_widget_draw_spec *spec);
size_t ui_widget_paint_scratch_limit(void);

#endif
