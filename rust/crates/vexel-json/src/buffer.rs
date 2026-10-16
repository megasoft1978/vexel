use crate::JsonDocument;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsonBufferError {
    Limit,
    Allocation,
    InvalidToken,
    Overflow,
}

/// An owned output buffer whose append operations are atomic on failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonBuffer {
    bytes: Vec<u8>,
    limit: usize,
}

impl JsonBuffer {
    pub fn new(initial_capacity: usize, limit: usize) -> Result<Self, JsonBufferError> {
        if limit == 0 || initial_capacity > limit { return Err(JsonBufferError::Limit); }
        let mut bytes = Vec::new();
        bytes.try_reserve_exact(initial_capacity).map_err(|_| JsonBufferError::Allocation)?;
        Ok(Self { bytes, limit })
    }

    pub fn as_bytes(&self) -> &[u8] { &self.bytes }
    pub fn len(&self) -> usize { self.bytes.len() }
    pub fn is_empty(&self) -> bool { self.bytes.is_empty() }
    pub fn limit(&self) -> usize { self.limit }

    pub fn append(&mut self, bytes: &[u8]) -> Result<(), JsonBufferError> {
        let needed = self.bytes.len().checked_add(bytes.len()).ok_or(JsonBufferError::Overflow)?;
        if needed > self.limit { return Err(JsonBufferError::Limit); }
        self.bytes.try_reserve(bytes.len()).map_err(|_| JsonBufferError::Allocation)?;
        self.bytes.extend_from_slice(bytes);
        Ok(())
    }

    /// Appends a slice already in this buffer, using checked offsets instead of aliased pointers.
    pub fn append_within(&mut self, range: std::ops::Range<usize>) -> Result<(), JsonBufferError> {
        if range.start > range.end || range.end > self.bytes.len() {
            return Err(JsonBufferError::InvalidToken);
        }
        let count = range.end - range.start;
        let needed = self.bytes.len().checked_add(count).ok_or(JsonBufferError::Overflow)?;
        if needed > self.limit { return Err(JsonBufferError::Limit); }
        self.bytes.try_reserve(count).map_err(|_| JsonBufferError::Allocation)?;
        self.bytes.extend_from_within(range);
        Ok(())
    }

    pub fn literal(&mut self, literal: &str) -> Result<(), JsonBufferError> {
        self.append(literal.as_bytes())
    }

    pub fn quoted(&mut self, data: &[u8]) -> Result<(), JsonBufferError> {
        let escaped = data.iter().try_fold(2_usize, |size, byte| {
            size.checked_add(match byte {
                b'"' | b'\\' | 8 | 12 | b'\n' | b'\r' | b'\t' => 2,
                0..=0x1f => 6,
                _ => 1,
            })
        }).ok_or(JsonBufferError::Overflow)?;
        let needed = self.bytes.len().checked_add(escaped).ok_or(JsonBufferError::Overflow)?;
        if needed > self.limit { return Err(JsonBufferError::Limit); }
        self.bytes.try_reserve(escaped).map_err(|_| JsonBufferError::Allocation)?;
        self.bytes.push(b'"');
        for byte in data {
            match *byte {
                b'"' | b'\\' => { self.bytes.push(b'\\'); self.bytes.push(*byte); }
                8 => self.bytes.extend_from_slice(b"\\b"),
                12 => self.bytes.extend_from_slice(b"\\f"),
                b'\n' => self.bytes.extend_from_slice(b"\\n"),
                b'\r' => self.bytes.extend_from_slice(b"\\r"),
                b'\t' => self.bytes.extend_from_slice(b"\\t"),
                0..=0x1f => {
                    const HEX: &[u8; 16] = b"0123456789abcdef";
                    self.bytes.extend_from_slice(&[b'\\', b'u', b'0', b'0', HEX[(byte >> 4) as usize], HEX[(byte & 0xf) as usize]]);
                }
                _ => self.bytes.push(*byte),
            }
        }
        self.bytes.push(b'"');
        Ok(())
    }

    pub fn raw_token(&mut self, document: &JsonDocument<'_>, token: usize) -> Result<(), JsonBufferError> {
        let raw = document.raw_token(token).ok_or(JsonBufferError::InvalidToken)?;
        self.append(raw)
    }
}
