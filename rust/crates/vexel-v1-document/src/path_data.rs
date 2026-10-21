//! Validated subset of SVG path data: `M m L l H h V v C c S s Q q T t Z z`.
//! Arcs (`A a`) are intentionally unsupported: scaling them to an equivalent
//! absolute path under non-uniform scale is not implemented.

use crate::{V1DocumentError, numeric_text};

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
        b'S' | b's' | b'Q' | b'q' => Some(4),
        b'T' | b't' => Some(2),
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

/// Reflected control point for `S`/`T`: the previous control point of the
/// same curve kind mirrored about the current point, else the current point.
fn reflect(prev: Option<(u8, f64, f64)>, kind: u8, cur: (f64, f64)) -> (f64, f64) {
    match prev {
        Some((k, px, py)) if k == kind => (2.0 * cur.0 - px, 2.0 * cur.1 - py),
        _ => cur,
    }
}

/// Conservative bounding box (endpoints and control points) in path space.
pub(crate) fn bounds(segments: &[Segment]) -> Option<(f64, f64, f64, f64)> {
    let (mut cx, mut cy, mut sx, mut sy) = (0.0f64, 0.0f64, 0.0f64, 0.0f64);
    let mut bbox: Option<(f64, f64, f64, f64)> = None;
    // Previous curve control point and its kind (b'C' cubic, b'Q' quadratic).
    let mut prev: Option<(u8, f64, f64)> = None;
    let mut include = |x: f64, y: f64| {
        bbox = Some(match bbox {
            None => (x, y, x, y),
            Some((a, b, c, d)) => (a.min(x), b.min(y), c.max(x), d.max(y)),
        });
    };
    for s in segments {
        let relative = s.command.is_ascii_lowercase();
        let (ox, oy) = if relative { (cx, cy) } else { (0.0, 0.0) };
        let before = prev.take();
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
                prev = Some((b'C', ox + s.args[2], oy + s.args[3]));
            }
            b'S' => {
                let (c1x, c1y) = reflect(before, b'C', (cx, cy));
                include(c1x, c1y);
                include(ox + s.args[0], oy + s.args[1]);
                prev = Some((b'C', ox + s.args[0], oy + s.args[1]));
                (cx, cy) = (ox + s.args[2], oy + s.args[3]);
                include(cx, cy);
            }
            b'Q' => {
                include(ox + s.args[0], oy + s.args[1]);
                prev = Some((b'Q', ox + s.args[0], oy + s.args[1]));
                (cx, cy) = (ox + s.args[2], oy + s.args[3]);
                include(cx, cy);
            }
            b'T' => {
                let (qx, qy) = reflect(before, b'Q', (cx, cy));
                include(qx, qy);
                prev = Some((b'Q', qx, qy));
                (cx, cy) = (ox + s.args[0], oy + s.args[1]);
                include(cx, cy);
            }
            _ => (cx, cy) = (sx, sy),
        }
    }
    bbox
}

/// Scales the supported path grammar into an absolute, equivalent path.
pub(crate) fn scale(
    data: &[u8],
    from: (f64, f64, f64, f64),
    to: (f64, f64, f64, f64),
) -> Result<Vec<u8>, V1DocumentError> {
    use std::fmt::Write;
    if from.2 <= from.0 || from.3 <= from.1 {
        return Err(V1DocumentError::InvalidGeometry);
    }
    let segments = parse(data)?;
    let (sx, sy) = (
        (to.2 - to.0) / (from.2 - from.0),
        (to.3 - to.1) / (from.3 - from.1),
    );
    let point = |x: f64, y: f64| (to.0 + (x - from.0) * sx, to.1 + (y - from.1) * sy);
    let mut out = String::new();
    let (mut x, mut y, mut sx0, mut sy0) = (0.0, 0.0, 0.0, 0.0);
    let mut prev: Option<(u8, f64, f64)> = None;
    for s in segments {
        let rel = s.command.is_ascii_lowercase();
        let base = if rel { (x, y) } else { (0.0, 0.0) };
        let before = prev.take();
        match s.command.to_ascii_uppercase() {
            b'M' | b'L' => {
                let p = point(base.0 + s.args[0], base.1 + s.args[1]);
                let cmd = if s.command.to_ascii_uppercase() == b'M' {
                    "M"
                } else {
                    "L"
                };
                write!(&mut out, "{cmd} {} {} ", num(p.0)?, num(p.1)?)
                    .map_err(|_| V1DocumentError::FormattingLimit)?;
                (x, y) = (base.0 + s.args[0], base.1 + s.args[1]);
                if cmd == "M" {
                    (sx0, sy0) = (x, y);
                }
            }
            b'H' => {
                let nx = base.0 + s.args[0];
                let p = point(nx, y);
                write!(&mut out, "L {} {} ", num(p.0)?, num(p.1)?)
                    .map_err(|_| V1DocumentError::FormattingLimit)?;
                x = nx;
            }
            b'V' => {
                let ny = base.1 + s.args[0];
                let p = point(x, ny);
                write!(&mut out, "L {} {} ", num(p.0)?, num(p.1)?)
                    .map_err(|_| V1DocumentError::FormattingLimit)?;
                y = ny;
            }
            b'C' => {
                let a = point(base.0 + s.args[0], base.1 + s.args[1]);
                let b = point(base.0 + s.args[2], base.1 + s.args[3]);
                let end = (base.0 + s.args[4], base.1 + s.args[5]);
                let e = point(end.0, end.1);
                write!(
                    &mut out,
                    "C {} {} {} {} {} {} ",
                    num(a.0)?,
                    num(a.1)?,
                    num(b.0)?,
                    num(b.1)?,
                    num(e.0)?,
                    num(e.1)?
                )
                .map_err(|_| V1DocumentError::FormattingLimit)?;
                (x, y) = end;
                prev = Some((b'C', base.0 + s.args[2], base.1 + s.args[3]));
            }
            b'S' | b'Q' | b'T' => {
                let upper = s.command.to_ascii_uppercase();
                // Normalise to explicit absolute C (from S) or Q (from Q/T).
                let (cmd, ctrl, end_at) = match upper {
                    b'S' => ('C', Some(reflect(before, b'C', (x, y))), 2),
                    b'Q' => ('Q', None, 2),
                    _ => ('Q', Some(reflect(before, b'Q', (x, y))), 0),
                };
                let own = match upper {
                    b'T' => None,
                    _ => Some((base.0 + s.args[0], base.1 + s.args[1])),
                };
                let end = (base.0 + s.args[end_at], base.1 + s.args[end_at + 1]);
                let mut pts: Vec<(f64, f64)> = Vec::new();
                pts.extend(ctrl);
                pts.extend(own);
                pts.push(end);
                write!(&mut out, "{cmd} ").map_err(|_| V1DocumentError::FormattingLimit)?;
                for (px, py) in pts.iter().map(|p| point(p.0, p.1)) {
                    write!(&mut out, "{} {} ", num(px)?, num(py)?)
                        .map_err(|_| V1DocumentError::FormattingLimit)?;
                }
                prev = Some(match upper {
                    b'S' => (b'C', own.map_or(x, |p| p.0), own.map_or(y, |p| p.1)),
                    b'Q' => (b'Q', own.map_or(x, |p| p.0), own.map_or(y, |p| p.1)),
                    _ => (b'Q', ctrl.map_or(x, |p| p.0), ctrl.map_or(y, |p| p.1)),
                });
                (x, y) = end;
            }
            _ => {
                out.push_str("Z ");
                (x, y) = (sx0, sy0);
            }
        }
    }
    validate_path_data(&out)?;
    Ok(out.into_bytes())
}

fn num(value: f64) -> Result<String, V1DocumentError> {
    if value.is_finite() && value.abs() <= MAX_COORDINATE {
        let bytes = numeric_text(value)?;
        String::from_utf8(bytes).map_err(|_| V1DocumentError::InvalidGeometry)
    } else {
        Err(V1DocumentError::InvalidGeometry)
    }
}
