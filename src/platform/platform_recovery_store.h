#ifndef VEXEL_PLATFORM_PLATFORM_RECOVERY_STORE_H
#define VEXEL_PLATFORM_PLATFORM_RECOVERY_STORE_H

#include "base.h"

#define PLATFORM_RECOVERY_STORE_PATH_MAX 4096U

typedef enum platform_recovery_store_error {
    PLATFORM_RECOVERY_STORE_OK = 0,
    PLATFORM_RECOVERY_STORE_INVALID_ARGUMENT,
    PLATFORM_RECOVERY_STORE_SOURCE_NOT_FOUND,
    PLATFORM_RECOVERY_STORE_SOURCE_NOT_REGULAR,
    PLATFORM_RECOVERY_STORE_DIRECTORY_UNSAFE,
    PLATFORM_RECOVERY_STORE_PATH_LIMIT,
    PLATFORM_RECOVERY_STORE_IO
} platform_recovery_store_error;

typedef struct platform_recovery_store_result {
    platform_recovery_store_error error;
    size_t path_length;
} platform_recovery_store_result;

/* Resolves the canonical, existing source path to its private recovery
 * capsule path. An empty root selects the current user's Application Support
 * directory; a non-empty root overrides it for isolated tests. `out_path`
 * receives a NUL-terminated absolute path and must remain caller-owned. */
platform_recovery_store_result platform_recovery_store_path(
    base_bytes application_support_root, base_bytes source_path,
    base_mut_bytes out_path);
const char *platform_recovery_store_error_name(platform_recovery_store_error error);

#endif
