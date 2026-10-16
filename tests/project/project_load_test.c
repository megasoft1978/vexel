#define _POSIX_C_SOURCE 200809L

#include "app/viewer_load.h"
#include "project/project_file.h"

#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

#define CHECK(condition) do { if (!(condition)) { \
    (void)fprintf(stderr, "FAIL %s:%d: %s\n", __FILE__, __LINE__, #condition); \
    return 1; } } while (0)

int main(void)
{
    static const uint8_t manifest[] = "{\"title\":\"Round trip\"}";
    static const uint8_t scene[] = "<svg xmlns='http://www.w3.org/2000/svg' width='16' height='16'><rect width='9' height='9'/></svg>";
    static const uint8_t timeline[] = "{\"version\":1,\"tracks\":[{\"opaque\":7}]}";
    static const uint8_t asset_data[] = {0U, 1U, 2U, 255U};
    static const uint8_t asset_name[] = "sprite.bin";
    static const uint8_t asset_type[] = "application/octet-stream";
    project_bundle_asset asset = {{asset_name, sizeof(asset_name) - 1U},
        {asset_type, sizeof(asset_type) - 1U}, {asset_data, sizeof(asset_data)}};
    project_bundle_input input = {{manifest, sizeof(manifest) - 1U},
        {scene, sizeof(scene) - 1U}, {timeline, sizeof(timeline) - 1U}, &asset, 1U};
    char path[] = "/tmp/vexel-project-load-XXXXXX";
    int fd = mkstemp(path);
    CHECK(fd >= 0 && close(fd) == 0);
    CHECK(unlink(path) == 0);
    project_file_result saved = project_file_save_as(
        (base_bytes){(const uint8_t *)path, strlen(path)}, &input);
    CHECK(saved.error == PROJECT_FILE_OK && saved.published);

    viewer_loaded_document loaded = {0};
    viewer_load_status load_status = viewer_load_document(path, NULL, &loaded);
    if (load_status != VIEWER_LOAD_OK) (void)fprintf(stderr, "load=%d error=%s shape=%s\n",
        load_status, loaded.load_error, svg_shape_error_name(loaded.shape_error));
    CHECK(load_status == VIEWER_LOAD_OK);
    CHECK(loaded.native_project && loaded.source_owner != NULL &&
        loaded.source >= loaded.source_owner &&
        loaded.project.encoded.ptr == loaded.source_owner);
    CHECK(loaded.timeline_compatibility == PROJECT_TIMELINE_COMPATIBILITY_OPAQUE);
    CHECK(loaded.source_length == sizeof(scene) - 1U &&
        memcmp(loaded.source, scene, loaded.source_length) == 0);
    CHECK(loaded.project.manifest_json.len == sizeof(manifest) - 1U &&
        memcmp(loaded.project.manifest_json.ptr, manifest,
            sizeof(manifest) - 1U) == 0);
    CHECK(loaded.project.timeline_json.len == sizeof(timeline) - 1U &&
        memcmp(loaded.project.timeline_json.ptr, timeline,
            sizeof(timeline) - 1U) == 0);
    project_bundle_asset loaded_asset;
    CHECK(project_bundle_asset_at(&loaded.project, 0U, &loaded_asset));
    CHECK(loaded_asset.data.len == sizeof(asset_data) &&
        memcmp(loaded_asset.data.ptr, asset_data, sizeof(asset_data)) == 0);
    static const uint8_t edited_scene[] =
        "<svg xmlns='http://www.w3.org/2000/svg' width='16' height='16'><rect width='10' height='9'/></svg>";
    project_bundle_input edited = {
        .manifest_json = loaded.project.manifest_json,
        .scene_svg = {edited_scene, sizeof(edited_scene) - 1U},
        .timeline_json = loaded.project.timeline_json,
        .assets = &loaded_asset,
        .asset_count = 1U
    };
    saved = project_file_save_as((base_bytes){(const uint8_t *)path, strlen(path)},
        &edited);
    CHECK(saved.error == PROJECT_FILE_OK && saved.published);
    viewer_loaded_document_destroy(&loaded);
    CHECK(loaded.source_owner == NULL && loaded.source == NULL);

    CHECK(viewer_load_document(path, NULL, &loaded) == VIEWER_LOAD_OK);
    CHECK(loaded.source_length == sizeof(edited_scene) - 1U &&
        memcmp(loaded.source, edited_scene, loaded.source_length) == 0 &&
        loaded.project.timeline_json.len == sizeof(timeline) - 1U &&
        memcmp(loaded.project.timeline_json.ptr, timeline, sizeof(timeline) - 1U) == 0);
    CHECK(project_bundle_asset_at(&loaded.project, 0U, &loaded_asset) &&
        loaded_asset.data.len == sizeof(asset_data) &&
        memcmp(loaded_asset.data.ptr, asset_data, sizeof(asset_data)) == 0);
    viewer_loaded_document_destroy(&loaded);

    static const uint8_t native_timeline[] =
        "{\"schema\":\"vexel.animation\",\"version\":1,"
        "\"ticks_per_second\":24,\"duration_ticks\":24,"
        "\"fps\":{\"numerator\":24,\"denominator\":1},"
        "\"loop\":\"once\",\"onion\":{\"before\":0,\"after\":0,\"opacity\":1},"
        "\"tracks\":[]}";
    project_bundle_input native_input = {
        .manifest_json = {manifest, sizeof(manifest) - 1U},
        .scene_svg = {scene, sizeof(scene) - 1U},
        .timeline_json = {native_timeline, sizeof(native_timeline) - 1U},
        .assets = &asset,
        .asset_count = 1U
    };
    saved = project_file_save_as((base_bytes){(const uint8_t *)path, strlen(path)},
        &native_input);
    CHECK(saved.error == PROJECT_FILE_OK && saved.published);
    CHECK(viewer_load_document(path, NULL, &loaded) == VIEWER_LOAD_OK &&
        loaded.timeline_compatibility == PROJECT_TIMELINE_COMPATIBILITY_NATIVE_V1 &&
        loaded.timeline_session != NULL);
    viewer_loaded_document_destroy(&loaded);
    CHECK(loaded.timeline_session == NULL);

    static const uint8_t invalid_native_timeline[] =
        "{\"schema\":\"vexel.animation\",\"version\":1,"
        "\"ticks_per_second\":24,\"duration_ticks\":24,"
        "\"fps\":{\"numerator\":24,\"denominator\":1},"
        "\"loop\":\"once\",\"onion\":{\"before\":0,\"after\":0,\"opacity\":1},"
        "\"tracks\":[{\"id\":\"p\",\"kind\":\"property\","
        "\"target\":\"missing\",\"visible\":true,\"property\":\"opacity\","
        "\"value_type\":\"scalar\",\"interpolation\":\"linear\","
        "\"keys\":[{\"tick\":0,\"value\":1}]}]}";
    project_bundle_input invalid_native = {
        .manifest_json = {manifest, sizeof(manifest) - 1U},
        .scene_svg = {edited_scene, sizeof(edited_scene) - 1U},
        .timeline_json = {invalid_native_timeline,
            sizeof(invalid_native_timeline) - 1U}
    };
    saved = project_file_save_as((base_bytes){(const uint8_t *)path, strlen(path)},
        &invalid_native);
    CHECK(saved.error == PROJECT_FILE_OK && saved.published);
    load_status = viewer_load_document(path, NULL, &loaded);
    CHECK(load_status == VIEWER_LOAD_FAILED && loaded.source_owner == NULL &&
        loaded.timeline_error == PROJECT_TIMELINE_INVALID_REFERENCE &&
        strstr(loaded.load_error, "invalid_reference") != NULL);
    viewer_loaded_document_destroy(&loaded);

    static const uint8_t drawing_timeline[] =
        "{\"schema\":\"vexel.animation\",\"version\":1,"
        "\"ticks_per_second\":24,\"duration_ticks\":24,"
        "\"fps\":{\"numerator\":24,\"denominator\":1},"
        "\"loop\":\"once\",\"onion\":{\"before\":0,\"after\":0,\"opacity\":1},"
        "\"tracks\":[{\"id\":\"draw\",\"kind\":\"drawing\","
        "\"target\":\"r\",\"visible\":true,\"cels\":["
        "{\"tick\":0,\"asset\":\"cel0\"}]}]}";
    static const uint8_t drawing_scene[] =
        "<svg width='16' height='16'><rect id='r' width='9' height='9'/></svg>";
    static const uint8_t bad_svg_asset[] = "<svg><script>bad()</script></svg>";
    static const uint8_t cel_name[] = "cel0", svg_media[] = "image/svg+xml";
    project_bundle_asset bad_svg = {{cel_name, sizeof(cel_name) - 1U},
        {svg_media, sizeof(svg_media) - 1U},
        {bad_svg_asset, sizeof(bad_svg_asset) - 1U}};
    project_bundle_input bad_drawing = {
        .manifest_json = {manifest, sizeof(manifest) - 1U},
        .scene_svg = {drawing_scene, sizeof(drawing_scene) - 1U},
        .timeline_json = {drawing_timeline, sizeof(drawing_timeline) - 1U},
        .assets = &bad_svg,
        .asset_count = 1U
    };
    saved = project_file_save_as((base_bytes){(const uint8_t *)path, strlen(path)},
        &bad_drawing);
    CHECK(saved.error == PROJECT_FILE_OK && saved.published);
    load_status = viewer_load_document(path, NULL, &loaded);
    CHECK(load_status == VIEWER_LOAD_FAILED && loaded.source_owner == NULL &&
        loaded.shape_error != SVG_SHAPE_ERROR_NONE &&
        strstr(loaded.load_error, "drawing asset cel0 invalid") != NULL);
    viewer_loaded_document_destroy(&loaded);

    fd = open(path, O_RDWR);
    CHECK(fd >= 0);
    uint8_t corrupt = 0xffU;
    off_t scene_crc_offset = (off_t)(PROJECT_BUNDLE_HEADER_BYTES + sizeof(manifest) - 1U - 1U);
    CHECK(pwrite(fd, &corrupt, 1U, scene_crc_offset) == 1);
    CHECK(close(fd) == 0);
    load_status = viewer_load_document(path, NULL, &loaded);
    if (load_status != VIEWER_LOAD_FAILED) (void)fprintf(stderr, "corrupt load=%d error=%s\n", load_status, loaded.load_error);
    CHECK(load_status == VIEWER_LOAD_FAILED);
    CHECK(loaded.source_owner == NULL && loaded.load_error[0] != '\0');
    viewer_loaded_document_destroy(&loaded);
    CHECK(unlink(path) == 0);

    static const uint8_t svg[] = "<svg width='16' height='16'><rect width='1' height='1'/></svg>";
    uint8_t *owned = malloc(sizeof(svg) - 1U);
    CHECK(owned != NULL);
    memcpy(owned, svg, sizeof(svg) - 1U);
    CHECK(viewer_decode_owned_source(owned, sizeof(svg) - 1U, NULL, &loaded) ==
        VIEWER_LOAD_OK);
    CHECK(!loaded.native_project && loaded.source_owner == owned &&
        loaded.source == owned);
    viewer_loaded_document_destroy(&loaded);
    return 0;
}
