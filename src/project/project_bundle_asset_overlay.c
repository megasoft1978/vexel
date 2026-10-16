#include "project_bundle_asset_overlay.h"

#include "project_drawing_asset.h"

#include <stdlib.h>
#include <string.h>

static project_bundle_result overlay_result(project_bundle_error error,
    size_t offset)
{
    return (project_bundle_result){error, 0U, offset};
}

static bool name_valid(base_bytes name)
{
    if (!base_bytes_valid(name) || name.len == 0U ||
        name.len > PROJECT_BUNDLE_MAX_ASSET_NAME_BYTES ||
        (name.len == 1U && name.ptr[0] == '.') ||
        (name.len == 2U && name.ptr[0] == '.' && name.ptr[1] == '.')) return false;
    for (size_t i = 0U; i < name.len; ++i) {
        uint8_t c = name.ptr[i];
        if (!((c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') ||
              (c >= '0' && c <= '9') || c == '.' || c == '_' || c == '-'))
            return false;
    }
    return true;
}

bool project_bundle_asset_overlay_init(project_bundle_asset_overlay *overlay,
    project_bundle_view *bundle)
{
    if (overlay == NULL || bundle == NULL || bundle->version != PROJECT_BUNDLE_VERSION ||
        !base_bytes_valid(bundle->encoded) || bundle->asset_count >
            PROJECT_BUNDLE_MAX_ASSET_COUNT) return false;
    *overlay = (project_bundle_asset_overlay){.bundle = bundle};
    bundle->base_asset_count = bundle->asset_count;
    bundle->overlay_assets = NULL;
    for (size_t i = 0U; i < bundle->asset_count; ++i) {
        project_bundle_asset asset;
        if (!project_bundle_asset_at(bundle, i, &asset) ||
            !base_size_add(overlay->total_asset_bytes, asset.data.len,
                &overlay->total_asset_bytes)) {
            *overlay = (project_bundle_asset_overlay){0};
            return false;
        }
    }
    return true;
}

project_bundle_result project_bundle_asset_overlay_add_svg(
    project_bundle_asset_overlay *overlay, base_bytes name, base_bytes svg)
{
    static const uint8_t media_type[] = "image/svg+xml";
    if (overlay == NULL || overlay->bundle == NULL || !name_valid(name) ||
        !base_bytes_valid(svg) || svg.len == 0U ||
        svg.len > PROJECT_BUNDLE_MAX_ASSET_BYTES)
        return overlay_result(PROJECT_BUNDLE_INVALID_ARGUMENT, 0U);
    project_bundle_asset_overlay *state = overlay;
    for (size_t i = 0U; i < state->bundle->asset_count; ++i) {
        project_bundle_asset existing;
        if (!project_bundle_asset_at(state->bundle, i, &existing))
            return overlay_result(PROJECT_BUNDLE_CORRUPT, 0U);
        if (existing.name.len == name.len &&
            memcmp(existing.name.ptr, name.ptr, name.len) == 0)
            return overlay_result(PROJECT_BUNDLE_INVALID_ARGUMENT, 0U);
    }
    if (state->bundle->asset_count >= PROJECT_BUNDLE_MAX_ASSET_COUNT ||
        svg.len > PROJECT_BUNDLE_MAX_ASSET_TOTAL_BYTES - state->total_asset_bytes)
        return overlay_result(PROJECT_BUNDLE_LIMIT, 0U);
    bool canceled = false;
    project_drawing_asset_result validation = project_drawing_asset_validate(
        svg, NULL, NULL, &canceled);
    if (validation.error != SVG_SHAPE_ERROR_NONE)
        return overlay_result(PROJECT_BUNDLE_CORRUPT, validation.error_offset);
    if (state->count == state->capacity) {
        size_t capacity = state->capacity == 0U ? 4U : state->capacity * 2U;
        if (capacity > PROJECT_BUNDLE_MAX_ASSET_COUNT) capacity =
            PROJECT_BUNDLE_MAX_ASSET_COUNT;
        if (capacity <= state->capacity ||
            capacity > SIZE_MAX / sizeof(*state->assets) ||
            capacity > SIZE_MAX / sizeof(*state->storage))
            return overlay_result(PROJECT_BUNDLE_LIMIT, 0U);
        project_bundle_asset *assets = malloc(capacity * sizeof(*assets));
        uint8_t **storage = malloc(capacity * sizeof(*storage));
        if (assets == NULL || storage == NULL) {
            free(assets); free(storage);
            return overlay_result(PROJECT_BUNDLE_OUT_OF_MEMORY, 0U);
        }
        if (state->count != 0U) {
            memcpy(assets, state->assets, state->count * sizeof(*assets));
            memcpy(storage, state->storage, state->count * sizeof(*storage));
        }
        free(state->assets); free(state->storage);
        state->assets = assets; state->storage = storage;
        state->capacity = capacity;
    }
    size_t block_size = 0U;
    if (!base_size_add(name.len, svg.len, &block_size))
        return overlay_result(PROJECT_BUNDLE_LIMIT, 0U);
    uint8_t *block = malloc(block_size);
    if (block == NULL) return overlay_result(PROJECT_BUNDLE_OUT_OF_MEMORY, 0U);
    memcpy(block, name.ptr, name.len);
    memcpy(block + name.len, svg.ptr, svg.len);
    state->storage[state->count] = block;
    state->assets[state->count] = (project_bundle_asset){
        {block, name.len}, {media_type, sizeof(media_type) - 1U},
        {block + name.len, svg.len}};
    state->count++;
    state->total_asset_bytes += svg.len;
    state->bundle->asset_count++;
    state->bundle->overlay_assets = state->assets;
    return overlay_result(PROJECT_BUNDLE_OK, 0U);
}

bool project_bundle_asset_overlay_is_dirty(
    const project_bundle_asset_overlay *overlay)
{
    return overlay != NULL && overlay->count > overlay->saved_count;
}

void project_bundle_asset_overlay_mark_saved(
    project_bundle_asset_overlay *overlay)
{
    if (overlay != NULL) overlay->saved_count = overlay->count;
}

void project_bundle_asset_overlay_destroy(project_bundle_asset_overlay *overlay)
{
    if (overlay == NULL) return;
    if (overlay->storage != NULL)
        for (size_t i = 0U; i < overlay->count; ++i) free(overlay->storage[i]);
    if (overlay->bundle != NULL) {
        overlay->bundle->asset_count = overlay->bundle->base_asset_count;
        overlay->bundle->overlay_assets = NULL;
    }
    free(overlay->storage); free(overlay->assets);
    *overlay = (project_bundle_asset_overlay){0};
}
