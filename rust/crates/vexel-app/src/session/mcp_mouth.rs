//! `mouth_set`: the viseme -> group table of a character, stored in the
//! document as `<desc id="vexel-mouths">{json}</desc>` under the root (next to
//! the rig), so it saves, reopens, undoes and is retargeted by renames.

use super::mcp_rig::{desc_text, inside};
use crate::project_session::SessionEditError;
use crate::session::{AppDocument, McpOutcome, check_room};
use vexel_json::JsonDocument;
use vexel_timeline::lipsync::{self, VISEMES};

/// Element id of the stored mouth sets.
pub(crate) const MOUTHS_ID: &str = "vexel-mouths";
/// Most characters with a mouth set in one document.
pub(crate) const MAX_CHARACTERS: usize = 32;
const TAG: &[u8] = b"<desc id=\"vexel-mouths\"";
const NO_MOUTHS: &str = "no mouth set for this character; call mouth_set first";

#[derive(Clone, Debug, PartialEq)]
pub struct Mouth {
    pub character: String,
    /// (viseme index, group id), ascending by viseme, `rest` present.
    pub shapes: Vec<(u8, String)>,
}

/// Whether `id` can be stored unescaped: 1-128 of `A-Za-z0-9_.:-`.
pub(crate) fn valid_id(id: &str) -> bool {
    (1..=128).contains(&id.len())
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b':' | b'-'))
}

fn to_json(sets: &[Mouth]) -> String {
    let mut out = String::from("{\"v\":1,\"sets\":[");
    for (i, set) in sets.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str(&format!("{{\"c\":\"{}\",\"s\":{{", set.character));
        for (n, (viseme, group)) in set.shapes.iter().enumerate() {
            if n > 0 {
                out.push(',');
            }
            out.push_str(&format!("\"{}\":\"{group}\"", VISEMES[*viseme as usize]));
        }
        out.push_str("}}");
    }
    out.push_str("]}");
    out
}

fn from_json(text: &[u8]) -> Option<Vec<Mouth>> {
    let doc = JsonDocument::parse(text).ok()?;
    let root = doc.root();
    if doc.uint(doc.object_get(root, b"v")?)? != 1 {
        return None;
    }
    let list = doc.object_get(root, b"sets")?;
    let count = doc.token(list)?.child_count;
    if count > MAX_CHARACTERS {
        return None;
    }
    let mut sets = Vec::new();
    for i in 0..count {
        let item = doc.array_get(list, i)?;
        let character = String::from_utf8(doc.decoded_string(doc.object_get(item, b"c")?)?).ok()?;
        let table = doc.object_get(item, b"s")?;
        let members = crate::preview::native_args::rig::members(&doc, table, VISEMES.len())?;
        let mut shapes = Vec::new();
        for (name, value) in members {
            let group = String::from_utf8(doc.decoded_string(value)?).ok()?;
            if !valid_id(&group) {
                return None;
            }
            shapes.push((lipsync::index(&name)?, group));
        }
        shapes.sort();
        let distinct = shapes.windows(2).all(|w| w[0].0 != w[1].0);
        if !valid_id(&character) || !distinct || shapes.first().map(|s| s.0) != Some(lipsync::REST)
        {
            return None;
        }
        sets.push(Mouth { character, shapes });
    }
    Some(sets)
}

/// All stored sets: `Ok(empty)` when the document has none, `Err` when the
/// stored element is forged or malformed.
fn stored(app: &AppDocument) -> Result<Vec<Mouth>, &'static str> {
    let source = app
        .source_bytes()
        .map_err(|_| "project history unavailable")?;
    if super::mcp_rig::find(&source, TAG).is_none() {
        return Ok(Vec::new());
    }
    let text =
        desc_text(&source, TAG).ok_or("the mouth sets stored in the document are invalid")?;
    from_json(text).ok_or("the mouth sets stored in the document are invalid")
}

/// The stored set of `character`.
pub(crate) fn load(app: &AppDocument, character: &str) -> Result<Mouth, &'static str> {
    stored(app)?
        .into_iter()
        .find(|m| m.character == character)
        .ok_or(NO_MOUTHS)
}

fn write(app: &mut AppDocument, sets: &[Mouth], tool: &'static str) -> Result<(), &'static str> {
    let xml = format!("<desc id=\"{MOUTHS_ID}\">{}</desc>", to_json(sets));
    app.edit(|doc| {
        if doc.element_get(MOUTHS_ID).is_ok() {
            doc.delete(MOUTHS_ID)?;
        }
        doc.add_fragment("root", xml.as_bytes()).map(|_| ())
    })
    .map_err(|e| match e {
        SessionEditError::Operation(_) => tool,
        SessionEditError::History(_) => "project history unavailable",
    })?;
    if stored(app).ok().as_deref() != Some(sets) {
        app.undo().map_err(|_| "project history unavailable")?;
        return Err(tool);
    }
    Ok(())
}

pub(crate) fn apply(app: &mut AppDocument, spec: Mouth) -> Result<McpOutcome, &'static str> {
    check_room(app, 1)?;
    if let Ok(info) = app.document().element_get(MOUTHS_ID)
        && info.tag != "desc"
    {
        return Err(
            "mouth_set: an element other than the mouth sets already uses the id vexel-mouths",
        );
    }
    let character = app
        .document()
        .element_get(&spec.character)
        .map_err(|_| "mouth_set: character must be the id of a <g> group in the document")?;
    if character.tag != "g" {
        return Err("mouth_set: character must be a <g> group (use element_group)");
    }
    for (_, group) in &spec.shapes {
        let info = app
            .document()
            .element_get(group)
            .map_err(|_| "mouth_set: every shape must be an element id in the document")?;
        if info.tag != "g" {
            return Err("mouth_set: every shape must name a <g> group (use element_group)");
        }
        if !inside(app, group, &spec.character) {
            return Err("mouth_set: every shape group must lie inside the character group");
        }
    }
    let mut sets = stored(app).unwrap_or_default();
    if let Some(at) = sets.iter().position(|m| m.character == spec.character) {
        sets[at] = spec.clone();
    } else if sets.len() >= MAX_CHARACTERS {
        return Err("mouth_set: at most 32 characters per document");
    } else {
        sets.push(spec.clone());
    }
    write(
        app,
        &sets,
        "mouth_set: could not store the mouth set in the document",
    )?;
    Ok(McpOutcome::Reply(format!(
        "{{\"ok\":true,\"operation\":\"mouth_set\",\"character\":\"{}\",\"shapes\":{}}}",
        spec.character,
        spec.shapes.len()
    )))
}

/// Renames group `old` to `new` in every stored set (after the group itself
/// was renamed). No-op when no stored set mentions `old`.
pub(crate) fn retarget(app: &mut AppDocument, old: &str, new: &str) -> Result<(), &'static str> {
    let Ok(mut sets) = stored(app) else {
        return Ok(());
    };
    let mut changed = false;
    for set in &mut sets {
        if set.character == old {
            set.character = new.to_owned();
            changed = true;
        }
        for (_, group) in &mut set.shapes {
            if group == old {
                *group = new.to_owned();
                changed = true;
            }
        }
    }
    if !changed {
        return Ok(());
    }
    write(
        app,
        &sets,
        "could not update the mouth sets in the document",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set() -> Mouth {
        Mouth {
            character: "hero".into(),
            shapes: vec![(0, "m_rest".into()), (1, "m_a".into()), (4, "m_o".into())],
        }
    }

    #[test]
    fn json_round_trips() {
        let sets = vec![set()];
        let text = to_json(&sets);
        assert_eq!(from_json(text.as_bytes()), Some(sets));
    }

    #[test]
    fn malformed_stored_sets_are_rejected() {
        for bad in [
            "",
            "{}",
            "{\"v\":2,\"sets\":[]}",
            "{\"v\":1,\"sets\":[{\"c\":\"h\",\"s\":{\"A\":\"x\"}}]}",
            "{\"v\":1,\"sets\":[{\"c\":\"h\",\"s\":{\"rest\":\"x\",\"Q\":\"y\"}}]}",
            "{\"v\":1,\"sets\":[{\"c\":\"h\",\"s\":{\"rest\":\"x\",\"rest\":\"y\"}}]}",
            "{\"v\":1,\"sets\":[{\"c\":\"h h\",\"s\":{\"rest\":\"x\"}}]}",
            "{\"v\":1,\"sets\":[{\"c\":\"h\",\"s\":{\"rest\":\"x y\"}}]}",
            "{\"v\":1,\"sets\":[{\"c\":\"h\",\"s\":{\"rest\":3}}]}",
        ] {
            assert_eq!(from_json(bad.as_bytes()), None, "{bad}");
        }
    }
}
