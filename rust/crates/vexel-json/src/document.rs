use std::borrow::Cow;

use vexel_base::Span;

pub const MAX_BYTES: usize = 2 * 1024 * 1024;
pub const MAX_DEPTH: usize = 32;
pub const MAX_TOKENS: usize = 65_536;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsonType {
    Object,
    Array,
    String,
    Number,
    True,
    False,
    Null,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JsonToken {
    pub kind: JsonType,
    pub span: Span,
    pub first_child: Option<usize>,
    pub last_child: Option<usize>,
    pub next_sibling: Option<usize>,
    pub child_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsonError {
    Empty,
    TooLarge,
    TooDeep,
    TooManyTokens,
    Invalid,
    Allocation,
}

/// A parsed document retains its bytes and indexes every token with stable offsets.
/// `parse` borrows its input; `parse_owned` copies it into document-owned storage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonDocument<'a> {
    source: Cow<'a, [u8]>,
    tokens: Vec<JsonToken>,
    root: usize,
}

impl<'a> JsonDocument<'a> {
    pub fn parse(source: &'a [u8]) -> Result<Self, JsonError> {
        Self::from_source(Cow::Borrowed(source))
    }

    pub fn parse_owned(source: &[u8]) -> Result<JsonDocument<'static>, JsonError> {
        let mut owned = Vec::new();
        owned.try_reserve_exact(source.len()).map_err(|_| JsonError::Allocation)?;
        owned.extend_from_slice(source);
        JsonDocument::<'static>::from_source(Cow::Owned(owned))
    }

    fn from_source(source: Cow<'a, [u8]>) -> Result<Self, JsonError> {
        if source.is_empty() {
            return Err(JsonError::Empty);
        }
        if source.len() > MAX_BYTES {
            return Err(JsonError::TooLarge);
        }
        let (tokens, root) = {
            let mut parser = Parser { source: &source, cursor: 0, tokens: Vec::new() };
            let root = parser.value(None, 0)?;
            parser.space();
            if parser.cursor != source.len() {
                return Err(JsonError::Invalid);
            }
            (parser.tokens, root)
        };
        Ok(Self { source, tokens, root })
    }

    pub fn source(&self) -> &[u8] { &self.source }
    pub fn tokens(&self) -> &[JsonToken] { &self.tokens }
    pub fn root(&self) -> usize { self.root }

    pub fn token(&self, token: usize) -> Option<&JsonToken> { self.tokens.get(token) }

    /// Returns the first matching member, preserving the C lookup rule for duplicate keys.
    pub fn object_get(&self, object: usize, key: &[u8]) -> Option<usize> {
        let parent = self.tokens.get(object)?;
        if parent.kind != JsonType::Object { return None; }
        let mut key_token = parent.first_child;
        while let Some(key_index) = key_token {
            let value_index = self.tokens.get(key_index)?.next_sibling?;
            if self.string_equals(key_index, key) { return Some(value_index); }
            key_token = self.tokens.get(value_index)?.next_sibling;
        }
        None
    }

    pub fn array_get(&self, array: usize, index: usize) -> Option<usize> {
        let parent = self.tokens.get(array)?;
        if parent.kind != JsonType::Array { return None; }
        let mut child = parent.first_child;
        for _ in 0..index { child = self.tokens.get(child?)?.next_sibling; }
        child
    }

    pub fn raw_token(&self, token: usize) -> Option<&[u8]> {
        self.tokens.get(token)?.span.slice(&self.source)
    }

    pub fn decoded_string(&self, token: usize) -> Option<Vec<u8>> {
        let item = self.tokens.get(token)?;
        if item.kind != JsonType::String { return None; }
        decode_string(self.raw_token(token)?)
    }

    pub fn string_equals(&self, token: usize, literal: &[u8]) -> bool {
        self.decoded_string(token).is_some_and(|decoded| decoded == literal)
    }

    pub fn uint(&self, token: usize) -> Option<u64> {
        let item = self.tokens.get(token)?;
        if item.kind != JsonType::Number { return None; }
        let raw = self.raw_token(token)?;
        if raw.is_empty() { return None; }
        let mut result = 0_u64;
        for byte in raw {
            if !byte.is_ascii_digit() { return None; }
            result = result.checked_mul(10)?.checked_add(u64::from(byte - b'0'))?;
        }
        Some(result)
    }

    pub fn bool(&self, token: usize) -> Option<bool> {
        match self.tokens.get(token)?.kind {
            JsonType::True => Some(true),
            JsonType::False => Some(false),
            _ => None,
        }
    }
}

struct Parser<'a> {
    source: &'a [u8],
    cursor: usize,
    tokens: Vec<JsonToken>,
}

impl Parser<'_> {
    fn space(&mut self) {
        while self.source.get(self.cursor).is_some_and(u8::is_ascii_whitespace) {
            // JSON permits only the four ASCII whitespace bytes.
            if !matches!(self.source[self.cursor], b' ' | b'\t' | b'\r' | b'\n') { break; }
            self.cursor += 1;
        }
    }

    fn add(&mut self, kind: JsonType, start: usize) -> Result<usize, JsonError> {
        if self.tokens.len() >= MAX_TOKENS { return Err(JsonError::TooManyTokens); }
        self.tokens.try_reserve(1).map_err(|_| JsonError::Allocation)?;
        let index = self.tokens.len();
        self.tokens.push(JsonToken {
            kind,
            span: Span { start, end: start },
            first_child: None,
            last_child: None,
            next_sibling: None,
            child_count: 0,
        });
        Ok(index)
    }

    fn child(&mut self, parent: usize, child: usize) -> Result<(), JsonError> {
        let parent_token = self.tokens.get(parent).ok_or(JsonError::Invalid)?;
        let previous = parent_token.last_child;
        let next_count = parent_token.child_count.checked_add(1).ok_or(JsonError::Invalid)?;
        if let Some(last) = previous {
            self.tokens.get_mut(last).ok_or(JsonError::Invalid)?.next_sibling = Some(child);
        } else {
            self.tokens.get_mut(parent).ok_or(JsonError::Invalid)?.first_child = Some(child);
        }
        let parent_token = self.tokens.get_mut(parent).ok_or(JsonError::Invalid)?;
        parent_token.last_child = Some(child);
        parent_token.child_count = next_count;
        Ok(())
    }

    fn value(&mut self, parent: Option<usize>, depth: usize) -> Result<usize, JsonError> {
        self.space();
        let byte = *self.source.get(self.cursor).ok_or(JsonError::Invalid)?;
        match byte {
            b'{' => self.container(parent, depth, true),
            b'[' => self.container(parent, depth, false),
            b'"' => self.string(parent),
            b'-' | b'0'..=b'9' => self.number(parent),
            b't' => self.literal(parent, b"true", JsonType::True),
            b'f' => self.literal(parent, b"false", JsonType::False),
            b'n' => self.literal(parent, b"null", JsonType::Null),
            _ => Err(JsonError::Invalid),
        }
    }

    fn container(&mut self, parent: Option<usize>, depth: usize, object: bool) -> Result<usize, JsonError> {
        if depth >= MAX_DEPTH { return Err(JsonError::TooDeep); }
        let start = self.cursor;
        let opening = if object { b'{' } else { b'[' };
        let closing = if object { b'}' } else { b']' };
        if self.source.get(self.cursor) != Some(&opening) { return Err(JsonError::Invalid); }
        self.cursor += 1;
        let index = self.add(if object { JsonType::Object } else { JsonType::Array }, start)?;
        if let Some(parent) = parent { self.child(parent, index)?; }
        self.space();
        if self.source.get(self.cursor) == Some(&closing) {
            self.cursor += 1;
            self.tokens[index].span.end = self.cursor;
            return Ok(index);
        }
        loop {
            self.space();
            if object {
                if self.source.get(self.cursor) != Some(&b'"') { return Err(JsonError::Invalid); }
                self.string(Some(index))?;
                self.space();
                if self.source.get(self.cursor) != Some(&b':') { return Err(JsonError::Invalid); }
                self.cursor += 1;
                self.space();
            }
            self.value(Some(index), depth + 1)?;
            self.space();
            match self.source.get(self.cursor) {
                Some(byte) if *byte == closing => {
                    self.cursor += 1;
                    self.tokens[index].span.end = self.cursor;
                    return Ok(index);
                }
                Some(b',') => {
                    self.cursor += 1;
                    self.space();
                    if self.source.get(self.cursor) == Some(&closing) { return Err(JsonError::Invalid); }
                }
                _ => return Err(JsonError::Invalid),
            }
        }
    }

    fn string(&mut self, parent: Option<usize>) -> Result<usize, JsonError> {
        let start = self.cursor;
        self.cursor += 1;
        loop {
            let byte = *self.source.get(self.cursor).ok_or(JsonError::Invalid)?;
            match byte {
                b'"' => {
                    self.cursor += 1;
                    let index = self.add(JsonType::String, start)?;
                    self.tokens[index].span.end = self.cursor;
                    if let Some(parent) = parent { self.child(parent, index)?; }
                    return Ok(index);
                }
                0..=0x1f => return Err(JsonError::Invalid),
                b'\\' => {
                    self.cursor += 1;
                    match self.source.get(self.cursor).copied().ok_or(JsonError::Invalid)? {
                        b'"' | b'\\' | b'/' | b'b' | b'f' | b'n' | b'r' | b't' => self.cursor += 1,
                        b'u' => {
                            self.cursor += 1;
                            let code = self.hex4()?;
                            if (0xd800..=0xdbff).contains(&code) {
                                if self.source.get(self.cursor..self.cursor + 2) != Some(&b"\\u"[..]) { return Err(JsonError::Invalid); }
                                self.cursor += 2;
                                let low = self.hex4()?;
                                if !(0xdc00..=0xdfff).contains(&low) { return Err(JsonError::Invalid); }
                            } else if (0xdc00..=0xdfff).contains(&code) { return Err(JsonError::Invalid); }
                        }
                        _ => return Err(JsonError::Invalid),
                    }
                }
                0x20..=0x7f => self.cursor += 1,
                _ => {
                    let width = utf8_width(self.source, self.cursor).ok_or(JsonError::Invalid)?;
                    self.cursor += width;
                }
            }
        }
    }

    fn hex4(&mut self) -> Result<u32, JsonError> {
        let end = self.cursor.checked_add(4).ok_or(JsonError::Invalid)?;
        let digits = self.source.get(self.cursor..end).ok_or(JsonError::Invalid)?;
        let mut value = 0_u32;
        for byte in digits {
            let digit = match byte {
                b'0'..=b'9' => u32::from(byte - b'0'),
                b'a'..=b'f' => u32::from(byte - b'a') + 10,
                b'A'..=b'F' => u32::from(byte - b'A') + 10,
                _ => return Err(JsonError::Invalid),
            };
            value = (value << 4) | digit;
        }
        self.cursor = end;
        Ok(value)
    }

    fn number(&mut self, parent: Option<usize>) -> Result<usize, JsonError> {
        let start = self.cursor;
        if self.source.get(self.cursor) == Some(&b'-') { self.cursor += 1; }
        match self.source.get(self.cursor) {
            Some(b'0') => self.cursor += 1,
            Some(b'1'..=b'9') => {
                self.cursor += 1;
                while self.source.get(self.cursor).is_some_and(u8::is_ascii_digit) { self.cursor += 1; }
            }
            _ => return Err(JsonError::Invalid),
        }
        if self.source.get(self.cursor) == Some(&b'.') {
            self.cursor += 1;
            let first = self.cursor;
            while self.source.get(self.cursor).is_some_and(u8::is_ascii_digit) { self.cursor += 1; }
            if first == self.cursor { return Err(JsonError::Invalid); }
        }
        if matches!(self.source.get(self.cursor), Some(b'e' | b'E')) {
            self.cursor += 1;
            if matches!(self.source.get(self.cursor), Some(b'+' | b'-')) { self.cursor += 1; }
            let first = self.cursor;
            while self.source.get(self.cursor).is_some_and(u8::is_ascii_digit) { self.cursor += 1; }
            if first == self.cursor { return Err(JsonError::Invalid); }
        }
        let index = self.add(JsonType::Number, start)?;
        self.tokens[index].span.end = self.cursor;
        if let Some(parent) = parent { self.child(parent, index)?; }
        Ok(index)
    }

    fn literal(&mut self, parent: Option<usize>, literal: &[u8], kind: JsonType) -> Result<usize, JsonError> {
        let start = self.cursor;
        let end = start.checked_add(literal.len()).ok_or(JsonError::Invalid)?;
        if self.source.get(start..end) != Some(literal) { return Err(JsonError::Invalid); }
        self.cursor = end;
        let index = self.add(kind, start)?;
        self.tokens[index].span.end = end;
        if let Some(parent) = parent { self.child(parent, index)?; }
        Ok(index)
    }
}

fn utf8_width(source: &[u8], offset: usize) -> Option<usize> {
    let first = *source.get(offset)?;
    if first < 0x80 { return Some(1); }
    let width = match first { 0xc2..=0xdf => 2, 0xe0..=0xef => 3, 0xf0..=0xf4 => 4, _ => return None };
    let bytes = source.get(offset..offset.checked_add(width)?)?;
    let second = bytes[1];
    if second & 0xc0 != 0x80 ||
        (first == 0xe0 && second < 0xa0) || (first == 0xed && second >= 0xa0) ||
        (first == 0xf0 && second < 0x90) || (first == 0xf4 && second >= 0x90) { return None; }
    if bytes[2..].iter().any(|byte| byte & 0xc0 != 0x80) { return None; }
    Some(width)
}

fn decode_string(raw: &[u8]) -> Option<Vec<u8>> {
    let inner = raw.get(1..raw.len().checked_sub(1)?)?;
    let mut output = Vec::new();
    output.try_reserve_exact(inner.len()).ok()?;
    let mut cursor = 0;
    while cursor < inner.len() {
        if inner[cursor] != b'\\' {
            let byte = inner[cursor];
            if byte < 0x80 {
                output.push(byte);
                cursor += 1;
            } else {
                let width = utf8_width(inner, cursor)?;
                output.extend_from_slice(&inner[cursor..cursor + width]);
                cursor += width;
            }
            continue;
        }
        cursor += 1;
        let escape = *inner.get(cursor)?;
        cursor += 1;
        let simple = match escape {
            b'"' => Some(b'"'), b'\\' => Some(b'\\'), b'/' => Some(b'/'),
            b'b' => Some(8), b'f' => Some(12), b'n' => Some(b'\n'),
            b'r' => Some(b'\r'), b't' => Some(b'\t'), _ => None,
        };
        if let Some(byte) = simple { output.push(byte); continue; }
        if escape != b'u' { return None; }
        let mut codepoint = read_hex4(inner, &mut cursor)?;
        if (0xd800..=0xdbff).contains(&codepoint) {
            if inner.get(cursor..cursor.checked_add(2)?)? != b"\\u" { return None; }
            cursor += 2;
            let low = read_hex4(inner, &mut cursor)?;
            if !(0xdc00..=0xdfff).contains(&low) { return None; }
            codepoint = 0x10000 + ((codepoint - 0xd800) << 10) + (low - 0xdc00);
        }
        push_utf8(codepoint, &mut output)?;
    }
    Some(output)
}

fn read_hex4(source: &[u8], cursor: &mut usize) -> Option<u32> {
    let end = cursor.checked_add(4)?;
    let mut value = 0_u32;
    for byte in source.get(*cursor..end)? {
        value = (value << 4) | match byte {
            b'0'..=b'9' => u32::from(byte - b'0'), b'a'..=b'f' => u32::from(byte - b'a') + 10,
            b'A'..=b'F' => u32::from(byte - b'A') + 10, _ => return None,
        };
    }
    *cursor = end;
    Some(value)
}

fn push_utf8(codepoint: u32, output: &mut Vec<u8>) -> Option<()> {
    let ch = char::from_u32(codepoint)?;
    let mut encoded = [0; 4];
    output.extend_from_slice(ch.encode_utf8(&mut encoded).as_bytes());
    Some(())
}
