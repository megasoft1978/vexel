//! FROZEN INTERFACE between the Rust scene encoder and the MSL kernels
//! (ADR-0035). Every struct is `#[repr(C)]` plain data made only of 4-byte
//! scalars (and arrays of them) so the C layout equals the MSL layout with no
//! padding. `shaders/layout.h` mirrors this file by hand; the tests below
//! parse it and compare sizes, field offsets and constants. Change both files
//! together, and bump [`LAYOUT_VERSION`].
//!
//! Scene buffers, all indexed by `u32`:
//! `GpuSceneHeader`, `GpuShape[shape_count]` (z-ordered, z == index),
//! `GpuEdge[edge_count]`, `GpuTile[tiles_x * tiles_y]` (row-major),
//! `GpuTileEntry[tile_entry_count]` tile lists (ascending z per tile),
//! `GpuGradient[gradient_count]`, `GpuStop[stop_count]`,
//! `GpuClip[clip_count]` (child records, see [`GpuClip`]).
//!
//! Strokes are expanded to filled polygons on the CPU (non-zero union), so a
//! shape is always an edge list plus a fill rule. `stroke_width` is
//! informational (0 for fills).
//!
//! Edge coordinates are fixed point with [`EDGE_FIXED_SHIFT`] fractional
//! bits, relative to the origin of the tile whose entry owns them (exact
//! integer translation), y-ascending not required (direction is the sign of
//! `y1 - y0`). A sample at `(px, py)` counts an edge with `y0 <= py < y1` or
//! `y1 <= py < y0` whose crossing is strictly right of `px`; up adds +1, down
//! -1. Each [`GpuTileEntry`] owns its own edge range: the encoder keeps the
//! edges that straddle the tile verbatim, drops those entirely left of it,
//! and collapses those entirely right of it into vertical unit edges on the
//! tile's right border (`x == TILE_SIZE << EDGE_FIXED_SHIFT`), so the kernel's
//! work is proportional to the edges actually in the tile. A shape's own
//! `edge_start` / `edge_count` are informational (source edge count; start is
//! 0): the kernel only reads tile entries.

/// Header `magic`.
pub const LAYOUT_MAGIC: u32 = 0x5658_4C31;
/// Bump on any change to a struct or constant here.
pub const LAYOUT_VERSION: u32 = 5;

pub const TILE_SIZE: u32 = 16;
/// Samples per pixel axis; equals `vexel_render::SAMPLES_PER_AXIS`.
pub const SAMPLES_PER_AXIS: u32 = 4;
pub const SAMPLE_COUNT: u32 = SAMPLES_PER_AXIS * SAMPLES_PER_AXIS;
/// Sample centres within a pixel per axis; equals `vexel_render` `SAMPLE_LOCATIONS`.
pub const SAMPLE_LOCATIONS: [f32; 4] = [0.125, 0.375, 0.625, 0.875];
/// Fractional bits of [`GpuEdge`] coordinates (1/4096 pixel).
pub const EDGE_FIXED_SHIFT: u32 = 12;
/// Largest absolute device coordinate (pixels) the fixed-point edges hold:
/// `MAX_COORDINATE << EDGE_FIXED_SHIFT` is 2^30, and the crossing products
/// of two such differences stay inside `i64`.
pub const MAX_COORDINATE: u32 = 262_144;

pub const MAX_SHAPES: u32 = 262_144;
pub const MAX_EDGES: u32 = 4_194_304;
/// Maximum number of [`GpuTileEntry`] records.
pub const MAX_TILE_LIST: u32 = 8_388_608;
pub const MAX_GRADIENTS: u32 = 65_536;
pub const MAX_STOPS: u32 = 262_144;
/// Deepest opacity-group nesting the GPU route renders; deeper is CPU only.
pub const MAX_GROUP_DEPTH: u32 = 4;
/// Longest clip chain per shape; the CPU limit is 256, longer is CPU only.
pub const MAX_CLIP_CHAIN: u32 = 4;
/// Most [`GpuClip`] child records in a scene.
pub const MAX_CLIP_RECORDS: u32 = 65_536;

pub const PAINT_SOLID: u32 = 0;
pub const PAINT_LINEAR: u32 = 1;
pub const PAINT_RADIAL: u32 = 2;
pub const FILL_NONZERO: u32 = 0;
pub const FILL_EVENODD: u32 = 1;
pub const SPREAD_PAD: u32 = 0;
pub const SPREAD_REFLECT: u32 = 1;
pub const SPREAD_REPEAT: u32 = 2;

/// Scene header, 80 bytes.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GpuSceneHeader {
    pub magic: u32,
    pub version: u32,
    pub width: u32,
    pub height: u32,
    pub tiles_x: u32,
    pub tiles_y: u32,
    pub shape_count: u32,
    pub edge_count: u32,
    pub tile_entry_count: u32,
    pub gradient_count: u32,
    pub stop_count: u32,
    pub clip_count: u32,
    pub flags: u32,
    /// Straight-alpha RGBA8 packed little-endian (r in the low byte).
    pub background_rgba: u32,
    pub reserved0: u32,
    pub reserved1: u32,
    /// Content rectangle (the CPU `ViewMap` content rectangle, meet
    /// letterboxing) as a half-open range of sample indices, where sample
    /// column `c * SAMPLES_PER_AXIS + sx` is at x = (index + 0.5) / 4 px:
    /// a sample is drawn only when `content_x0 <= column < content_x1` and
    /// `content_y0 <= row < content_y1`. Exactly the CPU test
    /// `offset <= sample < offset + size`. Full canvas: `0, 0, 4w, 4h`.
    pub content_x0: u32,
    pub content_y0: u32,
    pub content_x1: u32,
    pub content_y1: u32,
}

/// One shape record, 64 bytes.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct GpuShape {
    /// Device-space bounds `[min_x, min_y, max_x, max_y]`.
    pub bbox: [f32; 4],
    pub z: u32,
    /// `PAINT_*`.
    pub paint_kind: u32,
    /// Solid colour, straight-alpha RGBA8 packed little-endian.
    pub color_rgba: u32,
    /// Group-folded opacity in `0..=1`.
    pub opacity: f32,
    /// `FILL_*`.
    pub fill_rule: u32,
    pub stroke_width: f32,
    pub edge_start: u32,
    pub edge_count: u32,
    /// Index into the gradient array, or `u32::MAX` for solid.
    pub gradient_index: u32,
    /// First [`GpuClip`] record of the clip chain (see [`GpuClip`]); the
    /// number of chain links is `clip_count <= MAX_CLIP_CHAIN`.
    pub clip_start: u32,
    pub clip_count: u32,
    /// Opacity-group nesting depth, `<= MAX_GROUP_DEPTH`.
    pub group_depth: u32,
}

/// One edge, 16 bytes, tile-local fixed point (see module docs).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GpuEdge {
    pub x0: i32,
    pub y0: i32,
    pub x1: i32,
    pub y1: i32,
}

/// Per-tile list header, 8 bytes: a range into the [`GpuTileEntry`] list.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GpuTile {
    pub list_start: u32,
    pub list_count: u32,
}

/// Tile list entry, 12 bytes: one shape in one tile with that tile's edges
/// (a range of the edge array, tile-local coordinates, see module docs).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GpuTileEntry {
    pub shape: u32,
    pub edge_start: u32,
    pub edge_count: u32,
}

/// Gradient record, 48 bytes.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct GpuGradient {
    /// Affine `[a, b, c, d, e, f]` from device space to gradient space
    /// (linear: unit axis, radial: unit circle).
    pub inverse: [f32; 6],
    /// `SPREAD_*`.
    pub spread: u32,
    pub stop_start: u32,
    pub stop_count: u32,
    pub focal_ratio: f32,
    pub reserved0: u32,
    pub reserved1: u32,
}

/// Gradient stop, 20 bytes, straight-alpha channels in `0..=255`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct GpuStop {
    pub offset: f32,
    pub rgba: [f32; 4],
}

/// Clip child record, 16 bytes: one filled shape with its own rule.
///
/// A shape's clip is a chain of `GpuShape::clip_count` links, stored as
/// consecutive child records from `GpuShape::clip_start`. A link is the UNION
/// of its children: its first child has `union_size >= 1` (the number of
/// consecutive child records in the link, itself included), the others have
/// `union_size == 0`. A sample is clipped in when it is inside the union of
/// every link (INTERSECTION along the chain). The kernel walks
/// `i = clip_start; repeat clip_count times { size = clip[i].union_size;
/// children i..i+size; i += size }`. Chains may be shared between shapes.
///
/// Child edges are NOT per tile: `edge_start` / `edge_count` index the same
/// edge array as the tile pools (after them, so `header.edge_count` covers
/// both) with canvas-space coordinates; the kernel subtracts the tile origin.
/// A clip lane that profiles badly may add per-tile clip ranges later.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GpuClip {
    pub edge_start: u32,
    pub edge_count: u32,
    pub fill_rule: u32,
    pub union_size: u32,
}

/// Convert device pixels to [`GpuEdge`] fixed point; `None` if not finite or
/// out of `i32` range.
pub fn to_fixed(value: f64) -> Option<i32> {
    let scaled = (value * f64::from(1u32 << EDGE_FIXED_SHIFT)).round();
    if scaled.is_finite() && scaled >= f64::from(i32::MIN) && scaled <= f64::from(i32::MAX) {
        Some(scaled as i32)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::{align_of, offset_of, size_of};
    use std::collections::BTreeMap;

    #[test]
    fn sizes_and_alignments() {
        assert_eq!(
            (size_of::<GpuSceneHeader>(), align_of::<GpuSceneHeader>()),
            (80, 4)
        );
        assert_eq!((size_of::<GpuShape>(), align_of::<GpuShape>()), (64, 4));
        assert_eq!((size_of::<GpuEdge>(), align_of::<GpuEdge>()), (16, 4));
        assert_eq!((size_of::<GpuTile>(), align_of::<GpuTile>()), (8, 4));
        assert_eq!(
            (size_of::<GpuTileEntry>(), align_of::<GpuTileEntry>()),
            (12, 4)
        );
        assert_eq!(
            (size_of::<GpuGradient>(), align_of::<GpuGradient>()),
            (48, 4)
        );
        assert_eq!((size_of::<GpuStop>(), align_of::<GpuStop>()), (20, 4));
        assert_eq!((size_of::<GpuClip>(), align_of::<GpuClip>()), (16, 4));
    }

    #[test]
    fn key_offsets() {
        assert_eq!(offset_of!(GpuShape, z), 16);
        assert_eq!(offset_of!(GpuShape, edge_start), 40);
        assert_eq!(offset_of!(GpuShape, group_depth), 60);
        assert_eq!(offset_of!(GpuGradient, spread), 24);
        assert_eq!(offset_of!(GpuClip, union_size), 12);
        assert_eq!(offset_of!(GpuSceneHeader, content_x0), 64);
    }

    #[test]
    fn sample_pattern_matches_render() {
        assert_eq!(SAMPLES_PER_AXIS as usize, vexel_render::SAMPLES_PER_AXIS);
        for (i, loc) in SAMPLE_LOCATIONS.iter().enumerate() {
            assert_eq!(f64::from(*loc), (2 * i + 1) as f64 / 8.0);
        }
    }

    #[test]
    fn limits_fit_u32_indices() {
        assert!(u64::from(MAX_SHAPES) * size_of::<GpuShape>() as u64 <= 1 << 28);
        assert!(u64::from(MAX_EDGES) * size_of::<GpuEdge>() as u64 <= 1 << 27);
        assert!(u64::from(MAX_TILE_LIST) * size_of::<GpuTileEntry>() as u64 <= 1 << 27);
        assert!(MAX_CLIP_CHAIN <= 4 && MAX_GROUP_DEPTH == 4);
    }

    #[test]
    fn fixed_point() {
        assert_eq!(to_fixed(1.5), Some(6144));
        assert_eq!(
            to_fixed(f64::from(MAX_COORDINATE)),
            Some(1 << (EDGE_FIXED_SHIFT + 18))
        );
        assert_eq!(to_fixed(f64::NAN), None);
        assert_eq!(to_fixed(1e30), None);
    }

    /// (field, offset) per struct plus its size, from Rust.
    fn rust_layouts() -> BTreeMap<&'static str, (usize, Vec<(&'static str, usize)>)> {
        let mut m = BTreeMap::new();
        macro_rules! s {
            ($t:ty, $name:expr, [$($f:ident),*]) => {
                m.insert($name, (size_of::<$t>(), vec![$((stringify!($f), offset_of!($t, $f))),*]));
            };
        }
        s!(
            GpuSceneHeader,
            "GpuSceneHeader",
            [
                magic,
                version,
                width,
                height,
                tiles_x,
                tiles_y,
                shape_count,
                edge_count,
                tile_entry_count,
                gradient_count,
                stop_count,
                clip_count,
                flags,
                background_rgba,
                reserved0,
                reserved1,
                content_x0,
                content_y0,
                content_x1,
                content_y1
            ]
        );
        s!(
            GpuShape,
            "GpuShape",
            [
                bbox,
                z,
                paint_kind,
                color_rgba,
                opacity,
                fill_rule,
                stroke_width,
                edge_start,
                edge_count,
                gradient_index,
                clip_start,
                clip_count,
                group_depth
            ]
        );
        s!(GpuEdge, "GpuEdge", [x0, y0, x1, y1]);
        s!(GpuTile, "GpuTile", [list_start, list_count]);
        s!(
            GpuTileEntry,
            "GpuTileEntry",
            [shape, edge_start, edge_count]
        );
        s!(
            GpuGradient,
            "GpuGradient",
            [
                inverse,
                spread,
                stop_start,
                stop_count,
                focal_ratio,
                reserved0,
                reserved1
            ]
        );
        s!(GpuStop, "GpuStop", [offset, rgba]);
        s!(
            GpuClip,
            "GpuClip",
            [edge_start, edge_count, fill_rule, union_size]
        );
        m
    }

    /// Parse `struct Name { type name[N]?; ... };` bodies of the MSL header
    /// into (size, [(field, offset)]) assuming 4-byte scalars.
    fn parse_msl(src: &str) -> BTreeMap<String, (usize, Vec<(String, usize)>)> {
        let mut out = BTreeMap::new();
        let mut rest = src;
        while let Some(at) = rest.find("\nstruct ") {
            rest = &rest[at + 8..];
            let open = rest.find('{').unwrap();
            let name = rest[..open].trim().to_string();
            let close = rest.find("};").unwrap();
            let mut offset = 0usize;
            let mut fields = Vec::new();
            for decl in rest[open + 1..close].split(';') {
                let decl = decl.trim();
                if decl.is_empty() {
                    continue;
                }
                let mut parts = decl.split_whitespace();
                let ty = parts.next().unwrap();
                assert!(matches!(ty, "uint" | "int" | "float"), "{name}: type {ty}");
                let field = parts.next().unwrap();
                let (field, count) = match field.split_once('[') {
                    Some((f, n)) => (f, n.trim_end_matches(']').parse::<usize>().unwrap()),
                    None => (field, 1),
                };
                fields.push((field.to_string(), offset));
                offset += 4 * count;
            }
            out.insert(name, (offset, fields));
            rest = &rest[close + 2..];
        }
        out
    }

    #[test]
    fn msl_header_matches_rust_layout() {
        let src = include_str!("../shaders/layout.h");
        let msl = parse_msl(src);
        let rust = rust_layouts();
        assert_eq!(
            msl.keys().map(String::as_str).collect::<Vec<_>>(),
            rust.keys().copied().collect::<Vec<_>>()
        );
        for (name, (size, fields)) in &rust {
            let (msl_size, msl_fields) = &msl[*name];
            assert_eq!(msl_size, size, "{name} size");
            let rust_fields: Vec<(String, usize)> =
                fields.iter().map(|(f, o)| (f.to_string(), *o)).collect();
            assert_eq!(msl_fields, &rust_fields, "{name} fields");
        }

        let defines: BTreeMap<&str, u64> = src
            .lines()
            .filter_map(|l| l.strip_prefix("#define VX_"))
            .filter_map(|l| {
                let (k, v) = l.split_once(' ')?;
                let v = v.trim();
                let v = match v.strip_prefix("0x") {
                    Some(h) => u64::from_str_radix(h, 16).ok()?,
                    None => v.parse().ok()?,
                };
                Some((k, v))
            })
            .collect();
        let expect: [(&str, u64); 23] = [
            ("LAYOUT_MAGIC", LAYOUT_MAGIC.into()),
            ("LAYOUT_VERSION", LAYOUT_VERSION.into()),
            ("TILE_SIZE", TILE_SIZE.into()),
            ("SAMPLES_PER_AXIS", SAMPLES_PER_AXIS.into()),
            ("SAMPLE_COUNT", SAMPLE_COUNT.into()),
            ("EDGE_FIXED_SHIFT", EDGE_FIXED_SHIFT.into()),
            ("MAX_COORDINATE", MAX_COORDINATE.into()),
            ("MAX_SHAPES", MAX_SHAPES.into()),
            ("MAX_EDGES", MAX_EDGES.into()),
            ("MAX_TILE_LIST", MAX_TILE_LIST.into()),
            ("MAX_GRADIENTS", MAX_GRADIENTS.into()),
            ("MAX_STOPS", MAX_STOPS.into()),
            ("MAX_GROUP_DEPTH", MAX_GROUP_DEPTH.into()),
            ("MAX_CLIP_CHAIN", MAX_CLIP_CHAIN.into()),
            ("MAX_CLIP_RECORDS", MAX_CLIP_RECORDS.into()),
            ("PAINT_SOLID", PAINT_SOLID.into()),
            ("PAINT_LINEAR", PAINT_LINEAR.into()),
            ("PAINT_RADIAL", PAINT_RADIAL.into()),
            ("FILL_NONZERO", FILL_NONZERO.into()),
            ("FILL_EVENODD", FILL_EVENODD.into()),
            ("SPREAD_PAD", SPREAD_PAD.into()),
            ("SPREAD_REFLECT", SPREAD_REFLECT.into()),
            ("SPREAD_REPEAT", SPREAD_REPEAT.into()),
        ];
        for (k, v) in &expect {
            assert_eq!(defines.get(k), Some(v), "#define VX_{k}");
        }
        assert_eq!(defines.len(), 23);
        assert!(src.contains("{0.125f, 0.375f, 0.625f, 0.875f}"));
    }
}
