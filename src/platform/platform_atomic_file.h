#ifndef VEXEL_PLATFORM_ATOMIC_FILE_H
#define VEXEL_PLATFORM_ATOMIC_FILE_H

#include "base.h"

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <sys/types.h>

#define PLATFORM_ATOMIC_FILE_MAX_PATH_BYTES 1023u

typedef enum platform_atomic_file_error {
    PLATFORM_ATOMIC_FILE_OK = 0,
    PLATFORM_ATOMIC_FILE_INVALID,
    PLATFORM_ATOMIC_FILE_PATH_LIMIT,
    PLATFORM_ATOMIC_FILE_NOT_FOUND,
    PLATFORM_ATOMIC_FILE_NOT_REGULAR,
    PLATFORM_ATOMIC_FILE_IO,
    PLATFORM_ATOMIC_FILE_DURABILITY
} platform_atomic_file_error;

typedef ssize_t (*platform_atomic_file_write_fn)(int fd, const void *bytes,
                                                  size_t length, void *context);
typedef int (*platform_atomic_file_rename_fn)(const char *from, const char *to,
                                               void *context);

typedef struct platform_atomic_file_hooks {
    platform_atomic_file_write_fn write;
    platform_atomic_file_rename_fn rename;
    void *context;
} platform_atomic_file_hooks;

typedef struct platform_atomic_file_result {
    platform_atomic_file_error error;
    bool file_fsynced;
    bool renamed;
    bool directory_fsynced;
} platform_atomic_file_result;

/* Hooks may replace write and/or rename for deterministic tests. A successful
 * rename is the commit point; a later directory-fsync error is post-commit. */
platform_atomic_file_result platform_atomic_file_replace(
    base_bytes path, base_bytes bytes, const platform_atomic_file_hooks *hooks);
const char *platform_atomic_file_error_name(platform_atomic_file_error error);

#endif
