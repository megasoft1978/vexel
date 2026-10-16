//! Window-free SVG composition for the ADR-0029 V1 subset:
//! bytes -> `vexel-svg-engine` decode -> `vexel-scene` -> `vexel-render` ->
//! RGBA frame, with pointer mapping and hit testing over the same shapes.
//!
//! Masks, opacity/clip groups, pattern paint, filters, dashes and (macOS)
//! text render through the `vexel-render` entry points; see `render_scene`.
//! Nothing here touches a window or the platform layer except the CoreText
//! layout provider handed to the engine.

mod clips;
mod decode;
mod error;
mod launch;
mod layers;
mod open;
mod pointer;
mod present;
mod render_scene;
mod shapes;
#[cfg(target_os = "macos")]
mod text;
mod worker;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_chrome;
#[cfg(test)]
mod tests_open;
#[cfg(test)]
mod tests_parity;
#[cfg(test)]
mod tests_stale;
#[cfg(test)]
mod tests_wire;

pub use decode::MAX_SVG_BYTES;
pub(crate) use decode::{Gate, SessionError, install_text, run};
pub use error::{ComposeError, Disclosure};
pub use launch::{Launch, parse_launch, parse_launch_with_home};
pub use open::{OpenError, open_path};
pub use pointer::PointMap;
pub use present::{DocumentView, blit_document, fill, status_text};
pub(crate) use render_scene::render_scene;
pub use shapes::OwnedShape;
pub use worker::{ComposeWorker, WorkerEvent};

use vexel_render::{PreserveAspect, ViewBox, offscreen_tile_size};

/// A composed straight-alpha RGBA8 frame (row 0 at the top, no padding).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    /// Deferred items that were skipped, never silently.
    pub disclosures: Vec<Disclosure>,
}

/// A frame plus what is needed to map pointers and hit test it.
#[derive(Debug, Clone)]
pub struct Composed {
    pub frame: Frame,
    pub map: PointMap,
    pub shapes: Vec<OwnedShape>,
}

impl Composed {
    /// Topmost visible shape under a canvas pixel position, as a scene index.
    pub fn hit_test(&self, canvas_x: f64, canvas_y: f64) -> Option<usize> {
        let (x, y) = self.map.canvas_to_doc(canvas_x, canvas_y)?;
        self.shapes
            .iter()
            .rev()
            .find(|s| s.hit(x, y))
            .map(|s| s.scene_index)
    }
}

/// Composes `bytes` into a `width` x `height` frame at 1 pixel per point.
pub fn compose_svg(bytes: &[u8], width: u32, height: u32) -> Result<Frame, ComposeError> {
    compose_document(bytes, width, height, &|| false).map(|c| c.frame)
}

/// Like [`compose_svg`], keeping hit-test data and honouring `cancel`, which
/// is polled during decode and per shape and pixel row while rendering.
pub fn compose_document(
    bytes: &[u8],
    width: u32,
    height: u32,
    cancel: &dyn Fn() -> bool,
) -> Result<Composed, ComposeError> {
    let (tile_width, tile_height) = offscreen_tile_size(f64::from(width), f64::from(height))
        .map_err(|_| ComposeError::InvalidSize)?;
    let graph = decode::decode(bytes, cancel)?;
    let scene = graph.root();
    let viewport = scene.viewport;
    let view = ViewBox {
        x: viewport.view_x,
        y: viewport.view_y,
        width: viewport.view_width,
        height: viewport.view_height,
    };
    let aspect = viewport.aspect.set.then_some(PreserveAspect {
        align_x: viewport.aspect.align_x,
        align_y: viewport.aspect.align_y,
        none: viewport.aspect.none,
        slice: viewport.aspect.slice,
    });
    let mut tile = blank_tile(tile_width, tile_height)?;
    let drawn = render_scene(&scene, view, aspect, &mut tile, false, cancel)?;
    let source = graph.source();
    let mut disclosures: Vec<Disclosure> = drawn
        .skipped
        .iter()
        .map(|skipped| {
            let offset = scene
                .shapes
                .get(skipped.scene_index)
                .map(|shape| shape.source.element.start);
            Disclosure {
                feature: error::feature_name(skipped.kind),
                offset,
                element: offset.and_then(|o| error::element_at(source, o)),
            }
        })
        .collect();
    if !graph.animation().tracks.is_empty() {
        disclosures.push(Disclosure {
            feature: "animation (static first frame shown)",
            offset: None,
            element: None,
        });
    }
    let map = PointMap::new(view, width, height, aspect).ok_or(ComposeError::InvalidSize)?;
    let frame = Frame {
        width,
        height,
        rgba: tile.rgba().to_vec(),
        disclosures,
    };
    Ok(Composed {
        frame,
        map,
        shapes: drawn.shapes,
    })
}

/// A fresh transparent tile.
pub(crate) fn blank_tile(width: u32, height: u32) -> Result<vexel_raster::Tile, ComposeError> {
    let bytes = (width as usize)
        .checked_mul(height as usize)
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or(ComposeError::LimitExceeded)?;
    let mut rgba = Vec::new();
    rgba.try_reserve_exact(bytes)
        .map_err(|_| ComposeError::LimitExceeded)?;
    rgba.resize(bytes, 0);
    vexel_raster::Tile::from_owned_rgba(width, height, rgba)
        .map_err(|_| ComposeError::LimitExceeded)
}
