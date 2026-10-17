//! The view transform of the canvas well: zoom and pan of the artboard.
//!
//! Coordinates. *Document* points are artboard pixels at 100% (the document's
//! natural size, origin at the artboard's top-left). *View* points are logical
//! window points relative to the stage's top-left. `view = pan + doc * zoom`.

pub const MIN_ZOOM: f64 = 0.1;
pub const MAX_ZOOM: f64 = 32.0;
/// One zoom-in or zoom-out step.
pub const ZOOM_STEP: f64 = 1.25;
/// Minimum free space around a fitted artboard, in logical points.
pub const FIT_MARGIN: f64 = 24.0;
/// Fit never magnifies beyond this (400%).
pub const FIT_MAX_ZOOM: f64 = 4.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct View {
    pub zoom: f64,
    /// View position of the artboard's top-left corner.
    pub pan: (f64, f64),
}

impl Default for View {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl View {
    pub const IDENTITY: View = View {
        zoom: 1.0,
        pan: (0.0, 0.0),
    };

    /// Clamps to 10%..3200%; a non-finite zoom becomes 100%.
    pub fn clamp_zoom(zoom: f64) -> f64 {
        if zoom.is_finite() {
            zoom.clamp(MIN_ZOOM, MAX_ZOOM)
        } else {
            1.0
        }
    }

    fn centred(stage: (f64, f64), doc: (f64, f64), zoom: f64) -> View {
        View {
            zoom,
            pan: (
                (stage.0 - doc.0 * zoom) / 2.0,
                (stage.1 - doc.1 * zoom) / 2.0,
            ),
        }
    }

    /// Whole artboard visible and centred with at least [`FIT_MARGIN`] around
    /// it; never above [`FIT_MAX_ZOOM`].
    pub fn fit(stage: (f64, f64), doc: (f64, f64)) -> View {
        if !(doc.0 > 0.0 && doc.1 > 0.0 && doc.0.is_finite() && doc.1.is_finite()) {
            return Self::IDENTITY;
        }
        let fit_x = (stage.0 - 2.0 * FIT_MARGIN) / doc.0;
        let fit_y = (stage.1 - 2.0 * FIT_MARGIN) / doc.1;
        Self::centred(
            stage,
            doc,
            Self::clamp_zoom(fit_x.min(fit_y).min(FIT_MAX_ZOOM)),
        )
    }

    /// 100%, artboard centred.
    pub fn actual(stage: (f64, f64), doc: (f64, f64)) -> View {
        Self::centred(stage, doc, 1.0)
    }

    /// Sets the zoom (clamped) keeping the document point under `point` fixed.
    pub fn zoom_at(self, point: (f64, f64), zoom: f64) -> View {
        let zoom = Self::clamp_zoom(zoom);
        let doc = self.view_to_doc(point);
        View {
            zoom,
            pan: (point.0 - doc.0 * zoom, point.1 - doc.1 * zoom),
        }
    }

    pub fn zoom_by(self, point: (f64, f64), factor: f64) -> View {
        self.zoom_at(point, self.zoom * factor)
    }

    pub fn pan_by(self, delta: (f64, f64)) -> View {
        View {
            zoom: self.zoom,
            pan: (self.pan.0 + delta.0, self.pan.1 + delta.1),
        }
    }

    pub fn doc_to_view(self, p: (f64, f64)) -> (f64, f64) {
        (self.pan.0 + p.0 * self.zoom, self.pan.1 + p.1 * self.zoom)
    }

    pub fn view_to_doc(self, p: (f64, f64)) -> (f64, f64) {
        (
            (p.0 - self.pan.0) / self.zoom,
            (p.1 - self.pan.1) / self.zoom,
        )
    }

    /// The artboard rectangle `(x0, y0, x1, y1)` in view points.
    pub fn artboard(self, doc: (f64, f64)) -> (f64, f64, f64, f64) {
        let (x1, y1) = self.doc_to_view(doc);
        (self.pan.0, self.pan.1, x1, y1)
    }

    /// Whole percent for the zoom label.
    pub fn percent(self) -> u32 {
        (self.zoom * 100.0).round() as u32
    }
}

/// The one zoom readout string (`NNN%`), shared by the zoom cluster and the
/// status bar.
pub fn percent_label(percent: u32) -> String {
    format!("{percent}%")
}
