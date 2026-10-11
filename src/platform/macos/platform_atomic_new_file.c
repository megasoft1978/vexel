#include "platform_atomic_new_file.h"

#include <errno.h>
#include <fcntl.h>
#include <limits.h>
#include <stdbool.h>
#include <stdint.h>
#include <string.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <unistd.h>

#ifndef PATH_MAX
#define PATH_MAX 1024
#endif

#define NEW_FILE_TEMP_NAME ".vexel-new-XXXXXX"

static platform_atomic_new_file_result result_for(
    platform_atomic_new_file_error error)
{
    platform_atomic_new_file_result result = {0};
    result.error = error;
    return result;
}

static bool path_string(base_bytes path, char out[PATH_MAX])
{
    if (!base_bytes_valid(path) || path.len == 0U || path.len >= PATH_MAX ||
        path.len > PLATFORM_ATOMIC_FILE_MAX_PATH_BYTES ||
        memchr(path.ptr, '\0', path.len) != NULL) return false;
    memcpy(out, path.ptr, path.len);
    out[path.len] = '\0';
    return true;
}

static bool paths_for_target(const char *target, size_t target_length,
                             char directory[PATH_MAX], char temporary[PATH_MAX])
{
    size_t slash = target_length;
    while (slash > 0U && target[slash - 1U] != '/') --slash;
    size_t name_offset = slash;
    if (name_offset == target_length) return false;
    size_t name_length = target_length - name_offset;
    if (name_length == 0U ||
        (name_length == 1U && target[name_offset] == '.') ||
        (name_length == 2U && target[name_offset] == '.' &&
         target[name_offset + 1U] == '.')) return false;

    if (slash == 0U) {
        memcpy(directory, ".", 2U);
    } else {
        size_t directory_length = slash == 1U ? 1U : slash - 1U;
        memcpy(directory, target, directory_length);
        directory[directory_length] = '\0';
    }
    size_t directory_length = strlen(directory);
    size_t separator = directory_length > 0U &&
        directory[directory_length - 1U] == '/' ? 0U : 1U;
    size_t template_length;
    if (!base_size_add(directory_length, separator, &template_length) ||
        !base_size_add(template_length, sizeof(NEW_FILE_TEMP_NAME), &template_length) ||
        template_length > PATH_MAX) return false;
    memcpy(temporary, directory, directory_length);
    size_t cursor = directory_length;
    if (separator != 0U) temporary[cursor++] = '/';
    memcpy(temporary + cursor, NEW_FILE_TEMP_NAME, sizeof(NEW_FILE_TEMP_NAME));
    return true;
}

static ssize_t native_write(int descriptor, const void *bytes, size_t length,
                            void *context)
{
    (void)context;
    return write(descriptor, bytes, length);
}

static int native_publish(const char *temporary, const char *target, void *context)
{
    (void)context;
    return link(temporary, target);
}

static bool write_all(int descriptor, base_bytes bytes,
                      const platform_atomic_new_file_hooks *hooks)
{
    platform_atomic_file_write_fn write_fn = hooks != NULL && hooks->write != NULL ?
        hooks->write : native_write;
    size_t offset = 0U;
    while (offset < bytes.len) {
        size_t remaining = bytes.len - offset;
        ssize_t amount = write_fn(descriptor, bytes.ptr + offset, remaining,
            hooks == NULL ? NULL : hooks->context);
        if (amount < 0 && errno == EINTR) continue;
        if (amount <= 0 || (size_t)amount > remaining) return false;
        offset += (size_t)amount;
    }
    return true;
}

static bool close_checked(int *descriptor)
{
    int value = *descriptor;
    *descriptor = -1;
    return close(value) == 0;
}

platform_atomic_new_file_result platform_atomic_file_create_new(
    base_bytes path, base_bytes bytes,
    const platform_atomic_new_file_hooks *hooks)
{
    char target[PATH_MAX];
    char directory[PATH_MAX];
    char temporary[PATH_MAX];
    if (!base_bytes_valid(bytes) || !base_bytes_valid(path) || path.len == 0U)
        return result_for(PLATFORM_ATOMIC_NEW_FILE_INVALID);
    if (path.len > PLATFORM_ATOMIC_FILE_MAX_PATH_BYTES || path.len >= PATH_MAX)
        return result_for(PLATFORM_ATOMIC_NEW_FILE_PATH_LIMIT);
    if (!path_string(path, target))
        return result_for(PLATFORM_ATOMIC_NEW_FILE_INVALID);
    if (!paths_for_target(target, path.len, directory, temporary))
        return result_for(path.len > PLATFORM_ATOMIC_FILE_MAX_PATH_BYTES ?
            PLATFORM_ATOMIC_NEW_FILE_PATH_LIMIT : PLATFORM_ATOMIC_NEW_FILE_INVALID);

    struct stat existing;
    if (lstat(target, &existing) == 0)
        return result_for(PLATFORM_ATOMIC_NEW_FILE_EXISTS);
    if (errno != ENOENT) return result_for(PLATFORM_ATOMIC_NEW_FILE_IO);

    int descriptor = mkstemp(temporary);
    if (descriptor < 0) return result_for(PLATFORM_ATOMIC_NEW_FILE_IO);
    bool keep_temporary = true;
    platform_atomic_new_file_result result =
        result_for(PLATFORM_ATOMIC_NEW_FILE_IO);
    if (!write_all(descriptor, bytes, hooks) || fsync(descriptor) != 0)
        goto cleanup;
    result.file_fsynced = true;
    if (!close_checked(&descriptor)) goto cleanup;

    platform_atomic_new_file_publish_fn publish_fn =
        hooks != NULL && hooks->publish != NULL ? hooks->publish : native_publish;
    if (publish_fn(temporary, target,
            hooks == NULL ? NULL : hooks->context) != 0) {
        if (errno == EEXIST) result.error = PLATFORM_ATOMIC_NEW_FILE_EXISTS;
        goto cleanup;
    }
    result.published = true;
    keep_temporary = false;
    if (unlink(temporary) != 0) {
        result.error = PLATFORM_ATOMIC_NEW_FILE_IO;
        keep_temporary = true;
        goto cleanup;
    }
    result.error = PLATFORM_ATOMIC_NEW_FILE_OK;

    int directory_fd = open(directory, O_RDONLY | O_CLOEXEC);
    if (directory_fd < 0 || fsync(directory_fd) != 0) {
        result.error = PLATFORM_ATOMIC_NEW_FILE_DURABILITY;
        if (directory_fd >= 0) (void)close(directory_fd);
        return result;
    }
    result.directory_fsynced = true;
    (void)close(directory_fd);
    return result;

cleanup:
    if (descriptor >= 0) (void)close(descriptor);
    if (keep_temporary) (void)unlink(temporary);
    return result;
}
