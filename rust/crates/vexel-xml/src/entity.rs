//! Internal general entities declared in a DOCTYPE internal subset.
//!
//! Only `<!ENTITY name "literal">` is supported. External (SYSTEM/PUBLIC) and
//! parameter entities are refused. Values stay as spans of the source, so the
//! table owns no heap memory and its footprint is a fixed `MAX_ENTITIES` slots.
//! Expansion work is bounded by `MAX_ENTITY_EXPANDED_BYTES`, counting every
//! produced byte, scanned literal byte and entity visit, so a zero-length
//! billion-laughs chain is bounded too.

use crate::{is_name_char, is_name_start, is_xml_char, is_xml_whitespace, utf8_next, XmlError, XmlSpan};

pub const MAX_ENTITIES: usize = 64;
pub const MAX_ENTITY_DEPTH: usize = 8;
pub const MAX_ENTITY_EXPANDED_BYTES: usize = 1024 * 1024;
pub const MAX_ENTITY_VALUE_BYTES: usize = 64 * 1024;

type Fail = (XmlError, usize);

#[derive(Clone, Copy)]
struct Entry { name: XmlSpan, value: XmlSpan, cost: usize, state: u8 }
const EMPTY_ENTRY: Entry = Entry { name: XmlSpan { offset: 0, length: 0 }, value: XmlSpan { offset: 0, length: 0 }, cost: 0, state: 0 };
const UNRESOLVED: u8 = 0;
const VISITING: u8 = 1;
const RESOLVED: u8 = 2;

#[derive(Clone)]
pub struct EntityTable { entries: [Entry; MAX_ENTITIES], count: usize, total: usize }

static NO_ENTITIES: EntityTable = EntityTable::empty();

impl EntityTable {
    pub const fn empty() -> Self { Self { entries: [EMPTY_ENTRY; MAX_ENTITIES], count: 0, total: 0 } }
    /// Shared table with no declarations (only predefined entities resolve).
    pub fn none() -> &'static Self { &NO_ENTITIES }
    pub fn len(&self) -> usize { self.count }
    pub fn is_empty(&self) -> bool { self.count == 0 }

    /// Finds the document DOCTYPE (before the root element) and parses its
    /// internal entities. A document without a DOCTYPE yields an empty table.
    pub fn from_document(source: &[u8]) -> Result<Self, (XmlError, usize)> {
        let mut p = if source.starts_with(b"\xef\xbb\xbf") { 3 } else { 0 };
        loop {
            while source.get(p).is_some_and(|b| is_xml_whitespace(*b)) { p += 1; }
            if source[p.min(source.len())..].starts_with(b"<?") { p = find(source, p + 2, b"?>").map_or(source.len(), |e| e + 2); }
            else if source[p.min(source.len())..].starts_with(b"<!--") { p = find(source, p + 4, b"-->").map_or(source.len(), |e| e + 3); }
            else if source[p.min(source.len())..].starts_with(b"<!DOCTYPE") { return Self::parse(source, p); }
            else { return Ok(Self::empty()); }
        }
    }

    /// Parses the DOCTYPE starting at `start` (`<!DOCTYPE`). Offsets in
    /// errors are absolute source offsets.
    pub fn parse(source: &[u8], start: usize) -> Result<Self, Fail> {
        let mut t = Self::empty();
        let mut p = start + 9;
        loop {
            match source.get(p) {
                None => return Err((XmlError::MalformedSyntax, start)),
                Some(&q @ (b'"' | b'\'')) => p = skip_quoted(source, p, q).ok_or((XmlError::MalformedSyntax, p))?,
                Some(b'[') => { p += 1; break; }
                Some(b'>') => return Ok(t),
                Some(_) => p += 1,
            }
        }
        loop {
            match source.get(p) {
                None => return Err((XmlError::MalformedSyntax, start)),
                Some(&q @ (b'"' | b'\'')) => p = skip_quoted(source, p, q).ok_or((XmlError::MalformedSyntax, p))?,
                Some(b']') => break,
                Some(b'%') => return Err((XmlError::UnsupportedDtd, p)),
                Some(b'<') if source[p..].starts_with(b"<!--") => p = find(source, p + 4, b"-->").ok_or((XmlError::MalformedSyntax, p))? + 3,
                Some(b'<') if source[p..].starts_with(b"<?") => p = find(source, p + 2, b"?>").ok_or((XmlError::MalformedSyntax, p))? + 2,
                Some(b'<') if source[p..].starts_with(b"<!ENTITY") && source.get(p + 8).is_some_and(|b| is_xml_whitespace(*b)) => p = t.declaration(source, p)?,
                Some(_) => p += 1,
            }
        }
        Ok(t)
    }

    fn declaration(&mut self, source: &[u8], decl: usize) -> Result<usize, Fail> {
        let mut p = decl + 8;
        skip_ws(source, &mut p);
        if source.get(p) == Some(&b'%') { return Err((XmlError::UnsupportedDtd, p)); }
        let name_at = p;
        let name = scan_name(source, &mut p).ok_or((XmlError::MalformedSyntax, name_at))?;
        let gap = p; skip_ws(source, &mut p);
        if p == gap { return Err((XmlError::MalformedSyntax, p)); }
        if source[p..].starts_with(b"SYSTEM") || source[p..].starts_with(b"PUBLIC") { return Err((XmlError::ExternalEntity, p)); }
        let Some(&quote @ (b'"' | b'\'')) = source.get(p) else { return Err((XmlError::MalformedSyntax, p)) };
        let end = skip_quoted(source, p, quote).ok_or((XmlError::MalformedSyntax, decl))? - 1;
        let value = XmlSpan { offset: p + 1, length: end - p - 1 };
        if value.length > MAX_ENTITY_VALUE_BYTES { return Err((XmlError::EntityValueLimit, value.offset)); }
        validate_value(source, value)?;
        p = end + 1; skip_ws(source, &mut p);
        if source.get(p) != Some(&b'>') { return Err((XmlError::MalformedSyntax, p)); }
        if self.find(source, name.slice(source).unwrap_or(&[])).is_none() {
            if self.count == MAX_ENTITIES { return Err((XmlError::EntityCountLimit, decl)); }
            self.entries[self.count] = Entry { name, value, cost: 0, state: UNRESOLVED };
            self.count += 1;
        }
        Ok(p + 1)
    }

    fn find(&self, source: &[u8], name: &[u8]) -> Option<usize> {
        self.entries[..self.count].iter().position(|e| e.name.slice(source) == Some(name))
    }

    pub fn contains(&self, source: &[u8], name: &[u8]) -> bool { self.find(source, name).is_some() }

    /// Tokenization-time accounting for one document reference to `name`:
    /// rejects cycles, nesting deeper than `MAX_ENTITY_DEPTH`, and a document
    /// total above `MAX_ENTITY_EXPANDED_BYTES`. `Ok(false)` means not declared.
    pub fn charge_reference(&mut self, source: &[u8], name: &[u8]) -> Result<bool, XmlError> {
        let Some(idx) = self.find(source, name) else { return Ok(false) };
        let cost = self.resolve(source, idx, 1)?;
        self.total = self.total.saturating_add(cost);
        if self.total > MAX_ENTITY_EXPANDED_BYTES { return Err(XmlError::EntityExpansionLimit); }
        Ok(true)
    }

    fn resolve(&mut self, source: &[u8], idx: usize, depth: usize) -> Result<usize, XmlError> {
        match self.entries[idx].state { RESOLVED => return Ok(self.entries[idx].cost), VISITING => return Err(XmlError::EntityCycle), _ => {} }
        if depth > MAX_ENTITY_DEPTH { return Err(XmlError::EntityDepthLimit); }
        self.entries[idx].state = VISITING;
        let value = self.entries[idx].value.slice(source).unwrap_or(&[]);
        let (mut cost, mut i) = (0usize, 0usize);
        while i < value.len() {
            let (add, next) = if value[i] == b'&' {
                let semi = value[i..].iter().position(|b| *b == b';').map_or(value.len(), |n| i + n);
                let body = &value[i + 1..semi.min(value.len())];
                let inner = if body.first() == Some(&b'#') { char_ref(body).map_or(1, |c| c.len_utf8()) }
                    else if predefined(body).is_some() { 1 }
                    else { let j = self.find(source, body).ok_or(XmlError::MalformedSyntax)?; self.resolve(source, j, depth + 1)? };
                (inner.saturating_add(1), semi + 1)
            } else { let n = utf8_next(value, i).map_or(i + 1, |(_, n)| n); (n - i, n) };
            cost = cost.saturating_add(add);
            if cost > MAX_ENTITY_EXPANDED_BYTES { return Err(XmlError::EntityExpansionLimit); }
            i = next;
        }
        self.entries[idx].cost = cost; self.entries[idx].state = RESOLVED;
        Ok(cost)
    }

    /// Appends the expansion of `name` to `out[*at..]`. Independently bounded
    /// (depth, work budget, output room), so it is safe on unvalidated tables.
    pub fn expand(&self, source: &[u8], name: &[u8], out: &mut [u8], at: &mut usize) -> Result<(), ()> {
        let idx = self.find(source, name).ok_or(())?;
        let mut budget = MAX_ENTITY_EXPANDED_BYTES;
        self.expand_entry(source, idx, 1, &mut budget, out, at)
    }

    fn expand_entry(&self, source: &[u8], idx: usize, depth: usize, budget: &mut usize, out: &mut [u8], at: &mut usize) -> Result<(), ()> {
        if depth > MAX_ENTITY_DEPTH { return Err(()); }
        let value = self.entries[idx].value.slice(source).ok_or(())?;
        let mut i = 0;
        while i < value.len() {
            *budget = budget.checked_sub(1).ok_or(())?;
            if value[i] == b'&' {
                let semi = value[i..].iter().position(|b| *b == b';').map(|n| i + n).ok_or(())?;
                let body = &value[i + 1..semi];
                if body.first() == Some(&b'#') { push_char(char_ref(body).ok_or(())?, out, at)?; }
                else if let Some(c) = predefined(body) { push_char(c, out, at)?; }
                else { let j = self.find(source, body).ok_or(())?; self.expand_entry(source, j, depth + 1, budget, out, at)?; }
                i = semi + 1;
            } else {
                let (_, n) = utf8_next(value, i).ok_or(())?;
                *budget = budget.checked_sub(n - i - 1).ok_or(())?;
                out.get_mut(*at..*at + (n - i)).ok_or(())?.copy_from_slice(&value[i..n]);
                *at += n - i; i = n;
            }
        }
        Ok(())
    }
}

fn push_char(c: char, out: &mut [u8], at: &mut usize) -> Result<(), ()> {
    let mut tmp = [0u8; 4]; let enc = c.encode_utf8(&mut tmp).as_bytes();
    out.get_mut(*at..*at + enc.len()).ok_or(())?.copy_from_slice(enc); *at += enc.len(); Ok(())
}

pub(crate) fn predefined(body: &[u8]) -> Option<char> {
    match body { b"amp" => Some('&'), b"lt" => Some('<'), b"gt" => Some('>'), b"quot" => Some('"'), b"apos" => Some('\''), _ => None }
}

/// `#123` / `#x1F` body (without `&` and `;`) to a valid XML character.
fn char_ref(body: &[u8]) -> Option<char> {
    let (digits, radix) = if body.get(1) == Some(&b'x') { (body.get(2..)?, 16) } else { (body.get(1..)?, 10) };
    if digits.is_empty() { return None; }
    let mut n = 0u32;
    for &c in digits {
        let d = (c as char).to_digit(radix)?;
        n = n.checked_mul(radix)?.checked_add(d)?;
        if n > 0x10ffff { return None; }
    }
    if !is_xml_char(n) { return None; }
    char::from_u32(n)
}

fn validate_value(source: &[u8], value: XmlSpan) -> Result<(), Fail> {
    let end = value.offset + value.length;
    let mut p = value.offset;
    while p < end {
        match source[p] {
            b'%' | b'<' => return Err((XmlError::UnsupportedDtd, p)),
            b'&' => {
                let semi = source[p..end].iter().position(|b| *b == b';').map(|n| p + n).ok_or((XmlError::MalformedSyntax, p))?;
                let body = &source[p + 1..semi];
                if body.first() == Some(&b'#') {
                    match char_ref(body) { Some('&' | '<') => return Err((XmlError::UnsupportedDtd, p)), Some(_) => {}, None => return Err((XmlError::MalformedSyntax, p)) }
                } else {
                    let mut q = p + 1;
                    if scan_name(source, &mut q).is_none() || q != semi { return Err((XmlError::MalformedSyntax, p)); }
                }
                p = semi + 1;
            }
            _ => p += 1,
        }
    }
    Ok(())
}

fn skip_ws(source: &[u8], p: &mut usize) { while source.get(*p).is_some_and(|b| is_xml_whitespace(*b)) { *p += 1; } }
fn find(source: &[u8], from: usize, needle: &[u8]) -> Option<usize> {
    source.get(from..)?.windows(needle.len()).position(|w| w == needle).map(|n| from + n)
}
/// Index just past the closing quote of the literal opening at `open`.
fn skip_quoted(source: &[u8], open: usize, quote: u8) -> Option<usize> {
    source.get(open + 1..)?.iter().position(|b| *b == quote).map(|n| open + 1 + n + 1)
}
fn scan_name(source: &[u8], p: &mut usize) -> Option<XmlSpan> {
    let start = *p;
    let (cp, mut q) = utf8_next(source, start)?;
    if !is_name_start(cp) { return None; }
    while let Some((cp, next)) = utf8_next(source, q) { if !is_name_char(cp) { break; } q = next; }
    *p = q;
    Some(XmlSpan { offset: start, length: q - start })
}
