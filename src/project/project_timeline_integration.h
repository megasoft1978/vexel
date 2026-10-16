#ifndef VEXEL_PROJECT_PROJECT_TIMELINE_INTEGRATION_H
#define VEXEL_PROJECT_PROJECT_TIMELINE_INTEGRATION_H

#include "project_timeline.h"
#include "project_drawing_asset.h"
#include "xml_tokenizer.h"

typedef enum project_timeline_compatibility {
    PROJECT_TIMELINE_COMPATIBILITY_EMPTY = 0,
    PROJECT_TIMELINE_COMPATIBILITY_NATIVE_V1,
    PROJECT_TIMELINE_COMPATIBILITY_OPAQUE
} project_timeline_compatibility;

typedef struct project_timeline_integration_result {
    project_timeline_compatibility compatibility;
    project_timeline_result validation;
    /* Nonempty only for a drawing asset parse failure; borrows bundle bytes. */
    project_drawing_asset_result asset_validation;
    base_bytes asset_name;
    bool canceled;
} project_timeline_integration_result;

/* Owns the decoded scene-ID arrays/pool and asset-reference array. Asset byte
 * slices still borrow from the encoded project bundle and must not outlive it. */
typedef struct project_timeline_context_owner {
    base_bytes *scene_ids;
    uint8_t *scene_id_pool;
    project_timeline_asset_ref *assets;
    project_timeline_context context;
} project_timeline_context_owner;

void project_timeline_context_owner_destroy(project_timeline_context_owner *owner);

/* Classifies before scene indexing: empty payload is an empty timeline, native v1
 * is validated against decoded SVG IDs and bundle assets, and other valid JSON
 * schemas/versions remain opaque byte-for-byte. Malformed JSON is an error.
 * Native validation accepts at most 1,000,000 IDs, each at most 256 decoded
 * bytes; source is bounded by the project scene/timeline limits. Cancellation is
 * polled during JSON scans, SVG visits, and native validation. The returned view
 * borrows timeline storage from bundle and is valid only while bundle is alive. */
project_timeline_integration_result project_timeline_validate_bundle(
    const project_bundle_view *bundle, base_bytes scene_svg,
    svg_xml_cancel_check_fn canceled, void *cancel_context,
    project_timeline_view *out_view);

/* Additive form that retains the validation context when known native v1 data
 * validates. Opaque/empty results leave `out_context` zeroed. */
project_timeline_integration_result project_timeline_validate_bundle_owned(
    const project_bundle_view *bundle, base_bytes scene_svg,
    svg_xml_cancel_check_fn canceled, void *cancel_context,
    project_timeline_view *out_view,
    project_timeline_context_owner *out_context);

#endif
