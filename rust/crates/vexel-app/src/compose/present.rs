//! Shell-side state for one opened SVG and its presentation into the stage.
//!
//! The composed frame is 1 pixel per point at the stage size. On a Retina
//! backing the presenter replicates each pixel scale x scale, so the document
//! fills the same logical area but is not rendered at native sharpness (a
//! disclosed later item).

use super::{ComposeError, ComposeWorker, Composed, Disclosure, open_path};
use crate::model::Layout;
use std::path::Path;
use std::sync::Arc;
use vexel_platform_apple::{RgbaFrame, TextEmphasis, TextFace, draw_text_rgba};
use vexel_ui::{Rect, Theme, theme_tokens};

const MAX_LISTED_DISCLOSURES: usize = 3;

/// The opened document, its latest composed frame and a status message.
pub struct DocumentView {
    worker: Option<ComposeWorker>,
    source: Option<(String, Arc<Vec<u8>>)>,
    stage: (u32, u32),
    generation: u64,
    composed: Option<Composed>,
    status: Option<(String, bool)>,
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
            source: None,
            stage: (0, 0),
            generation: 0,
            composed: None,
            status: None,
        }
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
        self.source.as_ref().map(|(name, _)| name.as_str())
    }

    /// Opens `path` (the one entry for CLI and menu open) and starts composing
    /// at `stage` size. On failure the previous document stays and the status
    /// reports the typed error.
    pub fn open(&mut self, path: &Path, stage: (u32, u32)) {
        match open_path(path) {
            Ok(opened) => {
                let name = opened.display_name();
                self.source = Some((name, Arc::new(opened.bytes)));
                self.stage = stage;
                self.composed = None;
                self.request();
            }
            Err(error) => {
                let name = path.file_name().map_or_else(
                    || path.display().to_string(),
                    |n| n.to_string_lossy().into_owned(),
                );
                self.status = Some((format!("Cannot open {name}: {error}"), true));
            }
        }
    }

    /// Recomposes at a new stage size, superseding any in-flight work.
    pub fn resize(&mut self, stage: (u32, u32)) {
        if stage != self.stage {
            self.stage = stage;
            if self.source.is_some() {
                self.composed = None;
                self.request();
            }
        }
    }

    fn request(&mut self) {
        let Some((name, bytes)) = &self.source else {
            return;
        };
        let (w, h) = self.stage;
        self.status = Some((format!("Composing {name}..."), false));
        match &self.worker {
            Some(worker) => self.generation = worker.request(bytes.clone(), w, h),
            None => self.status = Some(("Cannot start the compose worker".to_owned(), true)),
        }
    }

    /// Takes a finished result for the newest request. True when state changed.
    pub fn poll(&mut self) -> bool {
        let Some(event) = self.worker.as_ref().and_then(ComposeWorker::poll) else {
            return false;
        };
        if event.generation != self.generation {
            return false;
        }
        let name = self.name().unwrap_or("document").to_owned();
        match event.result {
            Ok(composed) => {
                self.status = Some((status_text(&name, &composed), false));
                self.composed = Some(composed);
            }
            Err(ComposeError::Cancelled) => return false,
            Err(error) => {
                self.composed = None;
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
        let text = match composed.hit_test(cx, cy) {
            Some(index) => format!("{name}: shape {index} at ({dx:.1}, {dy:.1})"),
            None => format!("{name}: no shape at ({dx:.1}, {dy:.1})"),
        };
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

fn fill(pixels: &mut [u8], width: u32, scale: u32, rect: Rect, color: u32) {
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

/// Copies the composed frame over white into the stage and writes the status
/// into the status bar. Returns the original frame when nothing is open or
/// the composed copy cannot be built; a status text failure only omits text.
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
    if let Some(composed) = view
        .composed()
        .filter(|c| c.frame.width == layout.stage.width && c.frame.height == layout.stage.height)
    {
        let (sw, s) = (composed.frame.width as usize, scale as usize);
        for (i, px) in composed.frame.rgba.chunks_exact(4).enumerate() {
            let a = u32::from(px[3]);
            let over = |c: u8| (u32::from(c) * a + 255 * (255 - a) + 127) / 255;
            let out = [over(px[0]) as u8, over(px[1]) as u8, over(px[2]) as u8, 255];
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
    } else {
        fill(&mut pixels, width, scale, layout.stage, tokens.canvas_well);
    }
    if let Some((text, error)) = view.status() {
        let bar = Rect {
            width: layout.status.width.saturating_sub(176),
            ..layout.status
        };
        fill(&mut pixels, width, scale, bar, tokens.panel_raised);
        let clip = (14, bar.y + 3, bar.width.saturating_sub(14), 22);
        let color = if error {
            tokens.error
        } else {
            tokens.text_secondary
        };
        let _ = draw_text_rgba(
            &mut pixels,
            width,
            height,
            width as usize * 4,
            scale,
            text,
            clip,
            color,
            TextFace::System,
            TextEmphasis::Regular,
            11,
        );
    }
    RgbaFrame::new(width, height, f64::from(scale), pixels).unwrap_or(frame)
}
