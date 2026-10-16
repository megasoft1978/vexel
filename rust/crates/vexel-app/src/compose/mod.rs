//! Window-free SVG composition for the ADR-0029 V1 subset:
//! bytes -> `vexel-svg-engine` decode -> `vexel-scene` -> `vexel-render` ->
//! RGBA frame, with pointer mapping and hit testing over the same shapes.
//!
//! Nothing here touches a window or the platform layer.

mod decode;
mod error;
mod launch;
mod open;
mod pointer;
mod present;
mod shapes;
mod worker;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_open;
#[cfg(test)]
mod tests_parity;
#[cfg(test)]
mod tests_stale;

pub use decode::MAX_SVG_BYTES;
pub use error::{ComposeError, Disclosure};
pub use launch::{Launch, parse_launch, parse_launch_with_home};
pub use open::{OpenError, open_path};
pub use pointer::PointMap;
pub use present::{DocumentView, blit_document, fill, status_text};
pub use shapes::OwnedShape;
pub use worker::{ComposeWorker, WorkerEvent};

use vexel_render::{
    PreserveAspect, RenderError, SceneFeatures, ViewBox, offscreen_tile_size, render_offscreen,
};

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
    offscreen_tile_size(f64::from(width), f64::from(height))
        .map_err(|_| ComposeError::InvalidSize)?;
    let graph = decode::decode(bytes, cancel)?;
    let scene = graph.root();
    let features = SceneFeatures {
        clips: scene.clips.len(),
        opacity_groups: scene.groups.len(),
        masks: scene.masks.len(),
        filters: scene.filters.len(),
        ..SceneFeatures::default()
    };
    if let Err(RenderError::Unsupported(kind)) = features.require_supported() {
        return Err(ComposeError::Unsupported {
            feature: error::feature_name(kind),
            offset: None,
            element: None,
        });
    }
    let (shapes, mut disclosures) = shapes::extract(&scene, graph.source())?;
    if !graph.animation().tracks.is_empty() {
        disclosures.push(Disclosure {
            feature: "animation (static first frame shown)",
            offset: None,
            element: None,
        });
    }
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
    let render_shapes: Vec<_> = shapes.iter().map(OwnedShape::as_render).collect();
    let tile = render_offscreen(
        &render_shapes,
        view,
        aspect,
        f64::from(width),
        f64::from(height),
        cancel,
    )
    .map_err(|e| match e {
        RenderError::Cancelled => ComposeError::Cancelled,
        RenderError::LimitExceeded => ComposeError::LimitExceeded,
        RenderError::Unsupported(kind) => ComposeError::Unsupported {
            feature: error::feature_name(kind),
            offset: None,
            element: None,
        },
        RenderError::InvalidGeometry | RenderError::TileNotEmpty => ComposeError::Malformed {
            detail: "render geometry",
            offset: 0,
        },
    })?;
    let map = PointMap::new(view, width, height, aspect).ok_or(ComposeError::InvalidSize)?;
    let frame = Frame {
        width,
        height,
        rgba: tile.rgba().to_vec(),
        disclosures,
    };
    Ok(Composed { frame, map, shapes })
}
