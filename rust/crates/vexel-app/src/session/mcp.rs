//! MCP preview operations applied to the shell's [`AppDocument`].
//!
//! `--mcp-preview` no longer runs a separate preview window with its own
//! document: the shell serves the MCP requests and every mutation goes through
//! [`AppDocument::edit`], so it marks the document dirty, bumps `revision` and
//! is saved, undone and re-rendered like any other edit. Element ids are read
//! from the document source, so opened documents work too.

use super::mcp_tools::{attribute_set, delete, export_svg};
use super::{AppDocument, McpGrants};
use crate::animation::driver::Driver;
use crate::animation::native_export::NativeExportSource;
use crate::animation::playback::LoopClock;
use crate::preview::{
    Control, DOCUMENT_ID, Operation, SCENE_LIMIT, Shape, animation_list_json, document_duration,
    object_id, validate_animation_fragment, validate_shape,
};
use crate::project_session::{SessionEditError, SessionError};
use vexel_mcp_access::AccessPolicy;
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

/// Id allocation for the MCP session. Playback is not kept here: the shell
/// passes the one shared [`Driver`] (keyboard, strip and MCP use the same
/// clock) to [`McpHost::handle_with`]. The document itself lives only in the
/// [`AppDocument`].
pub struct McpHost {
    pub(super) next_id: u64,
    /// The one grant policy of this process: every path tool and the GIF
    /// export job use it. Fixed at construction.
    policy: AccessPolicy,
}

impl Default for McpHost {
    fn default() -> Self {
        Self::new()
    }
}

impl McpHost {
    /// A host whose policy comes from the environment and config file only
    /// (no explicit `--grant-dir`).
    pub fn new() -> Self {
        Self::with_policy(McpGrants::none().policy())
    }

    pub fn with_policy(policy: AccessPolicy) -> Self {
        Self { next_id: 1, policy }
    }

    /// The grant policy used by path tools; hand a clone to `ExportJob::spawn`.
    pub fn policy(&self) -> &AccessPolicy {
        &self.policy
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
            Operation::Undo => {
                if !app.can_undo() {
                    return Err("nothing to undo");
                }
                app.undo()
                    .map_err(session_edit_error)?
                    .then_some(())
                    .ok_or("project history unavailable")
                    .and_then(|()| changed(app, "Undo complete"))
            }
            Operation::Redo => {
                if !app.can_redo() {
                    return Err("nothing to redo");
                }
                app.redo()
                    .map_err(session_edit_error)?
                    .then_some(())
                    .ok_or("project history unavailable")
                    .and_then(|()| changed(app, "Redo complete"))
            }
            Operation::Control(control) => control_playback(app, control, driver),
            Operation::Save => self.save(app),
            Operation::SaveAs { path } => self.save_as(app, &path),
            Operation::Open { path } => self.open(app, &path),
            Operation::AttributeSet { element_id, change } => {
                attribute_set(app, &element_id, &change)
            }
            Operation::Delete { element_id } => delete(app, &element_id),
            Operation::AddLine {
                x1,
                y1,
                x2,
                y2,
                stroke,
                width,
            } => self.add_element(app, |doc, id| {
                doc.add_line_with_id(x1, y1, x2, y2, &stroke, width, id)
            }),
            Operation::AddPath {
                d,
                fill,
                stroke,
                width,
            } => self.add_element(app, |doc, id| {
                doc.add_path_with_id(&d, &fill, &stroke, width, id)
            }),
            Operation::Native(op) => crate::preview::native_timeline::apply(app, op),
            Operation::ExportSvg => export_svg(app),
            Operation::RenderPng {
                path,
                width,
                height,
            } => super::mcp_png::render_png(app, &self.policy, &path, width, height),
            Operation::Export {
                path,
                fps,
                duration,
            } => match NativeExportSource::from_document(app) {
                Some(native) => {
                    let gif = native
                        .export_gif()
                        .map_err(|_| "export could not render the animation")?;
                    let mut packed = b"VEXEL_NATIVE_GIF\0".to_vec();
                    packed.extend_from_slice(&(gif.frames as u64).to_le_bytes());
                    packed.extend_from_slice(&gif.delay_cs.to_le_bytes());
                    packed.extend_from_slice(&gif.width.to_le_bytes());
                    packed.extend_from_slice(&gif.height.to_le_bytes());
                    packed.extend_from_slice(&gif.bytes);
                    Ok(McpOutcome::Export {
                        source: packed,
                        path,
                        fps: 0,
                        duration: 0.0,
                    })
                }
                None => Ok(McpOutcome::Export {
                    source: source(app)?,
                    path,
                    fps,
                    duration,
                }),
            },
        }
    }

    fn add(&mut self, app: &mut AppDocument, shape: Shape) -> Result<McpOutcome, &'static str> {
        validate_shape(shape)?;
        let (id, count) = self.reserve_id(app)?;
        app.edit(|doc| add_shape(doc, shape, &id))
            .map_err(|error| match error {
                SessionEditError::Operation(_) => "shape validation failed",
                SessionEditError::History(_) => "project history unavailable",
            })?;
        self.next_id += 1;
        Ok(McpOutcome::Reply(format!(
            "{{\"ok\":true,\"shapeCount\":{},\"elementId\":\"{id}\",\"message\":\"Shape added\"}}",
            count + 1
        )))
    }

    /// Checks the scene and source limits and returns the next free
    /// `vexel-object-N` id with the current element count. The id counter
    /// advances only when the caller's edit succeeds.
    pub(super) fn reserve_id(
        &mut self,
        app: &AppDocument,
    ) -> Result<(String, usize), &'static str> {
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
        Ok((id, ids.len()))
    }
}

fn session_edit_error(error: SessionError) -> &'static str {
    match error {
        SessionError::HistoryUnavailable => "project history unavailable",
        _ => "file operation failed",
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

pub(super) fn source(app: &AppDocument) -> Result<Vec<u8>, &'static str> {
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
        .map_err(|error| match error {
            SessionEditError::Operation(_) => "invalid preview animation fragment",
            SessionEditError::History(_) => "project history unavailable",
        })?;
    if document_duration(app.document()).is_err() {
        if !app.can_undo() {
            return Err("project history unavailable");
        }
        if !app.undo().map_err(session_edit_error)? {
            return Err("project history unavailable");
        }
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
pub(super) fn stable_ids(source: &[u8]) -> Vec<String> {
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
