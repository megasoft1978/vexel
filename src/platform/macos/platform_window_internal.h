#ifndef VEXEL_PLATFORM_MACOS_WINDOW_INTERNAL_H
#define VEXEL_PLATFORM_MACOS_WINDOW_INTERNAL_H

#include "platform/platform_window.h"

bool platform_window_dispatch_file_open(platform_window *window, const char *path, size_t length);
bool platform_window_dispatch_file_drop(platform_window *window, const char *path, size_t length);
bool platform_window_dispatch_new_file(platform_window *window, const char *path, size_t length);
bool platform_window_dispatch_startup_files(platform_window *window);
bool platform_queue_test_open_from_environment(void);
bool platform_window_emit_internal(platform_window *window, const platform_event *event);
bool platform_path_supported_internal(const char *path, size_t length);
void platform_window_dispatch_action(platform_window *window, platform_viewer_action action);
void platform_macos_menus_init(void);
void platform_macos_menus_set_owner(platform_window *window);
void platform_macos_menus_clear_owner(platform_window *window);
bool platform_macos_menus_add_recent(const char *path, size_t length);
size_t platform_macos_menus_copy_recent(platform_recent_file *out_files, size_t capacity);
bool platform_macos_menus_set_option_state(platform_window *window,
                                           platform_viewer_action action, bool enabled);

#endif
