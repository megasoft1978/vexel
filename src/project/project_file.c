#include "project_file.h"

#include <errno.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>

static project_file_result file_result(project_file_error error)
{
    return (project_file_result){.error = error};
}

project_file_result project_file_save_as(base_bytes path,
                                         const project_bundle_input *input)
{
    if (!base_bytes_valid(path) || path.len == 0U || input == NULL)
        return file_result(PROJECT_FILE_INVALID_ARGUMENT);
    if (path.len > PLATFORM_ATOMIC_FILE_MAX_PATH_BYTES)
        return file_result(PROJECT_FILE_PATH_LIMIT);
    if (memchr(path.ptr, '\0', path.len) != NULL)
        return file_result(PROJECT_FILE_INVALID_ARGUMENT);
    project_bundle_result measured = project_bundle_encoded_size(input);
    if (measured.error != PROJECT_BUNDLE_OK) {
        project_file_result result = file_result(measured.error == PROJECT_BUNDLE_LIMIT ?
            PROJECT_FILE_LIMIT : PROJECT_FILE_INVALID_ARGUMENT);
        result.bundle_error = measured.error;
        return result;
    }
    uint8_t *encoded = malloc(measured.encoded_size);
    if (encoded == NULL) return file_result(PROJECT_FILE_OUT_OF_MEMORY);
    project_bundle_result written = project_bundle_encode(input,
        (base_mut_bytes){encoded, measured.encoded_size});
    if (written.error != PROJECT_BUNDLE_OK || written.encoded_size != measured.encoded_size) {
        free(encoded);
        project_file_result result = file_result(written.error == PROJECT_BUNDLE_LIMIT ?
            PROJECT_FILE_LIMIT : PROJECT_FILE_INVALID_ARGUMENT);
        result.bundle_error = written.error;
        return result;
    }

    project_file_result result = project_file_save_encoded(path,
        (base_bytes){encoded, written.encoded_size});
    free(encoded);
    return result;
}

project_file_result project_file_save_encoded(base_bytes path,
    base_bytes encoded_bytes)
{
    if (!base_bytes_valid(path) || path.len == 0U ||
        path.len > PLATFORM_ATOMIC_FILE_MAX_PATH_BYTES ||
        memchr(path.ptr, '\0', path.len) != NULL ||
        !base_bytes_valid(encoded_bytes) || encoded_bytes.len == 0U)
        return file_result(PROJECT_FILE_INVALID_ARGUMENT);
    char path_string[PLATFORM_ATOMIC_FILE_MAX_PATH_BYTES + 1U];
    memcpy(path_string, path.ptr, path.len);
    path_string[path.len] = '\0';
    project_bundle_view validated = {0};
    project_bundle_result decoded = project_bundle_decode(encoded_bytes,
        &validated);
    if (decoded.error != PROJECT_BUNDLE_OK) {
        project_file_result invalid = file_result(
            decoded.error == PROJECT_BUNDLE_LIMIT ? PROJECT_FILE_LIMIT :
                PROJECT_FILE_INVALID_ARGUMENT);
        invalid.bundle_error = decoded.error;
        return invalid;
    }

    project_file_result result = file_result(PROJECT_FILE_IO);
    struct stat status;
    int stat_result = lstat(path_string, &status);
    if (stat_result == 0 && S_ISREG(status.st_mode)) {
        platform_atomic_file_result replaced = platform_atomic_file_replace(path,
            encoded_bytes, NULL);
        result.replace_error = replaced.error;
        result.published = replaced.renamed;
        result.directory_fsynced = replaced.directory_fsynced;
        result.bytes_written = encoded_bytes.len;
        if (replaced.error == PLATFORM_ATOMIC_FILE_OK) result.error = PROJECT_FILE_OK;
        else if (replaced.error == PLATFORM_ATOMIC_FILE_DURABILITY)
            result.error = PROJECT_FILE_DURABILITY;
        else result.error = PROJECT_FILE_IO;
    } else if (stat_result != 0 && errno == ENOENT) {
        platform_atomic_new_file_result created = platform_atomic_file_create_new(path,
            encoded_bytes, NULL);
        result.create_error = created.error;
        result.published = created.published;
        result.directory_fsynced = created.directory_fsynced;
        result.bytes_written = encoded_bytes.len;
        if (created.error == PLATFORM_ATOMIC_NEW_FILE_OK) result.error = PROJECT_FILE_OK;
        else if (created.error == PLATFORM_ATOMIC_NEW_FILE_EXISTS)
            result.error = PROJECT_FILE_ALREADY_EXISTS;
        else if (created.error == PLATFORM_ATOMIC_NEW_FILE_DURABILITY)
            result.error = PROJECT_FILE_DURABILITY;
        else result.error = PROJECT_FILE_IO;
    } else {
        result.error = PROJECT_FILE_IO;
    }
    return result;
}

const char *project_file_error_name(project_file_error error)
{
    switch (error) {
        case PROJECT_FILE_OK: return "ok";
        case PROJECT_FILE_INVALID_ARGUMENT: return "invalid_argument";
        case PROJECT_FILE_LIMIT: return "limit";
        case PROJECT_FILE_OUT_OF_MEMORY: return "out_of_memory";
        case PROJECT_FILE_PATH_LIMIT: return "path_limit";
        case PROJECT_FILE_ALREADY_EXISTS: return "already_exists";
        case PROJECT_FILE_IO: return "io";
        case PROJECT_FILE_DURABILITY: return "durability";
    }
    return "unknown";
}
