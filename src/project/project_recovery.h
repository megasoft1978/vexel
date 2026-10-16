#ifndef VEXEL_PROJECT_PROJECT_RECOVERY_H
#define VEXEL_PROJECT_PROJECT_RECOVERY_H

#include "platform_atomic_file.h"
#include "platform_atomic_new_file.h"
#include "project_bundle.h"

#define PROJECT_RECOVERY_VERSION 1U
#define PROJECT_RECOVERY_HEADER_BYTES 72U
#define PROJECT_RECOVERY_MAX_PATH_BYTES 1023U
#define PROJECT_RECOVERY_MAX_BYTES \
    (PROJECT_BUNDLE_MAX_BYTES + PROJECT_RECOVERY_HEADER_BYTES + \
     PROJECT_RECOVERY_MAX_PATH_BYTES)

typedef enum project_recovery_error {
    PROJECT_RECOVERY_OK = 0,
    PROJECT_RECOVERY_INVALID_ARGUMENT,
    PROJECT_RECOVERY_LIMIT,
    PROJECT_RECOVERY_OUT_OF_MEMORY,
    PROJECT_RECOVERY_PATH_LIMIT,
    PROJECT_RECOVERY_ALREADY_EXISTS,
    PROJECT_RECOVERY_IO,
    PROJECT_RECOVERY_DURABILITY,
    PROJECT_RECOVERY_CORRUPT,
    PROJECT_RECOVERY_UNSUPPORTED_VERSION,
    PROJECT_RECOVERY_PATH_MISMATCH,
    PROJECT_RECOVERY_INVALID_PROJECT
} project_recovery_error;

typedef struct project_recovery_input {
    base_bytes source_path;
    uint64_t source_size;
    int64_t source_modified_seconds;
    uint32_t source_modified_nanoseconds;
    uint64_t captured_time_nanoseconds;
    const project_bundle_input *project;
} project_recovery_input;

typedef struct project_recovery_view {
    base_bytes source_path;
    uint64_t source_size;
    int64_t source_modified_seconds;
    uint32_t source_modified_nanoseconds;
    uint64_t captured_time_nanoseconds;
    project_bundle_view project;
} project_recovery_view;

typedef struct project_recovery_result {
    project_recovery_error error;
    project_bundle_error bundle_error;
    platform_atomic_file_error replace_error;
    platform_atomic_new_file_error create_error;
    size_t encoded_size;
    size_t bytes_written;
    size_t error_offset;
    bool published;
    bool directory_fsynced;
} project_recovery_result;

project_recovery_result project_recovery_encoded_size(
    const project_recovery_input *input);
project_recovery_result project_recovery_encode(
    const project_recovery_input *input, base_mut_bytes output);
/* Views borrow encoded and project input bytes. Pass {NULL,0} as expected path
 * to skip identity comparison; otherwise the encoded path must match exactly. */
project_recovery_result project_recovery_decode(base_bytes encoded,
    base_bytes expected_source_path, project_recovery_view *out_view);
/* Atomically writes a capsule to a caller-selected path distinct from its
 * source project. Publication never changes the source or saved-state flags. */
project_recovery_result project_recovery_save(base_bytes recovery_path,
    const project_recovery_input *input);
const char *project_recovery_error_name(project_recovery_error error);

#endif
