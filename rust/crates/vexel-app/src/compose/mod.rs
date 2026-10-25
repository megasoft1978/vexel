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
mod diff;
mod error;
mod gpu_adapter;
mod gpu_route;
mod inflate;
mod launch;
mod layers;
mod open;
mod png;
mod pointer;
mod present;
mod render_scene;
mod shapes;
#[cfg(target_os = "macos")]
mod text;
mod viewport;
mod worker;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_chrome;
#[cfg(test)]
mod tests_dirty;
#[cfg(test)]
mod tests_gpu;
#[cfg(test)]
mod tests_image;
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
pub use diff::DiffCache;
pub use error::{ComposeError, Disclosure};
pub use gpu_route::{
    Decision as GpuDecision, app_launch, gpu_frames, last_decision as gpu_last_decision,
    reset_latch as gpu_reset_latch, set_gpu_display, set_gpu_display_policy, status as gpu_status,
};
pub use launch::{Launch, parse_launch, parse_launch_with_home};
pub use open::{OpenError, open_path};
pub use pointer::PointMap;
pub use present::{DocumentView, blit_document, fill, status_text};
pub(crate) use render_scene::render_scene;
pub use shapes::OwnedShape;
pub use viewport::{SvgViewport, svg_viewport};
pub use worker::{ComposeWorker, WorkerEvent};

use std::collections::HashMap;
use std::sync::Arc;
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
    /// Unique stable SVG ids keyed by source-order shape index.
    pub target_ids: Vec<(usize, String)>,
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
    if bytes.len() > MAX_SVG_BYTES {
        return Err(ComposeError::LimitDecode);
    }
    if let Err(error) = std::str::from_utf8(bytes) {
        return Err(ComposeError::InvalidUtf8 {
            valid_up_to: error.valid_up_to(),
        });
    }
    compose_document_owned(Arc::new(bytes.to_vec()), width, height, cancel)
}

/// Composes an already-owned source buffer without copying its bytes.
pub(crate) fn compose_document_owned(
    bytes: Arc<Vec<u8>>,
    width: u32,
    height: u32,
    cancel: &dyn Fn() -> bool,
) -> Result<Composed, ComposeError> {
    compose_owned(bytes, width, height, cancel, true, None)
}

/// The frame of [`compose_svg`] for an owned buffer: no source copy and no
/// element-id scan (playback frames are never hit tested). Same pixels.
pub(crate) fn compose_frame_owned(
    bytes: Arc<Vec<u8>>,
    width: u32,
    height: u32,
) -> Result<Frame, ComposeError> {
    if bytes.len() > MAX_SVG_BYTES {
        return Err(ComposeError::LimitDecode);
    }
    if let Err(error) = std::str::from_utf8(&bytes) {
        return Err(ComposeError::InvalidUtf8 {
            valid_up_to: error.valid_up_to(),
        });
    }
    compose_owned(bytes, width, height, &|| false, false, None).map(|c| c.frame)
}

/// [`compose_frame_owned`] repainting only what changed since the previous
/// frame composed with the same `cache`. Same pixels.
pub(crate) fn compose_frame_cached(
    bytes: Arc<Vec<u8>>,
    width: u32,
    height: u32,
    cache: &mut DiffCache,
) -> Result<Frame, ComposeError> {
    if bytes.len() > MAX_SVG_BYTES {
        return Err(ComposeError::LimitDecode);
    }
    if let Err(error) = std::str::from_utf8(&bytes) {
        return Err(ComposeError::InvalidUtf8 {
            valid_up_to: error.valid_up_to(),
        });
    }
    compose_owned(bytes, width, height, &|| false, false, Some(cache)).map(|c| c.frame)
}

/// [`compose_document_owned`] repainting only what changed since the previous
/// document composed with the same `cache` (hit-test data included). Same pixels.
pub(crate) fn compose_document_cached(
    bytes: Arc<Vec<u8>>,
    width: u32,
    height: u32,
    cancel: &dyn Fn() -> bool,
    cache: &mut DiffCache,
) -> Result<Composed, ComposeError> {
    compose_owned(bytes, width, height, cancel, true, Some(cache))
}

fn compose_owned(
    bytes: Arc<Vec<u8>>,
    width: u32,
    height: u32,
    cancel: &dyn Fn() -> bool,
    with_ids: bool,
    cache: Option<&mut DiffCache>,
) -> Result<Composed, ComposeError> {
    let (tile_width, tile_height) = offscreen_tile_size(f64::from(width), f64::from(height))
        .map_err(|_| ComposeError::InvalidSize)?;
    let ph = crate::profile::phase("c.decode");
    let graph = decode::decode_owned(bytes, cancel)?;
    drop(ph);
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
    let ph = crate::profile::phase("c.render");
    let drawn = match cache {
        Some(cache) => match gpu_route::try_render(&scene, view, aspect, &mut tile, cache) {
            Some(drawn) => drawn,
            None => {
                diff::render_scene_diff(&scene, view, aspect, &mut tile, cancel, cache, with_ids)?
            }
        },
        None => render_scene(&scene, view, aspect, &mut tile, false, cancel)?,
    };
    drop(ph);
    let source = graph.source();
    let target_ids_by_offset = if with_ids {
        unique_element_ids(source)
    } else {
        HashMap::new()
    };
    let target_ids = drawn
        .shapes
        .iter()
        .filter_map(|shape| {
            let source_shape = scene.shapes.get(shape.scene_index)?;
            let id = target_ids_by_offset.get(&source_shape.source.element.start)?;
            Some((shape.scene_index, id.clone()))
        })
        .collect();
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
        rgba: tile.into_rgba(),
        disclosures,
    };
    Ok(Composed {
        frame,
        map,
        shapes: drawn.shapes,
        target_ids,
    })
}

fn unique_element_ids(source: &[u8]) -> HashMap<usize, String> {
    use vexel_xml::TokenKind;

    let (tokens, result) = vexel_xml::tokenize_large(source);
    if result.diagnostic.category != vexel_xml::XmlError::None {
        return HashMap::new();
    }
    let mut found = Vec::new();
    let mut counts = HashMap::<String, usize>::new();
    for (index, token) in tokens.iter().enumerate() {
        if !matches!(
            token.kind,
            TokenKind::StartElement | TokenKind::EmptyElement
        ) {
            continue;
        }
        let Some(id) = tokens[index + 1..]
            .iter()
            .take_while(|next| next.kind == TokenKind::Attribute)
            .find(|attribute| attribute.name.slice(source) == Some(&b"id"[..]))
        else {
            continue;
        };
        let mut decoded = vec![0; id.value.length];
        let Ok(length) = vexel_xml::decode_attribute_value(source, id.value, &mut decoded) else {
            continue;
        };
        decoded.truncate(length);
        let Ok(value) = String::from_utf8(decoded) else {
            continue;
        };
        if value.is_empty() {
            continue;
        }
        *counts.entry(value.clone()).or_default() += 1;
        found.push((token.source.offset, value));
    }
    found
        .into_iter()
        .filter(|(_, value)| counts.get(value) == Some(&1))
        .collect()
}

/// A fresh transparent tile.
pub(crate) fn blank_tile(width: u32, height: u32) -> Result<vexel_raster::Tile, ComposeError> {
    let bytes = (width as usize)
        .checked_mul(height as usize)
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or(ComposeError::LimitRender)?;
    let mut rgba = Vec::new();
    rgba.try_reserve_exact(bytes)
        .map_err(|_| ComposeError::LimitRender)?;
    rgba.resize(bytes, 0);
    vexel_raster::Tile::from_owned_rgba(width, height, rgba).map_err(|_| ComposeError::LimitRender)
}
