#import <Foundation/Foundation.h>

#include "platform_window_internal.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define PLATFORM_STARTUP_FILE_CAP 8U

typedef struct platform_startup_file {
    platform_event_type type;
    char path[PLATFORM_PATH_CAP];
} platform_startup_file;

static platform_startup_file startup_files[PLATFORM_STARTUP_FILE_CAP];
static size_t startup_file_count;

bool platform_path_supported_internal(const char *path, size_t length)
{
    if (path == NULL || length == 0U || length >= PLATFORM_PATH_CAP ||
        memchr(path, '\0', length) != NULL) return false;
    NSString *value = [[NSString alloc] initWithBytes:path length:length
        encoding:NSUTF8StringEncoding];
    NSString *extension = [value.pathExtension lowercaseString];
    return value != nil && ([extension isEqualToString:@"svg"] ||
        [extension isEqualToString:@"vexel"]);
}

static bool startup_append(const char *path, size_t length, platform_event_type type)
{
    if (startup_file_count >= PLATFORM_STARTUP_FILE_CAP ||
        !platform_path_supported_internal(path, length)) return false;
    platform_startup_file *file = &startup_files[startup_file_count++];
    file->type = type;
    memcpy(file->path, path, length);
    file->path[length] = '\0';
    return true;
}

bool platform_window_dispatch_file_open(platform_window *window, const char *path,
                                        size_t length)
{
    if (!platform_path_supported_internal(path, length)) return false;
    if (window == NULL) {
        platform_event_type type = startup_file_count == 0U ?
            PLATFORM_EVENT_FILE_OPEN : PLATFORM_EVENT_FILE_NEW;
        return startup_append(path, length, type);
    }
    platform_event value = {.type = PLATFORM_EVENT_FILE_OPEN};
    memcpy(value.path, path, length);
    value.path[length] = '\0';
    return platform_window_emit_internal(window, &value);
}

bool platform_window_dispatch_file_drop(platform_window *window, const char *path,
                                        size_t length)
{
    if (window == NULL || !platform_path_supported_internal(path, length)) return false;
    platform_event value = {.type = PLATFORM_EVENT_FILE_DROP};
    memcpy(value.path, path, length);
    value.path[length] = '\0';
    return platform_window_emit_internal(window, &value);
}

bool platform_window_dispatch_new_file(platform_window *window, const char *path,
                                       size_t length)
{
    if (!platform_path_supported_internal(path, length)) return false;
    if (window == NULL) return startup_append(path, length, PLATFORM_EVENT_FILE_NEW);
    platform_event value = {.type = PLATFORM_EVENT_FILE_NEW};
    memcpy(value.path, path, length);
    value.path[length] = '\0';
    return platform_window_emit_internal(window, &value);
}

bool platform_window_dispatch_startup_files(platform_window *window)
{
    if (window == NULL) return false;
    for (size_t index = 0U; index < startup_file_count; ++index) {
        platform_event value = {.type = startup_files[index].type};
        memcpy(value.path, startup_files[index].path,
            sizeof(startup_files[index].path));
        if (!platform_window_emit_internal(window, &value)) return false;
    }
    memset(startup_files, 0, sizeof(startup_files));
    startup_file_count = 0U;
    return true;
}

bool platform_queue_test_open_from_environment(void)
{
    const char *path = getenv("VEXEL_TEST_OPEN");
    if (path == NULL || path[0] == '\0') return true;
    size_t length = strnlen(path, PLATFORM_PATH_CAP);
    if (length == 0U || length >= PLATFORM_PATH_CAP ||
        !platform_path_supported_internal(path, length)) {
        (void)fprintf(stderr,
            "platform: VEXEL_TEST_OPEN must be a UTF-8 .svg or .vexel path shorter than %u bytes\n",
            PLATFORM_PATH_CAP);
        return false;
    }
    if (!platform_window_dispatch_file_open(NULL, path, length)) {
        (void)fputs("platform: could not queue VEXEL_TEST_OPEN file event\n", stderr);
        return false;
    }
    return true;
}
