#ifndef VEXEL_PROJECT_PROJECT_BUNDLE_H
#define VEXEL_PROJECT_PROJECT_BUNDLE_H

#include "base.h"

#define PROJECT_BUNDLE_VERSION 1U
#define PROJECT_BUNDLE_HEADER_BYTES 56U
#define PROJECT_BUNDLE_MAX_BYTES ((size_t)1073741824U)
#define PROJECT_BUNDLE_MAX_MANIFEST_BYTES ((size_t)1048576U)
#define PROJECT_BUNDLE_MAX_SCENE_BYTES ((size_t)268435456U)
#define PROJECT_BUNDLE_MAX_TIMELINE_BYTES ((size_t)67108864U)
#define PROJECT_BUNDLE_MAX_ASSET_COUNT 1024U
#define PROJECT_BUNDLE_MAX_ASSET_NAME_BYTES 128U
#define PROJECT_BUNDLE_MAX_MEDIA_TYPE_BYTES 128U
#define PROJECT_BUNDLE_MAX_ASSET_BYTES ((size_t)268435456U)
#define PROJECT_BUNDLE_MAX_ASSET_TOTAL_BYTES ((size_t)536870912U)

typedef enum project_bundle_error {
    PROJECT_BUNDLE_OK = 0,
    PROJECT_BUNDLE_INVALID_ARGUMENT,
    PROJECT_BUNDLE_LIMIT,
    PROJECT_BUNDLE_BUFFER_TOO_SMALL,
    PROJECT_BUNDLE_BAD_MAGIC,
    PROJECT_BUNDLE_UNSUPPORTED_VERSION,
    PROJECT_BUNDLE_CORRUPT,
    PROJECT_BUNDLE_OUT_OF_MEMORY
} project_bundle_error;

typedef struct project_bundle_asset {
    base_bytes name;
    base_bytes media_type;
    base_bytes data;
} project_bundle_asset;

typedef struct project_bundle_input {
    base_bytes manifest_json;
    base_bytes scene_svg;
    base_bytes timeline_json;
    const project_bundle_asset *assets;
    size_t asset_count;
} project_bundle_input;

typedef struct project_bundle_view {
    uint32_t version;
    base_bytes manifest_json;
    base_bytes scene_svg;
    base_bytes timeline_json;
    base_bytes encoded;
    size_t asset_count;
    size_t base_asset_count;
    const project_bundle_asset *overlay_assets;
} project_bundle_view;

typedef struct project_bundle_result {
    project_bundle_error error;
    size_t encoded_size;
    size_t error_offset;
} project_bundle_result;

project_bundle_result project_bundle_encoded_size(const project_bundle_input *input);
project_bundle_result project_bundle_encode(const project_bundle_input *input,
                                            base_mut_bytes output);
project_bundle_result project_bundle_decode(base_bytes encoded,
                                            project_bundle_view *out_view);
bool project_bundle_has_magic(base_bytes encoded);
bool project_bundle_asset_at(const project_bundle_view *view, size_t index,
                             project_bundle_asset *out_asset);
const char *project_bundle_error_name(project_bundle_error error);

#endif
