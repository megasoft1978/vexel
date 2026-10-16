//! Test-only first-party GIF decoder (no shared code with the encoder).
#![allow(dead_code)]

#[derive(Debug)]
pub struct Decoded {
    pub width: u16,
    pub height: u16,
    pub palette: Vec<[u8; 3]>,
    pub loop_count: Option<u16>,
    pub frames: Vec<DecodedFrame>,
}

#[derive(Debug)]
pub struct DecodedFrame {
    pub delay_cs: u16,
    pub transparent: Option<u8>,
    pub disposal: u8,
    pub gce_count: usize,
    pub indices: Vec<u8>,
}

impl Decoded {
    /// RGBA of frame `i` using the global palette; transparent index -> [0,0,0,0].
    pub fn rgba(&self, i: usize) -> Vec<u8> {
        let f = &self.frames[i];
        let mut out = Vec::new();
        for &ix in &f.indices {
            if Some(ix) == f.transparent {
                out.extend_from_slice(&[0, 0, 0, 0]);
            } else {
                let c = self.palette[ix as usize];
                out.extend_from_slice(&[c[0], c[1], c[2], 255]);
            }
        }
        out
    }
}

fn u16le(b: &[u8], at: usize) -> u16 {
    u16::from(b[at]) | (u16::from(b[at + 1]) << 8)
}

fn sub_blocks(b: &[u8], pos: &mut usize) -> Vec<u8> {
    let mut data = Vec::new();
    loop {
        let n = b[*pos] as usize;
        *pos += 1;
        if n == 0 {
            return data;
        }
        data.extend_from_slice(&b[*pos..*pos + n]);
        *pos += n;
    }
}

pub fn lzw_decode(data: &[u8], min_code: u32, expected: usize) -> Result<Vec<u8>, String> {
    let clear = 1u32 << min_code;
    let end = clear + 1;
    let mut table: Vec<(u32, u8)> = Vec::new(); // (prefix code or u32::MAX, byte) for codes >= end+1
    let reset = |t: &mut Vec<(u32, u8)>| t.clear();
    let mut width = min_code + 1;
    let (mut acc, mut nbits, mut at) = (0u32, 0u32, 0usize);
    let mut out: Vec<u8> = Vec::new();
    let mut prev: Option<u32> = None;
    let expand = |t: &Vec<(u32, u8)>, mut code: u32| -> Vec<u8> {
        let mut rev = Vec::new();
        while code > end {
            let (p, b) = t[(code - end - 1) as usize];
            rev.push(b);
            code = p;
        }
        rev.push(code as u8);
        rev.reverse();
        rev
    };
    loop {
        while nbits < width {
            if at >= data.len() {
                return Err("ran out of LZW data before end code".into());
            }
            acc |= u32::from(data[at]) << nbits;
            at += 1;
            nbits += 8;
        }
        let code = acc & ((1 << width) - 1);
        acc >>= width;
        nbits -= width;
        if code == clear {
            reset(&mut table);
            width = min_code + 1;
            prev = None;
            continue;
        }
        if code == end {
            break;
        }
        let known = end + 1 + table.len() as u32;
        let string = match prev {
            None => {
                if code >= clear {
                    return Err(format!("bad first code {code}"));
                }
                vec![code as u8]
            }
            Some(p) => {
                let entry = if code < known {
                    expand(&table, code)
                } else if code == known {
                    let mut s = expand(&table, p);
                    s.push(s[0]);
                    s
                } else {
                    return Err(format!("code {code} beyond table {known}"));
                };
                if known < 4096 {
                    table.push((p, entry[0]));
                }
                entry
            }
        };
        out.extend_from_slice(&string);
        prev = Some(code);
        let next = end + 1 + table.len() as u32;
        if next == (1 << width) && width < 12 {
            width += 1;
        }
        if out.len() > expected {
            return Err("decoded more pixels than expected".into());
        }
    }
    if out.len() != expected {
        return Err(format!("decoded {} pixels, expected {expected}", out.len()));
    }
    Ok(out)
}

pub fn decode(b: &[u8]) -> Result<Decoded, String> {
    if b.len() < 13 || &b[..6] != b"GIF89a" {
        return Err("bad header".into());
    }
    let (width, height) = (u16le(b, 6), u16le(b, 8));
    let packed = b[10];
    let mut pos = 13;
    let mut palette = Vec::new();
    if packed & 0x80 != 0 {
        let n = 1usize << ((packed & 7) + 1);
        for i in 0..n {
            palette.push([b[pos + i * 3], b[pos + i * 3 + 1], b[pos + i * 3 + 2]]);
        }
        pos += n * 3;
    }
    let mut loop_count = None;
    let mut frames = Vec::new();
    let (mut delay, mut transparent, mut disposal, mut gce_count) = (0u16, None, 0u8, 0usize);
    loop {
        match *b.get(pos).ok_or("missing trailer")? {
            0x3B => {
                if pos + 1 != b.len() {
                    return Err("bytes after trailer".into());
                }
                break;
            }
            0x21 => {
                let label = b[pos + 1];
                pos += 2;
                if label == 0xF9 {
                    if b[pos] != 4 || b[pos + 5] != 0 {
                        return Err("bad GCE".into());
                    }
                    let p = b[pos + 1];
                    disposal = (p >> 2) & 7;
                    delay = u16le(b, pos + 2);
                    transparent = (p & 1 != 0).then_some(b[pos + 4]);
                    gce_count += 1;
                    pos += 6;
                } else if label == 0xFF {
                    let n = b[pos] as usize;
                    let id = b[pos + 1..pos + 1 + n].to_vec();
                    pos += 1 + n;
                    let data = sub_blocks(b, &mut pos);
                    if id == b"NETSCAPE2.0" && data.len() == 3 && data[0] == 1 {
                        loop_count = Some(u16le(&data, 1));
                    }
                } else {
                    sub_blocks(b, &mut pos);
                }
            }
            0x2C => {
                let (w, h) = (u16le(b, pos + 5), u16le(b, pos + 7));
                if (u16le(b, pos + 1), u16le(b, pos + 3)) != (0, 0) || b[pos + 9] != 0 {
                    return Err("unsupported image descriptor".into());
                }
                pos += 10;
                let min_code = u32::from(b[pos]);
                pos += 1;
                let data = sub_blocks(b, &mut pos);
                let indices = lzw_decode(&data, min_code, w as usize * h as usize)?;
                frames.push(DecodedFrame {
                    delay_cs: delay,
                    transparent,
                    disposal,
                    gce_count,
                    indices,
                });
                gce_count = 0;
            }
            other => return Err(format!("unknown block 0x{other:02x}")),
        }
    }
    Ok(Decoded {
        width,
        height,
        palette,
        loop_count,
        frames,
    })
}

/// Deterministic xorshift for test data.
pub struct Rng(pub u64);
impl Rng {
    pub fn next(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 16) as u32
    }
}

/// `w*h` opaque pixels using exactly `colors` distinct colours (cycled).
pub fn n_colors(w: u32, h: u32, colors: u32) -> Vec<u8> {
    let mut v = Vec::new();
    for i in 0..w * h {
        let c = i % colors;
        v.extend_from_slice(&[
            (c & 255) as u8,
            (c >> 8) as u8 + 1,
            (c * 7 % 251) as u8,
            255,
        ]);
    }
    v
}
