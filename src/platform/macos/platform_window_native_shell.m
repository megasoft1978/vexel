#import <Cocoa/Cocoa.h>

#include "platform/platform_window.h"
#include "platform_window_internal.h"

#include <string.h>

bool platform_window_recent_add(platform_window *window, const char *path, size_t path_len)
{
    if (![NSThread isMainThread] || window == NULL) return false;
    return platform_recents_add(path, path_len);
}

void platform_window_recent_open_failed(platform_window *window, const char *path,
                                        size_t path_len)
{
    if (![NSThread isMainThread] || window == NULL) return;
    platform_macos_welcome_open_failure(window, path, path_len);
}

void platform_window_update_command_state(platform_window *window,
                                          const platform_command_state *state)
{
    if (![NSThread isMainThread] || window == NULL || state == NULL) return;
    platform_macos_menus_update_state(window, state);
}

bool platform_window_show_welcome(platform_window *window)
{
    if (![NSThread isMainThread] || window == NULL) return false;
    platform_macos_welcome_show(window);
    return true;
}

void platform_window_hide_welcome(platform_window *window)
{
    if (![NSThread isMainThread] || window == NULL) return;
    platform_macos_welcome_hide();
}

size_t platform_window_recent_files(platform_window *window,
    platform_recent_file *out_files, size_t capacity)
{
    if (![NSThread isMainThread] || window == NULL || out_files == NULL || capacity == 0U)
        return 0U;
    if (capacity > PLATFORM_RECENT_FILES_MAX) capacity = PLATFORM_RECENT_FILES_MAX;
    size_t count = platform_recents_count();
    if (count > capacity) count = capacity;
    size_t copied = 0U;
    for (size_t index = 0U; index < count; ++index) {
        platform_recent_item recent = {0};
        if (!platform_recents_get(index, &recent) ||
            recent.path_length >= sizeof(out_files[index].path) ||
            recent.name_offset > recent.path_length ||
            recent.name_length > recent.path_length - recent.name_offset) continue;
        platform_recent_file *file = &out_files[copied];
        *file = (platform_recent_file){0};
        memcpy(file->path, recent.path, recent.path_length);
        file->path[recent.path_length] = '\0';
        file->path_length = recent.path_length;
        size_t name_length = recent.name_length;
        if (name_length >= sizeof(file->display_name))
            name_length = sizeof(file->display_name) - 1U;
        memcpy(file->display_name, recent.path + recent.name_offset, name_length);
        file->display_name[name_length] = '\0';
        copied++;
    }
    return copied;
}
