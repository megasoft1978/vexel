#ifndef VEXEL_NATIVE_INJECTION_H
#define VEXEL_NATIVE_INJECTION_H

#include "platform/platform_window.h"

/* Private smoke-only hooks; callers must use the AppKit main thread. */
bool platform_test_inject_key(platform_window *window, bool key_down);
bool platform_test_inject_pointer(platform_window *window, platform_event_type type,
                                  int32_t logical_x, int32_t logical_y);
bool platform_test_inject_scroll(platform_window *window, double delta_x, double delta_y,
                                 double logical_x, double logical_y);
bool platform_test_inject_magnify(platform_window *window, double magnification,
                                  double logical_x, double logical_y);
bool platform_test_request_close(platform_window *window);

#endif
