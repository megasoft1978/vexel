#define _POSIX_C_SOURCE 200809L
#define _DARWIN_C_SOURCE

#include "mcp_broker_internal.h"

#include <ctype.h>
#include <errno.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>

static bool copy_canonical_directory(const char *input, char out[MCP_BROKER_PATH_CAP])
{
    if (input == NULL || input[0] != '/') return false;
    char *resolved = realpath(input, NULL);
    if (resolved == NULL) return false;
    struct stat status;
    size_t length = strlen(resolved);
    bool okay = length < MCP_BROKER_PATH_CAP && stat(resolved, &status) == 0 &&
        S_ISDIR(status.st_mode);
    if (okay) memcpy(out, resolved, length + 1U);
    free(resolved);
    return okay;
}

bool mcp_access_resolve_existing(const char *path, char out[MCP_BROKER_PATH_CAP])
{
    if (path == NULL || out == NULL || path[0] == '\0') return false;
    size_t input_length = strnlen(path, MCP_BROKER_PATH_CAP);
    if (input_length == 0U || input_length >= MCP_BROKER_PATH_CAP) return false;
    char *resolved = realpath(path, NULL);
    if (resolved == NULL) return false;
    size_t length = strlen(resolved);
    if (length >= MCP_BROKER_PATH_CAP) { free(resolved); return false; }
    struct stat status;
    bool okay = stat(resolved, &status) == 0 && S_ISREG(status.st_mode);
    if (okay) memcpy(out, resolved, length + 1U);
    free(resolved);
    return okay;
}

static bool path_is_inside(const char *directory, const char *path)
{
    size_t length = strlen(directory);
    if (strncmp(directory, path, length) != 0) return false;
    if (length == 1U && directory[0] == '/') return path[0] == '/';
    return path[length] == '\0' || path[length] == '/';
}

static bool path_is_granted_by_env(const char *path)
{
    const char *value = getenv("VEXEL_MCP_ALLOWED_DIRS");
    if (value == NULL) return false;
    size_t length = strnlen(value, 8192U);
    if (length == 0U || length >= 8192U) return false;
    size_t start = 0U;
    while (start < length) {
        size_t end = start;
        while (end < length && value[end] != ':') end++;
        size_t item_length = end - start;
        if (item_length > 0U && item_length < MCP_BROKER_PATH_CAP) {
            char item[MCP_BROKER_PATH_CAP];
            char canonical[MCP_BROKER_PATH_CAP];
            memcpy(item, value + start, item_length);
            item[item_length] = '\0';
            if (copy_canonical_directory(item, canonical) && path_is_inside(canonical, path)) return true;
        }
        start = end + 1U;
    }
    return false;
}

static bool path_is_granted_by_config(const char *path)
{
    const char *home = getenv("HOME");
    if (home == NULL || home[0] != '/') return false;
    size_t home_length = strnlen(home, MCP_BROKER_PATH_CAP);
    const char suffix[] = "/.config/vexel/mcp-allowed-dirs.txt";
    if (home_length == 0U || home_length + sizeof(suffix) > MCP_BROKER_PATH_CAP) return false;
    char config[MCP_BROKER_PATH_CAP];
    memcpy(config, home, home_length);
    memcpy(config + home_length, suffix, sizeof(suffix));
    struct stat status;
    if (lstat(config, &status) != 0 || !S_ISREG(status.st_mode) || status.st_uid != getuid() ||
        (status.st_mode & (S_IWGRP | S_IWOTH)) != 0) return false;
    FILE *file = fopen(config, "r");
    if (file == NULL) return false;
    bool granted = false;
    char line[MCP_BROKER_PATH_CAP];
    size_t lines = 0U;
    while (lines < 128U && fgets(line, sizeof(line), file) != NULL) {
        lines++;
        size_t length = strnlen(line, sizeof(line));
        if (length == sizeof(line) || (length > 0U && line[length - 1U] != '\n' && !feof(file))) continue;
        while (length > 0U && (line[length - 1U] == '\n' || line[length - 1U] == '\r' ||
               isspace((unsigned char)line[length - 1U]))) line[--length] = '\0';
        size_t first = 0U;
        while (first < length && isspace((unsigned char)line[first])) first++;
        if (first == length || line[first] == '#') continue;
        char canonical[MCP_BROKER_PATH_CAP];
        if (copy_canonical_directory(line + first, canonical) && path_is_inside(canonical, path)) {
            granted = true;
            break;
        }
    }
    (void)fclose(file);
    return granted;
}

bool mcp_access_path_allowed(const char *path)
{
    char canonical[MCP_BROKER_PATH_CAP];
    if (!mcp_access_resolve_existing(path, canonical)) return false;
    return path_is_granted_by_env(canonical) || path_is_granted_by_config(canonical);
}

bool mcp_access_resolve_save_path(const char *path,
                                  char out[MCP_BROKER_PATH_CAP])
{
    if (path == NULL || out == NULL || path[0] == '\0') return false;
    size_t length = strnlen(path, MCP_BROKER_PATH_CAP);
    if (length == 0U || length >= MCP_BROKER_PATH_CAP) return false;

    struct stat status;
    if (lstat(path, &status) == 0) {
        char canonical[MCP_BROKER_PATH_CAP];
        if (!mcp_access_resolve_existing(path, canonical) ||
            (!path_is_granted_by_env(canonical) &&
             !path_is_granted_by_config(canonical))) return false;
        memcpy(out, canonical, strlen(canonical) + 1U);
        return true;
    }
    if (errno != ENOENT) return false;

    const char *slash = strrchr(path, '/');
    const char *name = slash == NULL ? path : slash + 1;
    size_t name_length = length - (size_t)(name - path);
    if (name_length == 0U ||
        (name_length == 1U && name[0] == '.') ||
        (name_length == 2U && name[0] == '.' && name[1] == '.')) return false;
    char parent[MCP_BROKER_PATH_CAP];
    if (slash == NULL) {
        memcpy(parent, ".", 2U);
    } else if (slash == path) {
        memcpy(parent, "/", 2U);
    } else {
        size_t parent_length = (size_t)(slash - path);
        if (parent_length >= sizeof(parent)) return false;
        memcpy(parent, path, parent_length);
        parent[parent_length] = '\0';
    }
    char *canonical_parent = realpath(parent, NULL);
    if (canonical_parent == NULL) return false;
    struct stat parent_status;
    size_t canonical_length = strlen(canonical_parent);
    size_t separator = canonical_length > 0U &&
        canonical_parent[canonical_length - 1U] == '/' ? 0U : 1U;
    size_t output_length;
    bool okay = stat(canonical_parent, &parent_status) == 0 &&
        S_ISDIR(parent_status.st_mode) &&
        base_size_add(canonical_length, separator, &output_length) &&
        base_size_add(output_length, name_length, &output_length) &&
        output_length < MCP_BROKER_PATH_CAP;
    if (okay) {
        memcpy(out, canonical_parent, canonical_length);
        size_t cursor = canonical_length;
        if (separator != 0U) out[cursor++] = '/';
        memcpy(out + cursor, name, name_length);
        out[output_length] = '\0';
        okay = path_is_granted_by_env(out) || path_is_granted_by_config(out);
    }
    free(canonical_parent);
    return okay;
}
