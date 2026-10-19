//! Copied root viewport geometry decoded through the compose SVG pipeline.

use super::{decode::decode_owned, error::ComposeError};
use std::sync::Arc;
use vexel_render::{PreserveAspect, ViewBox};

/// Root SVG output dimensions and user-space viewport geometry.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SvgViewport {
    /// Intrinsic output width in pixels.
    pub output_width: u32,
    /// Intrinsic output height in pixels.
    pub output_height: u32,
    /// User-space viewBox, including its origin.
    pub view_box: ViewBox,
    /// Explicit preserveAspectRatio value; `None` means SVG's default meet.
    pub aspect: Option<PreserveAspect>,
}

/// Decodes an owned SVG and returns a copy of its root viewport geometry.
///
/// The bytes are passed to the same owned decoder used by composition. The
/// callback is polled by that decoder and cancellation is returned as
/// [`ComposeError::Cancelled`].
pub fn svg_viewport(
    owner: Arc<Vec<u8>>,
    cancel: &dyn Fn() -> bool,
) -> Result<SvgViewport, ComposeError> {
    let graph = decode_owned(owner, cancel)?;
    let viewport = graph.root().viewport;
    Ok(SvgViewport {
        output_width: viewport.width,
        output_height: viewport.height,
        view_box: ViewBox {
            x: viewport.view_x,
            y: viewport.view_y,
            width: viewport.view_width,
            height: viewport.view_height,
        },
        aspect: viewport.aspect.set.then_some(PreserveAspect {
            align_x: viewport.aspect.align_x,
            align_y: viewport.aspect.align_y,
            none: viewport.aspect.none,
            slice: viewport.aspect.slice,
        }),
    })
}
