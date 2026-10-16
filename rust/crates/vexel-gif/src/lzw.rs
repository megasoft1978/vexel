//! Variable-width GIF LZW encoder (clear/end codes, 12-bit maximum width).

use std::collections::HashMap;

const MAX_CODE_BITS: u32 = 12;
const TABLE_LIMIT: u32 = 1 << MAX_CODE_BITS;

struct BitSink<'a> {
    out: &'a mut Vec<u8>,
    block: Vec<u8>,
    acc: u32,
    bits: u32,
}

impl<'a> BitSink<'a> {
    fn put(&mut self, code: u32, width: u32) {
        self.acc |= code << self.bits;
        self.bits += width;
        while self.bits >= 8 {
            self.push_byte((self.acc & 0xff) as u8);
            self.acc >>= 8;
            self.bits -= 8;
        }
    }

    fn push_byte(&mut self, byte: u8) {
        self.block.push(byte);
        if self.block.len() == 255 {
            self.flush_block();
        }
    }

    fn flush_block(&mut self) {
        if !self.block.is_empty() {
            self.out.push(self.block.len() as u8);
            self.out.extend_from_slice(&self.block);
            self.block.clear();
        }
    }

    fn finish(&mut self) {
        if self.bits > 0 {
            self.push_byte((self.acc & 0xff) as u8);
            self.acc = 0;
            self.bits = 0;
        }
        self.flush_block();
        self.out.push(0);
    }
}

/// Appends the minimum-code-size byte, the LZW data sub-blocks and the block
/// terminator for `indices` to `out`. `min_code_size` must be 2..=8 and every
/// index must be below `1 << min_code_size`. `indices` must not be empty.
pub(crate) fn encode(indices: &[u8], min_code_size: u8, out: &mut Vec<u8>) {
    debug_assert!((2..=8).contains(&min_code_size) && !indices.is_empty());
    let clear = 1u32 << min_code_size;
    let end = clear + 1;
    out.push(min_code_size);
    let mut sink = BitSink {
        out,
        block: Vec::with_capacity(255),
        acc: 0,
        bits: 0,
    };
    let mut width = u32::from(min_code_size) + 1;
    let mut next = end + 1;
    let mut dict: HashMap<u32, u16> = HashMap::new();
    sink.put(clear, width);
    let mut prefix = u32::from(indices[0]);
    for &byte in &indices[1..] {
        let key = (prefix << 8) | u32::from(byte);
        if let Some(&code) = dict.get(&key) {
            prefix = u32::from(code);
            continue;
        }
        sink.put(prefix, width);
        if next < TABLE_LIMIT {
            dict.insert(key, next as u16);
            next += 1;
            if next > (1 << width) && width < MAX_CODE_BITS {
                width += 1;
            }
        } else {
            sink.put(clear, width);
            dict.clear();
            width = u32::from(min_code_size) + 1;
            next = end + 1;
        }
        prefix = u32::from(byte);
    }
    sink.put(prefix, width);
    // The decoder adds an entry when it reads the last code, which may widen
    // its codes before it reads the end code; the encoder added none here.
    if next == (1 << width) && width < MAX_CODE_BITS {
        width += 1;
    }
    sink.put(end, width);
    sink.finish();
}
