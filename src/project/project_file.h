#ifndef VEXEL_PROJECT_PROJECT_FILE_H
#define VEXEL_PROJECT_PROJECT_FILE_H

#include "project_bundle.h"

#include "platform_atomic_file.h"
#include "platform_atomic_new_file.h"

typedef enum project_file_error {
    PROJECT_FILE_OK = 0,
    PROJECT_FILE_INVALID_ARGUMENT,
    PROJECT_FILE_LIMIT,
    PROJECT_FILE_OUT_OF_MEMORY,
    PROJECT_FILE_PATH_LIMIT,
    PROJECT_FILE_ALREADY_EXISTS,
    PROJECT_FILE_IO,
    PROJECT_FILE_DURABILITY
} project_file_error;

typedef struct project_file_result {
    project_file_error error;
    project_bundle_error bundle_error;
    platform_atomic_file_error replace_error;
    platform_atomic_new_file_error create_error;
    size_t bytes_written;
    bool published;
    bool directory_fsynced;
} project_file_result;

/* Creates a new file without replacing an existing path, or atomically replaces
 * an existing regular file. The project is fully encoded before publication. */
project_file_result project_file_save_as(base_bytes path,
                                         const project_bundle_input *input);
/* Atomically publishes an already encoded, checksum-valid project bundle.
 * The caller retains ownership of `encoded`. */
project_file_result project_file_save_encoded(base_bytes path,
    base_bytes encoded);
const char *project_file_error_name(project_file_error error);

#endif
