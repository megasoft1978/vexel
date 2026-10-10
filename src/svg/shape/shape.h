#ifndef VEXEL_SVG_SHAPE_H
#define VEXEL_SVG_SHAPE_H

#include "base.h"
#include "geometry.h"
#include "media.h"
#include "../animation/animation.h"
#include "xml_tokenizer.h"

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#define SVG_SHAPE_MAX_EDGES 262144U
#define SVG_SHAPE_MAX_COMMANDS 65536U
#define SVG_SHAPE_MAX_DASH_VALUES 16U
#define SVG_SHAPE_MAX_PATH_NUMBERS (SVG_SHAPE_MAX_COMMANDS * 7U)

typedef enum svg_shape_linecap { SVG_SHAPE_CAP_BUTT, SVG_SHAPE_CAP_SQUARE, SVG_SHAPE_CAP_ROUND } svg_shape_linecap;
typedef enum svg_shape_linejoin { SVG_SHAPE_JOIN_MITER, SVG_SHAPE_JOIN_BEVEL, SVG_SHAPE_JOIN_ROUND } svg_shape_linejoin;
typedef struct svg_shape_stroke {
    uint8_t rgba[4];
    double width;
    double miter_limit;
    double dash_offset;
    double dash[SVG_SHAPE_MAX_DASH_VALUES];
    uint8_t dash_count;
    svg_shape_linecap cap;
    svg_shape_linejoin join;
    bool paint_server;
    bool crisp_edges;
    uint8_t paint_order;
} svg_shape_stroke;
typedef struct svg_shape_path_subpath {
    size_t first_edge;
    size_t edge_count; /* includes the implicit fill-closing edge */
    bool closed;       /* whether closepath explicitly closes the stroke path */
} svg_shape_path_subpath;
#define SVG_SHAPE_MAX_FILTER_PRIMITIVES 16U
#define SVG_SHAPE_MAX_FILTER_MERGE_INPUTS 8U
typedef enum svg_shape_filter_kind {
    SVG_SHAPE_FILTER_GAUSSIAN_BLUR,
    SVG_SHAPE_FILTER_OFFSET,
    SVG_SHAPE_FILTER_FLOOD,
    SVG_SHAPE_FILTER_BLEND,
    SVG_SHAPE_FILTER_COMPOSITE,
    SVG_SHAPE_FILTER_MERGE,
    SVG_SHAPE_FILTER_COLOR_MATRIX,
    SVG_SHAPE_FILTER_DROP_SHADOW
} svg_shape_filter_kind;

#define SVG_SHAPE_FILTER_INPUT_PREVIOUS (-1)
#define SVG_SHAPE_FILTER_INPUT_GRAPHIC (-2)
#define SVG_SHAPE_FILTER_INPUT_ALPHA (-3)
typedef struct svg_shape_filter_primitive {
    svg_shape_filter_kind kind;
    int32_t input;
    int32_t input2;
    int32_t merge_inputs[SVG_SHAPE_MAX_FILTER_MERGE_INPUTS];
    uint8_t merge_count;
    uint8_t mode;
    uint8_t rgba[4];
    double values[20];
    size_t source_offset;
} svg_shape_filter_primitive;

typedef struct svg_shape_filter {
    size_t first_primitive;
    size_t primitive_count;
    bool linear_rgb;
    bool filter_units_object_bounding_box;
    bool primitive_units_object_bounding_box;
    bool custom_region;
    double region[4];
} svg_shape_filter;

typedef struct svg_shape_document svg_shape_document;
#define SVG_SHAPE_MAX_MASKS 4096U
#define SVG_SHAPE_NO_MASK SIZE_MAX
typedef struct svg_shape_mask {
    svg_shape_document *content;
    bool luminance;
    bool region_object_bounding_box;
    bool custom_region;
    double region[4];
    geom_affine region_transform;
    size_t source_offset;
} svg_shape_mask;

#define SVG_SHAPE_MAX_PATTERNS 4096U
#define SVG_SHAPE_MAX_PATTERN_BYTES (16U * 1024U * 1024U)
#define SVG_SHAPE_NO_PATTERN SIZE_MAX
typedef struct svg_shape_pattern {
    uint8_t *rgba;
    size_t byte_count;
    uint32_t width;
    uint32_t height;
    size_t definition_element;
    double tile_width;
    double tile_height;
    geom_affine content_transform;
} svg_shape_pattern;

#define SVG_SHAPE_NO_FILTER SIZE_MAX
typedef struct svg_shape_opacity_group {
    size_t first_shape;
    size_t end_shape;
    size_t parent;
    double opacity;
    size_t filter_index;
    size_t mask_index;
    uint8_t blend_mode;
    bool isolation;
} svg_shape_opacity_group;
#define SVG_SHAPE_NO_GROUP SIZE_MAX
typedef struct svg_shape_aspect {
    double align_x;
    double align_y;
    bool none;
    bool slice;
    bool set;
} svg_shape_aspect;

typedef struct svg_shape_paint svg_shape_paint;
typedef struct svg_paint_stop svg_paint_stop;
typedef struct svg_shape_clip svg_shape_clip;

typedef struct svg_shape_source_map {
    size_t offset;
    size_t length;
    geom_affine transform;
} svg_shape_source_map;

typedef struct svg_shape_path_number {
    size_t offset;
    size_t length;
    double value;
} svg_shape_path_number;

typedef struct svg_shape_path_segment {
    char command; /* authored command, including relative/absolute case */
    size_t source_offset;
    size_t source_length;
    size_t command_offset; /* SIZE_MAX for an implicit repeated command */
    size_t first_number;
    size_t number_count;
    size_t first_edge;
    size_t edge_count;
    size_t shape_index;
} svg_shape_path_segment;

typedef struct svg_shape_image {
    const uint8_t *rgba; /* provider-owned premultiplied RGBA8 */
    size_t byte_count;
    size_t row_stride;
    uint32_t pixel_width;
    uint32_t pixel_height;
    void *owner;
    double x;
    double y;
    double width;
    double height;
    double draw_x;
    double draw_y;
    double draw_width;
    double draw_height;
} svg_shape_image;

typedef enum svg_shape_error {
    SVG_SHAPE_ERROR_NONE = 0,
    SVG_SHAPE_ERROR_INVALID_ARGUMENT,
    SVG_SHAPE_ERROR_XML,
    SVG_SHAPE_ERROR_UNSUPPORTED,
    SVG_SHAPE_ERROR_NUMBER,
    SVG_SHAPE_ERROR_LIMIT,
    SVG_SHAPE_ERROR_PATH,
    SVG_SHAPE_ERROR_VIEWPORT,
    SVG_SHAPE_ERROR_OOM,
    SVG_SHAPE_ERROR_UNSUPPORTED_FE_TURBULENCE,
    SVG_SHAPE_ERROR_UNSUPPORTED_FE_DISPLACEMENT_MAP,
    SVG_SHAPE_ERROR_UNSUPPORTED_FE_CONVOLVE_MATRIX,
    SVG_SHAPE_ERROR_UNSUPPORTED_FE_IMAGE,
    SVG_SHAPE_ERROR_UNSUPPORTED_FE_LIGHTING,
    SVG_SHAPE_ERROR_UNSUPPORTED_FE_MORPHOLOGY
} svg_shape_error;

typedef struct svg_shape_diagnostic { svg_shape_error category; size_t offset; } svg_shape_diagnostic;
struct svg_shape_document {
    uint32_t width;
    uint32_t height;
    double view_x, view_y, view_width, view_height;
    svg_shape_aspect preserve_aspect;
    size_t edge_count;
    geom_edge *edges;
    uint8_t *colors; /* one owned RGBA color per edge */
    uint32_t *shape_ids; /* edge-to-element map; preserves independent fills */
    size_t shape_count;
    bool *shape_evenodd; /* one fill rule per shape element */
    bool *shape_filled; /* false for open line geometry, which awaits stroke */
    bool *shape_stroke_closed;
    size_t *shape_edge_start;
    size_t *shape_edge_count;
    size_t *shape_subpath_start;
    size_t *shape_subpath_count;
    svg_shape_path_subpath *path_subpaths;
    size_t path_subpath_count;
    svg_shape_path_segment *path_segments;
    size_t path_segment_count;
    svg_shape_path_number *path_numbers;
    size_t path_number_count;
    svg_shape_source_map *shape_sources;
    svg_shape_image *images; /* one optional provider-owned image per shape */
    void *media_context;
    svg_shape_image_release_fn release_image;
    svg_shape_opacity_group *opacity_groups;
    size_t opacity_group_count;
    svg_shape_filter *filters;
    size_t filter_count;
    svg_shape_filter_primitive *filter_primitives;
    size_t filter_primitive_count;
    svg_shape_mask *masks;
    size_t mask_count;
    svg_shape_pattern *patterns;
    size_t pattern_count;
    size_t pattern_bytes;
    geom_affine *shape_stroke_inverse;
    svg_shape_stroke *strokes;
    svg_shape_paint *paints;
    svg_shape_paint *stroke_paints; /* allocated only for pattern strokes */
    size_t stroke_paint_capacity;
    svg_paint_stop *paint_stops;
    size_t paint_stop_count;
    svg_shape_clip **clips;
    bool has_clips;
    size_t clip_bytes;
    svg_anim_document animation;
    base_bytes source; /* borrowed; caller keeps source alive with document */
};

/* out owns decoded arrays and borrows source until document_destroy. Failure leaves out empty. */
svg_shape_error svg_shape_decode(base_bytes source, svg_shape_document *out,
                                 svg_shape_diagnostic *diagnostic);
/* Decodes conditional <switch> children for one injected BCP-47 language. */
svg_shape_error svg_shape_decode_with_system_language(
    base_bytes source, base_bytes system_language, svg_shape_document *out,
    svg_shape_diagnostic *diagnostic);
/* Additive media-aware decode entry point. The document retains decoded image
 * buffers through the provider's release callback and borrows source bytes. */
svg_shape_error svg_shape_decode_with_media(
    base_bytes source, const svg_shape_media_providers *providers,
    svg_shape_document *out, svg_shape_diagnostic *diagnostic);
/* Releases only owned arrays; the borrowed source remains caller-owned. */
void svg_shape_document_destroy(svg_shape_document *document);
const char *svg_shape_error_name(svg_shape_error error);

typedef struct svg_shape_rect {
    double x;
    double y;
    double width;
    double height;
} svg_shape_rect;

typedef struct svg_shape_selection {
    size_t shape_index;
    size_t source_offset;
    size_t source_length;
    svg_shape_rect user_bounds;
    svg_shape_rect device_bounds;
    geom_affine transform;
} svg_shape_selection;

/* Hit-test coordinates and tolerance use decoded document units. */
bool svg_shape_bounds_query(const svg_shape_document *document, size_t shape,
                            svg_shape_selection *out_selection);
bool svg_shape_hit_test(const svg_shape_document *document, double x, double y,
                        double tolerance, svg_shape_selection *out_selection);
/* Resolves an editor byte offset to the most specific rendered source element. */
bool svg_shape_element_at_offset(const svg_shape_document *document,
                                 size_t source_offset,
                                 svg_shape_selection *out_selection);

#endif
