#include "welcome_model.h"

#include <ctype.h>
#include <string.h>

bool welcome_recent_path_supported(const char *path, size_t path_length)
{
    if (path == NULL || path_length == 0U ||
        path_length >= WELCOME_RECENT_PATH_CAP ||
        memchr(path, '\0', path_length) != NULL || path_length < 4U) return false;
    size_t suffix = path_length - 4U;
    return path[suffix] == '.' &&
        tolower((unsigned char)path[suffix + 1U]) == 's' &&
        tolower((unsigned char)path[suffix + 2U]) == 'v' &&
        tolower((unsigned char)path[suffix + 3U]) == 'g';
}

static bool welcome_recent_list_valid(const welcome_recent_list *list)
{
    return list != NULL && list->count <= WELCOME_RECENT_LIMIT;
}

bool welcome_recent_add(welcome_recent_list *list, const char *path,
                        size_t path_length)
{
    if (!welcome_recent_list_valid(list) ||
        !welcome_recent_path_supported(path, path_length)) return false;
    size_t found = list->count;
    for (size_t index = 0U; index < list->count; ++index) {
        const welcome_recent_entry *entry = &list->entries[index];
        if (entry->path_length == path_length &&
            memcmp(entry->path, path, path_length) == 0) {
            found = index;
            break;
        }
    }
    welcome_recent_entry value = {0};
    memcpy(value.path, path, path_length);
    value.path[path_length] = '\0';
    value.path_length = path_length;
    if (found < list->count) {
        if (found > 0U) {
            memmove(&list->entries[1], &list->entries[0],
                found * sizeof(list->entries[0]));
        }
    } else {
        size_t retained = list->count;
        if (retained == WELCOME_RECENT_LIMIT) retained--;
        else list->count++;
        if (retained > 0U) {
            memmove(&list->entries[1], &list->entries[0],
                retained * sizeof(list->entries[0]));
        }
    }
    list->entries[0] = value;
    return true;
}

bool welcome_recent_remove(welcome_recent_list *list, const char *path,
                           size_t path_length)
{
    if (!welcome_recent_list_valid(list) || path == NULL || path_length == 0U ||
        path_length >= WELCOME_RECENT_PATH_CAP || memchr(path, '\0', path_length) != NULL)
        return false;
    for (size_t index = 0U; index < list->count; ++index) {
        welcome_recent_entry *entry = &list->entries[index];
        if (entry->path_length != path_length ||
            memcmp(entry->path, path, path_length) != 0) continue;
        size_t following = list->count - index - 1U;
        if (following > 0U) {
            memmove(entry, entry + 1U, following * sizeof(*entry));
        }
        list->count--;
        memset(&list->entries[list->count], 0, sizeof(list->entries[0]));
        return true;
    }
    return false;
}

void welcome_recent_clear(welcome_recent_list *list)
{
    if (list != NULL) memset(list, 0, sizeof(*list));
}

bool welcome_recent_get(const welcome_recent_list *list, size_t index,
                        welcome_recent_view *out_view)
{
    if (!welcome_recent_list_valid(list) || out_view == NULL || index >= list->count)
        return false;
    const welcome_recent_entry *entry = &list->entries[index];
    size_t name_start = 0U;
    size_t parent_length = 0U;
    bool has_parent = false;
    for (size_t cursor = 0U; cursor < entry->path_length; ++cursor) {
        if (entry->path[cursor] != '/') continue;
        name_start = cursor + 1U;
        parent_length = cursor == 0U ? 1U : cursor;
        has_parent = true;
    }
    *out_view = (welcome_recent_view){
        .path = entry->path,
        .path_length = entry->path_length,
        .name = entry->path + name_start,
        .name_length = entry->path_length - name_start,
        .parent = has_parent ? entry->path : "",
        .parent_length = has_parent ? parent_length : 0U
    };
    return true;
}
