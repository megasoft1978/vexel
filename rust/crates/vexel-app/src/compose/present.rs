//! Shell-side state for one opened SVG and its presentation into the stage.
//!
//! The composed frame is the artboard rendered at the resolution the current
//! zoom needs (see `canvas::size`); `blit_document` presents it through the
//! canvas view (artboard, shadow, checkerboard, overlays). A view that was never
//! adopted by the shell (`compose_target` not called) keeps the legacy
//! behaviour: the frame is the stage and view points equal frame pixels.

use super::{ComposeError, ComposeWorker, Composed, Disclosure, Frame, OwnedShape, WorkerEvent};
use crate::canvas::{self, CanvasState, StagePaint, ViewMap};
use crate::model::Layout;
use crate::session::AppDocument;
use crate::tools::CanvasMap;
use std::path::Path;
use std::sync::Arc;
use vexel_platform_apple::RgbaFrame;
use vexel_ui::{Rect, Theme, theme_tokens};

const MAX_LISTED_DISCLOSURES: usize = 3;

/// The latest composed frame of the session document and a status message.
///
/// The view keeps no copy of the document source: the session supplies bytes
/// and a revision through [`DocumentView::show`], and the worker composes them.
pub struct DocumentView {
    worker: Option<ComposeWorker>,
    name: Option<String>,
    /// Revision last handed to `show`; `None` until something is shown.
    revision: Option<u64>,
    stage: (u32, u32),
    pub(super) generation: u64,
    composed: Option<Composed>,
    /// The last composed frame, kept while a compose for a new revision or
    /// stage size is pending so the stage never goes blank; only the blit
    /// reads it (scaled to fit).
    previous: Option<Composed>,
    /// True while `composed.frame` holds an animated frame (not the static one).
    animated: bool,
    /// Static compose pixels parked while a native or SMIL frame is shown.
    static_rgba: Option<Vec<u8>>,
    status: Option<(String, bool)>,
    /// Scene index of the shape picked with the select tool, shown by the
    /// inspector.
    selected: Option<usize>,
    /// Natural size of the last shown document, read from its root tag.
    parsed: Option<(f64, f64)>,
    /// Natural size adopted by the shell through `compose_target`.
    artboard: Option<(f64, f64)>,
    compose_requests: u64,
    /// Live rotate-drag angle in degrees, set by the shell from the tools.
    rotation: Option<f64>,
    /// Zoom, pan, fit and hover state of the canvas well.
    pub canvas: CanvasState,
}

impl Default for DocumentView {
    fn default() -> Self {
        Self::new()
    }
}

impl DocumentView {
    pub fn new() -> Self {
        Self {
            worker: ComposeWorker::new(),
            name: None,
            revision: None,
            stage: (0, 0),
            generation: 0,
            composed: None,
            previous: None,
            animated: false,
            static_rgba: None,
            status: None,
            selected: None,
            parsed: None,
            artboard: None,
            compose_requests: 0,
            rotation: None,
            canvas: CanvasState::default(),
        }
    }

    /// The picked shape, if it still exists in the composed document.
    pub fn selected_shape(&self) -> Option<&OwnedShape> {
        let index = self.selected?;
        self.composed
            .as_ref()?
            .shapes
            .iter()
            .find(|s| s.scene_index == index)
    }

    /// Stable id of the selected source shape, only when it is unique.
    pub fn selected_target_id(&self) -> Option<&str> {
        let selected = self.selected?;
        self.composed
            .as_ref()?
            .target_ids
            .iter()
            .find(|(index, _)| *index == selected)
            .map(|(_, id)| id.as_str())
    }

    /// Sets the live rotation angle shown on the selection (`None` clears it).
    /// True when it changed.
    pub fn set_rotation(&mut self, degrees: Option<f64>) -> bool {
        let changed = self.rotation != degrees;
        self.rotation = degrees;
        changed
    }

    pub fn deselect(&mut self) {
        self.selected = None;
    }

    /// True while the stage shows a frame set by [`DocumentView::present_animated`].
    pub fn animated_shown(&self) -> bool {
        self.animated
    }

    /// Replaces the pixels of the composed frame with an animated frame of the
    /// same `revision`, keeping the pointer map and hit-test shapes. Refused
    /// (false) for any other revision, before the static compose finished, or
    /// when the size differs from the composed frame's.
    pub fn present_animated(&mut self, revision: u64, frame: Frame) -> bool {
        let Some(composed) = self.composed.as_mut() else {
            return false;
        };
        if self.revision != Some(revision)
            || (composed.frame.width, composed.frame.height) != (frame.width, frame.height)
            || frame.rgba.len() != composed.frame.rgba.len()
        {
            return false;
        }
        if self.animated {
            composed.frame.rgba = frame.rgba;
        } else {
            self.static_rgba = Some(std::mem::replace(&mut composed.frame.rgba, frame.rgba));
        }
        self.animated = true;
        true
    }

    /// Restores the latest complete static compose after animated presentation.
    pub fn restore_static(&mut self) -> bool {
        if !self.animated {
            return false;
        }
        let (Some(composed), Some(rgba)) = (self.composed.as_mut(), self.static_rgba.take()) else {
            self.animated = false;
            return false;
        };
        composed.frame.rgba = rgba;
        self.animated = false;
        true
    }

    pub fn composed(&self) -> Option<&Composed> {
        self.composed.as_ref()
    }

    /// Status line and whether it reports a problem.
    pub fn status(&self) -> Option<(&str, bool)> {
        self.status
            .as_ref()
            .map(|(text, error)| (text.as_str(), *error))
    }

    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    pub fn set_name(&mut self, name: &str) {
        if self.name.as_deref() != Some(name) {
            self.name = Some(name.to_owned());
        }
    }

    /// True while a frame of an earlier compose is parked for drawing (tests).
    #[cfg(test)]
    pub(super) fn has_previous(&self) -> bool {
        self.previous.is_some()
    }

    /// True when `show(_, revision, stage)` would not start new work.
    pub fn is_current(&self, revision: u64, stage: (u32, u32)) -> bool {
        self.revision == Some(revision) && self.stage == stage
    }

    /// Composes the session's `bytes` at `stage` size, superseding any
    /// in-flight work. A repeat of the same revision and stage is ignored.
    pub fn show(&mut self, bytes: &[u8], revision: u64, stage: (u32, u32)) {
        if self.is_current(revision, stage) {
            return;
        }
        self.revision = Some(revision);
        self.stage = stage;
        self.parsed = canvas::natural_size(bytes);
        if let Some(shown) = self.composed.take() {
            self.previous = Some(shown);
        }
        self.animated = false;
        self.static_rgba = None;
        let name = self.name().unwrap_or("Untitled").to_owned();
        self.status = Some((format!("Composing {name}..."), false));
        match &self.worker {
            Some(worker) => {
                self.generation = worker.request(Arc::new(bytes.to_vec()), stage.0, stage.1);
                self.compose_requests += 1;
            }
            None => self.status = Some(("Cannot start the compose worker".to_owned(), true)),
        }
    }

    /// Opens `path` into the session (the one entry for the CLI argument) and
    /// shows it at `stage` size. On failure the session and the previous frame
    /// stay and the status reports the typed error.
    pub fn open(&mut self, app: &mut AppDocument, path: &Path, stage: (u32, u32)) {
        let name = path.file_name().map_or_else(
            || path.display().to_string(),
            |n| n.to_string_lossy().into_owned(),
        );
        let failure = match app.open_path(path) {
            Ok(()) => match app.source_bytes() {
                Ok(bytes) => {
                    self.name = Some(name);
                    self.selected = None;
                    self.canvas.hover = None;
                    // A new document is fitted again (see `compose_target`).
                    self.artboard = None;
                    self.show(&bytes, app.revision(), stage);
                    // `show` parks the old frame as `previous`; it belongs to a different
                    // document here, so it must never be drawn while the new one composes.
                    self.previous = None;
                    return;
                }
                Err(error) => error,
            },
            Err(error) => error,
        };
        self.status = Some((
            format!("Cannot open {name}: {}", failure.user_message()),
            true,
        ));
    }

    /// Takes a finished result for the newest request. True when state changed.
    pub fn poll(&mut self) -> bool {
        let Some(event) = self.worker.as_ref().and_then(ComposeWorker::poll) else {
            return false;
        };
        self.accept(event)
    }

    /// Applies a worker event; results of any generation but the newest
    /// request are dropped. True when state changed.
    pub(super) fn accept(&mut self, event: WorkerEvent) -> bool {
        if event.generation != self.generation {
            return false;
        }
        let name = self.name().unwrap_or("document").to_owned();
        match event.result {
            Ok(composed) => {
                self.status = Some((status_text(&name, &composed), false));
                if self
                    .selected
                    .is_some_and(|i| !composed.shapes.iter().any(|s| s.scene_index == i))
                {
                    self.selected = None;
                }
                self.composed = Some(composed);
                self.previous = None;
                self.animated = false;
                self.static_rgba = None;
            }
            Err(ComposeError::Cancelled) => return false,
            Err(error) => {
                self.composed = None;
                self.previous = None;
                self.animated = false;
                self.static_rgba = None;
                self.status = Some((format!("{name}: {error}"), true));
            }
        }
        true
    }

    /// Window pointer position (logical points) to canvas pixels in `stage`.
    pub fn canvas_point(stage: Rect, logical: (u32, u32)) -> Option<(f64, f64)> {
        let x = logical
            .0
            .checked_sub(stage.x)
            .filter(|x| *x < stage.width)?;
        let y = logical
            .1
            .checked_sub(stage.y)
            .filter(|y| *y < stage.height)?;
        Some((f64::from(x), f64::from(y)))
    }

    /// Handles a primary click in the window. True when the status changed.
    pub fn pointer(&mut self, stage: Rect, logical: (u32, u32)) -> bool {
        self.pointer_with(stage, logical, false)
    }

    /// Like [`DocumentView::pointer`]; with `select` (the select tool is
    /// active) the click also picks the topmost shape under it, or clears the
    /// selection when it lands on empty canvas. The window position goes
    /// through the canvas view, so zoom and pan do not change what is hit.
    pub fn pointer_with(&mut self, stage: Rect, logical: (u32, u32), select: bool) -> bool {
        let Some(composed) = &self.composed else {
            return false;
        };
        let Some((cx, cy)) = Self::canvas_point(stage, logical) else {
            return false;
        };
        let Some(map) = self.view_map() else {
            return false;
        };
        let Some((dx, dy)) = map.to_doc(cx, cy) else {
            return false;
        };
        let (fx, fy) = map.view_to_frame((cx, cy));
        let name = self.name().unwrap_or("document");
        let hit = composed.hit_test(fx, fy);
        let text = match hit {
            Some(index) => format!("{name}: shape {index} at ({dx:.1}, {dy:.1})"),
            None => format!("{name}: no shape at ({dx:.1}, {dy:.1})"),
        };
        if select && !self.on_selected_handle((cx, cy)) {
            self.selected = hit;
        }
        self.status = Some((text, false));
        true
    }

    /// True when the view point `p` grabs a resize handle of the selected
    /// shape. Handles sit on the bounding box, often outside the shape, so such
    /// a press must keep the selection instead of clearing it as an empty click.
    fn on_selected_handle(&self, p: (f64, f64)) -> bool {
        let (Some(shown), Some(selected)) = (self.shown(), self.selected) else {
            return false;
        };
        let bounds = shown
            .shapes
            .iter()
            .find(|s| s.scene_index == selected)
            .and_then(|shape| self.map_for(shown)?.shape_bounds(shape));
        let Some(b) = bounds else {
            return false;
        };
        let reach = crate::canvas::HANDLE as f64 / 2.0 + 2.0;
        crate::canvas::handle_points(b)
            .iter()
            .any(|h| (h.0 - p.0).abs() <= reach && (h.1 - p.1).abs() <= reach)
    }

    /// Natural (100%) size of the artboard once the shell has adopted it.
    pub fn artboard(&self) -> Option<(f64, f64)> {
        self.artboard
    }

    /// Number of composes requested so far.
    pub fn compose_requests(&self) -> u64 {
        self.compose_requests
    }

    /// The compose size the shell should ask for (use it as the `stage` of
    /// `show` and `stage::sync`): the stage until the document's size is known,
    /// then the artboard at the resolution the current zoom needs, which is
    /// kept while the zoom stays within the cached resolution band. Also adopts
    /// the artboard and refits while the view follows the stage.
    pub fn compose_target(&mut self, stage: Rect, backing: u32) -> (u32, u32) {
        let Some(natural) = self.parsed else {
            return (stage.width, stage.height);
        };
        let size = (f64::from(stage.width), f64::from(stage.height));
        if self.artboard != Some(natural) {
            self.artboard = Some(natural);
            self.canvas.fit = true;
        }
        self.canvas.refit(size, natural);
        let current = (self.stage != (0, 0)).then_some(self.stage);
        canvas::target_compose_size(natural, self.canvas.view.zoom, backing, current)
    }

    /// The frame being presented: the newest compose, else the parked one.
    fn shown(&self) -> Option<&Composed> {
        self.composed.as_ref().or(self.previous.as_ref())
    }

    /// Window-point mapping for the newest compose (tools, picking, hover).
    pub fn view_map(&self) -> Option<ViewMap> {
        self.map_for(self.composed.as_ref()?)
    }

    fn map_for(&self, composed: &Composed) -> Option<ViewMap> {
        let size = (composed.frame.width, composed.frame.height);
        let artboard = self
            .artboard
            .unwrap_or((f64::from(size.0), f64::from(size.1)));
        ViewMap::new(self.canvas.view, artboard, size, composed.map)
    }

    /// Sets the hovered shape from a window point (`None` clears it). True
    /// when it changed.
    pub fn set_hover(&mut self, stage: Rect, logical: Option<(u32, u32)>) -> bool {
        let hit = logical.and_then(|at| {
            let (cx, cy) = Self::canvas_point(stage, at)?;
            let map = self.view_map()?;
            map.to_doc(cx, cy)?;
            let (fx, fy) = map.view_to_frame((cx, cy));
            self.composed.as_ref()?.hit_test(fx, fy)
        });
        let changed = hit != self.canvas.hover;
        self.canvas.hover = hit;
        changed
    }

    /// What the stage should show, or `None` while there is no usable frame.
    pub fn stage_paint(&self) -> Option<StagePaint<'_>> {
        let shown = self.shown()?;
        let map = self.map_for(shown)?;
        let size = (shown.frame.width, shown.frame.height);
        let artboard = self
            .artboard
            .unwrap_or((f64::from(size.0), f64::from(size.1)));
        let bounds = |index: usize| {
            let shape = shown.shapes.iter().find(|s| s.scene_index == index)?;
            map.shape_bounds(shape)
        };
        Some(StagePaint {
            view: self.canvas.view,
            artboard,
            frame: Some(&shown.frame),
            selection: self.selected.and_then(bounds),
            hover: self.canvas.hover.and_then(bounds),
            rotation: self.rotation,
        })
    }
}

/// Success status listing the first deferred items.
pub fn status_text(name: &str, composed: &Composed) -> String {
    let frame = &composed.frame;
    let mut text = format!(
        "{name} {}x{}, {} shapes",
        frame.width,
        frame.height,
        composed.shapes.len()
    );
    if !frame.disclosures.is_empty() {
        let listed: Vec<String> = frame
            .disclosures
            .iter()
            .take(MAX_LISTED_DISCLOSURES)
            .map(Disclosure::to_string)
            .collect();
        text.push_str(&format!(
            "; {} deferred: {}",
            frame.disclosures.len(),
            listed.join(", ")
        ));
        if frame.disclosures.len() > MAX_LISTED_DISCLOSURES {
            text.push_str(&format!(
                ", +{} more",
                frame.disclosures.len() - MAX_LISTED_DISCLOSURES
            ));
        }
    }
    text
}

pub fn fill(pixels: &mut [u8], width: u32, scale: u32, rect: Rect, color: u32) {
    let rgba = color.to_be_bytes();
    let s = scale as usize;
    for row in rect.y as usize * s..(rect.y + rect.height) as usize * s {
        let start = (row * width as usize + rect.x as usize * s) * 4;
        let end = start + rect.width as usize * s * 4;
        if let Some(span) = pixels.get_mut(start..end) {
            span.chunks_exact_mut(4)
                .for_each(|p| p.copy_from_slice(&rgba));
        }
    }
}

/// Paints the canvas well into the stage: the artboard presented through the
/// canvas view with its shadow, checkerboard and overlays, and the zoom
/// controls once the shell has adopted the artboard. Returns the original
/// frame when nothing is open. The status text is drawn by `render` from the
/// document-derived chrome text, inside the status bar.
pub fn blit_document(
    frame: RgbaFrame,
    view: &DocumentView,
    layout: &Layout,
    dark: bool,
) -> RgbaFrame {
    if view.status.is_none() {
        return frame;
    }
    let (width, height, scale) = (frame.width(), frame.height(), frame.backing_scale() as u32);
    let mut pixels = frame.pixels().to_vec();
    let mut tokens = theme_tokens(if dark { Theme::Dark } else { Theme::Light }, false);
    if view.artboard.is_none() {
        // Not adopted by the shell: the frame is the stage, shown over white.
        tokens.checker_a = 0xffff_ffff;
        tokens.checker_b = 0xffff_ffff;
    }
    let paint = view.stage_paint();
    canvas::paint_stage(
        &mut pixels,
        width,
        scale,
        layout.stage,
        &tokens,
        paint.as_ref(),
    );
    if paint.is_some() && view.artboard.is_some() {
        let percent = view.canvas.view.percent();
        canvas::paint_cluster(
            &mut pixels,
            (width, height),
            scale,
            layout.stage,
            dark,
            percent,
            true,
        );
    }
    RgbaFrame::new(width, height, f64::from(scale), pixels).unwrap_or(frame)
}
