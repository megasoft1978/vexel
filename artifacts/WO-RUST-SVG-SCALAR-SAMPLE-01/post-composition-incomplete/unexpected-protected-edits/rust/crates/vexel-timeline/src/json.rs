use crate::schema::{DecodeFailure, Error, MAX_STRING_BYTES};
use vexel_base::Span;

pub(crate) const MAX_FIELDS: usize = 16;
pub(crate) const MAX_DEPTH: usize = 32;

#[derive(Clone, Debug)]
pub(crate) struct Field {
    pub key: Vec<u8>,
    pub value: Span,
}

pub(crate) struct Reader<'a, 'c> {
    pub source: &'a [u8],
    pub cursor: usize,
    pub cancel: Option<&'c mut dyn FnMut() -> bool>,
    pub canceled: bool,
}

impl<'a, 'c> Reader<'a, 'c> {
    pub fn new(source: &'a [u8], cancel: Option<&'c mut dyn FnMut() -> bool>) -> Self {
        Self {
            source,
            cursor: 0,
            cancel,
            canceled: false,
        }
    }
    pub fn failure(&self, error: Error, offset: usize) -> DecodeFailure {
        DecodeFailure {
            error,
            offset,
            canceled: self.canceled,
        }
    }
    pub fn poll(&mut self) -> Result<(), DecodeFailure> {
        if self.cursor != 0 && self.cursor & 4095 == 0 {
            if self.cancel.as_mut().is_some_and(|check| check()) {
                self.canceled = true;
                return Err(self.failure(Error::InvalidArgument, self.cursor));
            }
        }
        Ok(())
    }
    pub fn space(&mut self) -> Result<(), DecodeFailure> {
        loop {
            self.poll()?;
            match self.source.get(self.cursor) {
                Some(b' ' | b'\t' | b'\r' | b'\n') => self.cursor += 1,
                _ => return Ok(()),
            }
        }
    }
    pub fn scan(&mut self, depth: usize) -> Result<(), DecodeFailure> {
        self.space()?;
        if depth > MAX_DEPTH {
            return Err(self.failure(Error::Limit, self.cursor));
        }
        let byte = *self
            .source
            .get(self.cursor)
            .ok_or_else(|| self.failure(Error::MalformedJson, self.cursor))?;
        match byte {
            b'{' => self.object(depth),
            b'[' => self.array(depth),
            b'"' => {
                self.string()?;
                Ok(())
            }
            b'-' | b'0'..=b'9' => self.number(),
            b't' => self.literal(b"true"),
            b'f' => self.literal(b"false"),
            b'n' => self.literal(b"null"),
            _ => Err(self.failure(Error::MalformedJson, self.cursor)),
        }
    }
    fn object(&mut self, depth: usize) -> Result<(), DecodeFailure> {
        self.cursor += 1;
        self.space()?;
        if self.source.get(self.cursor) == Some(&b'}') {
            self.cursor += 1;
            return Ok(());
        }
        let mut keys: Vec<Vec<u8>> = Vec::new();
        loop {
            let key_offset = self.cursor;
            if self.source.get(self.cursor) != Some(&b'"') {
                return Err(self.failure(Error::MalformedJson, self.cursor));
            }
            let key = self.string()?;
            if keys.iter().any(|prior| prior == &key) {
                return Err(self.failure(Error::Duplicate, key_offset));
            }
            if keys.len() == MAX_FIELDS {
                return Err(self.failure(Error::Limit, self.cursor));
            }
            keys.try_reserve(1)
                .map_err(|_| self.failure(Error::Limit, key_offset))?;
            keys.push(key);
            self.space()?;
            if self.source.get(self.cursor) != Some(&b':') {
                return Err(self.failure(Error::MalformedJson, self.cursor));
            }
            self.cursor += 1;
            self.scan(depth + 1)?;
            self.space()?;
            match self.source.get(self.cursor) {
                Some(b'}') => {
                    self.cursor += 1;
                    return Ok(());
                }
                Some(b',') => {
                    self.cursor += 1;
                    self.space()?;
                }
                _ => return Err(self.failure(Error::MalformedJson, self.cursor)),
            }
        }
    }
    fn array(&mut self, depth: usize) -> Result<(), DecodeFailure> {
        self.cursor += 1;
        self.space()?;
        if self.source.get(self.cursor) == Some(&b']') {
            self.cursor += 1;
            return Ok(());
        }
        loop {
            self.scan(depth + 1)?;
            self.space()?;
            match self.source.get(self.cursor) {
                Some(b']') => {
                    self.cursor += 1;
                    return Ok(());
                }
                Some(b',') => {
                    self.cursor += 1;
                    self.space()?;
                }
                _ => return Err(self.failure(Error::MalformedJson, self.cursor)),
            }
        }
    }
    fn string(&mut self) -> Result<Vec<u8>, DecodeFailure> {
        if self.source.get(self.cursor) != Some(&b'"') {
            return Err(self.failure(Error::MalformedJson, self.cursor));
        }
        let start = self.cursor + 1;
        self.cursor += 1;
        loop {
            self.poll()?;
            let byte = *self
                .source
                .get(self.cursor)
                .ok_or_else(|| self.failure(Error::MalformedJson, self.cursor))?;
            match byte {
                b'"' => {
                    let end = self.cursor;
                    self.cursor += 1;
                    return decode_string(self.source.get(start..end).unwrap_or_default())
                        .ok_or_else(|| self.failure(Error::Limit, self.cursor));
                }
                0..=0x1f => return Err(self.failure(Error::MalformedJson, self.cursor)),
                b'\\' => {
                    self.cursor += 1;
                    match self.source.get(self.cursor) {
                        Some(b'"' | b'\\' | b'/' | b'b' | b'f' | b'n' | b'r' | b't') => {
                            self.cursor += 1
                        }
                        Some(b'u') => {
                            self.cursor += 1;
                            let high = self.hex4()?;
                            if (0xd800..=0xdbff).contains(&high) {
                                if self.source.get(self.cursor..self.cursor.saturating_add(2))
                                    != Some(&b"\\u"[..])
                                {
                                    return Err(self.failure(Error::MalformedJson, self.cursor));
                                }
                                self.cursor += 2;
                                let low = self.hex4()?;
                                if !(0xdc00..=0xdfff).contains(&low) {
                                    return Err(self.failure(Error::MalformedJson, self.cursor));
                                }
                            } else if (0xdc00..=0xdfff).contains(&high) {
                                return Err(self.failure(Error::MalformedJson, self.cursor));
                            }
                        }
                        _ => return Err(self.failure(Error::MalformedJson, self.cursor)),
                    }
                }
                0x20..=0x7f => self.cursor += 1,
                _ => {
                    let width = utf8_width(self.source, self.cursor)
                        .ok_or_else(|| self.failure(Error::MalformedJson, self.cursor))?;
                    self.cursor += width;
                }
            }
        }
    }
    fn hex4(&mut self) -> Result<u32, DecodeFailure> {
        let start = self.cursor;
        let end = start
            .checked_add(4)
            .ok_or_else(|| self.failure(Error::MalformedJson, start))?;
        let digits = self
            .source
            .get(start..end)
            .ok_or_else(|| self.failure(Error::MalformedJson, start))?;
        let mut value = 0;
        for byte in digits {
            value = value * 16
                + match byte {
                    b'0'..=b'9' => u32::from(byte - b'0'),
                    b'a'..=b'f' => u32::from(byte - b'a') + 10,
                    b'A'..=b'F' => u32::from(byte - b'A') + 10,
                    _ => return Err(self.failure(Error::MalformedJson, start)),
                };
        }
        self.cursor = end;
        Ok(value)
    }
    fn number(&mut self) -> Result<(), DecodeFailure> {
        let start = self.cursor;
        if self.source.get(self.cursor) == Some(&b'-') {
            self.cursor += 1;
        }
        match self.source.get(self.cursor) {
            Some(b'0') => self.cursor += 1,
            Some(b'1'..=b'9') => {
                self.cursor += 1;
                while self.source.get(self.cursor).is_some_and(u8::is_ascii_digit) {
                    self.cursor += 1;
                }
            }
            _ => return Err(self.failure(Error::MalformedJson, self.cursor)),
        }
        if self.source.get(self.cursor) == Some(&b'.') {
            self.cursor += 1;
            let digits = self.cursor;
            while self.source.get(self.cursor).is_some_and(u8::is_ascii_digit) {
                self.cursor += 1;
            }
            if digits == self.cursor {
                return Err(self.failure(Error::MalformedJson, self.cursor));
            }
        }
        if matches!(self.source.get(self.cursor), Some(b'e' | b'E')) {
            self.cursor += 1;
            if matches!(self.source.get(self.cursor), Some(b'+' | b'-')) {
                self.cursor += 1;
            }
            let digits = self.cursor;
            while self.source.get(self.cursor).is_some_and(u8::is_ascii_digit) {
                self.cursor += 1;
            }
            if digits == self.cursor {
                return Err(self.failure(Error::MalformedJson, self.cursor));
            }
        }
        let raw = self.source.get(start..self.cursor).unwrap_or_default();
        if raw.len() > 96 {
            return Err(self.failure(Error::Limit, self.cursor));
        }
        let text =
            std::str::from_utf8(raw).map_err(|_| self.failure(Error::MalformedJson, start))?;
        let value = text
            .parse::<f64>()
            .map_err(|_| self.failure(Error::InvalidValue, self.cursor))?;
        if !value.is_finite() {
            return Err(self.failure(Error::InvalidValue, self.cursor));
        }
        Ok(())
    }
    fn literal(&mut self, literal: &[u8]) -> Result<(), DecodeFailure> {
        let end = self
            .cursor
            .checked_add(literal.len())
            .ok_or_else(|| self.failure(Error::MalformedJson, self.cursor))?;
        if self.source.get(self.cursor..end) != Some(literal) {
            return Err(self.failure(Error::MalformedJson, self.cursor));
        }
        self.cursor = end;
        Ok(())
    }
}

pub(crate) fn utf8_width(source: &[u8], offset: usize) -> Option<usize> {
    let first = *source.get(offset)?;
    let width = match first {
        0xc2..=0xdf => 2,
        0xe0..=0xef => 3,
        0xf0..=0xf4 => 4,
        _ => return None,
    };
    let end = offset.checked_add(width)?;
    let bytes = source.get(offset..end)?;
    let second = bytes[1];
    if second & 0xc0 != 0x80
        || (first == 0xe0 && second < 0xa0)
        || (first == 0xed && second >= 0xa0)
        || (first == 0xf0 && second < 0x90)
        || (first == 0xf4 && second >= 0x90)
        || bytes[2..].iter().any(|b| b & 0xc0 != 0x80)
    {
        return None;
    }
    Some(width)
}

pub(crate) fn decode_string(raw: &[u8]) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    out.try_reserve(raw.len().min(MAX_STRING_BYTES)).ok()?;
    let mut i = 0;
    while i < raw.len() {
        if raw[i] != b'\\' {
            if raw[i] < 0x80 {
                if out.len() == MAX_STRING_BYTES {
                    return None;
                }
                out.push(raw[i]);
                i += 1;
            } else {
                let n = utf8_width(raw, i)?;
                if out.len().checked_add(n)? > MAX_STRING_BYTES {
                    return None;
                }
                out.extend_from_slice(raw.get(i..i.checked_add(n)?)?);
                i += n;
            }
        } else {
            i += 1;
            let esc = *raw.get(i)?;
            i += 1;
            let cp = match esc {
                b'"' => 34,
                b'\\' => 92,
                b'/' => 47,
                b'b' => 8,
                b'f' => 12,
                b'n' => 10,
                b'r' => 13,
                b't' => 9,
                b'u' => {
                    let mut cp = read_hex4(raw, &mut i)?;
                    if (0xd800..=0xdbff).contains(&cp) {
                        if raw.get(i..i.checked_add(2)?)? != b"\\u" {
                            return None;
                        }
                        i += 2;
                        let low = read_hex4(raw, &mut i)?;
                        if !(0xdc00..=0xdfff).contains(&low) {
                            return None;
                        }
                        cp = 0x10000 + ((cp - 0xd800) << 10) + low - 0xdc00;
                    }
                    cp
                }
                _ => return None,
            };
            let ch = char::from_u32(cp)?;
            let mut buf = [0; 4];
            let bytes = ch.encode_utf8(&mut buf).as_bytes();
            if out.len().checked_add(bytes.len())? > MAX_STRING_BYTES {
                return None;
            }
            out.extend_from_slice(bytes);
        }
    }
    Some(out)
}

/// Decodes raw JSON string contents into caller storage. `encoded` excludes the quotes.
/// The returned byte count is set only on success; malformed data and insufficient
/// output capacity both return `None`, matching the legacy boolean contract.
pub fn string_copy(encoded: &[u8], output: &mut [u8]) -> Option<usize> {
    let mut input = 0usize;
    let mut written = 0usize;
    while input < encoded.len() {
        let byte = encoded[input];
        if byte != b'\\' {
            if byte < 0x80 {
                if byte < 0x20 {
                    return None;
                }
                *output.get_mut(written)? = byte;
                written += 1;
                input += 1;
            } else {
                let width = utf8_width(encoded, input)?;
                let end = input.checked_add(width)?;
                let target = output.get_mut(written..written.checked_add(width)?)?;
                target.copy_from_slice(encoded.get(input..end)?);
                written += width;
                input = end;
            }
        } else {
            input += 1;
            let escape = *encoded.get(input)?;
            input += 1;
            let codepoint = match escape {
                b'"' => 34,
                b'\\' => 92,
                b'/' => 47,
                b'b' => 8,
                b'f' => 12,
                b'n' => 10,
                b'r' => 13,
                b't' => 9,
                b'u' => {
                    let mut high = read_hex4(encoded, &mut input)?;
                    if (0xd800..=0xdbff).contains(&high) {
                        if encoded.get(input..input.checked_add(2)?)? != b"\\u" {
                            return None;
                        }
                        input += 2;
                        let low = read_hex4(encoded, &mut input)?;
                        if !(0xdc00..=0xdfff).contains(&low) {
                            return None;
                        }
                        high = 0x10000 + ((high - 0xd800) << 10) + low - 0xdc00;
                    } else if (0xdc00..=0xdfff).contains(&high) {
                        return None;
                    }
                    high
                }
                _ => return None,
            };
            let character = char::from_u32(codepoint)?;
            let mut scratch = [0; 4];
            let bytes = character.encode_utf8(&mut scratch).as_bytes();
            let end = written.checked_add(bytes.len())?;
            output.get_mut(written..end)?.copy_from_slice(bytes);
            written = end;
        }
        if written > MAX_STRING_BYTES {
            return None;
        }
    }
    Some(written)
}
fn read_hex4(raw: &[u8], i: &mut usize) -> Option<u32> {
    let end = i.checked_add(4)?;
    let mut value = 0;
    for byte in raw.get(*i..end)? {
        value = value * 16
            + match byte {
                b'0'..=b'9' => u32::from(byte - b'0'),
                b'a'..=b'f' => u32::from(byte - b'a') + 10,
                b'A'..=b'F' => u32::from(byte - b'A') + 10,
                _ => return None,
            };
    }
    *i = end;
    Some(value)
}

pub(crate) fn object_fields(
    source: &[u8],
    span: Span,
    cancel: Option<&mut dyn FnMut() -> bool>,
) -> Result<Vec<Field>, DecodeFailure> {
    let mut reader = Reader::new(source, cancel);
    reader.cursor = span.start;
    reader.space()?;
    if reader.source.get(reader.cursor) != Some(&b'{') {
        return Err(reader.failure(Error::InvalidSchema, span.start));
    }
    reader.cursor += 1;
    reader.space()?;
    let mut fields = Vec::new();
    if reader.source.get(reader.cursor) == Some(&b'}') {
        reader.cursor += 1;
        if reader.cursor != span.end {
            return Err(reader.failure(Error::MalformedJson, reader.cursor));
        }
        return Ok(fields);
    }
    loop {
        let key_offset = reader.cursor;
        let key = reader.string()?;
        if fields.iter().any(|field: &Field| field.key == key) {
            return Err(reader.failure(Error::Duplicate, key_offset));
        }
        reader.space()?;
        if reader.source.get(reader.cursor) != Some(&b':') {
            return Err(reader.failure(Error::MalformedJson, reader.cursor));
        }
        reader.cursor += 1;
        reader.space()?;
        let start = reader.cursor;
        if fields.len() == MAX_FIELDS {
            return Err(reader.failure(Error::Limit, start));
        }
        reader.scan(1)?;
        let end = reader.cursor;
        fields
            .try_reserve(1)
            .map_err(|_| reader.failure(Error::Limit, start))?;
        fields.push(Field {
            key,
            value: Span { start, end },
        });
        reader.space()?;
        match reader.source.get(reader.cursor) {
            Some(b'}') => {
                reader.cursor += 1;
                break;
            }
            Some(b',') => {
                reader.cursor += 1;
                reader.space()?;
            }
            _ => return Err(reader.failure(Error::MalformedJson, reader.cursor)),
        }
    }
    if reader.cursor != span.end {
        return Err(reader.failure(Error::MalformedJson, reader.cursor));
    }
    Ok(fields)
}

pub(crate) fn array_spans(
    source: &[u8],
    span: Span,
    limit: usize,
    cancel: Option<&mut dyn FnMut() -> bool>,
) -> Result<Vec<Span>, DecodeFailure> {
    let mut reader = Reader::new(source, cancel);
    reader.cursor = span.start;
    reader.space()?;
    if reader.source.get(reader.cursor) != Some(&b'[') {
        return Err(reader.failure(Error::InvalidSchema, span.start));
    }
    reader.cursor += 1;
    reader.space()?;
    let mut spans = Vec::new();
    if reader.source.get(reader.cursor) == Some(&b']') {
        reader.cursor += 1;
    } else {
        loop {
            if spans.len() == limit {
                return Err(reader.failure(Error::Limit, reader.cursor));
            }
            let start = reader.cursor;
            reader.scan(1)?;
            spans
                .try_reserve(1)
                .map_err(|_| reader.failure(Error::Limit, start))?;
            spans.push(Span {
                start,
                end: reader.cursor,
            });
            reader.space()?;
            match reader.source.get(reader.cursor) {
                Some(b']') => {
                    reader.cursor += 1;
                    break;
                }
                Some(b',') => {
                    reader.cursor += 1;
                    reader.space()?;
                }
                _ => return Err(reader.failure(Error::MalformedJson, reader.cursor)),
            }
        }
    }
    if reader.cursor != span.end {
        return Err(reader.failure(Error::MalformedJson, reader.cursor));
    }
    Ok(spans)
}
