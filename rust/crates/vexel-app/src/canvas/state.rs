//! Mutable canvas presentation state: the view, hover, and gesture flags.

use super::view::{View, ZOOM_STEP};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CanvasState {
    pub view: View,
    /// Follow the stage: refit on every stage or document size change. Cleared
    /// by any explicit zoom or pan.
    pub fit: bool,
    /// Scene index of the shape under the pointer (select tool only).
    pub hover: Option<usize>,
    /// Space is held: a primary drag pans.
    pub space: bool,
    /// Last pointer position of an active pan drag.
    pub drag: Option<(f64, f64)>,
}

impl CanvasState {
    /// Fits when following the stage. True when the view changed.
    pub fn refit(&mut self, stage: (f64, f64), artboard: (f64, f64)) -> bool {
        if !self.fit {
            return false;
        }
        let next = View::fit(stage, artboard);
        let changed = next != self.view;
        self.view = next;
        changed
    }

    pub fn set_fit(&mut self, stage: (f64, f64), artboard: (f64, f64)) {
        self.fit = true;
        self.view = View::fit(stage, artboard);
    }

    pub fn set_actual(&mut self, stage: (f64, f64), artboard: (f64, f64)) {
        self.fit = false;
        self.view = View::actual(stage, artboard);
    }

    /// Zooms about `point` by `factor`.
    pub fn zoom_by(&mut self, point: (f64, f64), factor: f64) {
        self.fit = false;
        self.view = self.view.zoom_by(point, factor);
    }

    /// One 1.25x step in (`dir > 0`) or out about `point`.
    pub fn zoom_step(&mut self, point: (f64, f64), dir: i32) {
        self.zoom_by(point, ZOOM_STEP.powi(dir.signum()));
    }

    pub fn pan_by(&mut self, delta: (f64, f64)) {
        self.fit = false;
        self.view = self.view.pan_by(delta);
    }
}
