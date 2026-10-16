#ifndef VEXEL_MCP_MCP_EXPORT_SNAPSHOT_H
#define VEXEL_MCP_MCP_EXPORT_SNAPSHOT_H

#include "project_bundle.h"

#define MCP_EXPORT_SNAPSHOT_MAX_SCENE_BYTES ((size_t)16777216U)
#define MCP_EXPORT_SNAPSHOT_MAX_ENCODED_BYTES ((size_t)67108864U)

typedef struct mcp_export_snapshot mcp_export_snapshot;

/*
 * Source slices and the bundle view must remain stable for the duration of
 * create. The returned snapshot owns its encoded storage and retains no source
 * pointers. A worker may decode/render its view while the snapshot remains
 * alive; join that worker before destroying the snapshot.
 */
project_bundle_result mcp_export_snapshot_create(
    const project_bundle_view *bundle, base_bytes current_scene_svg,
    base_bytes current_timeline_json, mcp_export_snapshot **out_snapshot);

/* The view borrows storage owned by snapshot and is valid until destroy. */
const project_bundle_view *mcp_export_snapshot_bundle(
    const mcp_export_snapshot *snapshot);

void mcp_export_snapshot_destroy(mcp_export_snapshot *snapshot);

#endif
