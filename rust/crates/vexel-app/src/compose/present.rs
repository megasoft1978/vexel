//! Shell-side state for one opened SVG and its presentation into the stage.
//!
//! The composed frame is 1 pixel per point at the stage size. On a Retina
//! backing the presenter replicates each pixel scale x scale, so the document
//! fills the same logical area but is not rendered at native sharpness (a
//! disclosed later item).

use super::{ComposeError, ComposeWorker, Composed, Disclosure, Frame, OwnedShape, WorkerEvent};
use crate::model::Layout;
use crate::session::AppDocument;
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
    status: Option<(String, bool)>,
    /// Scene index of the shape picked with the select tool, shown by the
    /// inspector.
    selected: Option<usize>,
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
            status: None,
            selected: None,
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
        composed.frame.rgba = frame.rgba;
        self.animated = true;
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
        if let Some(shown) = self.composed.take() {
            self.previous = Some(shown);
        }
        self.animated = false;
        let name = self.name().unwrap_or("Untitled").to_owned();
        self.status = Some((format!("Composing {name}..."), false));
        match &self.worker {
            Some(worker) => {
                self.generation = worker.request(Arc::new(bytes.to_vec()), stage.0, stage.1);
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
                    self.previous = None;
                    self.show(&bytes, app.revision(), stage);
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
            }
            Err(ComposeError::Cancelled) => return false,
            Err(error) => {
                self.composed = None;
                self.previous = None;
                self.animated = false;
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
    /// selection when it lands on empty canvas.
    pub fn pointer_with(&mut self, stage: Rect, logical: (u32, u32), select: bool) -> bool {
        let Some(composed) = &self.composed else {
            return false;
        };
        if composed.frame.width != stage.width || composed.frame.height != stage.height {
            return false;
        }
        let Some((cx, cy)) = Self::canvas_point(stage, logical) else {
            return false;
        };
        let Some((dx, dy)) = composed.map.canvas_to_doc(cx, cy) else {
            return false;
        };
        let name = self.name().unwrap_or("document");
        let hit = composed.hit_test(cx, cy);
        let text = match hit {
            Some(index) => format!("{name}: shape {index} at ({dx:.1}, {dy:.1})"),
            None => format!("{name}: no shape at ({dx:.1}, {dy:.1})"),
        };
        if select {
            self.selected = hit;
        }
        self.status = Some((text, false));
        true
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

/// Copies the composed frame over white into the stage. Returns the original
/// frame when nothing is open or the composed copy cannot be built. The status
/// text is drawn by `render` from the document-derived chrome text, inside the
/// status bar (the old label here was positioned in points on a pixel buffer,
/// so it landed at mid-height over the tool rail on Retina backings).
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
    let tokens = theme_tokens(if dark { Theme::Dark } else { Theme::Light }, false);
    let exact = view
        .composed()
        .filter(|c| c.frame.width == layout.stage.width && c.frame.height == layout.stage.height);
    if let Some(composed) = exact {
        let (sw, s) = (composed.frame.width as usize, scale as usize);
        for (i, px) in composed.frame.rgba.chunks_exact(4).enumerate() {
            let out = over_white(px);
            let (x, y) = (i % sw, i / sw);
            for dy in 0..s {
                let row = (layout.stage.y as usize + y) * s + dy;
                let col = (layout.stage.x as usize + x) * s;
                let start = (row * width as usize + col) * 4;
                if let Some(span) = pixels.get_mut(start..start + s * 4) {
                    span.chunks_exact_mut(4)
                        .for_each(|p| p.copy_from_slice(&out));
                }
            }
        }
    } else if let Some(old) = view.previous.as_ref() {
        // A compose at the new stage size is pending: show the last frame
        // scaled to fit (nearest neighbour, centred) instead of a blank stage.
        fill(&mut pixels, width, scale, layout.stage, tokens.canvas_well);
        blit_scaled(
            &mut pixels,
            width as usize,
            scale as usize,
            &old.frame,
            layout.stage,
        );
    } else {
        fill(&mut pixels, width, scale, layout.stage, tokens.canvas_well);
    }
    RgbaFrame::new(width, height, f64::from(scale), pixels).unwrap_or(frame)
}

fn over_white(px: &[u8]) -> [u8; 4] {
    let a = u32::from(px[3]);
    let over = |c: u8| ((u32::from(c) * a + 255 * (255 - a) + 127) / 255) as u8;
    [over(px[0]), over(px[1]), over(px[2]), 255]
}

/// Nearest-neighbour copy of `frame` over white into `stage`, uniformly scaled
/// to fit and centred; the rest of the stage is left as it is.
fn blit_scaled(pixels: &mut [u8], width: usize, s: usize, frame: &Frame, stage: Rect) {
    let (fw, fh) = (frame.width as usize, frame.height as usize);
    let (sw, sh) = (stage.width as usize, stage.height as usize);
    if fw == 0 || fh == 0 || sw == 0 || sh == 0 || frame.rgba.len() != fw * fh * 4 {
        return;
    }
    // Fit: the smaller of the two ratios, kept as integer fractions.
    let (dw, dh) = if sw * fh <= sh * fw {
        (sw, (fh * sw / fw).max(1))
    } else {
        ((fw * sh / fh).max(1), sh)
    };
    let (ox, oy) = ((sw - dw) / 2, (sh - dh) / 2);
    for y in 0..dh {
        let src_row = (y * fh / dh).min(fh - 1);
        for x in 0..dw {
            let src = (src_row * fw + (x * fw / dw).min(fw - 1)) * 4;
            let out = over_white(&frame.rgba[src..src + 4]);
            for dy in 0..s {
                let row = (stage.y as usize + oy + y) * s + dy;
                let col = (stage.x as usize + ox + x) * s;
                let start = (row * width + col) * 4;
                if let Some(span) = pixels.get_mut(start..start + s * 4) {
                    span.chunks_exact_mut(4)
                        .for_each(|p| p.copy_from_slice(&out));
                }
            }
        }
    }
}
