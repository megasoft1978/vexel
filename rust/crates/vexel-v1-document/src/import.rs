//! Preparing a saved document's scene for import as one group: every id is
//! given a prefix, together with the references that name it (`href="#id"`,
//! `url(#id)`), so the same file can be imported many times without clashes.
//! The result is one fragment that `V1Document::add_import` inserts through
//! the same allow-list checks as `element_add_fragment`.

use crate::{V1Document, V1DocumentError, validate_stable_id};
use std::collections::HashSet;
use std::sync::Arc;
use vexel_document_edit::{
    EditError, EditLimits, Editor, document_view_first_child, document_view_next_sibling,
};

/// Largest source scene an import reads.
pub const MAX_IMPORT_BYTES: usize = 4 * 1024 * 1024;
/// Longest id prefix.
const MAX_PREFIX_BYTES: usize = 64;
const RIG_TAG: &[u8] = b"<desc id=\"vexel-rig\"";

/// A scene ready to insert: the fragment, the ids it declares (original and
/// prefixed) and the remapped rig JSON when the source stored one.
#[derive(Clone, Debug, PartialEq)]
pub struct PreparedImport {
    pub xml: Vec<u8>,
    /// Id of the wrapping group.
    pub group_id: String,
    /// Ids declared by the source scene, before the prefix.
    pub source_ids: Vec<String>,
    /// The source's rig JSON with every id that names an element prefixed.
    pub rig: Option<String>,
}

/// Whether `prefix` is usable at the start of an element id.
pub fn valid_prefix(prefix: &str) -> bool {
    let bytes = prefix.as_bytes();
    !bytes.is_empty()
        && bytes.len() <= MAX_PREFIX_BYTES
        && (bytes[0].is_ascii_alphabetic() || bytes[0] == b'_')
        && bytes
            .iter()
            .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(b))
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

fn skip_space(src: &[u8], mut at: usize) -> usize {
    while src.get(at).is_some_and(u8::is_ascii_whitespace) {
        at += 1;
    }
    at
}

/// Byte range between the root `<svg ...>` start tag and its `</svg>`.
fn root_inner(src: &[u8]) -> Option<(usize, usize)> {
    let mut at = if src.starts_with(b"\xef\xbb\xbf") {
        3
    } else {
        0
    };
    loop {
        at = skip_space(src, at);
        let rest = &src[at..];
        if rest.starts_with(b"<!--") {
            at += find(rest, b"-->")? + 3;
        } else if rest.starts_with(b"<?") {
            at += find(rest, b"?>")? + 2;
        } else if rest.starts_with(b"<!") {
            let end = rest.iter().position(|b| *b == b'>')?;
            if rest[..end].contains(&b'[') {
                return None;
            }
            at += end + 1;
        } else {
            break;
        }
    }
    let rest = &src[at..];
    if !rest.starts_with(b"<svg")
        || !rest
            .get(4)
            .is_some_and(|b| b.is_ascii_whitespace() || *b == b'>' || *b == b'/')
    {
        return None;
    }
    let mut quote = None;
    let mut cursor = at + 4;
    while let Some(byte) = src.get(cursor) {
        match (quote, *byte) {
            (Some(q), b) if b == q => quote = None,
            (Some(_), _) => {}
            (None, b'"' | b'\'') => quote = Some(*byte),
            (None, b'>') => break,
            _ => {}
        }
        cursor += 1;
    }
    if src.get(cursor) != Some(&b'>') || src[cursor - 1] == b'/' {
        return None;
    }
    let start = cursor + 1;
    let end = start + src[start..].windows(6).rposition(|w| w == b"</svg>")?;
    let tail = &src[end + 6..];
    tail.iter()
        .all(u8::is_ascii_whitespace)
        .then_some((start, end))
}

/// Offsets just after `#` in every `url(#id)` of an attribute value.
fn url_sites(value: &[u8], base: usize, sites: &mut Vec<usize>) {
    let mut at = 0;
    while at + 4 <= value.len() {
        if !value[at..at + 4].eq_ignore_ascii_case(b"url(") {
            at += 1;
            continue;
        }
        let mut cursor = at + 4;
        loop {
            let rest = &value[cursor..];
            if rest
                .first()
                .is_some_and(|b| b.is_ascii_whitespace() || *b == b'"' || *b == b'\'')
            {
                cursor += 1;
            } else if rest.starts_with(b"&quot;") {
                cursor += 6;
            } else if rest.starts_with(b"&apos;") {
                cursor += 6;
            } else {
                break;
            }
        }
        if value.get(cursor) == Some(&b'#') {
            sites.push(base + cursor + 1);
        }
        at = cursor.max(at + 1);
    }
}

/// `"id"` string values of `json` that name an element in `ids` get `prefix`;
/// object keys never change. Anything with an escape is left alone.
fn remap_json(json: &str, ids: &HashSet<&str>, prefix: &str) -> String {
    let bytes = json.as_bytes();
    let mut out = String::with_capacity(json.len() + 64);
    let mut at = 0;
    while at < bytes.len() {
        if bytes[at] != b'"' {
            let start = at;
            while at < bytes.len() && bytes[at] != b'"' {
                at += 1;
            }
            out.push_str(&json[start..at]);
            continue;
        }
        let start = at + 1;
        let mut end = start;
        while end < bytes.len() && bytes[end] != b'"' && bytes[end] != b'\\' {
            end += 1;
        }
        if end >= bytes.len() || bytes[end] == b'\\' {
            out.push_str(&json[at..]);
            return out;
        }
        let text = &json[start..end];
        let after = skip_space(bytes, end + 1);
        let is_key = bytes.get(after) == Some(&b':');
        out.push('"');
        if !is_key && ids.contains(text) {
            out.push_str(prefix);
        }
        out.push_str(text);
        out.push('"');
        at = end + 1;
    }
    out
}

fn text_of(editor: &Editor, span: vexel_document_edit::SourceSpan) -> Option<&str> {
    std::str::from_utf8(editor.span_bytes(span)?).ok()
}

/// `translate(tx ty) rotate(rot) scale(sx sy)` as one matrix.
fn trs_matrix(trs: [f64; 5]) -> Result<String, V1DocumentError> {
    if trs.iter().any(|v| !v.is_finite() || v.abs() > 1e9) {
        return Err(V1DocumentError::InvalidGeometry);
    }
    let [tx, ty, rot, sx, sy] = trs;
    let (sin, cos) = rot.to_radians().sin_cos();
    let m = [cos * sx, sin * sx, -sin * sy, cos * sy, tx, ty];
    Ok(m.map(|v| format!("{}", v + 0.0)).join(" "))
}

/// Parses `svg`, prefixes its ids and references and wraps its content in a
/// group `prefix` (trailing `_`, `-`, `.` removed) carrying `trs`
/// `[tx, ty, rotation degrees, sx, sy]` as its transform. Only parses and
/// rewrites text: the allow-list check runs when the fragment is inserted.
pub fn prepare_import(
    svg: &[u8],
    prefix: &str,
    trs: Option<[f64; 5]>,
) -> Result<PreparedImport, V1DocumentError> {
    if svg.len() > MAX_IMPORT_BYTES {
        return Err(V1DocumentError::Edit(EditError::Limit));
    }
    if !valid_prefix(prefix) {
        return Err(V1DocumentError::InvalidStableId);
    }
    let group_id = prefix.trim_end_matches(['_', '-', '.']);
    let group_id = if group_id.is_empty() {
        prefix
    } else {
        group_id
    }
    .to_owned();
    validate_stable_id(&group_id)?;
    if find(svg, b"<!ENTITY").is_some() {
        return Err(V1DocumentError::ExternalReference);
    }
    let (inner_start, inner_end) =
        root_inner(svg).ok_or(V1DocumentError::InvalidAnimationFragment)?;
    let parsed = Editor::create(&Arc::new(svg.to_vec()), EditLimits::default());
    if parsed.error != EditError::Ok {
        return Err(V1DocumentError::Edit(parsed.error));
    }
    let editor = parsed
        .editor
        .ok_or(V1DocumentError::Edit(EditError::InvalidOperation))?;
    let model = editor.try_model()?;
    let root = editor
        .root()
        .ok_or(V1DocumentError::Edit(EditError::InvalidOperation))?;
    let mut sites: Vec<usize> = Vec::new();
    let mut source_ids: Vec<String> = Vec::new();
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        for index in 0..editor.attribute_count(node) {
            let Some(attribute) = editor.attribute_at(node, index) else {
                continue;
            };
            let (Some(name), Some(value)) = (
                text_of(&editor, attribute.name),
                editor.span_bytes(attribute.value),
            ) else {
                return Err(V1DocumentError::DisallowedAttribute);
            };
            let offset = attribute.value.offset;
            if offset < inner_start || offset >= inner_end {
                continue;
            }
            if name == "id" {
                sites.push(offset);
                source_ids.push(String::from_utf8_lossy(value).into_owned());
            } else if name.rsplit(':').next() == Some("href") && value.first() == Some(&b'#') {
                sites.push(offset + 1);
            } else {
                url_sites(value, offset, &mut sites);
            }
        }
        let mut child = document_view_first_child(model, node);
        while let Some(item) = child {
            stack.push(item);
            child = document_view_next_sibling(model, item);
        }
    }
    sites.sort_unstable();
    sites.dedup();
    // The stored rig element is not copied as markup; its JSON is returned.
    let inner = &svg[inner_start..inner_end];
    let mut cut = None;
    let mut rig_text = None;
    if let Some(at) = find(inner, RIG_TAG) {
        let open = at
            + inner[at..]
                .iter()
                .position(|b| *b == b'>')
                .ok_or(V1DocumentError::InvalidAnimationFragment)?
            + 1;
        let close = open
            + find(&inner[open..], b"</desc>").ok_or(V1DocumentError::InvalidAnimationFragment)?;
        let text = std::str::from_utf8(&inner[open..close])
            .map_err(|_| V1DocumentError::InvalidAnimationFragment)?;
        rig_text = Some(text.to_owned());
        cut = Some((inner_start + at, inner_start + close + "</desc>".len()));
    }
    let mut xml = Vec::new();
    xml.try_reserve(inner.len() + sites.len() * prefix.len() + 256)
        .map_err(|_| V1DocumentError::Allocation)?;
    xml.extend_from_slice(format!("<g id=\"{group_id}\"").as_bytes());
    if let Some(trs) = trs.filter(|t| *t != [0.0, 0.0, 0.0, 1.0, 1.0]) {
        xml.extend_from_slice(format!(" transform=\"matrix({})\"", trs_matrix(trs)?).as_bytes());
    }
    xml.push(b'>');
    let segments = match cut {
        Some((from, to)) => vec![(inner_start, from), (to, inner_end)],
        None => vec![(inner_start, inner_end)],
    };
    for (from, to) in segments {
        let mut next = from;
        for site in sites.iter().filter(|s| **s >= from && **s < to) {
            xml.extend_from_slice(&svg[next..*site]);
            xml.extend_from_slice(prefix.as_bytes());
            next = *site;
        }
        xml.extend_from_slice(&svg[next..to]);
    }
    xml.extend_from_slice(b"</g>");
    let known: HashSet<&str> = source_ids.iter().map(String::as_str).collect();
    let rig = rig_text.map(|text| remap_json(&text, &known, prefix));
    Ok(PreparedImport {
        xml,
        group_id,
        source_ids,
        rig,
    })
}

impl V1Document {
    /// Inserts a prepared import as the last child of `parent` (`"root"` or
    /// any element id). Same allow-list, id and reference-cycle checks as
    /// `add_fragment`; one history entry. Returns the ids declared.
    pub fn add_import(
        &mut self,
        parent: &str,
        import: &PreparedImport,
    ) -> Result<Vec<String>, V1DocumentError> {
        self.add_fragment_limited(parent, &import.xml, MAX_IMPORT_BYTES + 4096)
    }
}

#[cfg(test)]
#[path = "import_tests.rs"]
mod tests;
