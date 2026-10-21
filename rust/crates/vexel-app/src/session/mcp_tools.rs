//! File, attribute, delete, line/path and SVG-export tools applied to the
//! shell's [`AppDocument`]. Every mutation goes through [`AppDocument::edit`]
//! (one undo entry, dirty, `revision` bumped); every rejection leaves the
//! source bytes, history and revision untouched.

use super::mcp::{source, McpHost, McpOutcome};
use super::AppDocument;
use crate::preview::{AttributeChange, DOCUMENT_ID};
use crate::project_session::{SessionEditError, SessionError};
use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;
use vexel_json::JsonBuffer;
use vexel_mcp_access::AccessError;
use vexel_v1_document::{V1Document, V1DocumentError};

/// Largest SVG source `export_svg` returns inline.
const EXPORT_SVG_LIMIT: usize = 64 * 1024;
const JSON_LIMIT: usize = 2 * 1024 * 1024;

fn access_error(error: AccessError) -> &'static str {
    match error {
        AccessError::NotGranted => "path not granted",
        AccessError::NotFound | AccessError::NotRegularFile | AccessError::NotDirectory => {
            "path not found"
        }
        AccessError::InvalidPath => "invalid preview tool arguments",
    }
}

fn session_error(error: SessionError) -> &'static str {
    match error {
        SessionError::NoPath => "document has no path",
        SessionError::InvalidPath => "invalid preview tool arguments",
        SessionError::NotAProject
        | SessionError::UnsupportedVersion
        | SessionError::Corrupt { .. }
        | SessionError::TooLarge
        | SessionError::InvalidScene(_) => "invalid project file",
        SessionError::Unreadable
        | SessionError::WriteFailed
        | SessionError::NotDurable
        | SessionError::OutOfMemory
        | SessionError::Document(_) => "file operation failed",
        SessionError::RecoveryConflict => "recovery conflict; use Save As",
        SessionError::SourceChanged => "project source changed; reopen it or use Save As",
        SessionError::PublishedStateUnknown => {
            "file was saved but project identity could not be confirmed; reopen it"
        }
        SessionError::RecoveryUnavailable => "recovery store unavailable",
        SessionError::RecoveryNotDurable => {
            "recovery snapshot published but durability is unconfirmed"
        }
        SessionError::RecoveryCleanup => "project saved but recovery cleanup failed",
        SessionError::RecoveryInvalid => "invalid recovery snapshot",
        SessionError::HistoryUnavailable => "project history unavailable",
    }
}

fn document_error(error: V1DocumentError) -> &'static str {
    match error {
        V1DocumentError::MissingStableId | V1DocumentError::InvalidStableId => "unknown element",
        V1DocumentError::InvalidColor => "invalid preview fill color",
        V1DocumentError::Unsupported | V1DocumentError::UnsupportedTransform => {
            "unsupported attribute"
        }
        _ => "shape validation failed",
    }
}

fn extension_is(path: &str, extension: &str) -> bool {
    path.len() > extension.len() + 1
        && path.as_bytes()[path.len() - extension.len() - 1] == b'.'
        && path.as_bytes()[path.len() - extension.len()..]
            .eq_ignore_ascii_case(extension.as_bytes())
}

fn done(operation: &str, element: Option<&str>, app: &AppDocument) -> McpOutcome {
    let element = element.map_or_else(String::new, |id| format!(",\"elementId\":\"{id}\""));
    McpOutcome::Reply(format!(
        "{{\"ok\":true,\"operation\":\"{operation}\"{element},\"dirty\":{}}}",
        app.is_dirty()
    ))
}

impl McpHost {
    /// `save`: the current path must still be inside a grant.
    pub(super) fn save(&mut self, app: &mut AppDocument) -> Result<McpOutcome, &'static str> {
        let path = app.path().ok_or("document has no path")?.to_vec();
        self.policy()
            .resolve_save_path(OsStr::from_bytes(&path))
            .map_err(access_error)?;
        app.save().map_err(session_error)?;
        Ok(done("save", None, app))
    }

    /// `save_as`: `.vexel` inside a granted directory (symlinks resolved first).
    pub(super) fn save_as(
        &mut self,
        app: &mut AppDocument,
        path: &str,
    ) -> Result<McpOutcome, &'static str> {
        if !extension_is(path, "vexel") {
            return Err("unsupported file type");
        }
        let canonical = self
            .policy()
            .resolve_save_path(path)
            .map_err(access_error)?;
        app.save_as(canonical.as_os_str().as_bytes())
            .map_err(session_error)?;
        Ok(done("save_as", None, app))
    }

    /// `document_open`: an existing `.vexel` or `.svg` inside a grant replaces
    /// the document; the id allocation restarts from the opened source ids.
    pub(super) fn open(
        &mut self,
        app: &mut AppDocument,
        path: &str,
    ) -> Result<McpOutcome, &'static str> {
        if !extension_is(path, "vexel") && !extension_is(path, "svg") {
            return Err("unsupported file type");
        }
        // The grant check runs first so a path outside every grant never
        // reveals whether it exists.
        let granted = self
            .policy()
            .resolve_save_path(path)
            .map_err(access_error)?;
        let canonical = self
            .policy()
            .resolve_existing(&granted)
            .map_err(access_error)?;
        app.open_path(&canonical).map_err(session_error)?;
        self.next_id = 1;
        Ok(McpOutcome::Reply(format!(
            "{{\"ok\":true,\"operation\":\"document_open\",\"documentId\":\"{DOCUMENT_ID}\",\"dirty\":{}}}",
            app.is_dirty()
        )))
    }

    /// `element_add` for templates with an explicit-id draw call.
    pub(super) fn add_element(
        &mut self,
        app: &mut AppDocument,
        add: impl FnOnce(&mut V1Document, &str) -> Result<(), V1DocumentError>,
    ) -> Result<McpOutcome, &'static str> {
        let (id, count) = self.reserve_id(app)?;
        app.edit(|doc| add(doc, &id)).map_err(|error| match error {
            SessionEditError::Operation(error) => document_error(error),
            SessionEditError::History(_) => "project history unavailable",
        })?;
        self.next_id += 1;
        Ok(McpOutcome::Reply(format!(
            "{{\"ok\":true,\"operation\":\"element_add\",\"elementId\":\"{id}\",\"dirty\":{},\"shapeCount\":{},\"message\":\"Shape added\"}}",
            app.is_dirty(),
            count + 1
        )))
    }
}

pub(super) fn attribute_set(
    app: &mut AppDocument,
    id: &str,
    change: &AttributeChange,
) -> Result<McpOutcome, &'static str> {
    app.document().element_kind(id).map_err(document_error)?;
    let current = |name: &str| -> Result<Option<String>, &'static str> {
        Ok(tag_attribute(&source(app)?, id, name))
    };
    match change {
        AttributeChange::Fill(colour) => app.edit(|doc| doc.set_fill(id, colour)),
        AttributeChange::Stroke(colour) => {
            let width = current("stroke-width")?
                .and_then(|w| w.parse::<f64>().ok())
                .filter(|w| w.is_finite())
                .unwrap_or(1.0);
            app.edit(|doc| doc.set_stroke(id, colour, width))
        }
        AttributeChange::StrokeWidth(width) => {
            // The width travels with a stroke colour; without one there is
            // nothing to widen.
            let colour = current("stroke")?
                .filter(|c| c.len() == 7 && c.starts_with('#'))
                .ok_or("invalid preview tool arguments")?;
            app.edit(|doc| doc.set_stroke(id, &colour, *width))
        }
        AttributeChange::Translate(dx, dy) => app.edit(|doc| doc.translate(id, *dx, *dy)),
        AttributeChange::Generic { name, value } => {
            app.edit(|doc| doc.set_attribute(id, name, value))
        }
    }
    .map_err(|error| match error {
        SessionEditError::Operation(error) => document_error(error),
        SessionEditError::History(_) => "project history unavailable",
    })?;
    Ok(done("attribute_set", Some(id), app))
}

pub(super) fn delete(app: &mut AppDocument, id: &str) -> Result<McpOutcome, &'static str> {
    app.document().element_kind(id).map_err(document_error)?;
    app.edit(|doc| doc.delete(id))
        .map_err(|error| match error {
            SessionEditError::Operation(error) => document_error(error),
            SessionEditError::History(_) => "project history unavailable",
        })?;
    Ok(done("element_delete", Some(id), app))
}

pub(super) fn export_svg(app: &AppDocument) -> Result<McpOutcome, &'static str> {
    let source = source(app)?;
    if source.len() > EXPORT_SVG_LIMIT {
        return Err("export_svg too large");
    }
    let mut out =
        JsonBuffer::new(source.len() + 16, JSON_LIMIT).map_err(|_| "preview allocation failed")?;
    let failed = |_| "preview allocation failed";
    out.literal("{\"svg\":").map_err(failed)?;
    out.quoted(&source).map_err(failed)?;
    out.literal("}").map_err(failed)?;
    String::from_utf8(out.as_bytes().to_vec())
        .map(McpOutcome::Reply)
        .map_err(|_| "preview allocation failed")
}

/// Value of attribute `name` on the start tag whose `id` is `id`.
fn tag_attribute(source: &[u8], id: &str, name: &str) -> Option<String> {
    let needle = format!(" id=\"{id}\"");
    let at = source
        .windows(needle.len())
        .position(|w| w == needle.as_bytes())?;
    let mut i = source[..at].iter().rposition(|b| *b == b'<')? + 1;
    let space = |b: u8| b.is_ascii_whitespace();
    while i < source.len() && !space(source[i]) && source[i] != b'>' && source[i] != b'/' {
        i += 1;
    }
    loop {
        while i < source.len() && space(source[i]) {
            i += 1;
        }
        if i >= source.len() || source[i] == b'>' || source[i] == b'/' {
            return None;
        }
        let start = i;
        while i < source.len() && source[i] != b'=' && !space(source[i]) && source[i] != b'>' {
            i += 1;
        }
        let attribute = &source[start..i];
        while i < source.len() && space(source[i]) {
            i += 1;
        }
        if source.get(i) != Some(&b'=') {
            continue;
        }
        i += 1;
        while i < source.len() && space(source[i]) {
            i += 1;
        }
        let quote = *source.get(i)?;
        if quote != b'"' && quote != b'\'' {
            return None;
        }
        i += 1;
        let length = source[i..].iter().position(|b| *b == quote)?;
        let value = &source[i..i + length];
        i += length + 1;
        if attribute == name.as_bytes() {
            return Some(String::from_utf8_lossy(value).into_owned());
        }
    }
}
