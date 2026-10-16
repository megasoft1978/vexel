#ifndef VEXEL_PROJECT_PROJECT_BUNDLE_ASSET_OVERLAY_H
#define VEXEL_PROJECT_PROJECT_BUNDLE_ASSET_OVERLAY_H

#include "project_bundle.h"

typedef struct project_bundle_asset_overlay {
    project_bundle_view *bundle;
    project_bundle_asset *assets;
    uint8_t **storage;
    size_t count;
    size_t capacity;
    size_t saved_count;
    size_t total_asset_bytes;
} project_bundle_asset_overlay;

bool project_bundle_asset_overlay_init(project_bundle_asset_overlay *overlay,
    project_bundle_view *bundle);
project_bundle_result project_bundle_asset_overlay_add_svg(
    project_bundle_asset_overlay *overlay, base_bytes name, base_bytes svg);
bool project_bundle_asset_overlay_is_dirty(
    const project_bundle_asset_overlay *overlay);
void project_bundle_asset_overlay_mark_saved(
    project_bundle_asset_overlay *overlay);
void project_bundle_asset_overlay_destroy(
    project_bundle_asset_overlay *overlay);

#endif
