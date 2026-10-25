//! `document_import`: a saved `.vexel` file becomes one group of the current
//! document, with its ids prefixed, its property tracks retargeted and,
//! on request, its rig merged in, as one undoable edit.

use super::mcp::{McpHost, McpOutcome, SOURCE_LIMIT, source, stable_ids};
use super::{AppDocument, check_room};
use crate::preview::native_timeline::{NativeOp, mcp_rig::RIG_ID, settings};
use crate::project_session::SessionEditError;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::sync::atomic::AtomicBool;
use vexel_json::{JsonDocument, JsonType};
use vexel_mcp_access::AccessError;
use vexel_timeline::EditLimits;
use vexel_v1_document::{
    ElementKind, PreparedImport, V1DocumentError, prepare_import, valid_prefix,
};

/// A validated `document_import` request.
#[derive(Clone, Debug, PartialEq)]
pub struct ImportOp {
    pub path: String,
    pub parent: String,
    /// `None`: derived from the file name.
    pub prefix: Option<String>,
    pub transform: Option<[f64; 5]>,
    pub include_timeline: bool,
    pub include_rig: bool,
}

fn access_error(error: AccessError) -> &'static str {
    match error {
        AccessError::NotGranted => "path not granted",
        AccessError::NotFound | AccessError::NotRegularFile | AccessError::NotDirectory => {
            "path not found"
        }
        AccessError::InvalidPath => "invalid preview tool arguments",
    }
}

fn import_error(error: V1DocumentError) -> &'static str {
    match error {
        V1DocumentError::InvalidStableId | V1DocumentError::MissingStableId => {
            "document_import: an id of the file is invalid or already used; choose another idPrefix"
        }
        V1DocumentError::DisallowedElement => {
            "document_import: the file holds an element that is not allowed (scripts, foreignObject, style and SMIL are refused)"
        }
        V1DocumentError::DisallowedAttribute => {
            "document_import: the file holds an event-handler (on*) attribute, which is not allowed"
        }
        V1DocumentError::ExternalReference => {
            "document_import: the file holds an external href or url(...); only local #id references are allowed"
        }
        V1DocumentError::UseCycle => "document_import: the file creates a <use> reference cycle",
        V1DocumentError::InvalidGeometry => {
            "document_import: transform values must be finite numbers up to 1000000000"
        }
        _ => "document_import: the file's scene is not a well-formed SVG within the 4 MiB limit",
    }
}

/// `kira_rig.vexel` -> `kira_rig_`: the stem made id-safe, plus a separator.
fn derived_prefix(path: &Path) -> String {
    let stem = path.file_stem().map(|s| s.to_string_lossy().into_owned());
    let mut prefix: String = stem
        .unwrap_or_default()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .take(40)
        .collect();
    if !prefix
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
    {
        prefix.insert(0, 'f');
    }
    if !prefix.ends_with('_') {
        prefix.push('_');
    }
    debug_assert!(valid_prefix(&prefix));
    prefix
}

/// Whether the renderer accepts the current source (decode plus an 8x8 render).
fn renders(app: &AppDocument) -> bool {
    app.source_bytes().is_ok_and(|source| {
        crate::animation::frame::decode(&source)
            .and_then(|scene| scene.render_sized(0.0, (8, 8), &AtomicBool::new(false)))
            .is_ok()
    })
}

fn raw(doc: &JsonDocument<'_>, at: usize) -> String {
    doc.raw_token(at)
        .map(|b| String::from_utf8_lossy(b).into_owned())
        .unwrap_or_default()
}

/// The text of the stored rig element of `source`, if any.
fn stored_rig(source: &[u8]) -> Option<String> {
    const TAG: &[u8] = b"<desc id=\"vexel-rig\"";
    let find = |hay: &[u8], needle: &[u8]| hay.windows(needle.len()).position(|w| w == needle);
    let at = find(source, TAG)?;
    let open = at + source[at..].iter().position(|b| *b == b'>')? + 1;
    let close = open + find(&source[open..], b"</desc>")?;
    String::from_utf8(source[open..close].to_vec()).ok()
}

/// `existing` rig merged with the `imported` one: bones appended, roles of
/// the existing rig win, other members (face metadata) are kept from the
/// existing rig and added from the imported one when absent.
fn merge_rig(existing: Option<&str>, imported: &str) -> Option<String> {
    let new = JsonDocument::parse(imported.as_bytes()).ok()?;
    let old = match existing {
        None => None,
        Some(text) => Some(JsonDocument::parse(text.as_bytes()).ok()?),
    };
    let mut bones = Vec::new();
    let mut roles: Vec<(String, String)> = Vec::new();
    let mut extra: Vec<(String, String)> = Vec::new();
    for doc in old.iter().chain(std::iter::once(&new)) {
        let root = doc.root();
        let list = doc.object_get(root, b"bones")?;
        let mut item = doc
            .token(list)
            .filter(|t| t.kind == JsonType::Array)?
            .first_child;
        while let Some(at) = item {
            bones.push(raw(doc, at));
            item = doc.token(at).and_then(|t| t.next_sibling);
        }
        let table = doc.object_get(root, b"roles")?;
        let mut key = doc
            .token(table)
            .filter(|t| t.kind == JsonType::Object)?
            .first_child;
        while let Some(k) = key {
            let value = doc.token(k)?.next_sibling?;
            let name = raw(doc, k);
            if !roles.iter().any(|(n, _)| *n == name) {
                roles.push((name, raw(doc, value)));
            }
            key = doc.token(value)?.next_sibling;
        }
        let mut key = doc.token(root)?.first_child;
        while let Some(k) = key {
            let value = doc.token(k)?.next_sibling?;
            let name = raw(doc, k);
            if !matches!(name.as_str(), "\"v\"" | "\"bones\"" | "\"roles\"")
                && !extra.iter().any(|(n, _)| *n == name)
            {
                extra.push((name, raw(doc, value)));
            }
            key = doc.token(value)?.next_sibling;
        }
    }
    let roles: Vec<String> = roles.iter().map(|(k, v)| format!("{k}:{v}")).collect();
    let mut out = format!(
        "{{\"v\":1,\"bones\":[{}],\"roles\":{{{}}}",
        bones.join(","),
        roles.join(",")
    );
    for (name, value) in extra {
        out.push_str(&format!(",{name}:{value}"));
    }
    out.push('}');
    Some(out)
}

impl McpHost {
    pub(super) fn import(
        &mut self,
        app: &mut AppDocument,
        op: ImportOp,
    ) -> Result<McpOutcome, &'static str> {
        if !op.path.ends_with(".vexel") {
            return Err("document_import: path must be a .vexel file");
        }
        // The grant check runs first so a path outside every grant never
        // reveals whether it exists.
        let granted = self
            .policy()
            .resolve_save_path(&op.path)
            .map_err(access_error)?;
        let canonical = self
            .policy()
            .resolve_existing(&granted)
            .map_err(access_error)?;
        let loaded = vexel_project_io::project_file_load(canonical.as_os_str().as_bytes())
            .map_err(
                |_| "document_import: not a valid .vexel project file within the size limits",
            )?;
        let bundle = loaded.view();
        match app.document().element_kind(&op.parent) {
            Ok(ElementKind::Group) => {}
            _ if op.parent == "root" => {}
            _ => return Err("document_import: parentId must be \"root\" or a group id"),
        }
        let prefix = op
            .prefix
            .clone()
            .unwrap_or_else(|| derived_prefix(&canonical));
        let prepared =
            prepare_import(bundle.scene_svg, &prefix, op.transform).map_err(import_error)?;
        let source = source(app)?;
        let ids = stable_ids(&source).len();
        // Id-less elements count too: one per `<` opening when that is more.
        let openings = prepared
            .xml
            .windows(2)
            .filter(|w| w[0] == b'<' && w[1].is_ascii_alphabetic())
            .count();
        let elements = prepared.source_ids.len().max(openings);
        if ids + elements + 1 > crate::preview::SCENE_LIMIT {
            return Err("preview scene limit reached");
        }
        if source.len() + prepared.xml.len() > SOURCE_LIMIT {
            return Err("preview source limit reached");
        }
        let host_tps = app
            .native_timeline_candidate(settings(), EditLimits::default())
            .map_err(|_| "unsupported native timeline")?
            .view()
            .map_err(|_| "unsupported native timeline")?
            .ticks_per_second();
        let tracks = if op.include_timeline {
            super::mcp_import_tracks::property_tracks(
                &bundle,
                &prepared.source_ids,
                &prefix,
                host_tps,
            )?
        } else {
            Vec::new()
        };
        let rig = match (op.include_rig, &prepared.rig) {
            (true, Some(imported)) => {
                if app
                    .document()
                    .element_get(RIG_ID)
                    .is_ok_and(|e| e.tag != "desc")
                {
                    return Err(
                        "document_import: an element other than the rig already uses the id vexel-rig",
                    );
                }
                let merged = merge_rig(stored_rig(&source).as_deref(), imported)
                    .ok_or("document_import: the file's rig is not valid")?;
                if source.len() + prepared.xml.len() + merged.len() > SOURCE_LIMIT {
                    return Err("preview source limit reached");
                }
                Some(merged)
            }
            _ => None,
        };
        drop(source);
        let bones = prepared
            .rig
            .as_ref()
            .filter(|_| op.include_rig)
            .map_or(0, |r| r.matches("\"pivot\"").count());
        let track_count = tracks.len();
        let group = app
            .begin_group()
            .map_err(|_| "project history unavailable")?;
        let result = self.import_steps(app, &op.parent, &prepared, tracks, rig);
        match result {
            Ok(()) => app
                .end_group(group)
                .map_err(|_| "project history unavailable")?,
            Err(message) => {
                app.rollback_group(group)
                    .map_err(|_| "project history unavailable")?;
                return Err(message);
            }
        }
        Ok(McpOutcome::Reply(format!(
            "{{\"ok\":true,\"operation\":\"document_import\",\"groupId\":\"{}\",\"idPrefix\":\"{prefix}\",\"elements\":{},\"tracks\":{track_count},\"rigBones\":{bones}}}",
            prepared.group_id,
            prepared
                .source_ids
                .iter()
                .filter(|id| id.as_str() != RIG_ID)
                .count()
        )))
    }

    fn import_steps(
        &mut self,
        app: &mut AppDocument,
        parent: &str,
        prepared: &PreparedImport,
        tracks: Vec<crate::preview::native_timeline::NativeTrack>,
        rig: Option<String>,
    ) -> Result<(), &'static str> {
        check_room(app, prepared.source_ids.len() + 1)?;
        let was = renders(app);
        app.edit(|doc| doc.add_import(parent, prepared))
            .map_err(|e| match e {
                SessionEditError::Operation(e) => import_error(e),
                SessionEditError::History(_) => "project history unavailable",
            })?;
        if was && !renders(app) {
            return Err("document_import: the renderer cannot draw a construct of the file");
        }
        if !tracks.is_empty() {
            crate::preview::native_timeline::apply(app, NativeOp::TracksSet(tracks))?;
        }
        if let Some(json) = rig {
            let xml = format!("<desc id=\"{RIG_ID}\">{json}</desc>");
            app.edit(|doc| {
                if doc.element_get(RIG_ID).is_ok() {
                    doc.delete(RIG_ID)?;
                }
                doc.add_fragment("root", xml.as_bytes()).map(|_| ())
            })
            .map_err(|e| match e {
                SessionEditError::Operation(_) => "document_import: could not store the rig",
                SessionEditError::History(_) => "project history unavailable",
            })?;
            if crate::preview::native_timeline::mcp_rig::load(app).is_err() {
                return Err(
                    "document_import: the merged rig is invalid (duplicate or unknown bone ids)",
                );
            }
        }
        Ok(())
    }
}
