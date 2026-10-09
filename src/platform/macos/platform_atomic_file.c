#include "platform_atomic_file.h"

#include <errno.h>
#include <fcntl.h>
#include <limits.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <unistd.h>

#ifndef PATH_MAX
#define PATH_MAX 1024
#endif

#define TEMP_SUFFIX "/.vexel-save-XXXXXX"

static platform_atomic_file_result result_for(platform_atomic_file_error error)
{
    platform_atomic_file_result result = {0};
    result.error = error;
    return result;
}

static bool path_string(base_bytes path, char out[PATH_MAX])
{
    if (!base_bytes_valid(path) || path.len == 0) return false;
    if (path.len >= PATH_MAX || memchr(path.ptr, '\0', path.len) != NULL) return false;
    memcpy(out, path.ptr, path.len);
    out[path.len] = '\0';
    return true;
}

static bool parent_path(const char *path, size_t length, char out[PATH_MAX])
{
    size_t slash = length;
    while (slash > 0 && path[slash - 1] != '/') slash--;
    if (slash == 0) {
        memcpy(out, ".", 2);
        return true;
    }
    size_t parent_length = slash == 1 ? 1 : slash - 1;
    if (parent_length + sizeof(TEMP_SUFFIX) > PATH_MAX) return false;
    memcpy(out, path, parent_length);
    out[parent_length] = '\0';
    return true;
}

static ssize_t native_write(int fd, const void *bytes, size_t length, void *context)
{
    (void)context;
    return write(fd, bytes, length);
}

static int native_rename(const char *from, const char *to, void *context)
{
    (void)context;
    return rename(from, to);
}

static bool write_all(int fd, base_bytes bytes, const platform_atomic_file_hooks *hooks)
{
    platform_atomic_file_write_fn write_fn = hooks != NULL && hooks->write != NULL
        ? hooks->write : native_write;
    size_t offset = 0;
    while (offset < bytes.len) {
        size_t remaining = bytes.len - offset;
        ssize_t amount = write_fn(fd, bytes.ptr + offset, remaining,
                                  hooks == NULL ? NULL : hooks->context);
        if (amount < 0 && errno == EINTR) continue;
        if (amount <= 0 || (size_t)amount > remaining) return false;
        offset += (size_t)amount;
    }
    return true;
}

static bool close_checked(int *fd)
{
    int value = *fd;
    *fd = -1;
    return close(value) == 0;
}

platform_atomic_file_result platform_atomic_file_replace(
    base_bytes path, base_bytes bytes, const platform_atomic_file_hooks *hooks)
{
    char target[PATH_MAX];
    char directory[PATH_MAX];
    char temporary[PATH_MAX];
    if (!base_bytes_valid(bytes) || !base_bytes_valid(path) || path.len == 0)
        return result_for(PLATFORM_ATOMIC_FILE_INVALID);
    if (path.len > PLATFORM_ATOMIC_FILE_MAX_PATH_BYTES || path.len >= PATH_MAX)
        return result_for(PLATFORM_ATOMIC_FILE_PATH_LIMIT);
    if (!path_string(path, target)) return result_for(PLATFORM_ATOMIC_FILE_INVALID);
    size_t target_length = path.len;
    size_t suffix_length = sizeof(TEMP_SUFFIX) - 1;
    struct stat initial;
    if (lstat(target, &initial) != 0)
        return result_for(errno == ENOENT ? PLATFORM_ATOMIC_FILE_NOT_FOUND : PLATFORM_ATOMIC_FILE_IO);
    if (!S_ISREG(initial.st_mode)) return result_for(PLATFORM_ATOMIC_FILE_NOT_REGULAR);
    int original_fd = open(target, O_RDONLY | O_CLOEXEC | O_NOFOLLOW);
    if (original_fd < 0) return result_for(PLATFORM_ATOMIC_FILE_IO);
    struct stat opened;
    if (fstat(original_fd, &opened) != 0 || !S_ISREG(opened.st_mode) ||
        opened.st_dev != initial.st_dev || opened.st_ino != initial.st_ino) {
        (void)close_checked(&original_fd);
        return result_for(PLATFORM_ATOMIC_FILE_IO);
    }
    int saved_mode = opened.st_mode & 07777;
    if (!close_checked(&original_fd)) return result_for(PLATFORM_ATOMIC_FILE_IO);
    if (!parent_path(target, target_length, directory))
        return result_for(PLATFORM_ATOMIC_FILE_PATH_LIMIT);
    size_t directory_length = strlen(directory);
    size_t template_length;
    if (!base_size_add(directory_length, suffix_length, &template_length) ||
        template_length >= sizeof(temporary))
        return result_for(PLATFORM_ATOMIC_FILE_PATH_LIMIT);
    memcpy(temporary, directory, directory_length);
    memcpy(temporary + directory_length, TEMP_SUFFIX, suffix_length + 1);

    int temp_fd = mkstemp(temporary);
    if (temp_fd < 0) return result_for(PLATFORM_ATOMIC_FILE_IO);
    bool keep_temp = true;
    platform_atomic_file_result result = result_for(PLATFORM_ATOMIC_FILE_IO);
    if (!write_all(temp_fd, bytes, hooks)) goto cleanup;
    if (fchmod(temp_fd, (mode_t)saved_mode) != 0) goto cleanup;
    if (fsync(temp_fd) != 0) goto cleanup;
    result.file_fsynced = true;
    if (!close_checked(&temp_fd)) goto cleanup;

    platform_atomic_file_rename_fn rename_fn = hooks != NULL && hooks->rename != NULL
        ? hooks->rename : native_rename;
    if (rename_fn(temporary, target, hooks == NULL ? NULL : hooks->context) != 0)
        goto cleanup;
    result.renamed = true;
    keep_temp = false;
    result.error = PLATFORM_ATOMIC_FILE_OK;

    int directory_fd = open(directory, O_RDONLY | O_CLOEXEC);
    if (directory_fd < 0 || fsync(directory_fd) != 0) {
        result.error = PLATFORM_ATOMIC_FILE_DURABILITY;
        if (directory_fd >= 0) (void)close(directory_fd);
        return result;
    }
    result.directory_fsynced = true;
    (void)close(directory_fd);
    return result;

cleanup:
    if (temp_fd >= 0) (void)close(temp_fd);
    if (keep_temp) (void)unlink(temporary);
    return result;
}

const char *platform_atomic_file_error_name(platform_atomic_file_error error)
{
    switch (error) {
        case PLATFORM_ATOMIC_FILE_OK: return "ok";
        case PLATFORM_ATOMIC_FILE_INVALID: return "invalid";
        case PLATFORM_ATOMIC_FILE_PATH_LIMIT: return "path_limit";
        case PLATFORM_ATOMIC_FILE_NOT_FOUND: return "not_found";
        case PLATFORM_ATOMIC_FILE_NOT_REGULAR: return "not_regular";
        case PLATFORM_ATOMIC_FILE_IO: return "io";
        case PLATFORM_ATOMIC_FILE_DURABILITY: return "durability";
        default: return "unknown";
    }
}
