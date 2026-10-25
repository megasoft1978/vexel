//! Client-facing reason text for a failed native render or export.
//!
//! Tool errors are fixed strings (no caller text, paths or OS detail), so each
//! cause gets its own message naming the feature or limit involved.

use super::native_export::NativeExportError;
use super::native_frame::NativeFrameError;
use super::native_properties::NativePropertyError;
use crate::compose::ComposeError;

/// Which tool failed; selects the message prefix.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    RenderPng,
    ExportGif,
}

macro_rules! messages {
    ($tool:expr, $name:expr, $( $key:literal => $text:literal ),+ $(,)?) => {
        match ($tool, $name) {
            $(
                (Tool::RenderPng, $key) => concat!("export png render failed: ", $text),
                (Tool::ExportGif, $key) => {
                    concat!("export could not render the animation: ", $text)
                }
            )+
            (Tool::RenderPng, _) => "export png render failed",
            (Tool::ExportGif, _) => "export could not render the animation",
        }
    };
}

fn compose_cause(error: &ComposeError) -> &'static str {
    match error {
        ComposeError::Unsupported { feature, .. } => match *feature {
            "filter" => "filter",
            "mask" => "mask",
            "text" => "text",
            "image" => "image",
            "marker" => "marker",
            "unsupported element or attribute" => "attribute",
            _ => "feature",
        },
        ComposeError::LimitDecode => "limit_decode",
        ComposeError::LimitRender => "limit_render",
        ComposeError::Malformed { .. } | ComposeError::InvalidUtf8 { .. } => "malformed",
        _ => "other",
    }
}

fn cause_name(error: &NativeExportError) -> &'static str {
    match error {
        NativeExportError::Frame(NativeFrameError::Property(
            NativePropertyError::PropertyNotOnElement,
        )) => "property_not_on_element",
        NativeExportError::Frame(NativeFrameError::Property(
            NativePropertyError::MissingTarget,
        )) => "missing_target",
        NativeExportError::Frame(NativeFrameError::Property(
            NativePropertyError::DuplicateTarget,
        )) => "duplicate_target",
        NativeExportError::Frame(NativeFrameError::Property(
            NativePropertyError::UnsupportedProperty,
        )) => "unsupported_property",
        NativeExportError::Frame(NativeFrameError::Property(_)) => "property_value",
        NativeExportError::Frame(NativeFrameError::Compose(e)) | NativeExportError::Viewport(e) => {
            compose_cause(e)
        }
        NativeExportError::Frame(NativeFrameError::Drawing(_)) => "drawing",
        NativeExportError::Frame(NativeFrameError::Allocation) => "limit_render",
        _ => "other",
    }
}

/// The fixed message naming why `error` stopped the render.
pub fn render_failure(tool: Tool, error: &NativeExportError) -> &'static str {
    let name = cause_name(error);
    messages!(
        tool,
        name,
        "property_not_on_element" => "a native property track sets a geometry attribute (x, y, width, height, cx, cy, r, rx or ry) that its target element type does not have, for example cy on a path; remove the track (native_timeline_remove_track) or animate position with a transform track",
        "missing_target" => "a native timeline track targets an element id that is not in the document; remove the track or re-add the element",
        "duplicate_target" => "a native timeline track target id is used by more than one element",
        "unsupported_property" => "a native timeline track animates a property that is not supported; use x, y, width, height, cx, cy, r, rx, ry, opacity, fill-opacity, stroke-opacity, stroke-width, stroke-dashoffset, stdDeviation, dx, dy, flood-opacity, values, offset, stop-opacity, d, points, fill, stroke, stop-color, visibility or transform",
        "property_value" => "a native timeline track has a value that cannot be applied (for example a colour channel outside 0 to 1)",
        "filter" => "the scene uses an SVG filter, which is not supported",
        "mask" => "the scene uses an SVG mask, which is not supported",
        "text" => "the scene uses SVG text, which is not supported",
        "image" => "the scene uses an SVG image, which is not supported",
        "marker" => "the scene uses an SVG marker, which is not supported",
        "attribute" => "the scene has an element or attribute outside the supported SVG subset after the timeline is applied; check element and attribute names",
        "feature" => "the scene uses an SVG feature that is not supported",
        "limit_decode" => "the scene exceeds a decode safety limit (document size or element count)",
        "limit_render" => "the scene exceeds a render safety limit (raster size or work budget)",
        "malformed" => "the scene SVG could not be parsed",
        "drawing" => "a native drawing cel could not be rendered (missing or invalid drawing asset)",
        "other" => "the scene could not be rendered",
    )
}
