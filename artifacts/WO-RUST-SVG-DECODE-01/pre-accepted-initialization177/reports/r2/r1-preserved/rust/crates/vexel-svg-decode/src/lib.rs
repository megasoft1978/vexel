#![forbid(unsafe_code)]
//! Source-preserving SVG shape decoding primitives and resumable decoder.
//!
//! The decoder borrows a canonical lexical document, owns its scene builder,
//! and asks the composition layer to resolve paints, definitions, text, images,
//! and native pattern tiles through the accepted contracts.

mod decode;
mod geometry;
mod ignored;
mod numbers;
mod style;
mod transform;
mod viewport;

pub use decode::{Decoder, DecoderState};
pub use geometry::{element_geometry, Geometry};
pub use ignored::{ignored_kind_name, scan_ignored, traverse_attributes_supported};
pub use numbers::{
    next_number_separator, read_arc_flag, read_dimension, read_length, read_number,
    read_number_list, skip_number_space,
};
pub use transform::{affine_inverse, parse_transform, transform_node};
pub use viewport::{
    nested_viewport, parse_preserve_aspect, root_viewport, select_switch_child, symbol_mapping,
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
