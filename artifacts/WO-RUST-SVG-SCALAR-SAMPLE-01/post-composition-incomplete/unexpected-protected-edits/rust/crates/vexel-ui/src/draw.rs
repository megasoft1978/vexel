//! Deterministic scalar ASCII fallback and RGBA blending used by retained UI painting.
use crate::Rect;

const FONT: [[u8; 7]; 36] = [
    [14, 17, 19, 21, 25, 17, 14],
    [4, 12, 4, 4, 4, 4, 14],
    [14, 17, 1, 2, 4, 8, 31],
    [30, 1, 1, 14, 1, 1, 30],
    [2, 6, 10, 18, 31, 2, 2],
    [31, 16, 16, 30, 1, 1, 30],
    [14, 16, 16, 30, 17, 17, 14],
    [31, 1, 2, 4, 8, 8, 8],
    [14, 17, 17, 14, 17, 17, 14],
    [14, 17, 17, 15, 1, 1, 14],
    [14, 17, 17, 31, 17, 17, 17],
    [30, 17, 17, 30, 17, 17, 30],
    [14, 17, 16, 16, 16, 17, 14],
    [30, 17, 17, 17, 17, 17, 30],
    [31, 16, 16, 30, 16, 16, 31],
    [31, 16, 16, 30, 16, 16, 16],
    [14, 17, 16, 23, 17, 17, 15],
    [17, 17, 17, 31, 17, 17, 17],
    [14, 4, 4, 4, 4, 4, 14],
    [7, 2, 2, 2, 18, 18, 12],
    [17, 18, 20, 24, 20, 18, 17],
    [16, 16, 16, 16, 16, 16, 31],
    [17, 27, 21, 21, 17, 17, 17],
    [17, 25, 21, 19, 17, 17, 17],
    [14, 17, 17, 17, 17, 17, 14],
    [30, 17, 17, 30, 16, 16, 16],
    [14, 17, 17, 17, 21, 18, 13],
    [30, 17, 17, 30, 20, 18, 17],
    [15, 16, 16, 14, 1, 1, 30],
    [31, 4, 4, 4, 4, 4, 4],
    [17, 17, 17, 17, 17, 17, 14],
    [17, 17, 17, 17, 17, 10, 4],
    [17, 17, 17, 21, 21, 21, 10],
    [17, 17, 10, 4, 10, 17, 17],
    [17, 17, 10, 4, 4, 4, 4],
    [31, 1, 2, 4, 8, 16, 31],
];

pub(crate) fn glyph(c: u8, row: usize) -> u8 {
    let c = if c.is_ascii_lowercase() {
        c.to_ascii_uppercase()
    } else {
        c
    };
    if c.is_ascii_digit() {
        return FONT[(c - b'0') as usize][row];
    }
    if c.is_ascii_uppercase() {
        return FONT[10 + (c - b'A') as usize][row];
    }
    match c {
        b' ' => 0,
        b'-' => {
            if row == 3 {
                14
            } else {
                0
            }
        }
        b'_' => {
            if row == 6 {
                31
            } else {
                0
            }
        }
        b'.' => {
            if row == 6 {
                4
            } else {
                0
            }
        }
        b',' => {
            if row == 6 {
                4
            } else if row == 5 {
                8
            } else {
                0
            }
        }
        b':' => {
            if row == 2 || row == 5 {
                4
            } else {
                0
            }
        }
        b';' => {
            if row == 2 || row == 5 {
                4
            } else if row == 6 {
                8
            } else {
                0
            }
        }
        b'/' => 1 << if row > 4 { 0 } else { 4 - row },
        b'\\' => 1 << if row < 5 { row } else { 4 },
        b'#' => {
            if row == 2 || row == 4 {
                31
            } else if row == 1 || row == 5 {
                10
            } else {
                0
            }
        }
        b'%' => {
            if row == 0 || row == 6 {
                17
            } else if row == 1 || row == 5 {
                2
            } else if row == 2 || row == 4 {
                4
            } else {
                8
            }
        }
        b'=' => {
            if row == 2 || row == 4 {
                14
            } else {
                0
            }
        }
        b'+' => {
            if row == 3 {
                14
            } else if matches!(row, 1 | 2 | 4 | 5) {
                4
            } else {
                0
            }
        }
        b'[' => {
            if row == 0 || row == 6 {
                14
            } else {
                8
            }
        }
        b']' => {
            if row == 0 || row == 6 {
                14
            } else {
                2
            }
        }
        b'<' => {
            if row == 3 {
                2
            } else if row == 2 || row == 4 {
                4
            } else if row == 1 || row == 5 {
                8
            } else {
                0
            }
        }
        b'>' => {
            if row == 3 {
                8
            } else if row == 2 || row == 4 {
                4
            } else if row == 1 || row == 5 {
                2
            } else {
                0
            }
        }
        b'(' => {
            if row == 0 || row == 6 {
                2
            } else if row == 1 || row == 5 {
                4
            } else {
                8
            }
        }
        b')' => {
            if row == 0 || row == 6 {
                8
            } else if row == 1 || row == 5 {
                4
            } else {
                2
            }
        }
        b'!' => {
            if row < 5 || row == 6 {
                4
            } else {
                0
            }
        }
        b'?' => {
            if row == 0 {
                14
            } else if row == 1 {
                17
            } else if row == 2 {
                2
            } else if row == 3 || row == 6 {
                4
            } else {
                0
            }
        }
        b'\'' => {
            if row < 2 {
                4
            } else {
                0
            }
        }
        b'"' => {
            if row < 2 {
                10
            } else {
                0
            }
        }
        b'*' => {
            if row == 1 || row == 5 {
                10
            } else if row == 2 || row == 4 {
                4
            } else if row == 3 {
                14
            } else {
                0
            }
        }
        b'|' => 4,
        b'&' => {
            if row == 0 {
                12
            } else if row == 1 || row == 2 {
                18
            } else if row == 3 {
                12
            } else if row == 4 || row == 5 {
                21
            } else {
                10
            }
        }
        _ => {
            if row == 0 || row == 6 {
                14
            } else if row == 1 || row == 5 {
                17
            } else if row == 2 || row == 4 {
                2
            } else {
                4
            }
        }
    }
}

pub(crate) fn valid_buffer(width: u32, height: u32, capacity: usize, stride: usize) -> bool {
    let Some(row) = (width as usize).checked_mul(4) else {
        return false;
    };
    width > 0
        && height > 0
        && stride >= row
        && (height as usize)
            .checked_mul(stride)
            .is_some_and(|n| capacity >= n)
}

pub(crate) fn blend(dst: &mut [u8], color: u32, coverage: u8) {
    let alpha = ((color & 255) * coverage as u32 + 127) / 255;
    let inv = 255 - alpha;
    let da = dst[3] as u32;
    let oa = alpha + (da * inv + 127) / 255;
    if oa == 0 {
        return;
    }
    for (i, shift) in [24, 16, 8].into_iter().enumerate() {
        let src = (color >> shift) & 255;
        let num = src * alpha * 255 + dst[i] as u32 * da * inv;
        dst[i] = ((num + oa * 127) / (oa * 255)) as u8;
    }
    dst[3] = oa as u8;
}

/// Draw bounded legacy 5x7 scalar glyphs into caller-owned RGBA storage.
/// Non-ASCII UTF-8 scalars produce one `?`; continuation bytes are skipped.
pub fn scalar_text_draw(
    text: &str,
    clip: Rect,
    font_size_px: u32,
    color: u32,
    rgba: &mut [u8],
    width: u32,
    height: u32,
    stride: usize,
) -> bool {
    if text.len() > 1_048_576
        || font_size_px == 0
        || clip.x > width
        || clip.y > height
        || clip.width > width - clip.x
        || clip.height > height - clip.y
        || !valid_buffer(width, height, rgba.len(), stride)
    {
        return false;
    }
    let scale = (font_size_px / 13 + u32::from(font_size_px % 13 != 0)).clamp(1, 16);
    let glyph_h = 7 * scale;
    let y0 = clip.y + (clip.height.saturating_sub(glyph_h)) / 2;
    let right = clip.x + clip.width;
    let bottom = clip.y + clip.height;
    let mut cursor = clip.x;
    for ch in text.chars().take(1_048_576) {
        let c = if ch.is_ascii() { ch as u8 } else { b'?' };
        if cursor > right || right - cursor < 5 * scale {
            break;
        }
        for row in 0..7 {
            for col in 0..5 {
                if glyph(c, row) & (1 << (4 - col)) == 0 {
                    continue;
                }
                for sy in 0..scale {
                    for sx in 0..scale {
                        let x = cursor + col as u32 * scale + sx;
                        let y = y0 + row as u32 * scale + sy;
                        if x >= clip.x
                            && x < right
                            && y >= clip.y
                            && y < bottom
                            && x < width
                            && y < height
                        {
                            let at = y as usize * stride + x as usize * 4;
                            blend(&mut rgba[at..at + 4], color, 255);
                        }
                    }
                }
            }
        }
        cursor = match cursor.checked_add(6 * scale) {
            Some(v) => v,
            None => break,
        };
    }
    true
}
