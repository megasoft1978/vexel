#![forbid(unsafe_code)]
//! Bounded XML 1.0 byte-preserving tokenizer and resumable tag reader.

use vexel_base::Span;

pub const MAX_READER_SOURCE_BYTES: usize = 1024 * 1024 * 1024;
pub const MAX_READER_STEP_BYTES: usize = 64 * 1024;
pub const MAX_READER_NAME_BYTES: usize = 4096;

mod tokenizer;
pub use tokenizer::*;
mod stream;
pub use stream::XmlTokenStream;
mod utf8_scan;
mod entity;
pub use entity::{EntityTable, MAX_ENTITIES, MAX_ENTITY_DEPTH, MAX_ENTITY_EXPANDED_BYTES, MAX_ENTITY_VALUE_BYTES};

/// XML 1.0 Fifth Edition `Char` production.
pub const fn is_xml_char(cp: u32) -> bool {
    cp == 0x9
        || cp == 0xa
        || cp == 0xd
        || (cp >= 0x20 && cp <= 0xd7ff)
        || (cp >= 0xe000 && cp <= 0xfffd)
        || (cp >= 0x10000 && cp <= 0x10ffff)
}

/// Decodes one strict UTF-8 scalar at `position` and returns it with the next
/// byte offset. On failure, callers retain their original position.
pub fn utf8_next(bytes: &[u8], position: usize) -> Option<(u32, usize)> {
    let first = *bytes.get(position)?;
    let (count, mut value) = match first {
        0x00..=0x7f => (1usize, u32::from(first)),
        0xc2..=0xdf => (2, u32::from(first & 0x1f)),
        0xe0..=0xef => (3, u32::from(first & 0x0f)),
        0xf0..=0xf4 => (4, u32::from(first & 0x07)),
        _ => return None,
    };
    let end = position.checked_add(count)?;
    let tail = bytes.get(position + 1..end)?;
    for &byte in tail {
        if byte & 0xc0 != 0x80 {
            return None;
        }
        value = (value << 6) | u32::from(byte & 0x3f);
    }
    let second = *tail.first().unwrap_or(&0);
    if (count == 3 && ((first == 0xe0 && second < 0xa0) || (first == 0xed && second >= 0xa0)))
        || (count == 4 && ((first == 0xf0 && second < 0x90) || (first == 0xf4 && second >= 0x90)))
        || value > 0x10ffff
        || (0xd800..=0xdfff).contains(&value)
    {
        return None;
    }
    Some((value, end))
}

/// XML 1.0 Fifth Edition `NameStartChar` production.
pub const fn is_name_start(cp: u32) -> bool {
    cp == 0x3a
        || cp == 0x5f
        || (cp >= 0x41 && cp <= 0x5a)
        || (cp >= 0x61 && cp <= 0x7a)
        || (cp >= 0xc0 && cp <= 0xd6)
        || (cp >= 0xd8 && cp <= 0xf6)
        || (cp >= 0xf8 && cp <= 0x2ff)
        || (cp >= 0x370 && cp <= 0x37d)
        || (cp >= 0x37f && cp <= 0x1fff)
        || (cp >= 0x200c && cp <= 0x200d)
        || (cp >= 0x2070 && cp <= 0x218f)
        || (cp >= 0x2c00 && cp <= 0x2fef)
        || (cp >= 0x3001 && cp <= 0xd7ff)
        || (cp >= 0xf900 && cp <= 0xfdcf)
        || (cp >= 0xfdf0 && cp <= 0xfffd)
        || (cp >= 0x10000 && cp <= 0xeffff)
}

/// XML 1.0 Fifth Edition `NameChar` production.
pub const fn is_name_char(cp: u32) -> bool {
    is_name_start(cp)
        || cp == 0x2d
        || cp == 0x2e
        || (cp >= 0x30 && cp <= 0x39)
        || cp == 0xb7
        || (cp >= 0x300 && cp <= 0x36f)
        || (cp >= 0x203f && cp <= 0x2040)
}

/// XML whitespace is exactly space, tab, line feed, and carriage return.
pub const fn is_xml_whitespace(byte: u8) -> bool {
    matches!(byte, 0x20 | 0x09 | 0x0a | 0x0d)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Diagnostic {
    pub category: XmlError,
    pub offset: usize,
}

impl Diagnostic {
    const fn clear() -> Self {
        Self { category: XmlError::None, offset: 0 }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReaderStatus {
    Ok,
    Complete,
    Invalid,
    Limit,
    Malformed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TagKind {
    StartElement,
    EndElement,
    EmptyElement,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReaderTag {
    pub kind: TagKind,
    pub source: Span,
    pub name: Span,
    pub attribute_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReaderProgress {
    pub bytes_read: usize,
    pub source_bytes: usize,
    pub diagnostic: Diagnostic,
    pub complete: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode { Text, Probe, Tag, Comment, Cdata, Pi, Doctype }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Probe { None, Comment, Cdata, Doctype }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DoctypePhase { Start, Main, Quote, Subset, SubsetQuote, Lt, Bang, BangDash, Comment, Pi }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DoctypeResult { More, Done, Bad }

#[derive(Debug, Clone, Copy)]
struct DoctypeState {
    phase: DoctypePhase,
    quote: u8,
    run: u8,
    subset_closed: bool,
}

impl Default for DoctypeState {
    fn default() -> Self {
        Self { phase: DoctypePhase::Start, quote: 0, run: 0, subset_closed: false }
    }
}

impl DoctypeState {
    fn step(&mut self, byte: u8) -> DoctypeResult {
        loop {
            match self.phase {
                DoctypePhase::Start => {
                    if !is_xml_whitespace(byte) { return DoctypeResult::Bad; }
                    self.phase = DoctypePhase::Main;
                    return DoctypeResult::More;
                }
                DoctypePhase::Main => match byte {
                    b'"' | b'\'' => { self.quote = byte; self.phase = DoctypePhase::Quote; }
                    b'[' => {
                        if self.subset_closed { return DoctypeResult::Bad; }
                        self.phase = DoctypePhase::Subset;
                    }
                    b'>' => return DoctypeResult::Done,
                    b'<' => return DoctypeResult::Bad,
                    _ => {}
                },
                DoctypePhase::Quote => if byte == self.quote { self.phase = DoctypePhase::Main; },
                DoctypePhase::Subset => match byte {
                    b'"' | b'\'' => { self.quote = byte; self.phase = DoctypePhase::SubsetQuote; }
                    b'<' => self.phase = DoctypePhase::Lt,
                    b']' => { self.subset_closed = true; self.phase = DoctypePhase::Main; }
                    _ => {}
                },
                DoctypePhase::SubsetQuote => if byte == self.quote { self.phase = DoctypePhase::Subset; },
                DoctypePhase::Lt => match byte {
                    b'!' => { self.phase = DoctypePhase::Bang; return DoctypeResult::More; }
                    b'?' => { self.run = 0; self.phase = DoctypePhase::Pi; return DoctypeResult::More; }
                    _ => { self.phase = DoctypePhase::Subset; continue; }
                },
                DoctypePhase::Bang => if byte == b'-' { self.phase = DoctypePhase::BangDash; return DoctypeResult::More; } else { self.phase = DoctypePhase::Subset; continue; },
                DoctypePhase::BangDash => if byte == b'-' { self.run = 0; self.phase = DoctypePhase::Comment; return DoctypeResult::More; } else { self.phase = DoctypePhase::Subset; continue; },
                DoctypePhase::Comment => {
                    if byte == b'-' { self.run = self.run.saturating_add(1).min(2); }
                    else if byte == b'>' && self.run >= 2 { self.phase = DoctypePhase::Subset; }
                    else { self.run = 0; }
                }
                DoctypePhase::Pi => {
                    if byte == b'>' && self.run == 1 { self.phase = DoctypePhase::Subset; }
                    else { self.run = u8::from(byte == b'?'); }
                }
            }
            return DoctypeResult::More;
        }
    }
}

/// Borrowing scanner. Source bytes remain immutable and are never normalized.
pub struct XmlReader<'a> {
    source: &'a [u8],
    bytes_read: usize,
    token_start: usize,
    name_offset: usize,
    name_length: usize,
    attribute_count: usize,
    probe_position: usize,
    mode: Mode,
    probe: Probe,
    closing: bool,
    name_complete: bool,
    in_quote: bool,
    pending_slash: bool,
    self_closing: bool,
    quote: u8,
    complete: bool,
    element_seen: bool,
    doctype_seen: bool,
    doctype: DoctypeState,
    diagnostic: Diagnostic,
}

impl<'a> XmlReader<'a> {
    /// Starts a reader borrowing `source` until this reader is dropped.
    pub fn new(source: &'a [u8]) -> Result<Self, ReaderStatus> {
        if source.len() > MAX_READER_SOURCE_BYTES { return Err(ReaderStatus::Limit); }
        Ok(Self {
            source, bytes_read: 0, token_start: 0, name_offset: 0, name_length: 0,
            attribute_count: 0, probe_position: 0, mode: Mode::Text, probe: Probe::None,
            closing: false, name_complete: false, in_quote: false, pending_slash: false,
            self_closing: false, quote: 0, complete: false, element_seen: false,
            doctype_seen: false, doctype: DoctypeState::default(), diagnostic: Diagnostic::clear(),
        })
    }

    /// Advances by at most `byte_budget` bytes, capped at 64 KiB, and emits
    /// complete lexical tags. Returning false from the callback is an error.
    pub fn step<F>(&mut self, byte_budget: usize, mut on_tag: F) -> ReaderStatus
    where F: FnMut(ReaderTag) -> bool {
        if byte_budget == 0 { return ReaderStatus::Invalid; }
        if self.diagnostic.category != XmlError::None { return ReaderStatus::Malformed; }
        if self.complete { return ReaderStatus::Complete; }
        let budget = byte_budget.min(MAX_READER_STEP_BYTES);
        let end = self.bytes_read.saturating_add(budget).min(self.source.len());
        while self.bytes_read < end && self.diagnostic.category == XmlError::None {
            let at = self.bytes_read;
            let byte = self.source[at];
            match self.mode {
                Mode::Text => if byte == b'<' {
                    self.mode = Mode::Probe;
                    self.token_start = at;
                    self.probe = Probe::None;
                    self.probe_position = 0;
                },
                Mode::Probe => self.scan_probe(byte, at),
                Mode::Tag => {
                    if !self.name_complete {
                        if is_reader_name_byte(byte) {
                            if self.name_length == MAX_READER_NAME_BYTES { self.fail(XmlError::InvalidName, self.name_offset); }
                            else { self.name_length += 1; }
                            self.bytes_read += 1;
                            continue;
                        }
                        if self.name_length == 0 { self.fail(XmlError::InvalidName, self.name_offset); }
                        else { self.name_complete = true; }
                    }
                    if self.in_quote {
                        if byte == self.quote { self.in_quote = false; }
                    } else if byte == b'\'' || byte == b'"' {
                        self.in_quote = true;
                        self.quote = byte;
                        self.attribute_count = self.attribute_count.saturating_add(1);
                    } else if byte == b'/' { self.pending_slash = true; }
                    else if byte == b'>' {
                        self.self_closing = self.pending_slash;
                        if !self.emit_tag(at + 1, &mut on_tag) { break; }
                    } else { self.pending_slash = false; }
                }
                Mode::Comment if byte == b'>' && at >= 2 && &self.source[at - 2..at] == b"--" => self.mode = Mode::Text,
                Mode::Cdata if byte == b'>' && at >= 2 && &self.source[at - 2..at] == b"]]" => self.mode = Mode::Text,
                Mode::Doctype => match self.doctype.step(byte) {
                    DoctypeResult::Bad => self.fail(XmlError::MalformedSyntax, at),
                    DoctypeResult::Done => self.mode = Mode::Text,
                    DoctypeResult::More => {}
                },
                Mode::Pi if byte == b'>' && at > 0 && self.source[at - 1] == b'?' => self.mode = Mode::Text,
                _ => {}
            }
            self.bytes_read += 1;
        }
        if self.diagnostic.category != XmlError::None { return ReaderStatus::Malformed; }
        if self.bytes_read == self.source.len() {
            if self.mode != Mode::Text { self.fail(XmlError::IncompleteDocument, self.source.len()); }
            else { self.complete = true; }
        }
        if self.diagnostic.category == XmlError::None {
            if self.complete { ReaderStatus::Complete } else { ReaderStatus::Ok }
        } else { ReaderStatus::Malformed }
    }

    pub fn progress(&self) -> ReaderProgress {
        ReaderProgress { bytes_read: self.bytes_read, source_bytes: self.source.len(), diagnostic: self.diagnostic, complete: self.complete }
    }

    fn scan_probe(&mut self, byte: u8, at: usize) {
        if self.probe_position == 0 {
            if byte == b'?' { self.mode = Mode::Pi; }
            else if byte == b'/' {
                self.begin_tag(true, at + 1);
            } else if byte == b'!' { self.probe_position = 1; }
            else if is_reader_name_byte(byte) {
                self.begin_tag(false, at);
                self.name_length = 1;
            } else { self.fail(XmlError::InvalidName, at); }
        } else if self.probe == Probe::None {
            if byte == b'-' { self.probe = Probe::Comment; self.probe_position = 1; }
            else if byte == b'[' { self.probe = Probe::Cdata; self.probe_position = 1; }
            else if byte == b'D' && !self.doctype_seen && !self.element_seen {
                self.probe = Probe::Doctype;
                self.probe_position = 1;
            } else { self.fail(XmlError::UnsupportedDtd, self.token_start); }
        } else if self.probe == Probe::Comment {
            if byte != b'-' { self.fail(XmlError::MalformedSyntax, self.token_start); }
            else { self.mode = Mode::Comment; }
        } else if self.probe == Probe::Doctype {
            const TAIL: &[u8] = b"OCTYPE";
            let index = self.probe_position - 1;
            if TAIL.get(index).copied() != Some(byte) { self.fail(XmlError::UnsupportedDtd, self.token_start); }
            else if self.probe_position == TAIL.len() {
                self.mode = Mode::Doctype;
                self.doctype_seen = true;
                self.doctype = DoctypeState::default();
            } else { self.probe_position += 1; }
        } else {
            const TAIL: &[u8] = b"CDATA[";
            let index = self.probe_position - 1;
            if index >= TAIL.len() || TAIL[index] != byte { self.fail(XmlError::MalformedSyntax, self.token_start); }
            else if self.probe_position == TAIL.len() { self.mode = Mode::Cdata; }
            else { self.probe_position += 1; }
        }
    }

    fn begin_tag(&mut self, closing: bool, name_offset: usize) {
        self.mode = Mode::Tag;
        self.closing = closing;
        self.name_offset = name_offset;
        self.name_length = 0;
        self.attribute_count = 0;
        self.name_complete = false;
        self.pending_slash = false;
    }

    fn emit_tag<F>(&mut self, end: usize, callback: &mut F) -> bool
    where F: FnMut(ReaderTag) -> bool {
        if self.name_length == 0 || self.name_length > MAX_READER_NAME_BYTES {
            self.fail(XmlError::InvalidName, self.name_offset);
            return false;
        }
        if !self.closing { self.element_seen = true; }
        let kind = if self.closing { TagKind::EndElement } else if self.self_closing { TagKind::EmptyElement } else { TagKind::StartElement };
        let tag = ReaderTag {
            kind,
            source: Span { start: self.token_start, end },
            name: Span { start: self.name_offset, end: self.name_offset + self.name_length },
            attribute_count: self.attribute_count,
        };
        if !callback(tag) {
            self.fail(XmlError::OutputExhausted, self.token_start);
            return false;
        }
        self.mode = Mode::Text;
        true
    }

    fn fail(&mut self, category: XmlError, offset: usize) {
        if self.diagnostic.category == XmlError::None {
            self.diagnostic = Diagnostic { category, offset: offset.min(self.source.len()) };
        }
    }
}

fn is_reader_name_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte >= 0x80 || matches!(byte, b'_' | b'-' | b'.' | b':')
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod entity_tests;
