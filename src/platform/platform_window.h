#ifndef VEXEL_PLATFORM_WINDOW_H
#define VEXEL_PLATFORM_WINDOW_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

/* AppKit window contract. All calls, including destroy and shutdown, must run
 * on the main thread. The opaque window is owned by the caller until destroy.
 * Pointer coordinates are logical points from the view's top-left; dimensions
 * are backing pixels. Text is UTF-8 and truncated only at a code-point edge. */
typedef struct platform_window platform_window;

typedef enum platform_event_type {
    PLATFORM_EVENT_NONE = 0,
    PLATFORM_EVENT_OPEN,
    PLATFORM_EVENT_RESIZE,
    PLATFORM_EVENT_SCALE,
    PLATFORM_EVENT_KEY_DOWN,
    PLATFORM_EVENT_KEY_UP,
    PLATFORM_EVENT_POINTER_MOVE,
    PLATFORM_EVENT_POINTER_DOWN,
    PLATFORM_EVENT_POINTER_UP,
    PLATFORM_EVENT_CLOSE_REQUEST
} platform_event_type;

typedef struct platform_event {
    platform_event_type type;
    int32_t logical_x;
    int32_t logical_y;
    uint32_t pixel_width;
    uint32_t pixel_height;
    double backing_scale;
    uint16_t key_code;
    uint32_t modifiers;
    uint8_t button;
    char text[256];
    size_t text_len;
} platform_event;

typedef struct platform_window_config {
    const char *title_utf8; /* borrowed UTF-8 slice, valid only during create */
    size_t title_len;
    int32_t width_points;
    int32_t height_points;
    bool resizable;
} platform_window_config;

bool platform_init(void);
bool platform_window_create(const platform_window_config *config, platform_window **out_window);
bool platform_window_resize(platform_window *window, int32_t width_points, int32_t height_points);
/* Pumps pending AppKit events without waiting. False means no event is ready. */
bool platform_poll_event(platform_window *window, platform_event *out_event);
void platform_window_destroy(platform_window *window);
void platform_shutdown(void);

#endif
