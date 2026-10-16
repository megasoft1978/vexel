//! Scene data of the render-mask fixtures, dumped from the C reference
//! (`svg_shape_decode` on rust/fixtures/render-mask/*.svg with the frozen
//! `svgworkbench` build recorded in manifest.json). Edges are the document-space
//! `doc->edges` slice of each shape; `transform_inverse` is C
//! `shape_stroke_inverse`; groups, masks (with their own content scene) and
//! clips are the C `opacity_groups`, `masks` and `clips` tables, offsets
//! rebased to their shape.
#![allow(dead_code)]

pub struct ShapeData {
    pub evenodd: bool,
    pub filled: bool,
    pub stroke_closed: bool,
    pub fill: [u8; 4],
    pub stroke: [u8; 4],
    pub stroke_width: f64,
    pub miter_limit: f64,
    pub cap: i32,
    pub join: i32,
    pub transform_inverse: [f64; 6],
    pub clip: Option<usize>,
    pub subpaths: &'static [(usize, usize, bool)],
    pub edges: &'static [[f64; 4]],
}

pub struct ClipData {
    pub object_bbox: bool,
    pub bounds: [f64; 4],
    pub user_inverse: [f64; 6],
    pub parent: Option<usize>,
    pub shapes: &'static [(usize, usize, bool, bool)],
    pub edges: &'static [[f64; 4]],
}

pub struct GroupData {
    pub first: usize,
    pub end: usize,
    pub parent: Option<usize>,
    pub opacity: f64,
    pub mask: Option<usize>,
}

pub struct MaskData {
    pub luminance: bool,
    pub region: [f64; 4],
    pub region_transform: [f64; 6],
    pub content: DocData,
}

pub struct DocData {
    pub shapes: &'static [ShapeData],
    pub groups: &'static [GroupData],
    pub masks: &'static [MaskData],
}

pub struct Case {
    pub id: &'static str,
    pub width: u32,
    pub height: u32,
    pub view: [f64; 4],
    pub doc: DocData,
    pub clips: &'static [ClipData],
}

pub const CASES: &[&Case] = &[
    &crate::group_cases_a::GROUP_OPACITY,
    &crate::group_cases_a::NESTED_OPACITY,
    &crate::group_cases_a::CLIP_GROUP_OBJECT_BBOX,
    &crate::group_cases_a::CLIP_GROUP_USERSPACE,
    &crate::group_cases_b::MASK_LUMINANCE,
    &crate::group_cases_b::MASK_ALPHA,
    &crate::group_cases_b::MASK_USERSPACE,
    &crate::group_cases_c::MASK_TRANSFORMED_GROUP,
    &crate::group_cases_c::MASK_OPACITY_COMBINED,
];
