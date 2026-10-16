//! Validated subset of SVG path data: `M m L l H h V v C c Z z`.

use crate::V1DocumentError;

/// Maximum accepted `d` length in bytes.
pub const MAX_PATH_DATA_BYTES: usize = 8 * 1024;
/// Maximum number of segments, counting implicit repeats.
pub const MAX_PATH_SEGMENTS: usize = 1024;
/// Largest accepted coordinate magnitude (matches `vexel-svg-path`).
const MAX_COORDINATE: f64 = 1_000_000.0;

#[derive(Clone, Copy, Debug)]
pub(crate) struct Segment {
    pub(crate) command: u8,
    pub(crate) args: [f64; 6],
}

fn arity(command: u8) -> Option<usize> {
    match command {
        b'M' | b'm' | b'L' | b'l' => Some(2),
        b'H' | b'h' | b'V' | b'v' => Some(1),
        b'C' | b'c' => Some(6),
        b'Z' | b'z' => Some(0),
        _ => None,
    }
}

fn is_space(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\r' | b'\n')
}

fn scan_number(bytes: &[u8], at: &mut usize) -> Option<f64> {
    let start = *at;
    let mut i = start;
    if matches!(bytes.get(i), Some(b'+' | b'-')) {
        i += 1;
    }
    let mut digits = 0usize;
    while bytes.get(i).is_some_and(u8::is_ascii_digit) {
        i += 1;
        digits += 1;
    }
    if bytes.get(i) == Some(&b'.') {
        i += 1;
        while bytes.get(i).is_some_and(u8::is_ascii_digit) {
            i += 1;
            digits += 1;
        }
    }
    if digits == 0 {
        return None;
    }
    if matches!(bytes.get(i), Some(b'e' | b'E')) {
        i += 1;
        if matches!(bytes.get(i), Some(b'+' | b'-')) {
            i += 1;
        }
        let exponent = i;
        while bytes.get(i).is_some_and(u8::is_ascii_digit) {
            i += 1;
        }
        if i == exponent {
            return None;
        }
    }
    let value: f64 = std::str::from_utf8(&bytes[start..i]).ok()?.parse().ok()?;
    if !value.is_finite() || value.abs() > MAX_COORDINATE {
        return None;
    }
    *at = i;
    Some(value)
}

fn starts_number(byte: Option<&u8>) -> bool {
    byte.is_some_and(|b| b.is_ascii_digit() || matches!(b, b'+' | b'-' | b'.'))
}

/// Parses path data into at most [`MAX_PATH_SEGMENTS`] segments.
pub(crate) fn parse(data: &[u8]) -> Result<Vec<Segment>, V1DocumentError> {
    let bad = V1DocumentError::InvalidPathData;
    if data.is_empty() || data.len() > MAX_PATH_DATA_BYTES {
        return Err(bad);
    }
    let mut segments: Vec<Segment> = Vec::new();
    let mut at = 0usize;
    // Command that a bare number group repeats; `None` after Z and at the start.
    let mut repeat: Option<u8> = None;
    loop {
        while data.get(at).is_some_and(|b| is_space(*b)) {
            at += 1;
        }
        if at == data.len() {
            break;
        }
        let command = if arity(data[at]).is_some() {
            let command = data[at];
            at += 1;
            command
        } else if starts_number(data.get(at)) {
            match repeat {
                Some(b'M') => b'L',
                Some(b'm') => b'l',
                Some(other) => other,
                None => return Err(bad),
            }
        } else {
            return Err(bad);
        };
        if segments.is_empty() && !matches!(command, b'M' | b'm') {
            return Err(bad);
        }
        if segments.len() == MAX_PATH_SEGMENTS {
            return Err(bad);
        }
        let count = arity(command).ok_or(bad)?;
        let mut args = [0.0; 6];
        for (k, slot) in args.iter_mut().enumerate().take(count) {
            while data.get(at).is_some_and(|b| is_space(*b)) {
                at += 1;
            }
            if k > 0 && data.get(at) == Some(&b',') {
                at += 1;
                while data.get(at).is_some_and(|b| is_space(*b)) {
                    at += 1;
                }
            }
            *slot = scan_number(data, &mut at).ok_or(bad)?;
        }
        segments
            .try_reserve(1)
            .map_err(|_| V1DocumentError::Allocation)?;
        segments.push(Segment { command, args });
        repeat = if matches!(command, b'Z' | b'z') {
            None
        } else {
            Some(command)
        };
        // A comma may separate repeated argument groups.
        if repeat.is_some() {
            let mut peek = at;
            while data.get(peek).is_some_and(|b| is_space(*b)) {
                peek += 1;
            }
            if data.get(peek) == Some(&b',') {
                peek += 1;
                while data.get(peek).is_some_and(|b| is_space(*b)) {
                    peek += 1;
                }
                if !starts_number(data.get(peek)) {
                    return Err(bad);
                }
                at = peek;
            }
        }
    }
    if segments.is_empty() {
        return Err(bad);
    }
    Ok(segments)
}

/// Validates path data against the supported grammar and limits.
pub fn validate_path_data(data: &str) -> Result<(), V1DocumentError> {
    parse(data.as_bytes()).map(|_| ())
}

/// Conservative bounding box (endpoints and control points) in path space.
pub(crate) fn bounds(segments: &[Segment]) -> Option<(f64, f64, f64, f64)> {
    let (mut cx, mut cy, mut sx, mut sy) = (0.0f64, 0.0f64, 0.0f64, 0.0f64);
    let mut bbox: Option<(f64, f64, f64, f64)> = None;
    let mut include = |x: f64, y: f64| {
        bbox = Some(match bbox {
            None => (x, y, x, y),
            Some((a, b, c, d)) => (a.min(x), b.min(y), c.max(x), d.max(y)),
        });
    };
    for s in segments {
        let relative = s.command.is_ascii_lowercase();
        let (ox, oy) = if relative { (cx, cy) } else { (0.0, 0.0) };
        match s.command.to_ascii_uppercase() {
            b'M' => {
                (cx, cy) = (ox + s.args[0], oy + s.args[1]);
                (sx, sy) = (cx, cy);
                include(cx, cy);
            }
            b'L' => {
                (cx, cy) = (ox + s.args[0], oy + s.args[1]);
                include(cx, cy);
            }
            b'H' => {
                cx = ox + s.args[0];
                include(cx, cy);
            }
            b'V' => {
                cy = oy + s.args[0];
                include(cx, cy);
            }
            b'C' => {
                include(ox + s.args[0], oy + s.args[1]);
                include(ox + s.args[2], oy + s.args[3]);
                (cx, cy) = (ox + s.args[4], oy + s.args[5]);
                include(cx, cy);
            }
            _ => (cx, cy) = (sx, sy),
        }
    }
    bbox
}
