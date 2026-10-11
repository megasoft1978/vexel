#ifndef VEXEL_PLATFORM_WINDOW_H
#define VEXEL_PLATFORM_WINDOW_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#define PLATFORM_PATH_CAP 4096U
#define PLATFORM_RECENT_FILES_MAX 5U
#define PLATFORM_RECENT_NAME_CAP 256U

/* AppKit window contract. All calls, including destroy and shutdown, must run
 * on the main thread. The opaque window is owned by the caller until destroy.
 * Pointer and precise gesture coordinates are logical points from the view's
 * top-left; dimensions are backing pixels. `backing_scale` is populated on
 * resize, scale, scroll, and magnify events. Convert cursor and viewport values
 * to backing pixels before applying a navigation transform. Scroll deltas are finite AppKit
 * scrollingDeltaX/Y values, bounded to +/-10000; magnification is finite and
 * bounded to +/-100. Text is UTF-8 and truncated only at a code-point edge. */
typedef struct platform_window platform_window;

typedef enum platform_event_type {
    PLATFORM_EVENT_NONE = 0,
    PLATFORM_EVENT_OPEN,
    PLATFORM_EVENT_FOCUS,
    PLATFORM_EVENT_APPEARANCE,
    PLATFORM_EVENT_DROP_ENTER,
    PLATFORM_EVENT_DROP_EXIT,
    PLATFORM_EVENT_RESIZE,
    PLATFORM_EVENT_SCALE,
    PLATFORM_EVENT_KEY_DOWN,
    PLATFORM_EVENT_KEY_UP,
    PLATFORM_EVENT_POINTER_MOVE,
    PLATFORM_EVENT_POINTER_DOWN,
    PLATFORM_EVENT_POINTER_UP,
    PLATFORM_EVENT_SCROLL,
    PLATFORM_EVENT_MAGNIFY,
    PLATFORM_EVENT_CLOSE_REQUEST,
    PLATFORM_EVENT_FILE_OPEN,
    PLATFORM_EVENT_FILE_DROP,
    PLATFORM_EVENT_FILE_NEW,
    PLATFORM_EVENT_ACTION,
    PLATFORM_EVENT_COMMAND
} platform_event_type;

typedef enum platform_cursor_kind {
    PLATFORM_CURSOR_ARROW,
    PLATFORM_CURSOR_OPEN_HAND,
    PLATFORM_CURSOR_CLOSED_HAND,
    PLATFORM_CURSOR_RESIZE_NORTH_SOUTH,
    PLATFORM_CURSOR_RESIZE_EAST_WEST,
    PLATFORM_CURSOR_RESIZE_NORTHWEST_SOUTHEAST,
    PLATFORM_CURSOR_RESIZE_NORTHEAST_SOUTHWEST,
    PLATFORM_CURSOR_ROTATE
} platform_cursor_kind;

typedef enum platform_viewer_action {
    PLATFORM_ACTION_NONE,
    PLATFORM_ACTION_RELOAD,
    PLATFORM_ACTION_TOGGLE_CHECKERBOARD,
    PLATFORM_ACTION_CYCLE_BACKGROUND,
    PLATFORM_ACTION_FIT_WIDTH,
    PLATFORM_ACTION_FIT_WINDOW,
    PLATFORM_ACTION_ACTUAL_SIZE,
    PLATFORM_ACTION_TOGGLE_PIXEL_GRID,
    PLATFORM_ACTION_NEW_WINDOW,
    PLATFORM_ACTION_TOGGLE_RULERS,
    PLATFORM_ACTION_TOGGLE_GUIDES,
    PLATFORM_ACTION_TOGGLE_GUIDE_LOCK,
    PLATFORM_ACTION_CLEAR_HORIZONTAL_GUIDES,
    PLATFORM_ACTION_CLEAR_VERTICAL_GUIDES,
    PLATFORM_ACTION_ZOOM_TO_SELECTION,
    PLATFORM_ACTION_QUIT
} platform_viewer_action;

typedef enum platform_modifier {
    PLATFORM_MODIFIER_COMMAND = 1U << 0,
    PLATFORM_MODIFIER_CONTROL = 1U << 1,
    PLATFORM_MODIFIER_SHIFT = 1U << 2,
    PLATFORM_MODIFIER_OPTION = 1U << 3
} platform_modifier;

typedef struct platform_event {
    platform_event_type type;
    int32_t logical_x;
    int32_t logical_y;
    uint32_t pixel_width;
    uint32_t pixel_height;
    double backing_scale;
    double delta_x;
    double delta_y;
    double magnification;
    double precise_x;
    double precise_y;
    uint16_t key_code;
    uint8_t click_count;
    uint32_t modifiers;
    uint8_t button;
    platform_viewer_action action;
    uint32_t command_id;
    char text[256];
    size_t text_len;
    /* Owned path bytes in this value event, NUL-terminated on file-open events. */
    char path[PLATFORM_PATH_CAP];
} platform_event;

typedef struct platform_window_config {
    const char *title_utf8; /* borrowed UTF-8 slice, valid only during create */
    size_t title_len;
    int32_t width_points;
    int32_t height_points;
    bool resizable;
} platform_window_config;

typedef struct platform_recent_file {
    char path[PLATFORM_PATH_CAP];
    char display_name[PLATFORM_RECENT_NAME_CAP];
    size_t path_length;
} platform_recent_file;

bool platform_init(void);
bool platform_window_create(const platform_window_config *config, platform_window **out_window);
bool platform_window_resize(platform_window *window, int32_t width_points, int32_t height_points);
/* Content-view top-left in the global point coordinates accepted by CGEvent. */
bool platform_window_content_screen_origin(platform_window *window,
                                           double *out_x, double *out_y);
bool platform_window_set_title(platform_window *window, const char *title_utf8, size_t title_len);
bool platform_window_set_cursor(platform_window *window, platform_cursor_kind cursor);
/* Starts a non-blocking macOS Open sheet; selection arrives as a later file event. */
bool platform_window_open_panel(platform_window *window);
bool platform_window_recent_add(platform_window *window, const char *path, size_t path_len);
size_t platform_window_recent_files(platform_window *window,
    platform_recent_file *out_files, size_t capacity);
bool platform_window_is_dark_appearance(platform_window *window);
bool platform_window_set_option_state(platform_window *window, platform_viewer_action action,
                                      bool enabled);
bool platform_window_show_ruler_context_menu(platform_window *window,
    bool horizontal_axis, int32_t logical_x, int32_t logical_y);
bool platform_window_set_command_state(platform_window *window, uint32_t command_id,
                                       bool enabled);
void platform_window_dispatch_command(platform_window *window, uint32_t command_id);
/* Pumps pending AppKit events without waiting. False means no event is ready. */
bool platform_poll_event(platform_window *window, platform_event *out_event);
void platform_window_destroy(platform_window *window);
void platform_shutdown(void);

#endif
