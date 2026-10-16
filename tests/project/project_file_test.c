#define _DARWIN_C_SOURCE

#include "project_file.h"

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

static bool read_bundle(const char *path, project_bundle_view *out)
{
    FILE *file = fopen(path, "rb");
    if (file == NULL || fseeko(file, 0, SEEK_END) != 0) {
        if (file != NULL) (void)fclose(file);
        return false;
    }
    off_t end = ftello(file);
    if (end < 0 || (uintmax_t)end > PROJECT_BUNDLE_MAX_BYTES ||
        fseeko(file, 0, SEEK_SET) != 0) {
        (void)fclose(file);
        return false;
    }
    size_t length = (size_t)end;
    uint8_t *data = malloc(length == 0U ? 1U : length);
    if (data == NULL) { (void)fclose(file); return false; }
    size_t read_length = fread(data, 1U, length, file);
    bool closed = fclose(file) == 0;
    bool okay = read_length == length && closed;
    project_bundle_result decoded = okay ? project_bundle_decode(
        (base_bytes){data, length}, out) : (project_bundle_result){.error = PROJECT_BUNDLE_CORRUPT};
    if (decoded.error == PROJECT_BUNDLE_OK) {
        /* Keep the backing allocation alive until the caller inspects the view. */
        return true;
    }
    free(data);
    return false;
}

static bool test_atomic_project_write(void)
{
    char directory[] = "/tmp/vexel-project-file-XXXXXX";
    CHECK(mkdtemp(directory) != NULL);
    char path[1024];
    int amount = snprintf(path, sizeof(path), "%s/drawing.vexel", directory);
    CHECK(amount > 0 && (size_t)amount < sizeof(path));
    static const uint8_t asset_bytes[] = {0x00U, 0x14U, 0xffU};
    project_bundle_asset assets[] = {
        {bytes("palette.bin"), bytes("application/octet-stream"),
         {asset_bytes, sizeof(asset_bytes)}}
    };
    project_bundle_input input = {bytes("{\"title\":\"Draft\"}"),
        bytes("<svg xmlns='urn:svg' xmlns:x='urn:unknown'><x:data/></svg>"),
        bytes("{\"tracks\":[]}"), assets, 1U};
    project_file_result saved = project_file_save_as(
        (base_bytes){(const uint8_t *)path, strlen(path)}, &input);
    CHECK(saved.error == PROJECT_FILE_OK && saved.published && saved.directory_fsynced &&
        saved.bytes_written > PROJECT_BUNDLE_HEADER_BYTES);
    struct stat status;
    CHECK(stat(path, &status) == 0 && (status.st_mode & 0777) == 0600);

    project_bundle_view view = {0};
    CHECK(read_bundle(path, &view));
    CHECK(view.manifest_json.len == input.manifest_json.len &&
        memcmp(view.manifest_json.ptr, input.manifest_json.ptr, input.manifest_json.len) == 0);
    project_bundle_asset loaded_asset;
    CHECK(project_bundle_asset_at(&view, 0U, &loaded_asset));
    CHECK(loaded_asset.name.len == assets[0].name.len &&
        memcmp(loaded_asset.name.ptr, assets[0].name.ptr, assets[0].name.len) == 0 &&
        loaded_asset.data.len == sizeof(asset_bytes) &&
        memcmp(loaded_asset.data.ptr, asset_bytes, sizeof(asset_bytes)) == 0);
    free((void *)view.encoded.ptr);

    input.scene_svg = bytes("<svg><rect width='20'/></svg>");
    saved = project_file_save_as((base_bytes){(const uint8_t *)path, strlen(path)}, &input);
    CHECK(saved.error == PROJECT_FILE_OK && saved.published && saved.directory_fsynced);
    view = (project_bundle_view){0};
    CHECK(read_bundle(path, &view));
    CHECK(view.scene_svg.len == input.scene_svg.len &&
        memcmp(view.scene_svg.ptr, input.scene_svg.ptr, input.scene_svg.len) == 0);
    CHECK(view.timeline_json.len == input.timeline_json.len &&
        memcmp(view.timeline_json.ptr, input.timeline_json.ptr, input.timeline_json.len) == 0);
    free((void *)view.encoded.ptr);

    project_bundle_asset invalid_asset = {bytes("../escape"),
        bytes("application/octet-stream"), {asset_bytes, sizeof(asset_bytes)}};
    project_bundle_input invalid = input;
    invalid.assets = &invalid_asset;
    saved = project_file_save_as((base_bytes){(const uint8_t *)path, strlen(path)},
        &invalid);
    CHECK(saved.error == PROJECT_FILE_INVALID_ARGUMENT && !saved.published);
    view = (project_bundle_view){0};
    CHECK(read_bundle(path, &view));
    CHECK(view.scene_svg.len == input.scene_svg.len &&
        memcmp(view.scene_svg.ptr, input.scene_svg.ptr, input.scene_svg.len) == 0);
    free((void *)view.encoded.ptr);

    char directory_target[1024];
    amount = snprintf(directory_target, sizeof(directory_target), "%s/directory", directory);
    CHECK(amount > 0 && (size_t)amount < sizeof(directory_target) &&
        mkdir(directory_target, 0700) == 0);
    saved = project_file_save_as((base_bytes){(const uint8_t *)directory_target,
        strlen(directory_target)}, &input);
    CHECK(saved.error == PROJECT_FILE_IO && stat(directory_target, &status) == 0 &&
        S_ISDIR(status.st_mode));

    char too_long[PLATFORM_ATOMIC_FILE_MAX_PATH_BYTES + 2U];
    memset(too_long, 'x', sizeof(too_long));
    saved = project_file_save_as((base_bytes){(const uint8_t *)too_long,
        sizeof(too_long)}, &input);
    CHECK(saved.error == PROJECT_FILE_PATH_LIMIT);

    CHECK(unlink(path) == 0 && rmdir(directory_target) == 0 && rmdir(directory) == 0);
    return true;
}

int main(void)
{
    if (!test_atomic_project_write()) return 1;
    (void)puts("project file: atomic create, replacement, and payload preservation passed");
    return 0;
}
