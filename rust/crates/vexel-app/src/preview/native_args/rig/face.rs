//! Arguments of the face tools (`mouth_set`, `lip_sync`, `eyes_set`, `blink`):
//! bounded, with messages that name the offending member.

use super::{JsonDocument, PreviewArgumentError, document, invalid, members};
use crate::preview::native_timeline::mcp_face::FaceOp;
use crate::preview::native_timeline::mcp_mouth::{Mouth, valid_id};
use vexel_timeline::lipsync;

fn id_at(doc: &JsonDocument<'_>, at: usize) -> Option<String> {
    let raw = doc.raw_token(at)?;
    if raw.len() > 6 * 128 + 2 {
        return None;
    }
    String::from_utf8(doc.decoded_string(at)?)
        .ok()
        .filter(|s| valid_id(s))
}

fn mouth_set(
    doc: &JsonDocument<'_>,
    args: usize,
    tool: &str,
) -> Result<FaceOp, PreviewArgumentError> {
    document(doc, args, tool)?;
    let character = doc
        .object_get(args, b"character")
        .and_then(|at| id_at(doc, at))
        .ok_or_else(|| invalid(tool, "character", "a <g> group id"))?;
    let form = "an object mapping rest, A, E, I, O, U, MBP, FV, L, WQ, CDGK or TH to <g> group ids; rest is required";
    let table = doc
        .object_get(args, b"shapes")
        .and_then(|at| members(doc, at, lipsync::VISEMES.len()))
        .ok_or_else(|| invalid(tool, "shapes", form))?;
    let mut shapes: Vec<(u8, String)> = Vec::new();
    for (name, value) in table {
        let viseme = lipsync::index(&name).ok_or_else(|| invalid(tool, "shapes", form))?;
        let group = id_at(doc, value).ok_or_else(|| invalid(tool, "shapes", form))?;
        if shapes.iter().any(|(v, _)| *v == viseme) {
            return Err(invalid(tool, "shapes", form));
        }
        shapes.push((viseme, group));
    }
    shapes.sort();
    if shapes.first().map(|s| s.0) != Some(lipsync::REST) {
        return Err(invalid(tool, "shapes.rest", form));
    }
    Ok(FaceOp::Mouth(Mouth { character, shapes }))
}

/// `None` when `tool` is not a face tool.
pub(super) fn parse(
    doc: &JsonDocument<'_>,
    args: usize,
    tool: &str,
) -> Option<Result<FaceOp, PreviewArgumentError>> {
    Some(match tool {
        "mouth_set" => mouth_set(doc, args, tool),
        _ => return None,
    })
}
