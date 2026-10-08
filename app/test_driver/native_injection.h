#ifndef VEXEL_NATIVE_INJECTION_H
#define VEXEL_NATIVE_INJECTION_H

#include "platform/platform_window.h"

/* Private smoke-only hooks; callers must use the AppKit main thread. */
bool platform_test_inject_key(platform_window *window);
bool platform_test_inject_pointer(platform_window *window);
bool platform_test_request_close(platform_window *window);

#endif
