#ifndef VEXEL_APP_WELCOME_MODEL_H
#define VEXEL_APP_WELCOME_MODEL_H

#include <stdbool.h>
#include <stddef.h>

#define WELCOME_RECENT_LIMIT 8U
#define WELCOME_RECENT_PATH_CAP 4096U

typedef struct welcome_recent_entry {
    char path[WELCOME_RECENT_PATH_CAP];
    size_t path_length;
} welcome_recent_entry;

typedef struct welcome_recent_list {
    welcome_recent_entry entries[WELCOME_RECENT_LIMIT];
    size_t count;
} welcome_recent_list;

typedef struct welcome_recent_view {
    const char *path;
    size_t path_length;
    const char *name;
    size_t name_length;
    const char *parent;
    size_t parent_length;
} welcome_recent_view;

bool welcome_recent_path_supported(const char *path, size_t path_length);
bool welcome_recent_add(welcome_recent_list *list, const char *path,
                        size_t path_length);
bool welcome_recent_remove(welcome_recent_list *list, const char *path,
                           size_t path_length);
void welcome_recent_clear(welcome_recent_list *list);
bool welcome_recent_get(const welcome_recent_list *list, size_t index,
                        welcome_recent_view *out_view);

#endif
