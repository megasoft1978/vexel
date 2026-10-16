#define _DARWIN_C_SOURCE 1
#include "project_recovery.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>

#define CHECK(condition) do { if (!(condition)) { \
    (void)fprintf(stderr, "FAIL %s:%d: %s\n", __FILE__, __LINE__, #condition); \
    return false; } } while (0)

static base_bytes bytes(const char *text)
{
    return (base_bytes){(const uint8_t *)text, strlen(text)};
}

static bool same(base_bytes a, base_bytes b)
{
    return a.len == b.len && (a.len == 0U || memcmp(a.ptr, b.ptr, a.len) == 0);
}

static bool test_round_trip_and_validation(void)
{
    static const uint8_t asset_data[] = {0U, 0x34U, 0xffU};
    project_bundle_asset assets[] = {{bytes("frame.svg"), bytes("image/svg+xml"),
        {asset_data, sizeof(asset_data)}}};
    project_bundle_input project = {bytes("{\"title\":\"Walk\"}"),
        bytes("<svg><path id='hero'/></svg>"),
        bytes("{\"version\":1,\"tracks\":[]}"), assets, 1U};
    project_recovery_input input = {bytes("/Users/example/Walk.vexel"),
        UINT64_C(123456), INT64_C(1791840000), 456U, UINT64_C(99887766), &project};
    project_recovery_result size = project_recovery_encoded_size(&input);
    CHECK(size.error == PROJECT_RECOVERY_OK && size.encoded_size > 72U);
    uint8_t *encoded = malloc(size.encoded_size);
    CHECK(encoded != NULL);
    project_recovery_result wrote = project_recovery_encode(&input,
        (base_mut_bytes){encoded, size.encoded_size});
    CHECK(wrote.error == PROJECT_RECOVERY_OK && wrote.encoded_size == size.encoded_size);
    project_recovery_view view = {0};
    project_recovery_result decoded = project_recovery_decode(
        (base_bytes){encoded, size.encoded_size}, input.source_path, &view);
    CHECK(decoded.error == PROJECT_RECOVERY_OK && same(view.source_path, input.source_path));
    CHECK(view.source_size == input.source_size &&
        view.source_modified_seconds == input.source_modified_seconds &&
        view.source_modified_nanoseconds == input.source_modified_nanoseconds &&
        view.captured_time_nanoseconds == input.captured_time_nanoseconds);
    CHECK(same(view.project.manifest_json, project.manifest_json) &&
        same(view.project.scene_svg, project.scene_svg) &&
        same(view.project.timeline_json, project.timeline_json) &&
        view.project.asset_count == 1U);
    project_bundle_asset decoded_asset;
    CHECK(project_bundle_asset_at(&view.project, 0U, &decoded_asset) &&
        same(decoded_asset.name, assets[0].name) &&
        same(decoded_asset.data, assets[0].data));
    CHECK(project_recovery_decode((base_bytes){encoded, size.encoded_size},
        bytes("/Users/example/Other.vexel"), &view).error ==
        PROJECT_RECOVERY_PATH_MISMATCH);
    CHECK(project_recovery_decode((base_bytes){encoded, size.encoded_size - 1U},
        (base_bytes){NULL, 0U}, &view).error == PROJECT_RECOVERY_CORRUPT);
    uint8_t *damaged = malloc(size.encoded_size + 1U);
    CHECK(damaged != NULL);
    memcpy(damaged, encoded, size.encoded_size);
    damaged[PROJECT_RECOVERY_HEADER_BYTES + 2U] ^= 1U;
    CHECK(project_recovery_decode((base_bytes){damaged, size.encoded_size},
        (base_bytes){NULL, 0U}, &view).error == PROJECT_RECOVERY_CORRUPT);
    memcpy(damaged, encoded, size.encoded_size);
    damaged[8U] = 2U;
    CHECK(project_recovery_decode((base_bytes){damaged, size.encoded_size},
        (base_bytes){NULL, 0U}, &view).error == PROJECT_RECOVERY_UNSUPPORTED_VERSION);
    memcpy(damaged, encoded, size.encoded_size);
    damaged[size.encoded_size] = 0U;
    CHECK(project_recovery_decode((base_bytes){damaged, size.encoded_size + 1U},
        (base_bytes){NULL, 0U}, &view).error == PROJECT_RECOVERY_CORRUPT);
    memcpy(damaged, encoded, size.encoded_size);
    damaged[20U] = 1U;
    CHECK(project_recovery_decode((base_bytes){damaged, size.encoded_size},
        (base_bytes){NULL, 0U}, &view).error == PROJECT_RECOVERY_CORRUPT);
    memcpy(damaged, encoded, size.encoded_size);
    damaged[24U] ^= 1U;
    CHECK(project_recovery_decode((base_bytes){damaged, size.encoded_size},
        (base_bytes){NULL, 0U}, &view).error == PROJECT_RECOVERY_CORRUPT);
    memcpy(damaged, encoded, size.encoded_size);
    memset(damaged + 44U, 0xff, 8U);
    CHECK(project_recovery_decode((base_bytes){damaged, size.encoded_size},
        (base_bytes){NULL, 0U}, &view).error == PROJECT_RECOVERY_CORRUPT);
    free(damaged);
    free(encoded);
    input.source_modified_nanoseconds = 1000000000U;
    CHECK(project_recovery_encoded_size(&input).error == PROJECT_RECOVERY_INVALID_ARGUMENT);
    input.source_modified_nanoseconds = 0U;
    uint8_t too_long_path[PROJECT_RECOVERY_MAX_PATH_BYTES + 1U];
    memset(too_long_path, 'p', sizeof(too_long_path));
    too_long_path[0] = '/';
    input.source_path = (base_bytes){too_long_path, sizeof(too_long_path)};
    CHECK(project_recovery_encoded_size(&input).error == PROJECT_RECOVERY_INVALID_ARGUMENT);
    return true;
}

static bool test_atomic_store(void)
{
    char directory[] = "/tmp/vexel-recovery-XXXXXX";
    CHECK(mkdtemp(directory) != NULL);
    char source[1024], target[1024], blocked[1024];
    int n = snprintf(source, sizeof(source), "%s/source.vexel", directory);
    CHECK(n > 0 && (size_t)n < sizeof(source));
    n = snprintf(target, sizeof(target), "%s/recovery.vxr", directory);
    CHECK(n > 0 && (size_t)n < sizeof(target));
    n = snprintf(blocked, sizeof(blocked), "%s/blocked", directory);
    CHECK(n > 0 && (size_t)n < sizeof(blocked) && mkdir(blocked, 0700) == 0);
    project_bundle_input project = {bytes("{}"), bytes("<svg/>"), bytes("{}"), NULL, 0U};
    project_recovery_input input = {(base_bytes){(const uint8_t *)source, strlen(source)},
        10U, 20, 30U, 40U, &project};
    base_bytes target_bytes = {(const uint8_t *)target, strlen(target)};
    project_recovery_result saved = project_recovery_save(target_bytes, &input);
    CHECK(saved.error == PROJECT_RECOVERY_OK && saved.published &&
        saved.directory_fsynced && saved.bytes_written > PROJECT_RECOVERY_HEADER_BYTES);
    saved = project_recovery_save(target_bytes, &input);
    CHECK(saved.error == PROJECT_RECOVERY_OK && saved.published && saved.directory_fsynced);
    FILE *file = fopen(target, "rb");
    CHECK(file != NULL && fseek(file, 0L, SEEK_END) == 0);
    long previous_length = ftell(file);
    CHECK(previous_length > 0L && fseek(file, 0L, SEEK_SET) == 0);
    uint8_t *previous = malloc((size_t)previous_length);
    CHECK(previous != NULL && fread(previous, 1U, (size_t)previous_length, file) ==
        (size_t)previous_length && fclose(file) == 0);
    base_bytes blocked_bytes = {(const uint8_t *)blocked, strlen(blocked)};
    CHECK(project_recovery_save(blocked_bytes, &input).error == PROJECT_RECOVERY_IO);
    file = fopen(target, "rb");
    CHECK(file != NULL && fseek(file, 0L, SEEK_END) == 0 &&
        ftell(file) == previous_length && fseek(file, 0L, SEEK_SET) == 0);
    uint8_t *after_failure = malloc((size_t)previous_length);
    CHECK(after_failure != NULL && fread(after_failure, 1U, (size_t)previous_length,
        file) == (size_t)previous_length && fclose(file) == 0 &&
        memcmp(previous, after_failure, (size_t)previous_length) == 0);
    free(after_failure);
    free(previous);
    CHECK(project_recovery_save(input.source_path, &input).error ==
        PROJECT_RECOVERY_INVALID_ARGUMENT);
    CHECK(unlink(target) == 0 && rmdir(blocked) == 0 && rmdir(directory) == 0);
    return true;
}

int main(void)
{
    if (!test_round_trip_and_validation() || !test_atomic_store()) return 1;
    (void)puts("project recovery: capsule integrity and atomic storage passed");
    return 0;
}
