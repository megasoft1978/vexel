//! SVG source -> decoded scene -> `evaluate_frame` -> `vexel-render` pixels.
//!
//! Nothing here approximates: anything the decoder, the frame evaluator or the
//! scalar renderer does not support is returned as a typed [`AnimationError`].

use std::sync::{Arc, atomic::AtomicBool};
use vexel_base::BudgetControl;
use vexel_raster::Tile;
use vexel_render::{
    FlatEdge, PreserveAspect, RenderError, SceneFeatures, ViewBox, from_scene_shape,
    render_offscreen,
};
use vexel_scene::{
    ChargeLedger, DecodeOptions, DecodeSource, DecodedScene, Diagnostic, EvaluatedFrame,
    LexicalDocument, Limits, RasterJob, SceneGraph, SceneView,
};
use vexel_svg_animation_frame::evaluate_frame;
use vexel_svg_decode::TraversalDecodeEvent;
use vexel_svg_engine::SvgEngineSession;

/// Longest loop the app plays or exports, matching the MCP seek bound.
pub const MAX_LOOP_SECONDS: f64 = 10.0;
const SOURCE_LIMIT: usize = 1 << 20;
const DECODE_WORK_LIMIT: u64 = 1 << 32;
const FRAME_WORK_LIMIT: u64 = 1 << 32;
const FRAME_BYTE_BUDGET: usize = 64 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AnimationError {
    /// The decoder rejected the source.
    Decode(Diagnostic),
    /// The document has no drawable content (hidden/empty scene).
    NotDrawable,
    /// Decoding needs pattern rendering, which V1 does not support.
    PatternUnsupported,
    /// `evaluate_frame` failed or the time was invalid.
    Evaluate(Diagnostic),
    /// The scalar renderer rejected the frame (including deferred features).
    Render(RenderError),
    /// The cancel flag was raised.
    Cancelled,
    /// Output size, frame count or scratch memory bound exceeded.
    Limit,
}

/// A decoded animated document ready for per-time frame rendering.
#[derive(Debug)]
pub struct AnimatedScene {
    graph: SceneGraph,
    loop_seconds: f64,
}

pub fn decode(source: &[u8]) -> Result<AnimatedScene, AnimationError> {
    let cancel = AtomicBool::new(false);
    let mut control = BudgetControl::new(&cancel, DECODE_WORK_LIMIT);
    let owner = Arc::new(source.to_vec());
    let doc = LexicalDocument::parse(owner.clone(), SOURCE_LIMIT, &mut control)
        .map_err(AnimationError::Decode)?;
    let mut src = DecodeSource::new(&doc, &mut control).map_err(AnimationError::Decode)?;
    let ledger = ChargeLedger::new(&owner, 1 << 24, 65536).map_err(AnimationError::Decode)?;
    let job = RasterJob::new(ledger).map_err(|_| AnimationError::Limit)?;
    let mut session = SvgEngineSession::from_source(
        &doc,
        &mut src,
        1,
        DecodeOptions {
            limits: Limits::default(),
            system_language: b"en",
            media: None,
        },
        &mut control,
    )
    .map_err(AnimationError::Decode)?;
    loop {
        match session
            .step(256, &job, &mut control)
            .map_err(AnimationError::Decode)?
        {
            TraversalDecodeEvent::Progress { .. } => {}
            TraversalDecodeEvent::Complete => break,
            TraversalDecodeEvent::NeedPattern(_) => return Err(AnimationError::PatternUnsupported),
        }
    }
    match session
        .finish(&mut control)
        .map_err(AnimationError::Decode)?
    {
        DecodedScene::Drawable(graph) => {
            let loop_seconds = loop_seconds(&graph);
            Ok(AnimatedScene {
                graph,
                loop_seconds,
            })
        }
        DecodedScene::Hidden(_) => Err(AnimationError::NotDrawable),
    }
}

/// Loop length: the latest end of any supported track (`begin + duration *
/// finite repeat count`, one period when repeating indefinitely), at most
/// [`MAX_LOOP_SECONDS`]. Zero when nothing is animated.
fn loop_seconds(graph: &SceneGraph) -> f64 {
    let mut end = 0.0f64;
    for track in graph
        .root()
        .animation()
        .tracks
        .iter()
        .filter(|t| t.supported)
    {
        let repeats = if track.repeat_indefinite || !track.repeat_count.is_finite() {
            1.0
        } else {
            track.repeat_count.max(1.0)
        };
        let begin = if track.begin_seconds.is_finite() {
            track.begin_seconds.max(0.0)
        } else {
            0.0
        };
        let last = begin + track.duration_seconds * repeats;
        if last.is_finite() {
            end = end.max(last);
        }
    }
    end.min(MAX_LOOP_SECONDS)
}

impl AnimatedScene {
    pub fn loop_seconds(&self) -> f64 {
        self.loop_seconds
    }

    /// Canvas size in pixels (1x offscreen rule).
    pub fn size(&self) -> (u32, u32) {
        let viewport = self.graph.root().viewport;
        (viewport.width, viewport.height)
    }

    /// `evaluate_frame` over the decoded scene at `time_seconds`.
    pub fn evaluate(
        &self,
        time_seconds: f64,
        cancel: &AtomicBool,
    ) -> Result<EvaluatedFrame<'_>, AnimationError> {
        let mut control = BudgetControl::new(cancel, FRAME_WORK_LIMIT);
        evaluate_frame(
            self.graph.root(),
            time_seconds,
            FRAME_BYTE_BUDGET,
            &mut control,
        )
        .map_err(|d| match d.kind {
            vexel_scene::ErrorKind::Canceled => AnimationError::Cancelled,
            _ => AnimationError::Evaluate(d),
        })
    }

    /// `evaluate_frame` + scalar render at `time_seconds`; returns straight-alpha RGBA.
    pub fn render(&self, time_seconds: f64, cancel: &AtomicBool) -> Result<Tile, AnimationError> {
        self.render_sized(time_seconds, self.size(), cancel)
    }

    /// Like [`AnimatedScene::render`] into a `size` canvas (the stage), with
    /// the same view box and aspect mapping the static compose path uses.
    pub fn render_sized(
        &self,
        time_seconds: f64,
        size: (u32, u32),
        cancel: &AtomicBool,
    ) -> Result<Tile, AnimationError> {
        use std::sync::atomic::Ordering::Relaxed;
        let frame = self.evaluate(time_seconds, cancel)?;
        let view = frame.view();
        let stop = || cancel.load(Relaxed);
        render_view(&view, size, &stop)
    }
}

fn render_view(
    view: &SceneView<'_>,
    size: (u32, u32),
    stop: &dyn Fn() -> bool,
) -> Result<Tile, AnimationError> {
    let features = SceneFeatures {
        clips: view.clips.len(),
        opacity_groups: view.groups.len(),
        masks: view.masks.len(),
        filters: view.filters.len(),
        text_runs: 0,
        animation_overlay: false,
        smil: false,
    };
    features
        .require_supported()
        .map_err(AnimationError::Render)?;

    struct Owned {
        edges: Vec<FlatEdge>,
        ranges: Vec<std::ops::Range<usize>>,
        closed: Vec<bool>,
    }
    let mut owned = Vec::new();
    owned
        .try_reserve_exact(view.shapes.len())
        .map_err(|_| AnimationError::Limit)?;
    for shape in view.shapes {
        let source = shape
            .edges
            .slice(view.edges)
            .ok_or(AnimationError::Render(RenderError::InvalidGeometry))?;
        let subs = shape
            .subpaths
            .slice(view.path_subpaths)
            .ok_or(AnimationError::Render(RenderError::InvalidGeometry))?;
        let mut ranges = Vec::new();
        let mut closed = Vec::new();
        for sub in subs {
            let start = sub
                .first_edge
                .checked_sub(shape.edges.first)
                .ok_or(AnimationError::Render(RenderError::InvalidGeometry))?;
            ranges.push(start..start + sub.edge_count);
            closed.push(sub.closed);
        }
        owned.push(Owned {
            edges: source.iter().copied().map(FlatEdge::from).collect(),
            ranges,
            closed,
        });
    }
    let mut shapes = Vec::new();
    shapes
        .try_reserve_exact(owned.len())
        .map_err(|_| AnimationError::Limit)?;
    for (shape, geometry) in view.shapes.iter().zip(&owned) {
        let fill = view
            .edge_colors
            .get(shape.edges.first)
            .copied()
            .unwrap_or([0; 4]);
        shapes.push(
            from_scene_shape(
                shape,
                fill,
                view.fill_paints,
                view.stroke_paints,
                &geometry.edges,
                &geometry.ranges,
                &geometry.closed,
            )
            .map_err(AnimationError::Render)?,
        );
    }
    let vp = view.viewport;
    let aspect = vp.aspect.set.then_some(PreserveAspect {
        align_x: vp.aspect.align_x,
        align_y: vp.aspect.align_y,
        none: vp.aspect.none,
        slice: vp.aspect.slice,
    });
    let view_box = ViewBox {
        x: vp.view_x,
        y: vp.view_y,
        width: vp.view_width,
        height: vp.view_height,
    };
    render_offscreen(
        &shapes,
        view_box,
        aspect,
        f64::from(size.0),
        f64::from(size.1),
        stop,
    )
    .map_err(|e| match e {
        RenderError::Cancelled => AnimationError::Cancelled,
        RenderError::LimitExceeded => AnimationError::Limit,
        other => AnimationError::Render(other),
    })
}

/// Composites straight-alpha RGBA over opaque white (the GIF and display
/// background); the result has alpha 255 everywhere.
pub fn over_white(tile: &Tile) -> Vec<u8> {
    let mut out = Vec::with_capacity(tile.byte_count());
    for px in tile.rgba().chunks_exact(4) {
        let a = u32::from(px[3]);
        for &c in &px[..3] {
            out.push(((u32::from(c) * a + 255 * (255 - a) + 127) / 255) as u8);
        }
        out.push(255);
    }
    out
}
