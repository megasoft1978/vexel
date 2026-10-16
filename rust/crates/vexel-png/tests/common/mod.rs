//! Test-only PNG decoder: chunk walk with CRC check, zlib with Adler check,
//! inflate for stored and fixed blocks (puff-style canonical decoding), and
//! unfiltering of filter type 0. Not part of the library API.
#![allow(dead_code)]

use vexel_png::{adler32, crc32};

pub struct Decoded {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    pub idat_chunks: usize,
}

struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
    bit: u32,
}

impl Reader<'_> {
    fn bit(&mut self) -> u32 {
        let byte = self.data[self.pos];
        let v = u32::from(byte >> self.bit) & 1;
        self.bit += 1;
        if self.bit == 8 {
            self.bit = 0;
            self.pos += 1;
        }
        v
    }
    fn bits(&mut self, n: u32) -> u32 {
        (0..n).fold(0, |acc, i| acc | self.bit() << i)
    }
}

/// Canonical Huffman decoding table: symbols per length plus length counts.
struct Huffman {
    count: [u16; 16],
    symbol: Vec<u16>,
}

impl Huffman {
    fn new(lengths: &[u8]) -> Self {
        let mut count = [0u16; 16];
        for &l in lengths {
            count[usize::from(l)] += 1;
        }
        let mut offs = [0u16; 16];
        for i in 1..15 {
            offs[i + 1] = offs[i] + count[i];
        }
        let mut symbol = vec![0u16; lengths.len()];
        for (sym, &l) in lengths.iter().enumerate() {
            if l != 0 {
                symbol[usize::from(offs[usize::from(l)])] = sym as u16;
                offs[usize::from(l)] += 1;
            }
        }
        Self { count, symbol }
    }

    fn decode(&self, r: &mut Reader<'_>) -> usize {
        let (mut code, mut first, mut index) = (0i32, 0i32, 0i32);
        for len in 1..16 {
            code |= r.bit() as i32;
            let count = i32::from(self.count[len]);
            if code - count < first {
                return usize::from(self.symbol[(index + (code - first)) as usize]);
            }
            index += count;
            first += count;
            first <<= 1;
            code <<= 1;
        }
        panic!("bad huffman code");
    }
}

const LEN_BASE: [u16; 29] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258,
];
const LEN_EXTRA: [u8; 29] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
];
const DIST_BASE: [u16; 30] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
    2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
];
const DIST_EXTRA: [u8; 30] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13,
];

pub fn inflate(data: &[u8]) -> Vec<u8> {
    let mut r = Reader {
        data,
        pos: 0,
        bit: 0,
    };
    let mut out = Vec::new();
    loop {
        let last = r.bit();
        match r.bits(2) {
            0 => {
                if r.bit != 0 {
                    r.bit = 0;
                    r.pos += 1;
                }
                let len = usize::from(u16::from_le_bytes([data[r.pos], data[r.pos + 1]]));
                let nlen = u16::from_le_bytes([data[r.pos + 2], data[r.pos + 3]]);
                assert_eq!(nlen, !(len as u16), "stored NLEN");
                assert!(len <= 65535);
                out.extend_from_slice(&data[r.pos + 4..r.pos + 4 + len]);
                r.pos += 4 + len;
            }
            1 => {
                let mut l = [0u8; 288];
                l[..144].fill(8);
                l[144..256].fill(9);
                l[256..280].fill(7);
                l[280..].fill(8);
                let lit = Huffman::new(&l);
                let dist = Huffman::new(&[5u8; 30]);
                loop {
                    let sym = lit.decode(&mut r);
                    match sym {
                        0..=255 => out.push(sym as u8),
                        256 => break,
                        _ => {
                            let i = sym - 257;
                            let len =
                                usize::from(LEN_BASE[i]) + r.bits(u32::from(LEN_EXTRA[i])) as usize;
                            let d = dist.decode(&mut r);
                            let back = usize::from(DIST_BASE[d])
                                + r.bits(u32::from(DIST_EXTRA[d])) as usize;
                            assert!(back <= out.len(), "distance before start");
                            for _ in 0..len {
                                out.push(out[out.len() - back]);
                            }
                        }
                    }
                }
            }
            other => panic!("unsupported block type {other}"),
        }
        if last == 1 {
            break;
        }
    }
    let consumed = r.pos + usize::from(r.bit != 0);
    assert_eq!(
        data.len(),
        consumed + 4,
        "bytes between deflate end and Adler"
    );
    let adler = u32::from_be_bytes(data[consumed..consumed + 4].try_into().unwrap());
    assert_eq!(adler, adler32(&out), "Adler-32");
    out
}

pub fn decode(png: &[u8]) -> Decoded {
    assert_eq!(&png[..8], &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]);
    let mut pos = 8;
    let (mut width, mut height) = (0, 0);
    let mut idat = Vec::new();
    let mut idat_chunks = 0;
    let mut kinds = Vec::new();
    while pos < png.len() {
        let len = u32::from_be_bytes(png[pos..pos + 4].try_into().unwrap()) as usize;
        let kind = &png[pos + 4..pos + 8];
        let body = &png[pos + 8..pos + 8 + len];
        let crc = u32::from_be_bytes(png[pos + 8 + len..pos + 12 + len].try_into().unwrap());
        assert_eq!(crc, crc32(&png[pos + 4..pos + 8 + len]), "chunk CRC");
        match kind {
            b"IHDR" => {
                assert_eq!(len, 13);
                width = u32::from_be_bytes(body[..4].try_into().unwrap());
                height = u32::from_be_bytes(body[4..8].try_into().unwrap());
                assert_eq!(&body[8..], &[8, 6, 0, 0, 0], "RGBA8, no interlace");
            }
            b"IDAT" => {
                idat.extend_from_slice(body);
                idat_chunks += 1;
            }
            _ => {}
        }
        kinds.push(kind.to_vec());
        pos += 12 + len;
    }
    assert_eq!(pos, png.len());
    assert_eq!(kinds.first().unwrap().as_slice(), b"IHDR");
    assert_eq!(kinds.last().unwrap().as_slice(), b"IEND");
    assert_eq!(kinds.iter().filter(|k| k.as_slice() == b"IHDR").count(), 1);
    assert_eq!(kinds.iter().filter(|k| k.as_slice() == b"IEND").count(), 1);
    assert_eq!(idat[0], 0x78);
    assert_eq!(idat[1], 0x01);
    assert_eq!((u32::from(idat[0]) << 8 | u32::from(idat[1])) % 31, 0);
    let raw = inflate(&idat[2..]);
    let row = width as usize * 4;
    assert_eq!(raw.len(), (row + 1) * height as usize);
    let mut rgba = Vec::new();
    for line in raw.chunks_exact(row + 1) {
        assert_eq!(line[0], 0, "filter type 0 only");
        rgba.extend_from_slice(&line[1..]);
    }
    Decoded {
        width,
        height,
        rgba,
        idat_chunks,
    }
}
