use crate::control::Meter;
use crate::*;
use vexel_base::WorkKind;
use vexel_xml::{TokenKind, XmlSpan, XmlToken};

pub(crate) const MAX_PATH_BYTES: usize = 64 * 1024;
pub(crate) const MAX_PATH_NUMBERS: usize = 4096;

pub(crate) fn span(source: &[u8], s: XmlSpan) -> Option<&[u8]> {
    s.slice(source)
}
pub(crate) fn eq(source: &[u8], s: XmlSpan, b: &[u8], meter: &mut Meter<'_>) -> bool {
    let Some(bytes) = span(source, s) else {
        return false;
    };
    if bytes.len() != b.len() {
        return false;
    }
    for (left, right) in bytes.iter().zip(b) {
        if !meter.charge(WorkKind::SourceRead, 1) {
            return false;
        }
        if left != right {
            return false;
        }
    }
    true
}
pub(crate) fn attr(
    source: &[u8],
    tokens: &[XmlToken],
    e: usize,
    name: &[u8],
    meter: &mut Meter<'_>,
) -> Option<XmlSpan> {
    tokens.get(e)?;
    for a in tokens.iter().skip(e + 1) {
        if !meter.charge(WorkKind::TokenVisit, 1) {
            return None;
        }
        if a.kind != TokenKind::Attribute {
            break;
        }
        if a.prefix.length == 0 && eq(source, a.local_name, name, meter) {
            return Some(a.value);
        }
        if meter.stop().is_some() {
            return None;
        }
    }
    None
}
pub(crate) fn attr0(
    source: &[u8],
    tokens: &[XmlToken],
    e: usize,
    name: &[u8],
    meter: &mut Meter<'_>,
) -> XmlSpan {
    attr(source, tokens, e, name, meter).unwrap_or_default()
}
pub(crate) fn number(source: &[u8], s: XmlSpan, meter: &mut Meter<'_>) -> Option<f64> {
    let b = span(source, s)?;
    if b.is_empty() || b.len() >= 128 {
        return None;
    }
    let mut start = 0;
    while start < b.len() && b[start].is_ascii_whitespace() {
        if !meter.charge(WorkKind::SourceRead, 1) {
            return None;
        }
        start += 1;
    }
    let b = &b[start..];
    for _byte in b {
        if !meter.charge(WorkKind::SourceRead, 1) {
            return None;
        }
    }
    let t = std::str::from_utf8(b).ok()?;
    let n = t.parse::<f64>().ok()?;
    n.is_finite().then_some(n)
}
pub(crate) fn time(source: &[u8], s: XmlSpan, meter: &mut Meter<'_>) -> Option<f64> {
    let b = span(source, s)?;
    if b.is_empty() || b.len() >= 128 {
        return None;
    }
    let suffix_len = if b.ends_with(b"ms") {
        2
    } else if b.ends_with(b"s") {
        1
    } else {
        0
    };
    if !meter.charge(WorkKind::SourceRead, suffix_len) {
        return None;
    }
    let (body, mul) = if b.ends_with(b"ms") {
        (&b[..b.len() - 2], 0.001)
    } else if b.ends_with(b"s") {
        (&b[..b.len() - 1], 1.0)
    } else {
        (b, 1.0)
    };
    let mut start = 0;
    while start < body.len() && body[start].is_ascii_whitespace() {
        if !meter.charge(WorkKind::SourceRead, 1) {
            return None;
        }
        start += 1;
    }
    for _byte in &body[start..] {
        if !meter.charge(WorkKind::SourceRead, 1) {
            return None;
        }
    }
    let n = std::str::from_utf8(&body[start..])
        .ok()?
        .parse::<f64>()
        .ok()?;
    let v = n * mul;
    if n.is_finite() && n >= 0.0 && v.is_finite() && v <= MAX_TIMELINE_SECONDS {
        Some(v)
    } else {
        None
    }
}
pub(crate) fn diagnostic(d: &mut Document<'_>, kind: DiagnosticKind, offset: usize) {
    if d.diagnostics.len() < MAX_DIAGNOSTICS {
        d.diagnostics.push(Diagnostic { kind, offset })
    }
}
pub(crate) fn element_span(
    tokens: &[XmlToken],
    i: usize,
    meter: &mut Meter<'_>,
) -> Option<XmlSpan> {
    let start = tokens.get(i)?.source.offset;
    let mut end = start.checked_add(tokens[i].source.length)?;
    if tokens[i].kind == TokenKind::StartElement {
        let depth = tokens[i].depth;
        let mut close = None;
        for token in tokens.iter().skip(i + 1) {
            if !meter.charge(WorkKind::TokenVisit, 1) {
                return None;
            }
            if token.kind == TokenKind::EndElement && token.depth == depth {
                close = Some(token);
                break;
            }
        }
        let close = close?;
        end = close.source.offset.checked_add(close.source.length)?;
    }
    Some(XmlSpan {
        offset: start,
        length: end.checked_sub(start)?,
    })
}
pub(crate) fn attr_kind(source: &[u8], s: XmlSpan, meter: &mut Meter<'_>) -> (Attribute, bool) {
    let b = span(source, s).unwrap_or_default();
    for (name, attribute) in [
        (b"x".as_slice(), Attribute::X),
        (b"y", Attribute::Y),
        (b"width", Attribute::Width),
        (b"height", Attribute::Height),
        (b"cx", Attribute::Cx),
        (b"cy", Attribute::Cy),
        (b"r", Attribute::R),
        (b"rx", Attribute::Rx),
        (b"ry", Attribute::Ry),
        (b"opacity", Attribute::Opacity),
        (b"fill-opacity", Attribute::FillOpacity),
        (b"stroke-opacity", Attribute::StrokeOpacity),
        (b"fill", Attribute::Fill),
        (b"stroke", Attribute::Stroke),
        (b"stroke-width", Attribute::StrokeWidth),
        (b"stroke-dashoffset", Attribute::StrokeDashoffset),
        (b"d", Attribute::D),
        (b"transform", Attribute::Transform),
    ] {
        if source_eq(b, name, meter) == Some(true) {
            return (attribute, attribute == Attribute::Transform);
        }
    }
    (Attribute::Motion, false)
}
pub(crate) fn color(source: &[u8], s: XmlSpan, meter: &mut Meter<'_>) -> Option<[u8; 4]> {
    let b = span(source, s)?;
    if source_eq(b, b"none", meter)? {
        return Some([0; 4]);
    }
    if b.len() == 4 && b[0] == b'#' {
        if !meter.charge(WorkKind::SourceRead, 1) {
            return None;
        }
        let mut c = [0, 0, 0, 255];
        for i in 0..3 {
            if !meter.charge(WorkKind::SourceRead, 1) {
                return None;
            }
            c[i] = hex(b[i + 1])?.checked_mul(17)?
        }
        return Some(c);
    }
    if (b.len() == 7 || b.len() == 9) && b[0] == b'#' {
        if !meter.charge(WorkKind::SourceRead, 1) {
            return None;
        }
        let mut c = [0, 0, 0, 255];
        let n = if b.len() == 9 { 4 } else { 3 };
        for i in 0..n {
            if !meter.charge(WorkKind::SourceRead, 1) {
                return None;
            }
            if !meter.charge(WorkKind::SourceRead, 1) {
                return None;
            }
            c[i] = hex(b[1 + i * 2])?
                .checked_mul(16)?
                .checked_add(hex(b[2 + i * 2])?)?
        }
        return Some(c);
    }
    for (word, rgba) in [
        (b"black".as_slice(), [0, 0, 0, 255]),
        (b"white", [255, 255, 255, 255]),
        (b"red", [255, 0, 0, 255]),
        (b"green", [0, 128, 0, 255]),
        (b"blue", [0, 0, 255, 255]),
        (b"yellow", [255, 255, 0, 255]),
        (b"cyan", [0, 255, 255, 255]),
        (b"magenta", [255, 0, 255, 255]),
        (b"gray", [128, 128, 128, 255]),
        (b"grey", [128, 128, 128, 255]),
        (b"transparent", [0, 0, 0, 0]),
    ] {
        if source_eq(b, word, meter)? {
            return Some(rgba);
        }
    }
    None
}
fn source_eq(left: &[u8], right: &[u8], meter: &mut Meter<'_>) -> Option<bool> {
    if left.len() != right.len() {
        return Some(false);
    }
    for (a, b) in left.iter().zip(right) {
        if !meter.charge(WorkKind::SourceRead, 1) {
            return None;
        }
        if a != b {
            return Some(false);
        }
    }
    Some(true)
}
pub(crate) fn hex(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}
pub(crate) fn parse_value(
    source: &[u8],
    s: XmlSpan,
    a: Attribute,
    k: Kind,
    tt: TransformType,
    meter: &mut Meter<'_>,
) -> Option<Value> {
    let mut v = Value {
        raw: s,
        ..Value::default()
    };
    if k == Kind::Motion {
        v.kind = ValueKind::Motion;
        return Some(v);
    }
    if k == Kind::Transform {
        let b = span(source, s)?;
        if b.is_empty() || b.len() > 256 {
            return None;
        }
        let (vals, count) = number_list(b, 4, meter)?;
        let (min, max) = match tt {
            TransformType::Rotate => (1, 3),
            TransformType::Translate | TransformType::Scale => (1, 2),
            _ => (1, 1),
        };
        if count < min || count > max {
            return None;
        }
        v.kind = ValueKind::Transform;
        v.number[..count].copy_from_slice(&vals[..count]);
        v.number[3] = count as f64;
        return Some(v);
    }
    if a == Attribute::D {
        v.kind = ValueKind::Path;
        return Some(v);
    }
    if matches!(a, Attribute::Fill | Attribute::Stroke) {
        v.kind = ValueKind::Color;
        v.rgba = color(source, s, meter)?;
        return Some(v);
    }
    v.kind = ValueKind::Number;
    v.number[0] = number(source, s, meter)?;
    Some(v)
}
pub(crate) fn number_list(
    bytes: &[u8],
    limit: usize,
    meter: &mut Meter<'_>,
) -> Option<([f64; 4], usize)> {
    let mut values = [0.0; 4];
    let mut count = 0;
    let mut pos = 0;
    while pos < bytes.len() {
        while bytes
            .get(pos)
            .is_some_and(|b| b.is_ascii_whitespace() || *b == b',')
        {
            if !meter.charge(WorkKind::SourceRead, 1) {
                return None;
            }
            pos += 1;
        }
        if pos == bytes.len() {
            break;
        }
        if count >= limit || count >= values.len() {
            return None;
        }
        let start = pos;
        if matches!(bytes.get(pos), Some(b'+' | b'-')) {
            if !meter.charge(WorkKind::SourceRead, 1) {
                return None;
            }
            pos += 1;
        }
        let mut digits = false;
        while bytes.get(pos).is_some_and(u8::is_ascii_digit) {
            if !meter.charge(WorkKind::SourceRead, 1) {
                return None;
            }
            digits = true;
            pos += 1;
        }
        if bytes.get(pos) == Some(&b'.') {
            if !meter.charge(WorkKind::SourceRead, 1) {
                return None;
            }
            pos += 1;
            while bytes.get(pos).is_some_and(u8::is_ascii_digit) {
                if !meter.charge(WorkKind::SourceRead, 1) {
                    return None;
                }
                digits = true;
                pos += 1;
            }
        }
        if !digits {
            return None;
        }
        if matches!(bytes.get(pos), Some(b'e' | b'E')) {
            if !meter.charge(WorkKind::SourceRead, 1) {
                return None;
            }
            pos += 1;
            if matches!(bytes.get(pos), Some(b'+' | b'-')) {
                if !meter.charge(WorkKind::SourceRead, 1) {
                    return None;
                }
                pos += 1;
            }
            let exp = pos;
            while bytes.get(pos).is_some_and(u8::is_ascii_digit) {
                if !meter.charge(WorkKind::SourceRead, 1) {
                    return None;
                }
                pos += 1;
            }
            if exp == pos {
                return None;
            }
        }
        let value = std::str::from_utf8(&bytes[start..pos])
            .ok()?
            .parse::<f64>()
            .ok()?;
        if !value.is_finite() {
            return None;
        }
        values[count] = value;
        count += 1;
    }
    Some((values, count))
}
pub(crate) fn base_value(
    source: &[u8],
    tokens: &[XmlToken],
    target: usize,
    a: Attribute,
    k: Kind,
    tt: TransformType,
    meter: &mut Meter<'_>,
) -> Option<Value> {
    if k == Kind::Transform {
        let mut v = Value {
            kind: ValueKind::Transform,
            ..Value::default()
        };
        v.number[3] = match tt {
            TransformType::Scale => {
                v.number[0] = 1.;
                v.number[1] = 1.;
                2.
            }
            TransformType::Translate => 2.,
            _ => 1.,
        };
        return Some(v);
    }
    if k == Kind::Motion {
        return Some(Value {
            kind: ValueKind::Motion,
            ..Value::default()
        });
    }
    let Some(raw) = attr(source, tokens, target, a.name().as_bytes(), meter) else {
        let mut v = Value::default();
        if matches!(a, Attribute::Fill | Attribute::Stroke) {
            v.kind = ValueKind::Color;
            if a == Attribute::Fill {
                v.rgba[3] = 255
            }
        } else if matches!(
            a,
            Attribute::Opacity | Attribute::FillOpacity | Attribute::StrokeOpacity
        ) {
            v.number[0] = 1.
        }
        return Some(v);
    };
    parse_value(source, raw, a, k, tt, meter)
}
pub(crate) fn split(
    source: &[u8],
    s: XmlSpan,
    allocation_failed: &mut bool,
    meter: &mut Meter<'_>,
) -> Option<Vec<XmlSpan>> {
    let b = span(source, s)?;
    let mut separators = 0usize;
    for &byte in b {
        if !meter.charge(WorkKind::SourceRead, 1) {
            return None;
        }
        if byte == b';' {
            separators = separators.checked_add(1)?;
        }
    }
    let count = separators.checked_add(1)?;
    if count > MAX_VALUES {
        return None;
    }
    let mut out = Vec::new();
    if out.try_reserve_exact(count).is_err() {
        *allocation_failed = true;
        return None;
    }
    let mut start = 0;
    for i in 0..=b.len() {
        if !meter.charge(WorkKind::SourceRead, 1) {
            return None;
        }
        if i == b.len() || b[i] == b';' {
            let mut a = start;
            let mut z = i;
            while a < z && b" \t\r\n".contains(&b[a]) {
                if !meter.charge(WorkKind::SourceRead, 1) {
                    return None;
                }
                a += 1
            }
            while z > a && b" \t\r\n".contains(&b[z - 1]) {
                if !meter.charge(WorkKind::SourceRead, 1) {
                    return None;
                }
                z -= 1
            }
            if a == z || out.len() >= MAX_VALUES {
                return None;
            }
            out.push(XmlSpan {
                offset: s.offset.checked_add(a)?,
                length: z - a,
            });
            start = i.saturating_add(1)
        }
    }
    Some(out)
}
pub(crate) fn sync_time(
    src: &[u8],
    s: XmlSpan,
    t: &mut Track,
    is_begin: bool,
    meter: &mut Meter<'_>,
) -> bool {
    if eq(src, s, b"indefinite", meter) {
        if is_begin {
            t.begin_indefinite = true
        } else {
            t.has_end = false
        }
        return true;
    }
    if let Some(v) = time(src, s, meter) {
        if is_begin {
            t.begin_offset = v
        } else {
            t.end_seconds = v;
            t.has_end = true
        }
        return true;
    }
    let Some(b) = span(src, s) else { return false };
    let mut dot = None;
    for (i, &byte) in b.iter().enumerate() {
        if !meter.charge(WorkKind::SourceRead, 1) {
            return false;
        }
        if byte == b'.' {
            dot = Some(i);
            break;
        }
    }
    let Some(dot) = dot else { return false };
    if dot == 0 || dot + 1 >= b.len() {
        return false;
    }
    let mut suffix_end = b.len();
    for i in dot + 1..b.len() {
        if !meter.charge(WorkKind::SourceRead, 1) {
            return false;
        }
        if matches!(b[i], b'+' | b'-') {
            suffix_end = i;
            break;
        }
    }
    let suffix = &b[dot + 1..suffix_end];
    let sync_end = match suffix {
        b"end" => true,
        b"begin" => false,
        _ => return false,
    };
    let mut off = 0.;
    if suffix_end < b.len() {
        let n = XmlSpan {
            offset: s.offset + suffix_end + 1,
            length: b.len() - suffix_end - 1,
        };
        off = match time(src, n, meter) {
            Some(n) => n,
            None => return false,
        };
        if b[suffix_end] == b'-' {
            off = -off
        }
    }
    let id = XmlSpan {
        offset: s.offset,
        length: dot,
    };
    if id.length == 0 {
        return false;
    }
    if is_begin {
        t.has_begin_sync = true;
        t.begin_sync_end = sync_end;
        t.begin_offset = off;
        t.begin_sync_id = Some(id)
    } else {
        t.has_end = true;
        t.end_is_sync = true;
        t.end_seconds = off;
        t.end_sync_id = Some(id)
    }
    true
}
pub(crate) fn path_shape(
    src: &[u8],
    s: XmlSpan,
    meter: &mut Meter<'_>,
) -> Option<([u8; 1024], usize, usize)> {
    let b = span(src, s)?;
    if b.is_empty() || b.len() > MAX_PATH_BYTES {
        return None;
    }
    let (mut commands, mut command_count, mut nums, mut i) = ([0; 1024], 0, 0, 0);
    while i < b.len() {
        if !meter.charge(WorkKind::SourceRead, 1) {
            return None;
        }
        let c = b[i];
        if c.is_ascii_alphabetic() {
            if !b"MmLlHhVvCcSsQqTtAaZz".contains(&c) || command_count >= commands.len() {
                return None;
            }
            commands[command_count] = c;
            command_count += 1;
            i += 1;
            continue;
        }
        if b" \t\r\n,".contains(&c) {
            i += 1;
            continue;
        }
        let start = i;
        if matches!(c, b'+' | b'-') {
            i += 1
        }
        let mut digits = false;
        while b.get(i).is_some_and(u8::is_ascii_digit) {
            if !meter.charge(WorkKind::SourceRead, 1) {
                return None;
            }
            i += 1;
            digits = true
        }
        if b.get(i) == Some(&b'.') {
            if !meter.charge(WorkKind::SourceRead, 1) {
                return None;
            }
            i += 1;
            while b.get(i).is_some_and(u8::is_ascii_digit) {
                if !meter.charge(WorkKind::SourceRead, 1) {
                    return None;
                }
                i += 1;
                digits = true
            }
        }
        if !digits {
            return None;
        }
        if matches!(b.get(i), Some(b'e' | b'E')) {
            if !meter.charge(WorkKind::SourceRead, 1) {
                return None;
            }
            i += 1;
            if matches!(b.get(i), Some(b'+' | b'-')) {
                if !meter.charge(WorkKind::SourceRead, 1) {
                    return None;
                }
                i += 1
            }
            let exp = i;
            while b.get(i).is_some_and(u8::is_ascii_digit) {
                if !meter.charge(WorkKind::SourceRead, 1) {
                    return None;
                }
                i += 1
            }
            if i == exp {
                return None;
            }
        }
        if i == start {
            return None;
        }
        nums += 1;
        if nums > MAX_PATH_NUMBERS {
            return None;
        }
    }
    if command_count == 0 {
        None
    } else {
        Some((commands, command_count, nums))
    }
}
pub(crate) fn paths_match(src: &[u8], a: XmlSpan, b: XmlSpan, meter: &mut Meter<'_>) -> bool {
    match (path_shape(src, a, meter), path_shape(src, b, meter)) {
        (Some((x, xc, n)), Some((y, yc, m))) => {
            if xc != yc || n != m {
                return false;
            }
            for i in 0..xc {
                if !meter.charge(WorkKind::SourceRead, 1) {
                    return false;
                }
                if x[i] != y[i] {
                    return false;
                }
            }
            true
        }
        _ => false,
    }
}
