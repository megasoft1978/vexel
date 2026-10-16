//! MCP preview operations applied to the shell's [`AppDocument`].
//!
//! `--mcp-preview` no longer runs a separate preview window with its own
//! document: the shell serves the MCP requests and every mutation goes through
//! [`AppDocument::edit`], so it marks the document dirty, bumps `revision` and
//! is saved, undone and re-rendered like any other edit. Element ids are read
//! from the document source, so opened documents work too.

use super::AppDocument;
use crate::animation::driver::Driver;
use crate::animation::playback::LoopClock;
use crate::preview::{
    Control, DOCUMENT_ID, Operation, SCENE_LIMIT, Shape, animation_list_json, document_duration,
    object_id, validate_animation_fragment, validate_shape,
};
use std::cell::Cell;
use vexel_v1_document::{V1Document, V1DocumentError};

const SOURCE_LIMIT: usize = 64 * 1024;
const ANIMATION_LIMIT: usize = 16;

/// What the caller must do with one applied request.
#[derive(Debug, PartialEq)]
pub enum McpOutcome {
    /// Send this JSON to the client now.
    Reply(String),
    /// Start a GIF export of `source`; the export job sends the reply.
    Export {
        source: Vec<u8>,
        path: String,
        fps: u32,
        duration: f64,
    },
}

/// Seconds supplied by the caller (the legacy `handle` clock argument).
#[derive(Default)]
struct ExternalClock(Cell<f64>);

impl LoopClock for ExternalClock {
    fn now_seconds(&self) -> f64 {
        self.0.get()
    }
}

/// Id allocation for the MCP session. Playback is not kept here: the shell
/// passes the one shared [`Driver`] (keyboard, strip and MCP use the same
/// clock) to [`McpHost::handle_with`]. The document itself lives only in the
/// [`AppDocument`].
#[derive(Default)]
pub struct McpHost {
    next_id: u64,
    /// Driver for the clock-argument [`McpHost::handle`] form (tests, callers
    /// without a shell driver); the shell never uses it.
    standalone: Option<Driver<ExternalClock>>,
}

impl McpHost {
    pub fn new() -> Self {
        Self {
            next_id: 1,
            ..Self::default()
        }
    }

    /// Whether the standalone driver of [`McpHost::handle`] is playing.
    pub fn is_playing(&self) -> bool {
        self.standalone.as_ref().is_some_and(Driver::is_playing)
    }

    /// [`McpHost::handle_with`] on the host's standalone driver, whose clock
    /// reads `now` (the caller's monotonic seconds).
    pub fn handle(
        &mut self,
        app: &mut AppDocument,
        operation: Operation,
        now: f64,
    ) -> Result<McpOutcome, &'static str> {
        let mut driver = self
            .standalone
            .take()
            .unwrap_or_else(|| Driver::new(ExternalClock::default()));
        driver.playback().clock().0.set(now);
        let outcome = self.handle_with(app, operation, &mut driver);
        self.standalone = Some(driver);
        outcome
    }

    /// Applies one request; play/pause/seek steer `driver`.
    pub fn handle_with<C: LoopClock>(
        &mut self,
        app: &mut AppDocument,
        operation: Operation,
        driver: &mut Driver<C>,
    ) -> Result<McpOutcome, &'static str> {
        match operation {
            Operation::List => {
                let (w, h) = app.size();
                Ok(McpOutcome::Reply(format!(
                    "{{\"documents\":[{{\"documentId\":\"{DOCUMENT_ID}\",\"name\":\"Live preview\",\"width\":{w},\"height\":{h}}}]}}"
                )))
            }
            Operation::Info => {
                let source = source(app)?;
                let (w, h) = app.size();
                Ok(McpOutcome::Reply(format!(
                    "{{\"documentId\":\"{DOCUMENT_ID}\",\"width\":{w},\"height\":{h},\"elementCount\":{},\"sourceBytes\":{},\"dirty\":{}}}",
                    stable_ids(&source).len(),
                    source.len(),
                    app.is_dirty()
                )))
            }
            Operation::AnimationList => {
                let ids = stable_ids(&source(app)?);
                animation_list_json(app.document(), &ids).map(McpOutcome::Reply)
            }
            Operation::Add(shape) => self.add(app, shape),
            Operation::AnimationAdd { target_id, xml } => animation_add(app, &target_id, &xml),
            Operation::Undo => app
                .undo()
                .map_err(|_| "nothing to undo")
                .and_then(|()| changed(app, "Undo complete")),
            Operation::Redo => app
                .redo()
                .map_err(|_| "nothing to redo")
                .and_then(|()| changed(app, "Redo complete")),
            Operation::Control(control) => control_playback(app, control, driver),
            Operation::Export {
                path,
                fps,
                duration,
            } => Ok(McpOutcome::Export {
                source: source(app)?,
                path,
                fps,
                duration,
            }),
        }
    }

    fn add(&mut self, app: &mut AppDocument, shape: Shape) -> Result<McpOutcome, &'static str> {
        validate_shape(shape)?;
        let source = source(app)?;
        let ids = stable_ids(&source);
        if ids.len() >= SCENE_LIMIT {
            return Err("preview scene limit reached");
        }
        if source.len() > SOURCE_LIMIT {
            return Err("preview source limit reached");
        }
        let mut id = object_id(self.next_id);
        while ids.contains(&id) {
            self.next_id = self
                .next_id
                .checked_add(1)
                .ok_or("preview stable ID limit reached")?;
            id = object_id(self.next_id);
        }
        app.edit(|doc| add_shape(doc, shape, &id))
            .map_err(|_| "shape validation failed")?;
        self.next_id += 1;
        Ok(McpOutcome::Reply(format!(
            "{{\"ok\":true,\"shapeCount\":{},\"elementId\":\"{id}\",\"message\":\"Shape added\"}}",
            ids.len() + 1
        )))
    }
}

fn control_playback<C: LoopClock>(
    app: &AppDocument,
    control: Control,
    driver: &mut Driver<C>,
) -> Result<McpOutcome, &'static str> {
    let duration = document_duration(app.document())?;
    if !driver.knows(app.revision()) {
        driver.sync(app.revision(), &source(app)?);
    }
    let shown = match control {
        Control::Play => {
            if duration <= 0.0 || !driver.play() {
                return Err("no preview animation to play");
            }
            driver.time()
        }
        Control::Pause => {
            driver.pause();
            driver.time()
        }
        Control::Seek(seconds) => {
            if !seconds.is_finite() || !(0.0..=10.0).contains(&seconds) {
                return Err("invalid preview animation time");
            }
            driver.seek(seconds);
            seconds
        }
    };
    Ok(McpOutcome::Reply(format!(
        "{{\"ok\":true,\"playing\":{},\"timeSeconds\":{shown:.3}}}",
        driver.is_playing()
    )))
}

fn source(app: &AppDocument) -> Result<Vec<u8>, &'static str> {
    app.source_bytes().map_err(|_| "preview allocation failed")
}

fn changed(app: &AppDocument, message: &str) -> Result<McpOutcome, &'static str> {
    Ok(McpOutcome::Reply(format!(
        "{{\"ok\":true,\"shapeCount\":{},\"message\":\"{message}\"}}",
        stable_ids(&source(app)?).len()
    )))
}

fn animation_add(
    app: &mut AppDocument,
    target_id: &str,
    xml: &[u8],
) -> Result<McpOutcome, &'static str> {
    validate_animation_fragment(xml)?;
    let source = source(app)?;
    if count(&source, b"<animate") >= ANIMATION_LIMIT {
        return Err("preview animation limit reached");
    }
    if !stable_ids(&source).iter().any(|id| id == target_id) {
        return Err("unknown preview animation target");
    }
    app.edit(|doc| doc.add_animation_fragment(target_id, xml))
        .map_err(|_| "invalid preview animation fragment")?;
    if document_duration(app.document()).is_err() {
        let _ = app.undo();
        return Err("unsupported or malformed preview animation");
    }
    changed(app, "Animation added")
}

fn add_shape(doc: &mut V1Document, shape: Shape, id: &str) -> Result<(), V1DocumentError> {
    let (Shape::Rect { rgb, .. } | Shape::Ellipse { rgb, .. }) = shape;
    let color = format!("#{:06X}", rgb & 0xff_ffff);
    match shape {
        Shape::Rect {
            x,
            y,
            width,
            height,
            ..
        } => doc.add_rectangle_with_id(x, y, x + width, y + height, &color, id),
        Shape::Ellipse { cx, cy, rx, ry, .. } => {
            doc.add_ellipse_with_id(cx - rx, cy - ry, cx + rx, cy + ry, &color, id)
        }
    }
}

fn count(haystack: &[u8], needle: &[u8]) -> usize {
    haystack
        .windows(needle.len())
        .filter(|w| *w == needle)
        .count()
}

/// Values of every ` id="..."` attribute in `source`, in document order.
fn stable_ids(source: &[u8]) -> Vec<String> {
    const MARKER: &[u8] = b" id=\"";
    let mut ids = Vec::new();
    let mut at = 0;
    while let Some(found) = source[at..].windows(MARKER.len()).position(|w| w == MARKER) {
        let start = at + found + MARKER.len();
        let Some(len) = source[start..].iter().position(|b| *b == b'"') else {
            break;
        };
        ids.push(String::from_utf8_lossy(&source[start..start + len]).into_owned());
        at = start + len;
    }
    ids
}
