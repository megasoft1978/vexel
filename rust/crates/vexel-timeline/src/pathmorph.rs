//! Path data for morphing tracks (`d` of a path, `points` of a polygon or
//! polyline).
//!
//! A key's text is normalised to absolute `M`/`L`/`C`/`Q`/`Z` commands (arcs
//! are unsupported). Two keys can morph when their command kinds are
//! identical; the interpolated value is the per-coordinate blend of the
//! control points.

/// Which attribute the path text belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathKind {
    /// Path data of a `d` attribute.
    Data,
    /// Coordinate pairs of a `points` attribute.
    Points,
}

impl PathKind {
    /// The kind a property name morphs, `None` for other properties.
    pub fn of_property(property: &[u8]) -> Option<Self> {
        match property {
            b"d" => Some(Self::Data),
            b"points" => Some(Self::Points),
            _ => None,
        }
    }
}

/// A normalised path: one kind byte per command and its absolute coordinates.
#[derive(Debug, Clone, PartialEq)]
pub struct Shape {
    pub kinds: Vec<u8>,
    pub coords: Vec<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathErrorKind {
    /// Not valid path data (or an empty path).
    Syntax,
    /// An elliptical arc command, which cannot be morphed.
    ArcUnsupported,
    /// The command at the index differs from the base path.
    CommandMismatch { expected: u8, found: u8 },
    /// One path ends (after `expected` or `found` commands) before the other.
    CountMismatch { expected: usize, found: usize },
    /// A coordinate is not finite or exceeds 1e9 in magnitude.
    Range,
}

/// Typed path error; `index` is the zero based command (or point) it names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PathError {
    pub index: usize,
    pub kind: PathErrorKind,
}

impl PathError {
    pub fn message(&self) -> String {
        let at = self.index + 1;
        match self.kind {
            PathErrorKind::Syntax => format!("invalid path data near command {at}"),
            PathErrorKind::ArcUnsupported => {
                format!("command {at} is an arc, which cannot be morphed; convert it to C curves")
            }
            PathErrorKind::CommandMismatch { expected, found } => format!(
                "command {at} is {} but the base path has {} there",
                found as char, expected as char
            ),
            PathErrorKind::CountMismatch { expected, found } => format!(
                "path has {found} commands but the base path has {expected} (first difference at command {at})"
            ),
            PathErrorKind::Range => format!("coordinate in command {at} is out of range"),
        }
    }
}

const LIMIT: f64 = 1e9;
const MAX_COMMANDS: usize = 4096;

struct Scanner<'a> {
    text: &'a [u8],
    at: usize,
}

impl Scanner<'_> {
    fn skip(&mut self) {
        while self
            .text
            .get(self.at)
            .is_some_and(|b| matches!(b, b' ' | b'\t' | b'\n' | b'\r' | b','))
        {
            self.at += 1;
        }
    }
    fn done(&mut self) -> bool {
        self.skip();
        self.at >= self.text.len()
    }
    fn peek(&mut self) -> Option<u8> {
        self.skip();
        self.text.get(self.at).copied()
    }
    fn number(&mut self) -> Option<f64> {
        self.skip();
        let start = self.at;
        let t = self.text;
        let mut i = start;
        if matches!(t.get(i), Some(b'+' | b'-')) {
            i += 1;
        }
        let digits = i;
        while t.get(i).is_some_and(u8::is_ascii_digit) {
            i += 1;
        }
        if t.get(i) == Some(&b'.') {
            i += 1;
            while t.get(i).is_some_and(u8::is_ascii_digit) {
                i += 1;
            }
        }
        if i == digits || (i == digits + 1 && t[digits] == b'.') {
            return None;
        }
        if matches!(t.get(i), Some(b'e' | b'E')) {
            let mut j = i + 1;
            if matches!(t.get(j), Some(b'+' | b'-')) {
                j += 1;
            }
            if t.get(j).is_some_and(u8::is_ascii_digit) {
                while t.get(j).is_some_and(u8::is_ascii_digit) {
                    j += 1;
                }
                i = j;
            }
        }
        let value: f64 = std::str::from_utf8(&t[start..i]).ok()?.parse().ok()?;
        self.at = i;
        Some(value)
    }
}

fn err(index: usize, kind: PathErrorKind) -> PathError {
    PathError { index, kind }
}

/// Parses and normalises `text`.
pub fn parse(kind: PathKind, text: &[u8]) -> Result<Shape, PathError> {
    match kind {
        PathKind::Data => parse_data(text),
        PathKind::Points => parse_points(text),
    }
}

fn parse_points(text: &[u8]) -> Result<Shape, PathError> {
    let mut scan = Scanner { text, at: 0 };
    let mut coords = Vec::new();
    while !scan.done() {
        let v = scan.number().ok_or(err(coords.len() / 2, PathErrorKind::Syntax))?;
        if !v.is_finite() || v.abs() > LIMIT {
            return Err(err(coords.len() / 2, PathErrorKind::Range));
        }
        coords.push(v);
        if coords.len() > MAX_COMMANDS * 2 {
            return Err(err(MAX_COMMANDS, PathErrorKind::Syntax));
        }
    }
    if coords.is_empty() || coords.len() % 2 != 0 {
        return Err(err(coords.len() / 2, PathErrorKind::Syntax));
    }
    Ok(Shape {
        kinds: vec![b'P'; coords.len() / 2],
        coords,
    })
}

fn parse_data(text: &[u8]) -> Result<Shape, PathError> {
    let mut scan = Scanner { text, at: 0 };
    let mut out = Shape {
        kinds: Vec::new(),
        coords: Vec::new(),
    };
    let (mut cx, mut cy, mut sx, mut sy) = (0.0f64, 0.0f64, 0.0f64, 0.0f64);
    // Previous cubic or quadratic control point, for S and T reflection.
    let mut cubic: Option<(f64, f64)> = None;
    let mut quad: Option<(f64, f64)> = None;
    let mut command = 0u8;
    let mut first = true;
    while !scan.done() {
        let index = out.kinds.len();
        let bad = err(index, PathErrorKind::Syntax);
        let next = scan.peek().ok_or(bad)?;
        if next.is_ascii_alphabetic() {
            scan.at += 1;
            command = next;
            if matches!(command, b'A' | b'a') {
                return Err(err(index, PathErrorKind::ArcUnsupported));
            }
            if !b"MmLlHhVvCcSsQqTtZz".contains(&command) {
                return Err(bad);
            }
            if first && !matches!(command, b'M' | b'm') {
                return Err(bad);
            }
            if matches!(command, b'Z' | b'z') {
                out.kinds.push(b'Z');
                cx = sx;
                cy = sy;
                cubic = None;
                quad = None;
                continue;
            }
        } else if command == 0 || matches!(command, b'Z' | b'z') {
            return Err(bad);
        }
        first = false;
        if out.kinds.len() >= MAX_COMMANDS {
            return Err(err(MAX_COMMANDS, PathErrorKind::Syntax));
        }
        let rel = command.is_ascii_lowercase();
        let (ox, oy) = if rel { (cx, cy) } else { (0.0, 0.0) };
        let n = |s: &mut Scanner| s.number().ok_or(bad);
        match command.to_ascii_uppercase() {
            b'M' | b'L' => {
                let (x, y) = (n(&mut scan)? + ox, n(&mut scan)? + oy);
                let kind = if command.to_ascii_uppercase() == b'M' { b'M' } else { b'L' };
                out.kinds.push(kind);
                out.coords.extend([x, y]);
                if kind == b'M' {
                    sx = x;
                    sy = y;
                }
                cx = x;
                cy = y;
                // Further pairs after M are implicit L.
                command = if command == b'M' {
                    b'L'
                } else if command == b'm' {
                    b'l'
                } else {
                    command
                };
                cubic = None;
                quad = None;
            }
            b'H' => {
                let x = n(&mut scan)? + if rel { cx } else { 0.0 };
                out.kinds.push(b'L');
                out.coords.extend([x, cy]);
                cx = x;
                cubic = None;
                quad = None;
            }
            b'V' => {
                let y = n(&mut scan)? + if rel { cy } else { 0.0 };
                out.kinds.push(b'L');
                out.coords.extend([cx, y]);
                cy = y;
                cubic = None;
                quad = None;
            }
            b'C' => {
                let mut p = [0.0f64; 6];
                for (i, slot) in p.iter_mut().enumerate() {
                    *slot = n(&mut scan)? + if i % 2 == 0 { ox } else { oy };
                }
                out.kinds.push(b'C');
                out.coords.extend(p);
                cubic = Some((p[2], p[3]));
                quad = None;
                cx = p[4];
                cy = p[5];
            }
            b'S' => {
                let mut p = [0.0f64; 4];
                for (i, slot) in p.iter_mut().enumerate() {
                    *slot = n(&mut scan)? + if i % 2 == 0 { ox } else { oy };
                }
                let (rx, ry) = cubic.map_or((cx, cy), |(x, y)| (2.0 * cx - x, 2.0 * cy - y));
                out.kinds.push(b'C');
                out.coords.extend([rx, ry, p[0], p[1], p[2], p[3]]);
                cubic = Some((p[0], p[1]));
                quad = None;
                cx = p[2];
                cy = p[3];
            }
            b'Q' => {
                let mut p = [0.0f64; 4];
                for (i, slot) in p.iter_mut().enumerate() {
                    *slot = n(&mut scan)? + if i % 2 == 0 { ox } else { oy };
                }
                out.kinds.push(b'Q');
                out.coords.extend(p);
                quad = Some((p[0], p[1]));
                cubic = None;
                cx = p[2];
                cy = p[3];
            }
            _ => {
                let (x, y) = (n(&mut scan)? + ox, n(&mut scan)? + oy);
                let (rx, ry) = quad.map_or((cx, cy), |(qx, qy)| (2.0 * cx - qx, 2.0 * cy - qy));
                out.kinds.push(b'Q');
                out.coords.extend([rx, ry, x, y]);
                quad = Some((rx, ry));
                cubic = None;
                cx = x;
                cy = y;
            }
        }
    }
    if out.kinds.is_empty() {
        return Err(err(0, PathErrorKind::Syntax));
    }
    if out.coords.iter().any(|v| !v.is_finite() || v.abs() > LIMIT) {
        return Err(err(0, PathErrorKind::Range));
    }
    Ok(out)
}

/// Whether `other` can morph with `base`; names the first difference.
pub fn compare(base: &Shape, other: &Shape) -> Result<(), PathError> {
    for (index, (a, b)) in base.kinds.iter().zip(&other.kinds).enumerate() {
        if a != b {
            return Err(err(
                index,
                PathErrorKind::CommandMismatch {
                    expected: *a,
                    found: *b,
                },
            ));
        }
    }
    if base.kinds.len() != other.kinds.len() {
        return Err(err(
            base.kinds.len().min(other.kinds.len()),
            PathErrorKind::CountMismatch {
                expected: base.kinds.len(),
                found: other.kinds.len(),
            },
        ));
    }
    Ok(())
}

/// Per-coordinate blend `a + (b - a) * t` of two shapes of equal structure.
pub fn lerp(a: &Shape, b: &Shape, t: f64) -> Shape {
    Shape {
        kinds: a.kinds.clone(),
        coords: a
            .coords
            .iter()
            .zip(&b.coords)
            .map(|(x, y)| x + (y - x) * t)
            .collect(),
    }
}

/// Attribute text for `shape`.
pub fn format(kind: PathKind, shape: &Shape) -> Vec<u8> {
    let mut out = String::new();
    let mut at = 0usize;
    for (i, k) in shape.kinds.iter().enumerate() {
        if i != 0 {
            out.push(' ');
        }
        let count = match k {
            b'M' | b'L' | b'P' => 2,
            b'C' => 6,
            b'Q' => 4,
            _ => 0,
        };
        if *k != b'P' {
            out.push(*k as char);
        }
        for (j, v) in shape.coords[at..at + count].iter().enumerate() {
            if j != 0 {
                out.push(if kind == PathKind::Points && j == 1 { ',' } else { ' ' });
            } else if *k != b'P' {
                out.push(' ');
            }
            out.push_str(&v.to_string());
        }
        at += count;
    }
    out.into_bytes()
}

#[cfg(test)]
#[path = "pathmorph_tests.rs"]
mod tests;
