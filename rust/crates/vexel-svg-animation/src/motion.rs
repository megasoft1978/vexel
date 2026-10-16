use crate::control::Meter;
use crate::parse_values::attr;
use crate::{MotionPoint, Track};
use vexel_xml::{TokenKind, XmlSpan, XmlToken};

const MAX_POINTS: usize = 4096;
const MAX_BYTES: usize = 64 * 1024;

pub(crate) fn parse_motion(
    source: &[u8],
    span: XmlSpan,
    track: &mut Track,
    allocation_failed: &mut bool,
    meter: &mut Meter<'_>,
) -> bool {
    let Some(path) = span.slice(source) else {
        return false;
    };
    if path.is_empty() || path.len() > MAX_BYTES {
        return false;
    }
    let mut p = PathReader { b: path, pos: 0 };
    let (mut x, mut y, mut sx, mut sy) = (0.0, 0.0, 0.0, 0.0);
    let mut cmd = 0u8;
    let mut moved = false;
    while {
        p.ws(meter);
        if meter.stop().is_some() {
            return false;
        }
        p.pos < path.len()
    } {
        if !meter.charge(vexel_base::WorkKind::SourceRead, 1) {
            return false;
        }
        let b = path[p.pos];
        if b.is_ascii_alphabetic() {
            cmd = b;
            p.pos += 1;
            if !b"MmLlHhVvZz".contains(&cmd) {
                return false;
            }
            if matches!(cmd, b'M' | b'm') && moved {
                return false;
            }
            if matches!(cmd, b'Z' | b'z') {
                if !moved || !push(track, &mut sx, &mut sy, allocation_failed) {
                    return false;
                }
                x = sx;
                y = sy;
                cmd = 0;
                continue;
            }
        }
        if cmd == 0 || !p.has_num(meter) {
            return false;
        }
        let rel = cmd.is_ascii_lowercase();
        match cmd {
            b'M' | b'm' | b'L' | b'l' => {
                let Some(mut nx) = p.number(meter) else {
                    return false;
                };
                let Some(mut ny) = p.number(meter) else {
                    return false;
                };
                if rel {
                    nx += x;
                    ny += y
                }
                x = nx;
                y = ny;
                if !moved {
                    sx = x;
                    sy = y;
                    moved = true
                }
                if !push_xy(track, x, y, allocation_failed) {
                    return false;
                }
                if cmd == b'M' {
                    cmd = b'L'
                } else if cmd == b'm' {
                    cmd = b'l'
                }
            }
            b'H' | b'h' => {
                let Some(nx) = p.number(meter) else {
                    return false;
                };
                x = if rel { x + nx } else { nx };
                if !moved || !push_xy(track, x, y, allocation_failed) {
                    return false;
                }
            }
            b'V' | b'v' => {
                let Some(ny) = p.number(meter) else {
                    return false;
                };
                y = if rel { y + ny } else { ny };
                if !moved || !push_xy(track, x, y, allocation_failed) {
                    return false;
                }
            }
            _ => return false,
        }
    }
    moved && track.motion_points.len() >= 2 && track.motion_length > 0.0
}
struct PathReader<'a> {
    b: &'a [u8],
    pos: usize,
}
impl PathReader<'_> {
    fn ws(&mut self, meter: &mut Meter<'_>) {
        while self
            .b
            .get(self.pos)
            .is_some_and(|b| b" \t\r\n,".contains(b))
        {
            if !meter.charge(vexel_base::WorkKind::SourceRead, 1) {
                return;
            }
            self.pos += 1
        }
    }
    fn has_num(&mut self, meter: &mut Meter<'_>) -> bool {
        self.ws(meter);
        let Some(byte) = self.b.get(self.pos) else {
            return false;
        };
        if !meter.charge(vexel_base::WorkKind::SourceRead, 1) {
            return false;
        }
        byte.is_ascii_digit() || matches!(byte, b'+' | b'-' | b'.')
    }
    fn number(&mut self, meter: &mut Meter<'_>) -> Option<f64> {
        self.ws(meter);
        let start = self.pos;
        if matches!(self.b.get(self.pos), Some(b'+' | b'-')) {
            if !meter.charge(vexel_base::WorkKind::SourceRead, 1) {
                return None;
            }
            self.pos += 1
        }
        let mut digit = false;
        while self.b.get(self.pos).is_some_and(u8::is_ascii_digit) {
            if !meter.charge(vexel_base::WorkKind::SourceRead, 1) {
                return None;
            }
            digit = true;
            self.pos += 1
        }
        if self.b.get(self.pos) == Some(&b'.') {
            if !meter.charge(vexel_base::WorkKind::SourceRead, 1) {
                return None;
            }
            self.pos += 1;
            while self.b.get(self.pos).is_some_and(u8::is_ascii_digit) {
                if !meter.charge(vexel_base::WorkKind::SourceRead, 1) {
                    return None;
                }
                digit = true;
                self.pos += 1
            }
        }
        if !digit {
            return None;
        }
        if matches!(self.b.get(self.pos), Some(b'e' | b'E')) {
            if !meter.charge(vexel_base::WorkKind::SourceRead, 1) {
                return None;
            }
            self.pos += 1;
            if matches!(self.b.get(self.pos), Some(b'+' | b'-')) {
                if !meter.charge(vexel_base::WorkKind::SourceRead, 1) {
                    return None;
                }
                self.pos += 1
            }
            let e = self.pos;
            while self.b.get(self.pos).is_some_and(u8::is_ascii_digit) {
                if !meter.charge(vexel_base::WorkKind::SourceRead, 1) {
                    return None;
                }
                self.pos += 1
            }
            if e == self.pos {
                return None;
            }
        }
        if self.pos - start >= 128 {
            return None;
        }
        let n = std::str::from_utf8(&self.b[start..self.pos])
            .ok()?
            .parse::<f64>()
            .ok()?;
        if n.is_finite() && n.abs() <= 1_000_000.0 {
            Some(n)
        } else {
            None
        }
    }
}
fn push(track: &mut Track, x: &mut f64, y: &mut f64, allocation_failed: &mut bool) -> bool {
    push_xy(track, *x, *y, allocation_failed)
}
fn push_xy(track: &mut Track, x: f64, y: f64, allocation_failed: &mut bool) -> bool {
    if !x.is_finite()
        || !y.is_finite()
        || x.abs() > 1e6
        || y.abs() > 1e6
        || track.motion_points.len() >= MAX_POINTS
    {
        return false;
    }
    let d = track
        .motion_points
        .last()
        .map_or(0.0, |p| p.distance + (x - p.x).hypot(y - p.y));
    if !d.is_finite() || d > 1e9 {
        return false;
    }
    if track.motion_points.try_reserve(1).is_err() {
        *allocation_failed = true;
        return false;
    }
    track.motion_points.push(MotionPoint { x, y, distance: d });
    track.motion_length = d;
    true
}

pub(crate) fn resolve_motion_path(
    source: &[u8],
    tokens: &[XmlToken],
    animation: usize,
    meter: &mut Meter<'_>,
) -> Option<XmlSpan> {
    if let Some(direct) = attr(source, tokens, animation, b"path", meter) {
        return Some(direct);
    }
    let depth = tokens.get(animation)?.depth;
    let mut mp = None;
    for (i, t) in tokens.iter().enumerate().skip(animation + 1) {
        if !meter.charge(vexel_base::WorkKind::ReferenceProbe, 1) {
            return None;
        }
        if t.kind == TokenKind::EndElement && t.depth == depth {
            break;
        }
        if t.depth == depth + 1
            && matches!(t.kind, TokenKind::StartElement | TokenKind::EmptyElement)
            && t.prefix.length == 0
            && span_eq(source, t.local_name, b"mpath", meter)?
        {
            mp = Some(i);
            break;
        }
    }
    let m = mp?;
    let mut href = None;
    for t in tokens.iter().skip(m + 1) {
        if !meter.charge(vexel_base::WorkKind::TokenVisit, 1) {
            return None;
        }
        if t.kind != TokenKind::Attribute {
            break;
        }
        if span_eq(source, t.local_name, b"href", meter)?
            && (t.prefix.length == 0 || span_eq(source, t.prefix, b"xlink", meter)?)
        {
            href = Some(t.value);
            break;
        }
    }
    let href = href?;
    let value = href.slice(source)?;
    if value.len() < 2 || value[0] != b'#' {
        return None;
    }
    let id = &value[1..];
    let mut matching_paths = 0usize;
    let mut matched_path = None;
    for (i, t) in tokens.iter().enumerate() {
        if !meter.charge(vexel_base::WorkKind::ReferenceProbe, 1) {
            return None;
        }
        if !matches!(t.kind, TokenKind::StartElement | TokenKind::EmptyElement)
            || t.prefix.length != 0
            || !span_eq(source, t.local_name, b"path", meter)?
        {
            continue;
        }
        if let Some(path_id) = attr(source, tokens, i, b"id", meter) {
            if !slice_eq(path_id.slice(source)?, id, meter)? {
                continue;
            }
            matching_paths += 1;
            matched_path = attr(source, tokens, i, b"d", meter);
        }
    }
    if matching_paths == 1 {
        matched_path
    } else {
        None
    }
}
fn span_eq(source: &[u8], span: XmlSpan, target: &[u8], meter: &mut Meter<'_>) -> Option<bool> {
    slice_eq(span.slice(source)?, target, meter)
}
fn slice_eq(bytes: &[u8], target: &[u8], meter: &mut Meter<'_>) -> Option<bool> {
    if bytes.len() != target.len() {
        return Some(false);
    }
    for (left, right) in bytes.iter().zip(target) {
        if !meter.charge(vexel_base::WorkKind::SourceRead, 1) {
            return None;
        }
        if left != right {
            return Some(false);
        }
    }
    Some(true)
}
