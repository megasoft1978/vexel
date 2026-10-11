#import <Cocoa/Cocoa.h>

#include "platform/platform_recents.h"
#include "platform_window_internal.h"

#include <string.h>

#define PLATFORM_RECENT_SCAN_LIMIT 64U

static welcome_recent_list platform_recent_files;
static bool platform_recents_initialized;

static size_t platform_bounded_length(const char *text, size_t limit)
{
    size_t length = 0U;
    if (text == NULL) return 0U;
    while (length <= limit && text[length] != '\0') length++;
    return length;
}

static bool platform_path_from_string(NSString *value,
    char path[WELCOME_RECENT_PATH_CAP], size_t *out_length)
{
    if (value == nil || path == NULL || out_length == NULL) return false;
    NSString *normalized = value.stringByStandardizingPath;
    const char *bytes = normalized.fileSystemRepresentation;
    size_t length = platform_bounded_length(bytes, WELCOME_RECENT_PATH_CAP - 1U);
    if (!welcome_recent_path_supported(bytes, length)) return false;
    memcpy(path, bytes, length);
    path[length] = '\0';
    *out_length = length;
    return true;
}

static void platform_recents_persist(void)
{
    NSMutableArray<NSString *> *paths = [NSMutableArray arrayWithCapacity:platform_recent_files.count];
    for (size_t index = 0U; index < platform_recent_files.count; ++index) {
        const welcome_recent_entry *entry = &platform_recent_files.entries[index];
        NSString *path = [[NSString alloc] initWithBytes:entry->path
            length:entry->path_length encoding:NSUTF8StringEncoding];
        if (path != nil) [paths addObject:path];
    }
    [[NSUserDefaults standardUserDefaults] setObject:paths forKey:@"VexelRecentFiles"];
    platform_macos_menus_refresh_recent();
    platform_macos_welcome_refresh();
}

void platform_recents_init(void)
{
    if (platform_recents_initialized || ![NSThread isMainThread]) return;
    platform_recent_files = (welcome_recent_list){0};
    id saved = [[NSUserDefaults standardUserDefaults] objectForKey:@"VexelRecentFiles"];
    if ([saved isKindOfClass:[NSArray class]]) {
        NSArray *paths = (NSArray *)saved;
        NSUInteger count = paths.count < PLATFORM_RECENT_SCAN_LIMIT ?
            paths.count : PLATFORM_RECENT_SCAN_LIMIT;
        for (NSUInteger reverse = count; reverse > 0U; --reverse) {
            id item = paths[reverse - 1U];
            if (![item isKindOfClass:[NSString class]]) continue;
            char path[WELCOME_RECENT_PATH_CAP];
            size_t length = 0U;
            if (platform_path_from_string((NSString *)item, path, &length))
                (void)welcome_recent_add(&platform_recent_files, path, length);
        }
    }
    platform_recents_initialized = true;
    platform_recents_persist();
}

size_t platform_recents_count(void)
{
    if (!platform_recents_initialized) platform_recents_init();
    return platform_recents_initialized ? platform_recent_files.count : 0U;
}

bool platform_recents_get(size_t index, platform_recent_item *out_item)
{
    if (out_item == NULL || ![NSThread isMainThread]) return false;
    if (!platform_recents_initialized) platform_recents_init();
    welcome_recent_view view;
    if (!platform_recents_initialized ||
        !welcome_recent_get(&platform_recent_files, index, &view)) return false;
    *out_item = (platform_recent_item){0};
    memcpy(out_item->path, view.path, view.path_length);
    out_item->path[view.path_length] = '\0';
    out_item->path_length = view.path_length;
    out_item->name_offset = (size_t)(view.name - view.path);
    out_item->name_length = view.name_length;
    out_item->parent_length = view.parent_length;
    NSString *path_text = [[NSString alloc] initWithBytes:view.path length:view.path_length
        encoding:NSUTF8StringEncoding];
    BOOL directory = NO;
    BOOL exists = path_text != nil && [[NSFileManager defaultManager]
        fileExistsAtPath:path_text isDirectory:&directory];
    NSDictionary<NSFileAttributeKey, id> *attributes = exists && !directory ?
        [[NSFileManager defaultManager] attributesOfItemAtPath:path_text error:nil] : nil;
    out_item->available = exists && !directory &&
        [attributes[NSFileType] isEqualToString:NSFileTypeRegular] &&
        [[NSFileManager defaultManager] isReadableFileAtPath:path_text];
    return true;
}

bool platform_recents_contains(const char *path, size_t path_length)
{
    if (!platform_recents_initialized) platform_recents_init();
    if (!welcome_recent_path_supported(path, path_length)) return false;
    for (size_t index = 0U; index < platform_recent_files.count; ++index) {
        const welcome_recent_entry *entry = &platform_recent_files.entries[index];
        if (entry->path_length == path_length &&
            memcmp(entry->path, path, path_length) == 0) return true;
    }
    return false;
}

bool platform_recents_add(const char *path, size_t path_length)
{
    if (![NSThread isMainThread] || path == NULL || path_length == 0U ||
        path_length >= WELCOME_RECENT_PATH_CAP || memchr(path, '\0', path_length) != NULL)
        return false;
    if (!platform_recents_initialized) platform_recents_init();
    NSString *value = [[NSString alloc] initWithBytes:path length:path_length
        encoding:NSUTF8StringEncoding];
    char normalized[WELCOME_RECENT_PATH_CAP];
    size_t normalized_length = 0U;
    if (!platform_path_from_string(value, normalized, &normalized_length) ||
        !welcome_recent_add(&platform_recent_files, normalized, normalized_length)) return false;
    platform_recents_persist();
    return true;
}

bool platform_recents_remove(const char *path, size_t path_length)
{
    if (![NSThread isMainThread] || path == NULL || path_length == 0U ||
        path_length >= WELCOME_RECENT_PATH_CAP || memchr(path, '\0', path_length) != NULL)
        return false;
    if (!platform_recents_initialized) platform_recents_init();
    NSString *value = [[NSString alloc] initWithBytes:path length:path_length
        encoding:NSUTF8StringEncoding];
    char normalized[WELCOME_RECENT_PATH_CAP];
    size_t normalized_length = 0U;
    if (!platform_path_from_string(value, normalized, &normalized_length) ||
        !welcome_recent_remove(&platform_recent_files, normalized, normalized_length)) return false;
    platform_recents_persist();
    return true;
}

void platform_recents_clear(void)
{
    if (![NSThread isMainThread]) return;
    if (!platform_recents_initialized) platform_recents_init();
    welcome_recent_clear(&platform_recent_files);
    platform_recents_persist();
}
