//! Attribute-value entity expansion for consumers that read raw source spans.
//!
//! XML 1.0 section 3.3.3: a reference to a declared internal entity inside an
//! attribute value is replaced by its replacement text, and each whitespace
//! character of that text becomes a space. The tokenizer already validates and
//! charges every reference (undefined, cyclic, too deep, over budget), so this
//! pass only runs on a document the tokenizer accepted. Predefined and
//! character references, text, and all other bytes are copied unchanged.

use crate::{tokenize_large, EntityTable, TokenKind, XmlError, MAX_ENTITY_EXPANDED_BYTES};

fn is_declared_reference(body: &[u8]) -> bool {
    !body.starts_with(b"#") && !matches!(body, b"amp" | b"lt" | b"gt" | b"quot" | b"apos")
}

fn has_declared_reference(raw: &[u8]) -> bool {
    let mut i = 0;
    while let Some(n) = raw[i..].iter().position(|b| *b == b'&') {
        let start = i + n + 1;
        let Some(m) = raw[start..].iter().position(|b| *b == b';') else { return false };
        if is_declared_reference(&raw[start..start + m]) { return true; }
        i = start + m + 1;
    }
    false
}

/// Escapes expanded replacement text so it is valid inside either quote style.
fn push_escaped(out: &mut Vec<u8>, text: &[u8]) {
    for &b in text {
        match b {
            b'&' => out.extend_from_slice(b"&amp;"),
            b'<' => out.extend_from_slice(b"&lt;"),
            b'"' => out.extend_from_slice(b"&quot;"),
            b'\'' => out.extend_from_slice(b"&apos;"),
            b'\t' | b'\n' | b'\r' => out.push(b' '),
            _ => out.push(b),
        }
    }
}

/// Returns the source with declared entity references in attribute values
/// expanded, or `None` when there is nothing to expand (no internal entities,
/// or none referenced from an attribute value). A document the tokenizer
/// refuses yields its typed error and absolute offset.
pub fn expand_attribute_entities(source: &[u8]) -> Result<Option<Vec<u8>>, (XmlError, usize)> {
    let table = EntityTable::from_document(source)?;
    if table.is_empty() { return Ok(None); }
    let (tokens, result) = tokenize_large(source);
    if result.diagnostic.category != XmlError::None { return Err((result.diagnostic.category, result.diagnostic.offset)); }
    let mut out: Vec<u8> = Vec::new();
    let mut scratch: Vec<u8> = Vec::new();
    let mut copied = 0usize;
    for token in tokens.iter().filter(|t| t.kind == TokenKind::Attribute) {
        let Some(raw) = token.value.slice(source) else { continue };
        if !has_declared_reference(raw) { continue; }
        if scratch.is_empty() { scratch.resize(MAX_ENTITY_EXPANDED_BYTES, 0); }
        out.extend_from_slice(&source[copied..token.value.offset]);
        let mut i = 0;
        while i < raw.len() {
            let Some(n) = raw[i..].iter().position(|b| *b == b'&') else { break };
            let amp = i + n;
            let semi = amp + 1 + raw[amp + 1..].iter().position(|b| *b == b';').ok_or((XmlError::MalformedSyntax, token.value.offset + amp))?;
            let body = &raw[amp + 1..semi];
            out.extend_from_slice(&raw[i..amp]);
            if is_declared_reference(body) {
                let mut at = 0;
                table.expand(source, body, &mut scratch, &mut at).map_err(|_| (XmlError::EntityExpansionLimit, token.value.offset + amp))?;
                push_escaped(&mut out, &scratch[..at]);
            } else {
                out.extend_from_slice(&raw[amp..=semi]);
            }
            i = semi + 1;
        }
        out.extend_from_slice(&raw[i..]);
        copied = token.value.offset + token.value.length;
    }
    if copied == 0 { return Ok(None); }
    out.extend_from_slice(&source[copied..]);
    Ok(Some(out))
}
