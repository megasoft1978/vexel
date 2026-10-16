#include "mcp_export_snapshot.h"

#include <stdlib.h>

struct mcp_export_snapshot {
    uint8_t *encoded;
    project_bundle_view bundle;
};

static project_bundle_result snapshot_result(project_bundle_error error,
                                              size_t encoded_size)
{
    return (project_bundle_result){error, encoded_size, 0U};
}

static bool same_slice(base_bytes left, base_bytes right)
{
    return left.ptr == right.ptr && left.len == right.len;
}

static project_bundle_result validate_source_view(
    const project_bundle_view *source, project_bundle_view *decoded)
{
    if (source == NULL || decoded == NULL ||
        !base_bytes_valid(source->encoded))
        return snapshot_result(PROJECT_BUNDLE_INVALID_ARGUMENT, 0U);
    if (source->version != PROJECT_BUNDLE_VERSION)
        return snapshot_result(PROJECT_BUNDLE_UNSUPPORTED_VERSION, 0U);
    if (source->base_asset_count > source->asset_count)
        return snapshot_result(PROJECT_BUNDLE_INVALID_ARGUMENT, 0U);
    if (source->asset_count > PROJECT_BUNDLE_MAX_ASSET_COUNT ||
        source->base_asset_count > PROJECT_BUNDLE_MAX_ASSET_COUNT ||
        source->encoded.len > PROJECT_BUNDLE_MAX_BYTES)
        return snapshot_result(PROJECT_BUNDLE_LIMIT, 0U);
    if (source->encoded.len < PROJECT_BUNDLE_HEADER_BYTES)
        return snapshot_result(PROJECT_BUNDLE_CORRUPT, 0U);
    if (source->asset_count != source->base_asset_count &&
        source->overlay_assets == NULL)
        return snapshot_result(PROJECT_BUNDLE_INVALID_ARGUMENT, 0U);

    project_bundle_result result = project_bundle_decode(source->encoded,
                                                          decoded);
    if (result.error != PROJECT_BUNDLE_OK) return result;
    if (source->version != decoded->version ||
        source->base_asset_count != decoded->asset_count ||
        !same_slice(source->manifest_json, decoded->manifest_json) ||
        !same_slice(source->scene_svg, decoded->scene_svg) ||
        !same_slice(source->timeline_json, decoded->timeline_json))
        return snapshot_result(PROJECT_BUNDLE_INVALID_ARGUMENT, 0U);
    return snapshot_result(PROJECT_BUNDLE_OK, result.encoded_size);
}

project_bundle_result mcp_export_snapshot_create(
    const project_bundle_view *bundle, base_bytes current_scene_svg,
    base_bytes current_timeline_json, mcp_export_snapshot **out_snapshot)
{
    if (out_snapshot == NULL)
        return snapshot_result(PROJECT_BUNDLE_INVALID_ARGUMENT, 0U);
    *out_snapshot = NULL;
    if (!base_bytes_valid(current_scene_svg) ||
        !base_bytes_valid(current_timeline_json) || current_scene_svg.len == 0U)
        return snapshot_result(PROJECT_BUNDLE_INVALID_ARGUMENT, 0U);
    if (current_scene_svg.len > MCP_EXPORT_SNAPSHOT_MAX_SCENE_BYTES ||
        current_timeline_json.len > PROJECT_BUNDLE_MAX_TIMELINE_BYTES)
        return snapshot_result(PROJECT_BUNDLE_LIMIT, 0U);

    project_bundle_view validated = {0};
    project_bundle_result view_result = validate_source_view(bundle, &validated);
    if (view_result.error != PROJECT_BUNDLE_OK) return view_result;

    size_t asset_bytes = 0U;
    if (!base_size_mul(bundle->asset_count, sizeof(project_bundle_asset),
                       &asset_bytes))
        return snapshot_result(PROJECT_BUNDLE_LIMIT, 0U);
    project_bundle_asset *assets = NULL;
    if (asset_bytes != 0U) {
        assets = malloc(asset_bytes);
        if (assets == NULL)
            return snapshot_result(PROJECT_BUNDLE_OUT_OF_MEMORY, 0U);
    }
    for (size_t index = 0U; index < bundle->asset_count; ++index) {
        if (!project_bundle_asset_at(bundle, index, &assets[index])) {
            free(assets);
            return snapshot_result(PROJECT_BUNDLE_CORRUPT, 0U);
        }
    }

    project_bundle_input input = {
        .manifest_json = bundle->manifest_json,
        .scene_svg = current_scene_svg,
        .timeline_json = current_timeline_json,
        .assets = assets,
        .asset_count = bundle->asset_count
    };
    project_bundle_result measured = project_bundle_encoded_size(&input);
    if (measured.error != PROJECT_BUNDLE_OK) {
        free(assets);
        return measured;
    }
    if (measured.encoded_size > MCP_EXPORT_SNAPSHOT_MAX_ENCODED_BYTES) {
        free(assets);
        return snapshot_result(PROJECT_BUNDLE_LIMIT, measured.encoded_size);
    }

    mcp_export_snapshot *snapshot = malloc(sizeof(*snapshot));
    if (snapshot == NULL) {
        free(assets);
        return snapshot_result(PROJECT_BUNDLE_OUT_OF_MEMORY,
                               measured.encoded_size);
    }
    snapshot->encoded = malloc(measured.encoded_size);
    if (snapshot->encoded == NULL) {
        free(assets);
        free(snapshot);
        return snapshot_result(PROJECT_BUNDLE_OUT_OF_MEMORY,
                               measured.encoded_size);
    }
    project_bundle_result encoded = project_bundle_encode(&input,
        (base_mut_bytes){snapshot->encoded, measured.encoded_size});
    free(assets);
    if (encoded.error != PROJECT_BUNDLE_OK) {
        free(snapshot->encoded);
        free(snapshot);
        return encoded;
    }
    project_bundle_result decoded = project_bundle_decode(
        (base_bytes){snapshot->encoded, encoded.encoded_size},
        &snapshot->bundle);
    if (decoded.error != PROJECT_BUNDLE_OK) {
        free(snapshot->encoded);
        free(snapshot);
        return decoded;
    }
    *out_snapshot = snapshot;
    return encoded;
}

const project_bundle_view *mcp_export_snapshot_bundle(
    const mcp_export_snapshot *snapshot)
{
    return snapshot == NULL ? NULL : &snapshot->bundle;
}

void mcp_export_snapshot_destroy(mcp_export_snapshot *snapshot)
{
    if (snapshot == NULL) return;
    free(snapshot->encoded);
    free(snapshot);
}
