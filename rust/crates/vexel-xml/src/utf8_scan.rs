/// Byte-at-a-time strict UTF-8 validation. The leading byte's offset remains
/// available for diagnostics while continuation bytes cross step boundaries.
#[derive(Clone, Copy, Default)]
pub(crate) struct Utf8Scan {
    pub(crate) start: usize,
    remaining: u8,
    value: u32,
    minimum: u32,
}

impl Utf8Scan {
    pub(crate) fn incomplete(&self) -> bool { self.remaining != 0 }

    pub(crate) fn push(&mut self, byte: u8, offset: usize) -> Result<Option<u32>, usize> {
        if self.remaining == 0 {
            self.start = offset;
            let (remaining, value, minimum) = match byte {
                0x00..=0x7f => return Ok(Some(u32::from(byte))),
                0xc2..=0xdf => (1, u32::from(byte & 0x1f), 0x80),
                0xe0..=0xef => (2, u32::from(byte & 0x0f), 0x800),
                0xf0..=0xf4 => (3, u32::from(byte & 0x07), 0x10000),
                _ => return Err(offset),
            };
            self.remaining = remaining;
            self.value = value;
            self.minimum = minimum;
            return Ok(None);
        }
        if byte & 0xc0 != 0x80 { return Err(self.start); }
        self.value = (self.value << 6) | u32::from(byte & 0x3f);
        self.remaining -= 1;
        if self.remaining != 0 { return Ok(None); }
        if self.value < self.minimum || self.value > 0x10ffff
            || (0xd800..=0xdfff).contains(&self.value)
        {
            return Err(self.start);
        }
        Ok(Some(self.value))
    }
}
