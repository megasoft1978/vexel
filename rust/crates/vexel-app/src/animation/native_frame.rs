//! Renders one native timeline-v1 integer tick.
//!
//! Property samples are applied to an owned SVG snapshot before the base
//! scene is rendered. Visible drawing tracks are then composited in their
//! serialized order by `native_drawing`.

use crate::animation::native_drawing::{
    DrawingAsset, DrawingCel, DrawingError, DrawingTrack, SvgGeometry, compose_drawing_tracks,
};
use crate::animation::native_properties::{
    NativePropertyError, SourceIndex, apply_native_properties, apply_native_properties_indexed,
};
use crate::compose::{
    ComposeError, DiffCache, Frame, compose_frame_cached, compose_frame_owned,
    compose_graph_cached, compose_svg,
};
use std::sync::Arc;
use vexel_scene::SceneGraph;
use vexel_timeline::{TimelineIterator, TimelineView, TrackView, string_copy};

struct OwnedDrawingCel {
    tick: u32,
    asset: Vec<u8>,
}

/// Typed failures raised while rendering a single native timeline frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeFrameError {
    InvalidTick,
    Property(NativePropertyError),
    Drawing(DrawingError),
    Compose(ComposeError),
    Allocation,
}

/// Render one integer timeline tick into an owned, complete frame.
///
/// `timeline` must already be decoded and validated. The SVG source remains
/// untouched; property samples are written to a temporary snapshot, rendered,
/// and then overlaid by active drawing cels.
pub fn render_native_frame(
    source_svg: &[u8],
    timeline: &TimelineView<'_>,
    assets: &[DrawingAsset<'_>],
    tick: u32,
    output_width: u32,
    output_height: u32,
    geometry: SvgGeometry,
) -> Result<Frame, NativeFrameError> {
    render_with(
        source_svg,
        None,
        timeline,
        assets,
        tick,
        output_width,
        output_height,
        geometry,
        None,
    )
}

/// Playback variant of [`render_native_frame`]: property targets resolve
/// through `index` (built once per revision) and the patched SVG is composed
/// without copying it or scanning ids. Pixels equal `render_native_frame`.
pub fn render_native_frame_indexed(
    source_svg: &[u8],
    index: &SourceIndex,
    timeline: &TimelineView<'_>,
    assets: &[DrawingAsset<'_>],
    tick: u32,
    output_width: u32,
    output_height: u32,
    geometry: SvgGeometry,
) -> Result<Frame, NativeFrameError> {
    render_with(
        source_svg,
        Some(index),
        timeline,
        assets,
        tick,
        output_width,
        output_height,
        geometry,
        None,
    )
}

/// Frame-sequence variant of [`render_native_frame`] / [`_indexed`]: `cache`
/// carries the previous frame of the same source so only changed shapes are
/// repainted. Pixels are identical to the uncached functions.
///
/// [`_indexed`]: render_native_frame_indexed
pub fn render_native_frame_cached(
    source_svg: &[u8],
    index: Option<&SourceIndex>,
    timeline: &TimelineView<'_>,
    assets: &[DrawingAsset<'_>],
    tick: u32,
    output_width: u32,
    output_height: u32,
    geometry: SvgGeometry,
    cache: &mut DiffCache,
) -> Result<Frame, NativeFrameError> {
    render_with(
        source_svg,
        index,
        timeline,
        assets,
        tick,
        output_width,
        output_height,
        geometry,
        Some(cache),
    )
}

fn render_with(
    source_svg: &[u8],
    index: Option<&SourceIndex>,
    timeline: &TimelineView<'_>,
    assets: &[DrawingAsset<'_>],
    tick: u32,
    output_width: u32,
    output_height: u32,
    geometry: SvgGeometry,
    cache: Option<&mut DiffCache>,
) -> Result<Frame, NativeFrameError> {
    let snapshot = native_snapshot(source_svg, index, timeline, tick)?;
    let compose = crate::profile::phase("native.compose");
    let base = if let Some(cache) = cache {
        compose_frame_cached(Arc::new(snapshot), output_width, output_height, cache)
    } else if index.is_some() {
        compose_frame_owned(Arc::new(snapshot), output_width, output_height)
    } else {
        compose_svg(&snapshot, output_width, output_height)
    }
    .map_err(NativeFrameError::Compose)?;
    drop(compose);
    overlay_cels(base, timeline, assets, tick, geometry)
}

/// The source with `tick`'s property samples applied: the bytes a tick composes.
pub fn native_snapshot(
    source_svg: &[u8],
    index: Option<&SourceIndex>,
    timeline: &TimelineView<'_>,
    tick: u32,
) -> Result<Vec<u8>, NativeFrameError> {
    if tick >= timeline.duration_ticks() {
        return Err(NativeFrameError::InvalidTick);
    }
    let _props = crate::profile::phase("native.properties");
    let patched = match index {
        Some(index) => apply_native_properties_indexed(source_svg, index, timeline, tick),
        None => apply_native_properties(source_svg, timeline, tick),
    };
    patched.map_err(|error| {
        if error == NativePropertyError::InvalidTick {
            NativeFrameError::InvalidTick
        } else {
            NativeFrameError::Property(error)
        }
    })
}

/// [`render_native_frame_cached`] over a graph already decoded from
/// [`native_snapshot`]'s bytes (playback decode-ahead). Same pixels.
pub fn render_native_frame_predecoded(
    graph: &SceneGraph,
    timeline: &TimelineView<'_>,
    assets: &[DrawingAsset<'_>],
    tick: u32,
    output_width: u32,
    output_height: u32,
    geometry: SvgGeometry,
    cache: &mut DiffCache,
) -> Result<Frame, NativeFrameError> {
    if tick >= timeline.duration_ticks() {
        return Err(NativeFrameError::InvalidTick);
    }
    let compose = crate::profile::phase("native.compose");
    let base = compose_graph_cached(graph, output_width, output_height, cache)
        .map_err(NativeFrameError::Compose)?;
    drop(compose);
    overlay_cels(base, timeline, assets, tick, geometry)
}

fn overlay_cels(
    base: Frame,
    timeline: &TimelineView<'_>,
    assets: &[DrawingAsset<'_>],
    tick: u32,
    geometry: SvgGeometry,
) -> Result<Frame, NativeFrameError> {
    let _cels = crate::profile::phase("native.cels");

    let (cel_storage, visibility) = collect_drawing_cels(timeline)?;
    let mut cel_views = Vec::new();
    cel_views
        .try_reserve_exact(cel_storage.len())
        .map_err(|_| NativeFrameError::Allocation)?;
    for cels in &cel_storage {
        let mut views = Vec::new();
        views
            .try_reserve_exact(cels.len())
            .map_err(|_| NativeFrameError::Allocation)?;
        for cel in cels {
            views.push(DrawingCel {
                tick: cel.tick,
                asset: &cel.asset,
            });
        }
        cel_views.push(views);
    }
    let mut tracks = Vec::new();
    tracks
        .try_reserve_exact(cel_storage.len())
        .map_err(|_| NativeFrameError::Allocation)?;
    for (cels, visible) in cel_views.iter().zip(visibility) {
        tracks.push(DrawingTrack { visible, cels });
    }

    compose_drawing_tracks(&base, &tracks, assets, tick, geometry)
        .map_err(NativeFrameError::Drawing)
}

fn collect_drawing_cels(
    timeline: &TimelineView<'_>,
) -> Result<(Vec<Vec<OwnedDrawingCel>>, Vec<bool>), NativeFrameError> {
    let mut cel_storage = Vec::new();
    let mut visibility = Vec::new();
    cel_storage
        .try_reserve_exact(timeline.track_count())
        .map_err(|_| NativeFrameError::Allocation)?;
    visibility
        .try_reserve_exact(timeline.track_count())
        .map_err(|_| NativeFrameError::Allocation)?;

    for track_index in 0..timeline.track_count() {
        let track = timeline
            .track_at(track_index)
            .ok_or(NativeFrameError::Drawing(DrawingError::InvalidTimeline))?;
        if decode_json_string(track.kind)? != b"drawing" {
            continue;
        }
        let cels = collect_track_cels(timeline, track_index, track)?;
        cel_storage.push(cels);
        visibility.push(track.visible);
    }
    Ok((cel_storage, visibility))
}

fn collect_track_cels(
    timeline: &TimelineView<'_>,
    track_index: usize,
    track: TrackView<'_>,
) -> Result<Vec<OwnedDrawingCel>, NativeFrameError> {
    let mut cels = Vec::new();
    cels.try_reserve_exact(track.item_count)
        .map_err(|_| NativeFrameError::Allocation)?;
    let mut iterator = TimelineIterator::default();
    for _ in 0..track.item_count {
        let item = timeline
            .item_next(track_index, &mut iterator)
            .ok_or(NativeFrameError::Drawing(DrawingError::InvalidTimeline))?;
        cels.push(OwnedDrawingCel {
            tick: item.tick,
            asset: decode_json_string(
                item.asset
                    .ok_or(NativeFrameError::Drawing(DrawingError::InvalidTimeline))?,
            )?,
        });
    }
    if timeline.item_next(track_index, &mut iterator).is_some() {
        return Err(NativeFrameError::Drawing(DrawingError::InvalidTimeline));
    }
    Ok(cels)
}

fn decode_json_string(encoded: &[u8]) -> Result<Vec<u8>, NativeFrameError> {
    let mut decoded = Vec::new();
    decoded
        .try_reserve_exact(encoded.len())
        .map_err(|_| NativeFrameError::Allocation)?;
    decoded.resize(encoded.len(), 0);
    let length = string_copy(encoded, &mut decoded)
        .ok_or(NativeFrameError::Drawing(DrawingError::InvalidTimeline))?;
    decoded.truncate(length);
    Ok(decoded)
}

#[cfg(test)]
#[path = "native_frame_tests.rs"]
mod tests;
