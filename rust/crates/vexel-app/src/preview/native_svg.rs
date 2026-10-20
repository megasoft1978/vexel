//! Static, specific reasons for a rejected drawing-asset SVG. The validator
//! in `animation::native_drawing` reports only coarse categories, so this
//! re-scans the rejected bytes (never echoed) to name the first problem.

use std::collections::BTreeSet;
use vexel_xml::{TokenKind, tokenize_large};

const ROOT: &str = "native_timeline_add_drawing_asset svg root must be <svg> with width, height and viewBox attributes equal to the document size (width=W height=H viewBox=\"0 0 W H\", W and H from document_info)";

/// First problem found in `source`, as a static message.
pub(super) fn reason(source: &[u8]) -> &'static str {
    let (tokens, result) = tokenize_large(source);
    if result.diagnostic.category != vexel_xml::XmlError::None || tokens.is_empty() {
        return "native_timeline_add_drawing_asset svg is not well-formed XML; send one complete <svg> document";
    }
    let mut ids = BTreeSet::<Vec<u8>>::new();
    let mut refs = Vec::<Vec<u8>>::new();
    let (mut width, mut height, mut view_box, mut root) = (false, false, false, false);
    for token in &tokens {
        let local = token.local_name.slice(source).unwrap_or_default();
        let name = token.name.slice(source).unwrap_or_default();
        let value = token.value.slice(source).unwrap_or_default();
        match token.kind {
            TokenKind::StartElement | TokenKind::EmptyElement => {
                if token.depth == 0 {
                    if root || local != b"svg" {
                        return ROOT;
                    }
                    root = true;
                }
                if local.eq_ignore_ascii_case(b"script") {
                    return "native_timeline_add_drawing_asset svg must not contain <script>";
                }
            }
            TokenKind::Attribute => {
                if name.eq_ignore_ascii_case(b"xml:base")
                    || (name.len() > 2 && name[..2].eq_ignore_ascii_case(b"on"))
                {
                    return "native_timeline_add_drawing_asset svg must not use xml:base or on* event attributes";
                }
                if name.eq_ignore_ascii_case(b"id")
                    && (value.is_empty() || !ids.insert(value.to_vec()))
                {
                    return "native_timeline_add_drawing_asset svg ids must be non-empty and unique";
                }
                if name.eq_ignore_ascii_case(b"href")
                    || name.ends_with(b":href")
                    || name.eq_ignore_ascii_case(b"src")
                {
                    match value.strip_prefix(b"#") {
                        Some(id) if !id.is_empty() => refs.push(id.to_vec()),
                        _ => return EXTERNAL,
                    }
                }
                if value.windows(4).any(|w| w.eq_ignore_ascii_case(b"url("))
                    && !value.windows(5).any(|w| w.eq_ignore_ascii_case(b"url(#"))
                {
                    return EXTERNAL;
                }
                if token.depth == 0 {
                    match name {
                        b"width" => width = true,
                        b"height" => height = true,
                        b"viewBox" => view_box = true,
                        _ => {}
                    }
                }
            }
            TokenKind::Text | TokenKind::Cdata => {
                if value.contains(&b'\\')
                    || value.windows(7).any(|w| w.eq_ignore_ascii_case(b"@import"))
                {
                    return "native_timeline_add_drawing_asset svg text must not contain @import or backslashes";
                }
            }
            TokenKind::Doctype => {
                return "native_timeline_add_drawing_asset svg must not contain a DOCTYPE";
            }
            _ => {}
        }
    }
    if !root || !width || !height || !view_box {
        return ROOT;
    }
    if refs.iter().any(|id| !ids.contains(id)) {
        return "native_timeline_add_drawing_asset svg references an #id that is not defined in the same svg";
    }
    ROOT
}

const EXTERNAL: &str = "native_timeline_add_drawing_asset svg must not reference external resources; href and url() may only point to #ids inside the same svg";

pub(super) const GEOMETRY: &str = "native_timeline_add_drawing_asset svg root width, height and viewBox must equal the document size (width=W height=H viewBox=\"0 0 W H\", W and H from document_info)";
