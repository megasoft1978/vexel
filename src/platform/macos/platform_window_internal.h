#ifndef VEXEL_PLATFORM_MACOS_WINDOW_INTERNAL_H
#define VEXEL_PLATFORM_MACOS_WINDOW_INTERNAL_H

#include "platform/platform_window.h"
#include "platform/platform_recents.h"

bool platform_window_dispatch_file_open(platform_window *window, const char *path, size_t length);
void platform_window_dispatch_action(platform_window *window, platform_viewer_action action);
void platform_macos_menus_init(void);
void platform_macos_menus_set_owner(platform_window *window);
void platform_macos_menus_clear_owner(platform_window *window);
bool platform_macos_menus_set_option_state(platform_window *window,
                                           platform_viewer_action action, bool enabled);
void platform_macos_menus_refresh_recent(void);
void platform_macos_menus_update_state(platform_window *window,
                                       const platform_command_state *state);
void platform_macos_menus_set_full_screen(bool is_full_screen);
void platform_macos_welcome_show(platform_window *owner);
void platform_macos_welcome_hide(void);
void platform_macos_welcome_set_owner(platform_window *owner);
void platform_macos_welcome_clear_owner(platform_window *owner);
void platform_macos_welcome_refresh(void);
void platform_macos_welcome_open_failure(platform_window *owner,
                                         const char *path, size_t path_length);

#endif
