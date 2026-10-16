#![forbid(unsafe_code)]
//! Source-preserving SVG shape decoding primitives and resumable decoder.
//!
//! The decoder borrows a canonical lexical document, owns its scene builder,
//! and asks the composition layer to resolve paints, definitions, text, images,
//! and native pattern tiles through the accepted contracts.

mod clip_chain;
mod decode;
mod definition_finish;
mod definition_links;
mod definition_work;
mod geometry;
mod groups;
mod ignored;
mod numbers;
mod operations;
mod plan_validation;
mod publication;
mod session;
pub mod source;
mod storage;
mod style;
mod tasks;
mod transform;
mod traversal;
mod use_expansion;
mod viewport;

pub use decode::{Decoder, DecoderState};
pub use geometry::{
    element_geometry, element_geometry_controlled, empty_path, path_options, Geometry,
    GeometryBuilder,
};
pub use ignored::{ignored_kind_name, scan_ignored, traverse_attributes_supported, IgnoredSink};
pub use numbers::{
    next_number_separator, read_arc_flag, read_dimension, read_length, read_length_controlled,
    read_number, read_number_controlled, read_number_list, read_number_list_controlled,
    skip_number_space, NumberReader,
};
pub use publication::{
    append_custom_element, append_custom_element_production, append_geometry,
    append_geometry_production, geometry_storage,
};
pub use style::{
    apply_vector_effect, multiply_alpha, paint_server_value, parse_dasharray, parse_opacity_value,
    parse_opacity_value_controlled, parse_style, parse_style_color, style_display_none,
    uniform_scale, ResolvedStyle,
};
pub use transform::{affine_inverse, parse_transform, parse_transform_controlled, transform_node};
pub use viewport::{
    language_range_matches, nested_viewport, nested_viewport_controlled, parse_preserve_aspect,
    root_viewport, root_viewport_controlled, select_switch_child, select_switch_child_controlled,
    switch_child_matches, symbol_attributes_supported, symbol_mapping, symbol_mapping_controlled,
    viewbox_transform,
};

use vexel_scene::{Diagnostic, ErrorKind, LimitKind};

/// Match two source spans without normalizing or allocating.
pub fn span_eq(source: &[u8], span: vexel_xml::XmlSpan, literal: &[u8]) -> bool {
    span.slice(source) == Some(literal)
}

/// Reject coordinates outside the renderer's frozen finite-coordinate bound.
pub fn point_bounded(point: vexel_geometry::Point) -> bool {
    point.x.is_finite()
        && point.y.is_finite()
        && point.x.abs() <= 1_000_000.0
        && point.y.abs() <= 1_000_000.0
}

/// Error constructor shared by all source-aware decode helpers.
pub fn failure(kind: ErrorKind, offset: usize) -> Diagnostic {
    Diagnostic { kind, offset }
}

/// Transform aware cubic tolerance, preserving the original 0.25 device-pixel rule.
pub fn transformed_flatness_tolerance(transform: vexel_geometry::Affine) -> f64 {
    vexel_svg_path::transformed_tolerance(transform)
}

pub(crate) fn invalid(offset: usize) -> Diagnostic {
    Diagnostic {
        kind: ErrorKind::InvalidArgument,
        offset,
    }
}
pub fn map_path_error(error: vexel_svg_path::Diagnostic) -> Diagnostic {
    use vexel_svg_path::DiagnosticKind as P;
    let kind = match error.kind {
        P::InvalidSpan => ErrorKind::InvalidArgument,
        P::InvalidTransform => ErrorKind::Number,
        P::Path => ErrorKind::Path,
        P::Unsupported => ErrorKind::Unsupported,
        P::Number => ErrorKind::Number,
        P::Limit => ErrorKind::Limit(LimitKind::PathCommands),
        P::OutOfMemory => ErrorKind::OutOfMemory,
    };
    Diagnostic {
        kind,
        offset: error.offset,
    }
}

/// Explicit accepted-interface gaps; these are not SVG unsupported-feature outcomes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContractBlocker {
    DefinitionOwnerBeforeGeometry,
    PatternPaintHandoff,
    MaskPostTarget,
    DefinitionPaintChannel,
}

/// Preserve cancellation and limit diagnostics when translating numeric offsets.
pub(crate) fn numeric_error(error: Diagnostic, offset: usize) -> Diagnostic {
    if error.kind == ErrorKind::Number {
        failure(ErrorKind::Number, offset)
    } else {
        error
    }
}
