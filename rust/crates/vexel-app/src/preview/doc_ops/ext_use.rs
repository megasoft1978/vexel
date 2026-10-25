//! `element_add_fragment` (any validated XML element, e.g. `<defs>`, `<symbol>`,
//! `<mask>`, `<filter>`) and `use_add` (a `<use>` instance of an existing
//! element) routed onto the live document. Each call is one undoable edit.

use super::super::add_extras::{number, transform};
use super::super::ext_structure::{document, string};
use super::super::{JsonType, PreviewArgumentError};
use crate::project_session::SessionEditError;
use crate::session::{AppDocument, McpOutcome, check_room};
use std::sync::atomic::AtomicBool;
use vexel_json::JsonDocument;
use vexel_v1_document::{UseSpec, V1DocumentError, fragment_ids};

/// Longest `xml` accepted (matches the tool schema).
const XML_LIMIT: usize = 1024 * 1024;

#[derive(Clone, Debug, PartialEq)]
pub enum Op {
    Fragment {
        parent: String,
        xml: String,
    },
    Use {
        parent: String,
        target: String,
        id: Option<String>,
        spec: UseSpec,
    },
}

fn invalid(tool: &str, argument: &str, form: &str) -> PreviewArgumentError {
    PreviewArgumentError::Actionable(format!(
        "{tool} argument {argument} is missing or invalid; use {form}"
    ))
}

fn optional_number(
    doc: &JsonDocument<'_>,
    args: usize,
    key: &'static str,
) -> Result<Option<f64>, PreviewArgumentError> {
    doc.object_get(args, key.as_bytes())
        .map(|token| number(doc, token).ok_or_else(|| invalid("use_add", key, "a finite number")))
        .transpose()
}

fn optional_paint(
    doc: &JsonDocument<'_>,
    args: usize,
    key: &'static str,
) -> Result<Option<String>, PreviewArgumentError> {
    let Some(token) = doc.object_get(args, key.as_bytes()) else {
        return Ok(None);
    };
    doc.decoded_string(token)
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .filter(|v| v == "none" || v == "currentColor" || (v.len() == 7 && v.starts_with('#')))
        .map(Some)
        .ok_or_else(|| invalid("use_add", key, "#rrggbb, none or currentColor"))
}

/// `None` when `tool` is not in this family.
pub(in crate::preview) fn parse(
    doc: &JsonDocument<'_>,
    args: usize,
    tool: &str,
) -> Option<Result<Op, PreviewArgumentError>> {
    let operation = match tool {
        "element_add_fragment" => (|| {
            document(doc, args, tool)?;
            let xml = doc
                .object_get(args, b"xml")
                .and_then(|t| doc.decoded_string(t))
                .and_then(|bytes| String::from_utf8(bytes).ok())
                .filter(|xml| !xml.is_empty() && xml.len() <= XML_LIMIT)
                .ok_or_else(|| {
                    invalid(
                        tool,
                        "xml",
                        "one XML element of at most 1048576 bytes (1 MiB)",
                    )
                })?;
            Ok(Op::Fragment {
                parent: string(doc, args, "parentId", tool)?,
                xml,
            })
        })(),
        "use_add" => (|| {
            document(doc, args, tool)?;
            let parent = match doc.object_get(args, b"parentId") {
                None => "root".to_owned(),
                Some(_) => string(doc, args, "parentId", tool)?,
            };
            let id = match doc.object_get(args, b"id") {
                None => None,
                Some(_) => Some(string(doc, args, "id", tool)?),
            };
            let opacity = optional_number(doc, args, "opacity")?;
            if opacity.is_some_and(|o| !(0.0..=1.0).contains(&o)) {
                return Err(invalid(tool, "opacity", "a number from 0 through 1"));
            }
            let matrix = match doc.object_get(args, b"transform") {
                None => None,
                Some(token) if doc.token(token).is_some_and(|t| t.kind == JsonType::Object) => {
                    let spec = transform(doc, token)?;
                    let m = spec.to_matrix();
                    m.iter()
                        .all(|v| v.is_finite())
                        .then_some(m)
                        .ok_or_else(|| invalid(tool, "transform", "values giving a finite matrix"))?
                        .into()
                }
                Some(_) => return Err(invalid(tool, "transform", "an object like element_add")),
            };
            Ok(Op::Use {
                parent,
                target: string(doc, args, "refId", tool)?,
                id,
                spec: UseSpec {
                    x: optional_number(doc, args, "x")?.unwrap_or(0.0),
                    y: optional_number(doc, args, "y")?.unwrap_or(0.0),
                    fill: optional_paint(doc, args, "fill")?,
                    stroke: optional_paint(doc, args, "stroke")?,
                    opacity,
                    transform: matrix,
                },
            })
        })(),
        _ => return None,
    };
    Some(operation)
}

fn fragment_error(error: V1DocumentError) -> &'static str {
    match error {
        V1DocumentError::MissingStableId => {
            "element_add_fragment: parentId is not an element id in the document"
        }
        V1DocumentError::InvalidStableId => {
            "element_add_fragment: every id in the fragment must be new and valid"
        }
        V1DocumentError::DisallowedElement => {
            "element_add_fragment: element not allowed; use shapes, g, defs, symbol, use, mask, clipPath, filter/fe*, gradients, pattern, marker, text, tspan or image"
        }
        V1DocumentError::DisallowedAttribute => {
            "element_add_fragment: event-handler (on*) attributes are not allowed"
        }
        V1DocumentError::ExternalReference => {
            "element_add_fragment: href and url(...) must be local #id references (image may use data:image/)"
        }
        V1DocumentError::UseCycle => {
            "element_add_fragment: the fragment creates a <use> reference cycle"
        }
        _ => {
            "element_add_fragment: xml must be one well-formed element of at most 1048576 bytes (1 MiB)"
        }
    }
}

fn use_error(error: V1DocumentError) -> &'static str {
    match error {
        V1DocumentError::MissingStableId => "use_add: refId or parentId is not an element id",
        V1DocumentError::InvalidStableId => "use_add: id is invalid or already used",
        V1DocumentError::Unsupported | V1DocumentError::UseCycle => {
            "use_add: refId must not contain the parent (reference cycle)"
        }
        V1DocumentError::InvalidColor | V1DocumentError::InvalidGeometry => {
            "use_add: x, y, fill, stroke, opacity or transform is out of range"
        }
        _ => "use_add: could not insert the use element",
    }
}

fn history<E>(error: SessionEditError<E>, map: impl FnOnce(E) -> &'static str) -> &'static str {
    match error {
        SessionEditError::Operation(error) => map(error),
        SessionEditError::History(_) => "project history unavailable",
    }
}

/// Whether the renderer accepts the current source (decode plus an 8x8 render).
fn renders(app: &AppDocument) -> bool {
    app.source_bytes().is_ok_and(|source| {
        crate::animation::frame::decode(&source)
            .and_then(|scene| scene.render_sized(0.0, (8, 8), &AtomicBool::new(false)))
            .is_ok()
    })
}

/// Runs `edit`; when it turns a renderable document into one the renderer
/// rejects (unsupported filter primitive, blend mode...), the edit is undone.
fn guarded<R>(
    app: &mut AppDocument,
    edit: impl FnOnce(&mut AppDocument) -> Result<R, &'static str>,
    refused: &'static str,
) -> Result<R, &'static str> {
    let was = renders(app);
    let result = edit(app)?;
    if was && !renders(app) {
        app.undo().map_err(|_| "project history unavailable")?;
        return Err(refused);
    }
    Ok(result)
}

pub(crate) fn apply(app: &mut AppDocument, op: Op) -> Result<McpOutcome, &'static str> {
    match op {
        Op::Fragment { parent, xml } => {
            let declared = fragment_ids(xml.as_bytes()).map_err(fragment_error)?;
            // The fragment's root element gets an id when it has none.
            check_room(app, declared.len() + 1)?;
            let ids = guarded(
                app,
                |app| {
                    app.edit(|doc| doc.add_fragment(&parent, xml.as_bytes()))
                        .map_err(|e| history(e, fragment_error))
                },
                "element_add_fragment: the renderer does not support this construct (feTurbulence, mix-blend-mode...); edit undone. See docs/mcp.md",
            )?;
            let list = ids
                .iter()
                .map(|id| format!("\"{id}\""))
                .collect::<Vec<_>>()
                .join(",");
            Ok(McpOutcome::Reply(format!(
                "{{\"ok\":true,\"operation\":\"element_add_fragment\",\"elementIds\":[{list}]}}"
            )))
        }
        Op::Use {
            parent,
            target,
            id,
            spec,
        } => {
            check_room(app, 1)?;
            let id = app
                .edit(|doc| doc.add_use(&parent, id.as_deref(), &target, &spec))
                .map_err(|e| history(e, use_error))?;
            Ok(McpOutcome::Reply(format!(
                "{{\"ok\":true,\"operation\":\"use_add\",\"elementId\":\"{id}\",\"refId\":\"{target}\"}}"
            )))
        }
    }
}
