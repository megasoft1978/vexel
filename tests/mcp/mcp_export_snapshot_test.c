#include "mcp_export_snapshot.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static unsigned failures;
#define CHECK(condition) do { if (!(condition)) { \
    (void)fprintf(stderr, "FAIL %s:%d: %s\n", __FILE__, __LINE__, #condition); \
    failures++; return false; } } while (0)

static base_bytes bytes(const char *text)
{
    return (base_bytes){(const uint8_t *)text, strlen(text)};
}

static bool equal_bytes(base_bytes left, base_bytes right)
{
    return left.len == right.len &&
        (left.len == 0U || memcmp(left.ptr, right.ptr, left.len) == 0);
}

static bool expect_create_error(const project_bundle_view *bundle,
                                base_bytes scene, base_bytes timeline,
                                project_bundle_error expected)
{
    mcp_export_snapshot *snapshot = (mcp_export_snapshot *)(uintptr_t)1U;
    project_bundle_result result = mcp_export_snapshot_create(bundle, scene,
        timeline, &snapshot);
    CHECK(result.error == expected && snapshot == NULL);
    return true;
}

static bool make_bundle(const project_bundle_asset *assets, size_t asset_count,
                        uint8_t **out_encoded, project_bundle_view *out_view)
{
    project_bundle_input input = {
        bytes("{\"vendor:extension\":{\"keep\":true}}"),
        bytes("<svg id='saved'/>") ,
        bytes("{\"saved\":true}"), assets, asset_count
    };
    project_bundle_result measured = project_bundle_encoded_size(&input);
    if (measured.error != PROJECT_BUNDLE_OK) return false;
    uint8_t *encoded = malloc(measured.encoded_size);
    if (encoded == NULL) return false;
    project_bundle_result written = project_bundle_encode(&input,
        (base_mut_bytes){encoded, measured.encoded_size});
    if (written.error != PROJECT_BUNDLE_OK ||
        project_bundle_decode((base_bytes){encoded, written.encoded_size},
                              out_view).error != PROJECT_BUNDLE_OK) {
        free(encoded);
        return false;
    }
    *out_encoded = encoded;
    return true;
}

static bool test_owned_current_project_with_overlay(void)
{
    static const uint8_t scene_source[] = "<svg><!--unsaved--></svg>";
    static const uint8_t timeline_source[] = "{\"unsaved\":true}";
    uint8_t *base_asset_data = malloc(5U);
    uint8_t *overlay_data = malloc(6U);
    uint8_t *scene = malloc(sizeof(scene_source) - 1U);
    uint8_t *timeline = malloc(sizeof(timeline_source) - 1U);
    CHECK(base_asset_data != NULL && overlay_data != NULL && scene != NULL &&
          timeline != NULL);
    memcpy(base_asset_data, "BASE!", 5U);
    memcpy(overlay_data, "LIVE!!", 6U);
    memcpy(scene, scene_source, sizeof(scene_source) - 1U);
    memcpy(timeline, timeline_source, sizeof(timeline_source) - 1U);
    const project_bundle_asset base_assets[] = {{
        bytes("base.png"), bytes("image/png"), {base_asset_data, 5U}
    }};
    uint8_t *source_encoded = NULL;
    project_bundle_view source = {0};
    CHECK(make_bundle(base_assets, 1U, &source_encoded, &source));
    size_t source_encoded_length = source.encoded.len;

    project_bundle_asset *overlay = malloc(sizeof(*overlay));
    CHECK(overlay != NULL);
    *overlay = (project_bundle_asset){bytes("draw.svg"), bytes("image/svg+xml"),
                                     {overlay_data, 6U}};
    source.asset_count = 2U;
    source.base_asset_count = 1U;
    source.overlay_assets = overlay;

    mcp_export_snapshot *snapshot = NULL;
    project_bundle_result created = mcp_export_snapshot_create(&source,
        (base_bytes){scene, sizeof(scene_source) - 1U},
        (base_bytes){timeline, sizeof(timeline_source) - 1U}, &snapshot);
    CHECK(created.error == PROJECT_BUNDLE_OK && snapshot != NULL);

    memset(base_asset_data, 0xa1, 5U);
    memset(overlay_data, 0xb2, 6U);
    memset(scene, 0xc3, sizeof(scene_source) - 1U);
    memset(timeline, 0xd4, sizeof(timeline_source) - 1U);
    memset(source_encoded, 0xe5, source_encoded_length);
    free(base_asset_data);
    free(overlay_data);
    free(scene);
    free(timeline);
    free(overlay);
    free(source_encoded);

    const project_bundle_view *owned = mcp_export_snapshot_bundle(snapshot);
    CHECK(owned != NULL && owned->version == PROJECT_BUNDLE_VERSION &&
          owned->asset_count == 2U && owned->base_asset_count == 2U &&
          owned->overlay_assets == NULL);
    CHECK(equal_bytes(owned->manifest_json,
        bytes("{\"vendor:extension\":{\"keep\":true}}")));
    CHECK(equal_bytes(owned->scene_svg,
                      (base_bytes){scene_source, sizeof(scene_source) - 1U}));
    CHECK(equal_bytes(owned->timeline_json, bytes("{\"unsaved\":true}")));
    project_bundle_asset actual = {0};
    CHECK(project_bundle_asset_at(owned, 0U, &actual));
    CHECK(equal_bytes(actual.name, bytes("base.png")) &&
          equal_bytes(actual.media_type, bytes("image/png")) &&
          equal_bytes(actual.data, bytes("BASE!")));
    CHECK(project_bundle_asset_at(owned, 1U, &actual));
    CHECK(equal_bytes(actual.name, bytes("draw.svg")) &&
          equal_bytes(actual.media_type, bytes("image/svg+xml")) &&
          equal_bytes(actual.data, bytes("LIVE!!")));
    mcp_export_snapshot_destroy(snapshot);
    CHECK(mcp_export_snapshot_bundle(NULL) == NULL);
    mcp_export_snapshot_destroy(NULL);
    return true;
}

static bool test_arguments_limits_and_repeated_lifecycle(void)
{
    uint8_t *encoded = NULL;
    project_bundle_view bundle = {0};
    CHECK(make_bundle(NULL, 0U, &encoded, &bundle));
    static const uint8_t scene[] = "<svg/>";
    mcp_export_snapshot *snapshot = (mcp_export_snapshot *)(uintptr_t)1U;
    project_bundle_result result = mcp_export_snapshot_create(&bundle,
        (base_bytes){scene, sizeof(scene) - 1U}, (base_bytes){NULL, 0U}, NULL);
    CHECK(result.error == PROJECT_BUNDLE_INVALID_ARGUMENT);
    result = mcp_export_snapshot_create(NULL,
        (base_bytes){scene, sizeof(scene) - 1U}, (base_bytes){NULL, 0U},
        &snapshot);
    CHECK(result.error == PROJECT_BUNDLE_INVALID_ARGUMENT && snapshot == NULL);
    result = mcp_export_snapshot_create(&bundle, (base_bytes){NULL, 1U},
        (base_bytes){NULL, 0U}, &snapshot);
    CHECK(result.error == PROJECT_BUNDLE_INVALID_ARGUMENT && snapshot == NULL);
    CHECK(expect_create_error(&bundle, (base_bytes){scene, 0U},
        (base_bytes){NULL, 0U}, PROJECT_BUNDLE_INVALID_ARGUMENT));
    CHECK(expect_create_error(&bundle, (base_bytes){scene, sizeof(scene) - 1U},
        (base_bytes){NULL, 1U}, PROJECT_BUNDLE_INVALID_ARGUMENT));
    result = mcp_export_snapshot_create(&bundle,
        (base_bytes){(const uint8_t *)(uintptr_t)1U,
                     MCP_EXPORT_SNAPSHOT_MAX_SCENE_BYTES + 1U},
        (base_bytes){NULL, 0U}, &snapshot);
    CHECK(result.error == PROJECT_BUNDLE_LIMIT && snapshot == NULL);

    project_bundle_view invalid = bundle;
    invalid.version = PROJECT_BUNDLE_VERSION + 1U;
    CHECK(expect_create_error(&invalid, (base_bytes){scene, sizeof(scene) - 1U},
        (base_bytes){NULL, 0U}, PROJECT_BUNDLE_UNSUPPORTED_VERSION));
    invalid = bundle;
    invalid.base_asset_count = 1U;
    invalid.asset_count = 0U;
    CHECK(expect_create_error(&invalid, (base_bytes){scene, sizeof(scene) - 1U},
        (base_bytes){NULL, 0U}, PROJECT_BUNDLE_INVALID_ARGUMENT));
    project_bundle_asset unused_overlay = {0};
    invalid = bundle;
    invalid.asset_count = PROJECT_BUNDLE_MAX_ASSET_COUNT + 1U;
    invalid.overlay_assets = &unused_overlay;
    CHECK(expect_create_error(&invalid, (base_bytes){scene, sizeof(scene) - 1U},
        (base_bytes){NULL, 0U}, PROJECT_BUNDLE_LIMIT));
    invalid = bundle;
    invalid.asset_count = 1U;
    invalid.base_asset_count = 0U;
    invalid.overlay_assets = NULL;
    CHECK(expect_create_error(&invalid, (base_bytes){scene, sizeof(scene) - 1U},
        (base_bytes){NULL, 0U}, PROJECT_BUNDLE_INVALID_ARGUMENT));
    static const uint8_t wrong_manifest[] = "different";
    invalid = bundle;
    invalid.manifest_json = (base_bytes){wrong_manifest,
                                         sizeof(wrong_manifest) - 1U};
    CHECK(expect_create_error(&invalid, (base_bytes){scene, sizeof(scene) - 1U},
        (base_bytes){NULL, 0U}, PROJECT_BUNDLE_INVALID_ARGUMENT));

    uint8_t *bad_encoded = malloc(bundle.encoded.len);
    CHECK(bad_encoded != NULL);
    memcpy(bad_encoded, bundle.encoded.ptr, bundle.encoded.len);
    invalid = bundle;
    bad_encoded[0] ^= 0xffU;
    invalid.encoded = (base_bytes){bad_encoded, bundle.encoded.len};
    CHECK(expect_create_error(&invalid, (base_bytes){scene, sizeof(scene) - 1U},
        (base_bytes){NULL, 0U}, PROJECT_BUNDLE_BAD_MAGIC));
    memcpy(bad_encoded, bundle.encoded.ptr, bundle.encoded.len);
    bad_encoded[8] = (uint8_t)(PROJECT_BUNDLE_VERSION + 1U);
    invalid = bundle;
    invalid.encoded = (base_bytes){bad_encoded, bundle.encoded.len};
    CHECK(expect_create_error(&invalid, (base_bytes){scene, sizeof(scene) - 1U},
        (base_bytes){NULL, 0U}, PROJECT_BUNDLE_UNSUPPORTED_VERSION));
    invalid = bundle;
    invalid.encoded.len = PROJECT_BUNDLE_HEADER_BYTES - 1U;
    CHECK(expect_create_error(&invalid, (base_bytes){scene, sizeof(scene) - 1U},
        (base_bytes){NULL, 0U}, PROJECT_BUNDLE_CORRUPT));
    invalid = bundle;
    invalid.encoded = (base_bytes){NULL, 1U};
    CHECK(expect_create_error(&invalid, (base_bytes){scene, sizeof(scene) - 1U},
        (base_bytes){NULL, 0U}, PROJECT_BUNDLE_INVALID_ARGUMENT));
    invalid = bundle;
    invalid.encoded = (base_bytes){(const uint8_t *)(uintptr_t)1U,
                                  PROJECT_BUNDLE_MAX_BYTES + 1U};
    CHECK(expect_create_error(&invalid, (base_bytes){scene, sizeof(scene) - 1U},
        (base_bytes){NULL, 0U}, PROJECT_BUNDLE_LIMIT));
    free(bad_encoded);

    project_bundle_asset hostile_overlay = {
        bytes("large.bin"), bytes("application/octet-stream"),
        {(const uint8_t *)(uintptr_t)1U, 48U * 1024U * 1024U}
    };
    bundle.asset_count = 1U;
    bundle.base_asset_count = 0U;
    bundle.overlay_assets = &hostile_overlay;
    uint8_t *large_scene = malloc(MCP_EXPORT_SNAPSHOT_MAX_SCENE_BYTES);
    CHECK(large_scene != NULL);
    result = mcp_export_snapshot_create(&bundle,
        (base_bytes){large_scene, MCP_EXPORT_SNAPSHOT_MAX_SCENE_BYTES},
        (base_bytes){NULL, 0U}, &snapshot);
    CHECK(result.error == PROJECT_BUNDLE_LIMIT && snapshot == NULL);
    free(large_scene);

    bundle.asset_count = 0U;
    bundle.base_asset_count = 0U;
    bundle.overlay_assets = NULL;
    for (size_t iteration = 0U; iteration < 32U; ++iteration) {
        snapshot = NULL;
        result = mcp_export_snapshot_create(&bundle,
            (base_bytes){scene, sizeof(scene) - 1U}, (base_bytes){NULL, 0U},
            &snapshot);
        CHECK(result.error == PROJECT_BUNDLE_OK && snapshot != NULL);
        CHECK(mcp_export_snapshot_bundle(snapshot)->asset_count == 0U);
        mcp_export_snapshot_destroy(snapshot);
    }
    free(encoded);
    return true;
}

int main(void)
{
    bool okay = test_owned_current_project_with_overlay() &&
        test_arguments_limits_and_repeated_lifecycle();
    if (!okay || failures != 0U) return 1;
    (void)puts("MCP export snapshot ownership and bounds passed");
    return 0;
}
