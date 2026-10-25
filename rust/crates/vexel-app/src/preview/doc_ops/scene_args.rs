//! Arguments of `scene_apply`: the element tree flattened into steps, every
//! member parsed by the parser of the tool it mirrors, with messages that
//! name the member (`elements[12].children[3]: ...`).

use super::super::add_extras::{self, explicit_id};
use super::super::ext_paint;
use super::super::ext_structure::document;
use super::super::{
    JsonType, Operation, PreviewArgumentError, color::parse_color, native_args, parse_operation_in,
};
use super::{DocOp, ext_use};
use crate::preview::native_timeline::NativeOp;
use crate::session::mcp_scene::{Scene, Step, StepKind};
use vexel_json::JsonDocument;
use vexel_v1_document::MAX_FRAGMENT_BYTES;

const TOOL: &str = "scene_apply";
/// Deepest nesting of `children`.
const MAX_DEPTH: usize = 16;
const TOP_KEYS: [&[u8]; 6] = [
    b"documentId",
    b"elements",
    b"tracks",
    b"rig",
    b"duration",
    b"background",
];
const SHAPES: [&[u8]; 8] = [
    b"template",
    b"id",
    b"parentId",
    b"attributes",
    b"opacity",
    b"transform",
    b"gradientId",
    b"paint",
];
const TEXT: [&[u8]; 9] = [
    b"template",
    b"id",
    b"parentId",
    b"x",
    b"y",
    b"content",
    b"fontSize",
    b"anchor",
    b"fill",
];
const GROUP: [&[u8]; 6] = [
    b"template",
    b"id",
    b"parentId",
    b"opacity",
    b"transform",
    b"children",
];
const USE: [&[u8]; 10] = [
    b"template",
    b"id",
    b"parentId",
    b"refId",
    b"x",
    b"y",
    b"fill",
    b"stroke",
    b"opacity",
    b"transform",
];
const FRAGMENT: [&[u8]; 3] = [b"template", b"parentId", b"xml"];

fn fail(label: &str, message: &str) -> PreviewArgumentError {
    PreviewArgumentError::Actionable(format!("{TOOL} {label}: {message}"))
}

fn wrap(label: &str, error: PreviewArgumentError) -> PreviewArgumentError {
    match error {
        PreviewArgumentError::Generic(m) => fail(
            label,
            &format!("{m}; check the member names and that attribute values are strings"),
        ),
        PreviewArgumentError::Actionable(m) => fail(label, &m),
    }
}

/// Every key of `object` must be in `allowed`.
fn only_keys(
    doc: &JsonDocument<'_>,
    object: usize,
    allowed: &[&[u8]],
    label: &str,
) -> Result<(), PreviewArgumentError> {
    let mut key = doc.token(object).and_then(|t| t.first_child);
    while let Some(k) = key {
        let known = doc
            .decoded_string(k)
            .is_some_and(|name| allowed.contains(&name.as_slice()));
        if !known {
            let name = doc.decoded_string(k).unwrap_or_default();
            let name: String = String::from_utf8_lossy(&name)
                .chars()
                .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
                .take(32)
                .collect();
            return Err(fail(
                label,
                &format!(
                    "unknown member `{name}` for this template; see the scene_apply description"
                ),
            ));
        }
        key = doc
            .token(k)
            .and_then(|t| t.next_sibling)
            .and_then(|v| doc.token(v))
            .and_then(|t| t.next_sibling);
    }
    Ok(())
}

fn text_of(doc: &JsonDocument<'_>, object: usize, key: &[u8]) -> Option<String> {
    let token = doc.object_get(object, key)?;
    let raw = doc.raw_token(token)?;
    (raw.len() <= 1538)
        .then(|| doc.decoded_string(token))
        .flatten()
        .and_then(|b| String::from_utf8(b).ok())
}

fn optional_id(
    doc: &JsonDocument<'_>,
    object: usize,
    label: &str,
) -> Result<Option<String>, PreviewArgumentError> {
    if doc.object_get(object, b"id").is_none() {
        return Ok(None);
    }
    text_of(doc, object, b"id")
        .filter(|id| id.len() <= 128 && explicit_id(id))
        .map(Some)
        .ok_or_else(|| {
            fail(
                label,
                "id must be 1-128 characters from A-Z a-z 0-9 _ - . starting with a letter or _",
            )
        })
}

/// The object's source text with `documentId` first, for the parsers of
/// tools that require it.
fn with_document(doc: &JsonDocument<'_>, object: usize) -> Option<Vec<u8>> {
    let raw = doc.raw_token(object)?;
    let inner = raw.strip_prefix(b"{")?;
    let mut out = format!("{{\"documentId\":\"{}\"", crate::preview::DOCUMENT_ID).into_bytes();
    if !inner.iter().all(u8::is_ascii_whitespace) && inner != b"}" {
        out.push(b',');
    }
    out.extend_from_slice(inner);
    Some(out)
}

struct Walk<'a, 'b> {
    doc: &'a JsonDocument<'b>,
    steps: Vec<Step>,
}

impl Walk<'_, '_> {
    fn element(
        &mut self,
        at: usize,
        label: String,
        nested_in: Option<&str>,
        depth: usize,
    ) -> Result<(), PreviewArgumentError> {
        let doc = self.doc;
        if doc.token(at).is_none_or(|t| t.kind != JsonType::Object) {
            return Err(fail(&label, "must be an object with a template"));
        }
        if self.steps.len() >= crate::preview::SCENE_LIMIT {
            return Err(fail(&label, "more than 4096 elements"));
        }
        let template = text_of(doc, at, b"template").unwrap_or_default();
        let label = format!("{label} ({template})");
        let explicit_parent = match doc.object_get(at, b"parentId") {
            None => None,
            Some(_) => Some(
                text_of(doc, at, b"parentId")
                    .filter(|p| !p.is_empty() && p.len() <= 128)
                    .ok_or_else(|| fail(&label, "parentId must be \"root\" or a group id"))?,
            ),
        };
        if nested_in.is_some() && explicit_parent.is_some() {
            return Err(fail(
                &label,
                "nested elements go inside their group; parentId is for top-level entries",
            ));
        }
        let parent = nested_in
            .map(str::to_owned)
            .or(explicit_parent)
            .unwrap_or_else(|| "root".to_owned());
        let id = optional_id(doc, at, &label)?;
        let (kind, ids) = match template.as_str() {
            "rect" | "ellipse" | "line" | "path" => {
                only_keys(doc, at, &SHAPES, &label)?;
                let mut op = parse_operation_in(doc, at, "element_add", true)
                    .map_err(|e| wrap(&label, e))?;
                if nested_in.is_some() {
                    op = Operation::AddIn {
                        parent: parent.clone(),
                        operation: Box::new(op),
                    };
                }
                (StepKind::Op(op), id.into_iter().collect())
            }
            "text" => {
                only_keys(doc, at, &TEXT, &label)?;
                let Some(parsed) = ext_paint::parse(doc, at, "text_add") else {
                    return Err(fail(&label, "text is not available"));
                };
                let ext_paint::Op::TextAdd(spec) = parsed.map_err(|e| wrap(&label, e))? else {
                    return Err(fail(&label, "text is not available"));
                };
                let ids = id.iter().cloned().collect();
                (StepKind::Text { parent, id, spec }, ids)
            }
            "group" => {
                only_keys(doc, at, &GROUP, &label)?;
                let id = id.ok_or_else(|| {
                    fail(
                        &label,
                        "a group needs an id so that children and tracks can name it",
                    )
                })?;
                let extras = add_extras::parse(doc, at, Vec::new()).map_err(|e| wrap(&label, e))?;
                let mut xml = format!("<g id=\"{id}\"");
                if let Some(extras) = extras {
                    if let Some(opacity) = extras.opacity {
                        xml.push_str(&format!(" opacity=\"{opacity}\""));
                    }
                    if let Some(spec) = extras.transform {
                        let m = spec.to_matrix().map(|v| format!("{}", v + 0.0)).join(" ");
                        xml.push_str(&format!(" transform=\"matrix({m})\""));
                    }
                }
                xml.push_str("></g>");
                let op = Operation::Doc(DocOp::Use(ext_use::Op::Fragment { parent, xml }));
                (StepKind::Op(op), vec![id])
            }
            "use" => {
                only_keys(doc, at, &USE, &label)?;
                let bytes =
                    with_document(doc, at).ok_or_else(|| fail(&label, "must be an object"))?;
                let owned =
                    JsonDocument::parse(&bytes).map_err(|_| fail(&label, "invalid members"))?;
                let Some(parsed) = ext_use::parse(&owned, owned.root(), "use_add") else {
                    return Err(fail(&label, "use is not available"));
                };
                let mut op = parsed.map_err(|e| wrap(&label, e))?;
                if let ext_use::Op::Use { parent: p, .. } = &mut op {
                    p.clone_from(&parent);
                }
                (
                    StepKind::Op(Operation::Doc(DocOp::Use(op))),
                    id.into_iter().collect(),
                )
            }
            "fragment" => {
                only_keys(doc, at, &FRAGMENT, &label)?;
                let xml = doc
                    .object_get(at, b"xml")
                    .filter(|t| {
                        doc.raw_token(*t)
                            .is_some_and(|raw| raw.len() <= MAX_FRAGMENT_BYTES * 6 + 2)
                    })
                    .and_then(|t| doc.decoded_string(t))
                    .and_then(|b| String::from_utf8(b).ok())
                    .filter(|x| !x.is_empty() && x.len() <= MAX_FRAGMENT_BYTES)
                    .ok_or_else(|| {
                        fail(
                            &label,
                            "xml must be one XML element of at most 1048576 bytes",
                        )
                    })?;
                let ids = vexel_v1_document::fragment_ids(xml.as_bytes())
                    .map_err(|_| fail(&label, "xml must be one well-formed XML element"))?;
                (
                    StepKind::Op(Operation::Doc(DocOp::Use(ext_use::Op::Fragment {
                        parent,
                        xml,
                    }))),
                    ids,
                )
            }
            _ => {
                return Err(fail(
                    &label,
                    "template must be rect, ellipse, line, path, text, group, use or fragment",
                ));
            }
        };
        self.steps.push(Step {
            label: label.clone(),
            kind,
            ids,
        });
        if template == "group" {
            if let Some(children) = doc.object_get(at, b"children") {
                if depth + 1 >= MAX_DEPTH {
                    return Err(fail(&label, "groups are nested more than 16 deep"));
                }
                let array = doc
                    .token(children)
                    .filter(|t| t.kind == JsonType::Array)
                    .ok_or_else(|| fail(&label, "children must be an array"))?;
                let group = text_of(doc, at, b"id").unwrap_or_default();
                let base = label.split(" (").next().unwrap_or_default().to_owned();
                let mut item = array.first_child;
                let mut index = 0;
                while let Some(child) = item {
                    self.element(
                        child,
                        format!("{base}.children[{index}]"),
                        Some(&group),
                        depth + 1,
                    )?;
                    item = doc.token(child).and_then(|t| t.next_sibling);
                    index += 1;
                }
            }
        }
        Ok(())
    }
}

/// `None` when `tool` is not `scene_apply`.
pub(in crate::preview) fn parse(
    doc: &JsonDocument<'_>,
    args: usize,
    tool: &str,
) -> Option<Result<Scene, PreviewArgumentError>> {
    (tool == TOOL).then(|| {
        document(doc, args, tool)?;
        only_keys(doc, args, &TOP_KEYS, "arguments")?;
        let mut walk = Walk {
            doc,
            steps: Vec::new(),
        };
        if let Some(elements) = doc.object_get(args, b"elements") {
            let array = doc
                .token(elements)
                .filter(|t| {
                    t.kind == JsonType::Array
                        && (1..=crate::preview::SCENE_LIMIT).contains(&t.child_count)
                })
                .ok_or_else(|| fail("elements", "must be an array of 1-4096 entries"))?;
            let mut item = array.first_child;
            let mut index = 0;
            while let Some(at) = item {
                walk.element(at, format!("elements[{index}]"), None, 0)?;
                item = doc.token(at).and_then(|t| t.next_sibling);
                index += 1;
            }
        }
        let tracks = match doc.object_get(args, b"tracks") {
            None => None,
            Some(_) => match native_args::parse(doc, args, "tracks_set") {
                Some(Ok(Operation::Native(NativeOp::TracksSet(tracks)))) => Some(tracks),
                Some(Err(PreviewArgumentError::Actionable(m))) => {
                    return Err(PreviewArgumentError::Actionable(format!("{TOOL} {m}")));
                }
                _ => return Err(fail("tracks", "must be an array of 1-256 track objects")),
            },
        };
        let (rig, rig_bones) = match doc.object_get(args, b"rig") {
            None => (None, 0),
            Some(at) => {
                let bytes = with_document(doc, at)
                    .ok_or_else(|| fail("rig", "must be an object {bones, roles?}"))?;
                let owned = JsonDocument::parse(&bytes)
                    .map_err(|_| fail("rig", "must be an object {bones, roles?}"))?;
                let Some(parsed) = native_args::rig::parse(&owned, owned.root(), "rig_create")
                else {
                    return Err(fail("rig", "rig is not available"));
                };
                let op = parsed.map_err(|e| wrap("rig", e))?;
                let bones = match &op {
                    native_args::rig::Op::Create(spec) => spec.bones.len(),
                    _ => 0,
                };
                (Some(Operation::Doc(DocOp::Rig(op))), bones)
            }
        };
        let duration = match doc.object_get(args, b"duration") {
            None => None,
            Some(at) => Some(
                doc.token(at)
                    .filter(|t| t.kind == JsonType::Number)
                    .and_then(|_| doc.uint(at))
                    .and_then(|v| u32::try_from(v).ok())
                    .filter(|v| *v > 0)
                    .ok_or_else(|| {
                        fail(
                            "duration",
                            "must be an integer number of ticks from 1 through 4294967295",
                        )
                    })?,
            ),
        };
        let background = match doc.object_get(args, b"background") {
            None => None,
            Some(_) => Some(
                text_of(doc, args, b"background")
                    .and_then(|c| parse_color(&c).ok())
                    .map(|rgb| format!("#{rgb:06X}"))
                    .ok_or_else(|| fail("background", "must be a colour like #336699 or #369"))?,
            ),
        };
        if walk.steps.is_empty()
            && tracks.is_none()
            && rig.is_none()
            && duration.is_none()
            && background.is_none()
        {
            return Err(fail(
                "arguments",
                "give at least one of elements, tracks, rig, duration or background",
            ));
        }
        Ok(Scene {
            steps: walk.steps,
            tracks,
            rig,
            rig_bones,
            duration,
            background,
        })
    })
}
