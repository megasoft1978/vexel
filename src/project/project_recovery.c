#define _DARWIN_C_SOURCE 1
#include "project_recovery.h"

#include <errno.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>

static const uint8_t recovery_magic[8] = {'V','X','R','E','C','O','V',1};

static project_recovery_result recovery_result(project_recovery_error error)
{
    return (project_recovery_result){.error = error};
}

static void write_u32(uint8_t *out, uint32_t value)
{
    for (size_t i = 0U; i < 4U; ++i) out[i] = (uint8_t)(value >> (i * 8U));
}

static void write_u64(uint8_t *out, uint64_t value)
{
    for (size_t i = 0U; i < 8U; ++i) out[i] = (uint8_t)(value >> (i * 8U));
}

static uint32_t read_u32(const uint8_t *in)
{
    uint32_t value = 0U;
    for (size_t i = 0U; i < 4U; ++i) value |= (uint32_t)in[i] << (i * 8U);
    return value;
}

static uint64_t read_u64(const uint8_t *in)
{
    uint64_t value = 0U;
    for (size_t i = 0U; i < 8U; ++i) value |= (uint64_t)in[i] << (i * 8U);
    return value;
}

/* IEEE CRC-32, matching the accidental-corruption check in project_bundle.c. */
static uint32_t crc32_update(uint32_t crc, base_bytes bytes)
{
    for (size_t i = 0U; i < bytes.len; ++i) {
        crc ^= bytes.ptr[i];
        for (unsigned bit = 0U; bit < 8U; ++bit)
            crc = (crc >> 1U) ^ (UINT32_C(0xedb88320) &
                (uint32_t)-(int32_t)(crc & 1U));
    }
    return crc;
}

static uint32_t crc32(base_bytes bytes)
{
    return ~crc32_update(UINT32_MAX, bytes);
}

static bool path_valid(base_bytes path)
{
    return base_bytes_valid(path) && path.len > 0U &&
        path.len <= PROJECT_RECOVERY_MAX_PATH_BYTES && path.ptr[0] == '/' &&
        memchr(path.ptr, '\0', path.len) == NULL;
}

project_recovery_result project_recovery_encoded_size(
    const project_recovery_input *input)
{
    if (input == NULL || !path_valid(input->source_path) || input->project == NULL ||
        input->source_modified_nanoseconds >= 1000000000U)
        return recovery_result(PROJECT_RECOVERY_INVALID_ARGUMENT);
    project_bundle_result bundle = project_bundle_encoded_size(input->project);
    if (bundle.error != PROJECT_BUNDLE_OK) {
        project_recovery_result result = recovery_result(
            bundle.error == PROJECT_BUNDLE_LIMIT ? PROJECT_RECOVERY_LIMIT :
                PROJECT_RECOVERY_INVALID_PROJECT);
        result.bundle_error = bundle.error;
        return result;
    }
    project_recovery_result result = recovery_result(PROJECT_RECOVERY_OK);
    size_t size = PROJECT_RECOVERY_HEADER_BYTES;
    if (!base_size_add(size, input->source_path.len, &size) ||
        !base_size_add(size, bundle.encoded_size, &size) ||
        size > PROJECT_RECOVERY_MAX_BYTES) {
        result.error = PROJECT_RECOVERY_LIMIT;
        return result;
    }
    result.encoded_size = size;
    return result;
}

project_recovery_result project_recovery_encode(
    const project_recovery_input *input, base_mut_bytes output)
{
    project_recovery_result measured = project_recovery_encoded_size(input);
    if (measured.error != PROJECT_RECOVERY_OK) return measured;
    if ((output.ptr == NULL && output.len != 0U) || output.len < measured.encoded_size) {
        measured.error = PROJECT_RECOVERY_INVALID_ARGUMENT;
        return measured;
    }
    project_bundle_result bundle = project_bundle_encoded_size(input->project);
    uint8_t *header = output.ptr;
    memcpy(header, recovery_magic, sizeof(recovery_magic));
    write_u32(header + 8U, PROJECT_RECOVERY_VERSION);
    write_u32(header + 12U, PROJECT_RECOVERY_HEADER_BYTES);
    write_u32(header + 16U, (uint32_t)input->source_path.len);
    write_u32(header + 20U, 0U);
    write_u64(header + 24U, input->source_size);
    write_u64(header + 32U, (uint64_t)input->source_modified_seconds);
    write_u32(header + 40U, input->source_modified_nanoseconds);
    write_u64(header + 44U, bundle.encoded_size);
    write_u32(header + 52U, 0U);
    write_u32(header + 56U, crc32(input->source_path));
    write_u32(header + 60U, 0U);
    write_u64(header + 64U, input->captured_time_nanoseconds);
    memcpy(output.ptr + PROJECT_RECOVERY_HEADER_BYTES, input->source_path.ptr,
        input->source_path.len);
    base_mut_bytes bundle_bytes = {output.ptr + PROJECT_RECOVERY_HEADER_BYTES +
        input->source_path.len, bundle.encoded_size};
    project_bundle_result encoded = project_bundle_encode(input->project, bundle_bytes);
    if (encoded.error != PROJECT_BUNDLE_OK ||
        encoded.encoded_size != bundle.encoded_size) {
        measured.error = encoded.error == PROJECT_BUNDLE_LIMIT ?
            PROJECT_RECOVERY_LIMIT : PROJECT_RECOVERY_INVALID_PROJECT;
        measured.bundle_error = encoded.error;
        return measured;
    }
    write_u32(header + 52U, crc32((base_bytes){bundle_bytes.ptr, bundle_bytes.len}));
    uint8_t checksum_header[PROJECT_RECOVERY_HEADER_BYTES];
    memcpy(checksum_header, header, sizeof(checksum_header));
    memset(checksum_header + 60U, 0, 4U);
    uint32_t record_crc = crc32_update(UINT32_MAX,
        (base_bytes){checksum_header, sizeof(checksum_header)});
    record_crc = crc32_update(record_crc, (base_bytes){
        output.ptr + PROJECT_RECOVERY_HEADER_BYTES,
        input->source_path.len + encoded.encoded_size});
    write_u32(header + 60U, ~record_crc);
    return measured;
}

project_recovery_result project_recovery_decode(base_bytes encoded,
    base_bytes expected_source_path, project_recovery_view *out_view)
{
    if (out_view != NULL) *out_view = (project_recovery_view){0};
    if (out_view == NULL || !base_bytes_valid(encoded) ||
        (expected_source_path.len != 0U && !base_bytes_valid(expected_source_path)) ||
        encoded.len < PROJECT_RECOVERY_HEADER_BYTES)
        return recovery_result(PROJECT_RECOVERY_INVALID_ARGUMENT);
    const uint8_t *header = encoded.ptr;
    if (memcmp(header, recovery_magic, sizeof(recovery_magic)) != 0 ||
        read_u32(header + 12U) != PROJECT_RECOVERY_HEADER_BYTES ||
        read_u32(header + 20U) != 0U)
        return recovery_result(PROJECT_RECOVERY_CORRUPT);
    uint32_t version = read_u32(header + 8U);
    if (version != PROJECT_RECOVERY_VERSION)
        return recovery_result(PROJECT_RECOVERY_UNSUPPORTED_VERSION);
    size_t path_length = (size_t)read_u32(header + 16U);
    uint64_t raw_project_length = read_u64(header + 44U);
    uint32_t nanoseconds = read_u32(header + 40U);
    if (path_length == 0U || path_length > PROJECT_RECOVERY_MAX_PATH_BYTES ||
        nanoseconds >= 1000000000U || raw_project_length > PROJECT_BUNDLE_MAX_BYTES ||
        raw_project_length > SIZE_MAX)
        return recovery_result(PROJECT_RECOVERY_CORRUPT);
    size_t project_length = (size_t)raw_project_length;
    size_t expected_length = PROJECT_RECOVERY_HEADER_BYTES;
    if (!base_size_add(expected_length, path_length, &expected_length) ||
        !base_size_add(expected_length, project_length, &expected_length) ||
        expected_length != encoded.len || expected_length > PROJECT_RECOVERY_MAX_BYTES)
        return recovery_result(PROJECT_RECOVERY_CORRUPT);
    base_bytes path = {encoded.ptr + PROJECT_RECOVERY_HEADER_BYTES, path_length};
    base_bytes payload = {path.ptr + path.len, project_length};
    uint8_t checksum_header[PROJECT_RECOVERY_HEADER_BYTES];
    memcpy(checksum_header, header, sizeof(checksum_header));
    memset(checksum_header + 60U, 0, 4U);
    uint32_t record_crc = crc32_update(UINT32_MAX,
        (base_bytes){checksum_header, sizeof(checksum_header)});
    record_crc = crc32_update(record_crc, (base_bytes){
        encoded.ptr + PROJECT_RECOVERY_HEADER_BYTES,
        encoded.len - PROJECT_RECOVERY_HEADER_BYTES});
    if (!path_valid(path) || crc32(path) != read_u32(header + 56U) ||
        crc32(payload) != read_u32(header + 52U) ||
        ~record_crc != read_u32(header + 60U))
        return recovery_result(PROJECT_RECOVERY_CORRUPT);
    if (expected_source_path.len != 0U &&
        (expected_source_path.len != path.len ||
         memcmp(expected_source_path.ptr, path.ptr, path.len) != 0))
        return recovery_result(PROJECT_RECOVERY_PATH_MISMATCH);
    project_bundle_view project = {0};
    project_bundle_result decoded = project_bundle_decode(payload, &project);
    if (decoded.error != PROJECT_BUNDLE_OK) {
        project_recovery_result result = recovery_result(
            decoded.error == PROJECT_BUNDLE_UNSUPPORTED_VERSION ?
                PROJECT_RECOVERY_UNSUPPORTED_VERSION : PROJECT_RECOVERY_INVALID_PROJECT);
        result.bundle_error = decoded.error;
        result.error_offset = decoded.error_offset;
        return result;
    }
    *out_view = (project_recovery_view){
        .source_path = path,
        .source_size = read_u64(header + 24U),
        .source_modified_seconds = (int64_t)read_u64(header + 32U),
        .source_modified_nanoseconds = nanoseconds,
        .captured_time_nanoseconds = read_u64(header + 64U),
        .project = project
    };
    project_recovery_result result = recovery_result(PROJECT_RECOVERY_OK);
    result.encoded_size = encoded.len;
    return result;
}

project_recovery_result project_recovery_save(base_bytes recovery_path,
    const project_recovery_input *input)
{
    project_recovery_result result = recovery_result(PROJECT_RECOVERY_INVALID_ARGUMENT);
    if (!base_bytes_valid(recovery_path) || recovery_path.len == 0U ||
        memchr(recovery_path.ptr, '\0', recovery_path.len) != NULL || input == NULL)
        return result;
    if (recovery_path.len > PLATFORM_ATOMIC_FILE_MAX_PATH_BYTES)
        return recovery_result(PROJECT_RECOVERY_PATH_LIMIT);
    project_recovery_result measured = project_recovery_encoded_size(input);
    if (measured.error != PROJECT_RECOVERY_OK) return measured;
    if (recovery_path.len == input->source_path.len &&
        memcmp(recovery_path.ptr, input->source_path.ptr, recovery_path.len) == 0)
        return result;
    uint8_t *bytes = malloc(measured.encoded_size);
    if (bytes == NULL) return recovery_result(PROJECT_RECOVERY_OUT_OF_MEMORY);
    project_recovery_result encoded = project_recovery_encode(input,
        (base_mut_bytes){bytes, measured.encoded_size});
    if (encoded.error != PROJECT_RECOVERY_OK) {
        free(bytes);
        return encoded;
    }
    char target[PLATFORM_ATOMIC_FILE_MAX_PATH_BYTES + 1U];
    memcpy(target, recovery_path.ptr, recovery_path.len);
    target[recovery_path.len] = '\0';
    struct stat status;
    int stat_result = lstat(target, &status);
    if (stat_result == 0 && S_ISREG(status.st_mode)) {
        platform_atomic_file_result written = platform_atomic_file_replace(
            recovery_path, (base_bytes){bytes, encoded.encoded_size}, NULL);
        result.replace_error = written.error;
        result.published = written.renamed;
        result.directory_fsynced = written.directory_fsynced;
        result.bytes_written = encoded.encoded_size;
        result.error = written.error == PLATFORM_ATOMIC_FILE_OK ? PROJECT_RECOVERY_OK :
            written.error == PLATFORM_ATOMIC_FILE_DURABILITY ?
                PROJECT_RECOVERY_DURABILITY : PROJECT_RECOVERY_IO;
    } else if (stat_result != 0 && errno == ENOENT) {
        platform_atomic_new_file_result written = platform_atomic_file_create_new(
            recovery_path, (base_bytes){bytes, encoded.encoded_size}, NULL);
        result.create_error = written.error;
        result.published = written.published;
        result.directory_fsynced = written.directory_fsynced;
        result.bytes_written = encoded.encoded_size;
        result.error = written.error == PLATFORM_ATOMIC_NEW_FILE_OK ? PROJECT_RECOVERY_OK :
            written.error == PLATFORM_ATOMIC_NEW_FILE_EXISTS ?
                PROJECT_RECOVERY_ALREADY_EXISTS :
            written.error == PLATFORM_ATOMIC_NEW_FILE_DURABILITY ?
                PROJECT_RECOVERY_DURABILITY : PROJECT_RECOVERY_IO;
    } else {
        result.error = PROJECT_RECOVERY_IO;
    }
    free(bytes);
    return result;
}

const char *project_recovery_error_name(project_recovery_error error)
{
    switch (error) {
        case PROJECT_RECOVERY_OK: return "ok";
        case PROJECT_RECOVERY_INVALID_ARGUMENT: return "invalid_argument";
        case PROJECT_RECOVERY_LIMIT: return "limit";
        case PROJECT_RECOVERY_OUT_OF_MEMORY: return "out_of_memory";
        case PROJECT_RECOVERY_PATH_LIMIT: return "path_limit";
        case PROJECT_RECOVERY_ALREADY_EXISTS: return "already_exists";
        case PROJECT_RECOVERY_IO: return "io";
        case PROJECT_RECOVERY_DURABILITY: return "durability";
        case PROJECT_RECOVERY_CORRUPT: return "corrupt";
        case PROJECT_RECOVERY_UNSUPPORTED_VERSION: return "unsupported_version";
        case PROJECT_RECOVERY_PATH_MISMATCH: return "path_mismatch";
        case PROJECT_RECOVERY_INVALID_PROJECT: return "invalid_project";
    }
    return "unknown";
}
