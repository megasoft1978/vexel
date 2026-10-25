// FROZEN INTERFACE (ADR-0035). Hand-mirrored from src/layout.rs; the Rust
// test `msl_header_matches_rust_layout` parses this file and compares every
// field offset, struct size and constant. Change both files together.
// Only 4-byte scalars and arrays of them, so C and MSL layouts agree.
#ifndef VEXEL_GPU_LAYOUT_H
#define VEXEL_GPU_LAYOUT_H

#define VX_LAYOUT_MAGIC 0x56584C31
#define VX_LAYOUT_VERSION 8
#define VX_TILE_SIZE 16
#define VX_SAMPLES_PER_AXIS 4
#define VX_SAMPLE_COUNT 16
#define VX_EDGE_FIXED_SHIFT 16
#define VX_MAX_COORDINATE 16384
#define VX_MAX_SHAPES 262144
#define VX_MAX_EDGES 4194304
#define VX_MAX_TILE_LIST 6291456
#define VX_MAX_GRADIENTS 65536
#define VX_MAX_STOPS 262144
#define VX_MAX_GROUP_DEPTH 4
#define VX_MAX_CLIP_CHAIN 4
#define VX_MAX_FRAMES 5
#define VX_FRAME_RUN 0x80000000
#define VX_MAX_CLIP_RECORDS 65536
#define VX_PAINT_SOLID 0
#define VX_PAINT_LINEAR 1
#define VX_PAINT_RADIAL 2
#define VX_FILL_NONZERO 0
#define VX_FILL_EVENODD 1
#define VX_SPREAD_PAD 0
#define VX_SPREAD_REFLECT 1
#define VX_SPREAD_REPEAT 2
#define VX_MAX_STROKE_PRIMS 2097152
#define VX_MAX_TILE_PRIMS 4096
#define VX_SHAPE_FILL 1
#define VX_SHAPE_STROKE 2
#define VX_PRIM_SEG 0x100
#define VX_PRIM_JOIN 0x200
#define VX_SEG_ROUND_A 1
#define VX_SEG_ROUND_B 2
#define VX_SEG_DOT 4
#define VX_SEG_DOT_SQUARE 8
#define VX_JOIN_ROUND 1
#define VX_JOIN_TRIANGLE 2
#define VX_JOIN_MITER 3

// Sample centres within a pixel, per axis (vexel-render SAMPLE_LOCATIONS).
constant float VX_SAMPLE_LOCATIONS[4] = {0.125f, 0.375f, 0.625f, 0.875f};

struct GpuSceneHeader {
    uint magic;
    uint version;
    uint width;
    uint height;
    uint tiles_x;
    uint tiles_y;
    uint shape_count;
    uint edge_count;
    uint tile_entry_count;
    uint gradient_count;
    uint stop_count;
    uint clip_count;
    uint flags;
    uint background_rgba;
    uint group_count;
    uint reserved1;
    uint content_x0;
    uint content_y0;
    uint content_x1;
    uint content_y1;
};

struct GpuShape {
    float bbox[4];
    uint z;
    uint paint_kind;
    uint color_rgba;
    float opacity;
    uint fill_rule;
    float stroke_width;
    uint edge_start;
    uint edge_count;
    uint gradient_index;
    uint clip_start;
    uint clip_count;
    uint group_depth;
    uint frame_count;
    uint frames[5];
    uint flags;
    uint stroke_paint_kind;
    uint stroke_color_rgba;
    uint stroke_gradient_index;
    float stroke_m[4];
};

struct GpuGroup {
    uint lut[64];
};

struct GpuEdge {
    int x0;
    int y0;
    int x1;
    int y1;
};

struct GpuTile {
    uint list_start;
    uint list_count;
};

struct GpuTileEntry {
    uint shape;
    uint edge_start;
    uint edge_count;
    uint prim_start;
    uint prim_count;
};

struct GpuStrokePrim {
    uint kind;
    float w[15];
};

struct GpuGradient {
    float inverse[6];
    uint spread;
    uint stop_start;
    uint stop_count;
    float focal_ratio;
    uint reserved0;
    uint reserved1;
};

struct GpuStop {
    float offset;
    float rgba[4];
};

struct GpuClip {
    uint edge_start;
    uint edge_count;
    uint fill_rule;
    uint union_size;
};

#endif
