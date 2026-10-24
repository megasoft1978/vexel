//! Canvas-to-document coordinate mapping used by the tools.

use vexel_ui::Rect;

/// Maps a canvas-space point to document space. `None` means the point is not
/// over the artboard (the tools then ignore it or reuse the last valid point).
///
/// The compose order supplies the production implementation (a `ViewMap`-based
/// function); that wiring is an integration step and is not built here.
pub trait CanvasMap {
    fn to_doc(&self, x: f64, y: f64) -> Option<(f64, f64)>;
}

/// Document units per canvas pixel near `at`, measured with one-pixel probes
/// (1.0 when the point has no valid neighbour on the artboard).
pub fn doc_per_pixel(map: &dyn CanvasMap, at: (f64, f64)) -> f64 {
    let Some(here) = map.to_doc(at.0, at.1) else {
        return 1.0;
    };
    [1.0, -1.0]
        .into_iter()
        .filter_map(|step| map.to_doc(at.0 + step, at.1))
        .map(|there| (there.0 - here.0).abs())
        .find(|d| d.is_finite() && *d > 0.0)
        .unwrap_or(1.0)
}

/// `doc = (canvas - origin) / scale`, valid only inside `0..=width` by `0..=height`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScaleMap {
    origin: (f64, f64),
    scale: f64,
    size: (f64, f64),
}

impl ScaleMap {
    /// Returns `None` unless every value is finite and `scale`, `width`, `height` are positive.
    pub fn new(origin: (f64, f64), scale: f64, size: (f64, f64)) -> Option<Self> {
        let finite = [origin.0, origin.1, scale, size.0, size.1]
            .iter()
            .all(|v| v.is_finite());
        (finite && scale > 0.0 && size.0 > 0.0 && size.1 > 0.0).then_some(Self {
            origin,
            scale,
            size,
        })
    }

    /// Canvas space equals document space.
    pub fn identity(width: f64, height: f64) -> Option<Self> {
        Self::new((0.0, 0.0), 1.0, (width, height))
    }
}

impl CanvasMap for ScaleMap {
    fn to_doc(&self, x: f64, y: f64) -> Option<(f64, f64)> {
        let (dx, dy) = (
            (x - self.origin.0) / self.scale,
            (y - self.origin.1) / self.scale,
        );
        (dx.is_finite()
            && dy.is_finite()
            && (0.0..=self.size.0).contains(&dx)
            && (0.0..=self.size.1).contains(&dy))
        .then_some((dx, dy))
    }
}

/// Live pointer routing: converts a window point to stage-local coordinates, or
/// `None` when it is outside the stage rectangle (so chrome clicks never reach a tool).
pub fn stage_local(stage: Rect, point: (i32, i32)) -> Option<(f64, f64)> {
    let (x, y) = (
        i64::from(point.0) - i64::from(stage.x),
        i64::from(point.1) - i64::from(stage.y),
    );
    ((0..i64::from(stage.width)).contains(&x) && (0..i64::from(stage.height)).contains(&y))
        .then_some((x as f64, y as f64))
}
