#ifndef VEXEL_PLATFORM_RECENTS_H
#define VEXEL_PLATFORM_RECENTS_H

#include "app/welcome_model.h"

#include <stdbool.h>
#include <stddef.h>

typedef struct platform_recent_item {
    char path[WELCOME_RECENT_PATH_CAP];
    size_t path_length;
    size_t name_offset;
    size_t name_length;
    size_t parent_length;
    bool available;
} platform_recent_item;

void platform_recents_init(void);
size_t platform_recents_count(void);
bool platform_recents_get(size_t index, platform_recent_item *out_item);
bool platform_recents_contains(const char *path, size_t path_length);
bool platform_recents_add(const char *path, size_t path_length);
bool platform_recents_remove(const char *path, size_t path_length);
void platform_recents_clear(void);

#endif
