#ifndef VEXEL_PLATFORM_ATOMIC_NEW_FILE_H
#define VEXEL_PLATFORM_ATOMIC_NEW_FILE_H

#include "platform_atomic_file.h"

typedef enum platform_atomic_new_file_error {
    PLATFORM_ATOMIC_NEW_FILE_OK = 0,
    PLATFORM_ATOMIC_NEW_FILE_INVALID,
    PLATFORM_ATOMIC_NEW_FILE_PATH_LIMIT,
    PLATFORM_ATOMIC_NEW_FILE_EXISTS,
    PLATFORM_ATOMIC_NEW_FILE_IO,
    PLATFORM_ATOMIC_NEW_FILE_DURABILITY
} platform_atomic_new_file_error;

typedef int (*platform_atomic_new_file_publish_fn)(const char *temporary_path,
    const char *target_path, void *context);

typedef struct platform_atomic_new_file_hooks {
    platform_atomic_file_write_fn write;
    platform_atomic_new_file_publish_fn publish;
    void *context;
} platform_atomic_new_file_hooks;

typedef struct platform_atomic_new_file_result {
    platform_atomic_new_file_error error;
    bool file_fsynced;
    bool published;
    bool directory_fsynced;
} platform_atomic_new_file_result;

/* Writes and fsyncs an exclusive temporary sibling, then publishes it with an
 * atomic no-replace operation. A successful publish is the commit point. */
platform_atomic_new_file_result platform_atomic_file_create_new(
    base_bytes path, base_bytes bytes,
    const platform_atomic_new_file_hooks *hooks);

#endif
