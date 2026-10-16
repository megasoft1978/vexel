use crate::control::Meter;
use crate::motion::{parse_motion, resolve_motion_path};
use crate::parse_values::*;
use crate::*;
use std::sync::Arc;
use vexel_base::{ControlledError, Stop, WorkControl, WorkKind};
use vexel_xml::{TokenKind, XmlToken};

fn parse_track(
    src: &[u8],
    tokens: &[XmlToken],
    i: usize,
    parent: usize,
    d: &mut Document<'_>,
    allocation_failed: &mut bool,
    meter: &mut Meter<'_>,
) -> Option<Track> {
    if !meter.checkpoint() {
        return None;
    }
    let tag = &tokens[i];
    let name = span(src, tag.local_name)?;
    let mut t = Track::default();
    t.id = attr0(src, tokens, i, b"id", meter);
    t.target = element_span(tokens, parent, meter)?;
    t.element = element_span(tokens, i, meter)?;
    if tag.prefix.length != 0 {
        diagnostic(d, DiagnosticKind::UnsupportedElement, tag.name.offset);
        return None;
    }
    t.kind = match name {
        b"animate" => Kind::Animate,
        b"animateTransform" => Kind::Transform,
        b"set" => Kind::Set,
        b"animateMotion" => Kind::Motion,
        _ => {
            diagnostic(d, DiagnosticKind::UnsupportedElement, tag.name.offset);
            return None;
        }
    };
    let an = attr0(src, tokens, i, b"attributeName", meter);
    if t.kind == Kind::Motion {
        t.attribute = Attribute::Motion
    } else {
        if an.length == 0 {
            diagnostic(d, DiagnosticKind::InvalidTrack, tag.name.offset);
            return None;
        }
        let (a, tr) = attr_kind(src, an, meter);
        t.attribute = a;
        if t.kind == Kind::Transform {
            if !tr {
                diagnostic(d, DiagnosticKind::UnsupportedAttribute, an.offset);
                return None;
            }
        } else if tr || a == Attribute::Motion {
            diagnostic(d, DiagnosticKind::UnsupportedAttribute, an.offset);
            return None;
        }
    }
    let ttspan = attr0(src, tokens, i, b"type", meter);
    if t.kind == Kind::Transform {
        if ttspan.length == 0 || ttspan.length >= 16 {
            diagnostic(d, DiagnosticKind::InvalidTrack, tag.name.offset);
            return None;
        }
        t.transform_type = if eq(src, ttspan, b"translate", meter) {
            TransformType::Translate
        } else if eq(src, ttspan, b"scale", meter) {
            TransformType::Scale
        } else if eq(src, ttspan, b"rotate", meter) {
            TransformType::Rotate
        } else if eq(src, ttspan, b"skewX", meter) {
            TransformType::SkewX
        } else if eq(src, ttspan, b"skewY", meter) {
            TransformType::SkewY
        } else {
            diagnostic(d, DiagnosticKind::UnsupportedAttribute, ttspan.offset);
            return None;
        };
    }
    let at = attr0(src, tokens, i, b"attributeType", meter);
    if at.length > 0 && !eq(src, at, b"XML", meter) {
        diagnostic(d, DiagnosticKind::UnsupportedAttribute, at.offset);
        return None;
    }
    let cm = attr0(src, tokens, i, b"calcMode", meter);
    t.calc_mode = if cm.length == 0 || eq(src, cm, b"linear", meter) {
        CalcMode::Linear
    } else if eq(src, cm, b"discrete", meter) {
        CalcMode::Discrete
    } else if eq(src, cm, b"spline", meter) {
        CalcMode::Spline
    } else if eq(src, cm, b"paced", meter) {
        CalcMode::Paced
    } else {
        diagnostic(d, DiagnosticKind::UnsupportedAttribute, cm.offset);
        return None;
    };
    let fill = attr0(src, tokens, i, b"fill", meter);
    t.fill_mode = if fill.length == 0 || eq(src, fill, b"remove", meter) {
        FillMode::Remove
    } else if eq(src, fill, b"freeze", meter) {
        FillMode::Freeze
    } else {
        diagnostic(d, DiagnosticKind::UnsupportedAttribute, fill.offset);
        return None;
    };
    let add = attr0(src, tokens, i, b"additive", meter);
    if add.length > 0 && !eq(src, add, b"replace", meter) {
        if eq(src, add, b"sum", meter) {
            t.additive_sum = true
        } else {
            diagnostic(d, DiagnosticKind::UnsupportedComposition, add.offset);
            return None;
        }
    }
    let acc = attr0(src, tokens, i, b"accumulate", meter);
    if acc.length > 0 && !eq(src, acc, b"none", meter) {
        if eq(src, acc, b"sum", meter) {
            t.accumulate = true
        } else {
            diagnostic(d, DiagnosticKind::UnsupportedComposition, acc.offset);
            return None;
        }
    }
    if (t.kind == Kind::Motion && (t.additive_sum || t.accumulate))
        || (t.kind == Kind::Transform && t.accumulate)
        || (t.attribute == Attribute::D && (t.additive_sum || t.accumulate))
    {
        diagnostic(d, DiagnosticKind::UnsupportedComposition, tag.name.offset);
        return None;
    }
    let begin = attr0(src, tokens, i, b"begin", meter);
    if begin.length > 0 && !sync_time(src, begin, &mut t, true, meter) {
        diagnostic(d, DiagnosticKind::UnsupportedTiming, begin.offset);
        return None;
    }
    let end = attr0(src, tokens, i, b"end", meter);
    if end.length > 0 && !sync_time(src, end, &mut t, false, meter) {
        diagnostic(d, DiagnosticKind::UnsupportedTiming, end.offset);
        return None;
    }
    let dur = attr0(src, tokens, i, b"dur", meter);
    if dur.length == 0 && t.kind != Kind::Set {
        diagnostic(d, DiagnosticKind::InvalidTrack, tag.name.offset);
        return None;
    }
    if eq(src, dur, b"indefinite", meter) {
        t.duration_indefinite = true
    } else if dur.length > 0 {
        t.duration_seconds = match time(src, dur, meter) {
            Some(n) => n,
            None => {
                diagnostic(d, DiagnosticKind::UnsupportedTiming, dur.offset);
                return None;
            }
        }
    }
    if !t.duration_indefinite && t.kind != Kind::Set && t.duration_seconds <= 0. {
        diagnostic(d, DiagnosticKind::InvalidTrack, dur.offset);
        return None;
    }
    if t.kind == Kind::Set && dur.length == 0 {
        t.duration_indefinite = true
    }
    let rep = attr0(src, tokens, i, b"repeatCount", meter);
    if eq(src, rep, b"indefinite", meter) {
        t.repeat_indefinite = true;
        t.repeat_count = f64::INFINITY
    } else if rep.length > 0 {
        t.repeat_count = match number(src, rep, meter) {
            Some(n) if n > 0. && n <= 100000. => n,
            _ => {
                diagnostic(d, DiagnosticKind::UnsupportedTiming, tag.name.offset);
                return None;
            }
        }
    }
    if t.kind == Kind::Set {
        t.repeat_count = 1.
    }
    let rd = attr0(src, tokens, i, b"repeatDur", meter);
    if eq(src, rd, b"indefinite", meter) {
        t.repeat_duration_indefinite = true
    } else if rd.length > 0 {
        t.repeat_duration = match time(src, rd, meter) {
            Some(n) => n,
            None => {
                diagnostic(d, DiagnosticKind::UnsupportedTiming, rd.offset);
                return None;
            }
        }
    }
    t.base_value = match base_value(
        src,
        tokens,
        parent,
        t.attribute,
        t.kind,
        t.transform_type,
        meter,
    ) {
        Some(value) => value,
        None => {
            diagnostic(d, DiagnosticKind::InvalidTrack, tag.name.offset);
            return None;
        }
    };
    let vals = attr0(src, tokens, i, b"values", meter);
    let from = attr0(src, tokens, i, b"from", meter);
    let to = attr0(src, tokens, i, b"to", meter);
    let by = attr0(src, tokens, i, b"by", meter);
    if vals.length > 0 && (from.length > 0 || to.length > 0 || by.length > 0) {
        diagnostic(d, DiagnosticKind::InvalidTrack, vals.offset);
        return None;
    }
    let mut pieces = Vec::new();
    if pieces.try_reserve_exact(MAX_VALUES).is_err() {
        *allocation_failed = true;
        return None;
    }
    let mut implicit = false;
    let mut by_value = false;
    if t.kind == Kind::Motion {
        let path = resolve_motion_path(src, tokens, i, meter)
            .filter(|p| parse_motion(src, *p, &mut t, allocation_failed, meter));
        if path.is_none() {
            diagnostic(d, DiagnosticKind::UnsupportedPath, tag.name.offset);
            return None;
        }
        let rot = attr0(src, tokens, i, b"rotate", meter);
        if eq(src, rot, b"auto", meter) {
            t.motion_rotate_auto = true
        } else if eq(src, rot, b"auto-reverse", meter) {
            t.motion_rotate_auto = true;
            t.motion_rotate_reverse = true
        } else if rot.length > 0 {
            t.motion_fixed_rotation = match number(src, rot, meter) {
                Some(n) => n,
                _ => {
                    diagnostic(d, DiagnosticKind::UnsupportedAttribute, rot.offset);
                    return None;
                }
            }
        }
        pieces.push(path.unwrap());
    } else if vals.length > 0 {
        pieces = match split(src, vals, allocation_failed, meter) {
            Some(p) => p,
            None => {
                diagnostic(d, DiagnosticKind::InvalidTrack, vals.offset);
                return None;
            }
        }
    } else if t.kind == Kind::Set {
        if to.length == 0 {
            diagnostic(d, DiagnosticKind::InvalidTrack, tag.name.offset);
            return None;
        }
        pieces.push(to)
    } else if from.length > 0 && to.length > 0 {
        pieces.extend([from, to])
    } else if to.length > 0 {
        implicit = true;
        pieces.push(to)
    } else if by.length > 0 {
        implicit = true;
        by_value = true;
        pieces.push(by)
    } else {
        diagnostic(d, DiagnosticKind::InvalidTrack, tag.name.offset);
        return None;
    }
    if pieces.is_empty() || pieces.len() > MAX_VALUES {
        diagnostic(d, DiagnosticKind::InvalidTrack, tag.name.offset);
        return None;
    }
    let source_count = pieces.len();
    if implicit && source_count >= MAX_VALUES {
        diagnostic(d, DiagnosticKind::InvalidTrack, tag.name.offset);
        return None;
    }
    let count = source_count + usize::from(implicit);
    if t.keys.try_reserve_exact(count).is_err() {
        *allocation_failed = true;
        return None;
    }
    if implicit {
        t.keys.push(Keyframe {
            time: 0.,
            value: t.base_value,
            source: t.base_value.raw,
        })
    }
    for (j, p) in pieces.iter().enumerate() {
        if !meter.charge(WorkKind::AnimationTransform, 1) {
            return None;
        }
        let mut v = match parse_value(src, *p, t.attribute, t.kind, t.transform_type, meter) {
            Some(v) => v,
            None => {
                diagnostic(
                    d,
                    if t.attribute == Attribute::D {
                        DiagnosticKind::UnsupportedPath
                    } else {
                        DiagnosticKind::InvalidTrack
                    },
                    p.offset,
                );
                return None;
            }
        };
        if by_value {
            if v.kind != ValueKind::Number || t.base_value.kind != ValueKind::Number {
                diagnostic(d, DiagnosticKind::UnsupportedComposition, p.offset);
                return None;
            }
            v.number[0] += t.base_value.number[0]
        }
        let k = j + usize::from(implicit);
        t.keys.push(Keyframe {
            time: if count == 1 {
                0.
            } else {
                k as f64 / (count - 1) as f64
            },
            value: v,
            source: *p,
        })
    }
    if t.kind == Kind::Animate && t.attribute == Attribute::D {
        let base = attr0(src, tokens, parent, b"d", meter);
        if base.length == 0 {
            diagnostic(d, DiagnosticKind::UnsupportedPath, tag.name.offset);
            return None;
        }
        for key in &t.keys {
            if !meter.charge(WorkKind::AnimationTransform, 1) {
                return None;
            }
            if !paths_match(src, base, key.value.raw, meter) {
                diagnostic(d, DiagnosticKind::UnsupportedPath, key.value.raw.offset);
                return None;
            }
        }
    }
    let kt = attr0(src, tokens, i, b"keyTimes", meter);
    if kt.length > 0 {
        let ts = match split(src, kt, allocation_failed, meter) {
            Some(x) => x,
            None => {
                diagnostic(d, DiagnosticKind::InvalidTrack, kt.offset);
                return None;
            }
        };
        if ts.len() != t.keys.len() {
            diagnostic(d, DiagnosticKind::InvalidTrack, kt.offset);
            return None;
        }
        let mut prev = -1.;
        for (k, s) in t.keys.iter_mut().zip(ts) {
            if !meter.charge(WorkKind::AnimationTransform, 1) {
                return None;
            }
            let n = number(src, s, meter).filter(|n| *n >= 0. && *n <= 1. && *n > prev);
            if let Some(n) = n {
                k.time = n;
                prev = n
            } else {
                diagnostic(d, DiagnosticKind::InvalidTrack, kt.offset);
                return None;
            }
        }
        if t.keys[0].time != 0. || (t.kind != Kind::Set && t.keys.last()?.time != 1.) {
            diagnostic(d, DiagnosticKind::InvalidTrack, kt.offset);
            return None;
        }
        if t.calc_mode == CalcMode::Paced {
            diagnostic(d, DiagnosticKind::UnsupportedAttribute, kt.offset);
            return None;
        }
    }
    let spl = attr0(src, tokens, i, b"keySplines", meter);
    if t.calc_mode != CalcMode::Spline {
        if spl.length > 0 {
            diagnostic(d, DiagnosticKind::InvalidTrack, spl.offset);
            return None;
        }
    } else {
        let ss = match split(src, spl, allocation_failed, meter) {
            Some(x) => x,
            None => {
                diagnostic(d, DiagnosticKind::InvalidTrack, tag.name.offset);
                return None;
            }
        };
        if ss.len() != t.keys.len().saturating_sub(1) {
            diagnostic(d, DiagnosticKind::InvalidTrack, spl.offset);
            return None;
        }
        if t.splines.try_reserve_exact(ss.len()).is_err() {
            *allocation_failed = true;
            return None;
        }
        for s in ss {
            if !meter.charge(WorkKind::AnimationTransform, 1) {
                return None;
            }
            let b = span(src, s)?;
            if b.len() >= 256 {
                return None;
            }
            let (vals, count) = number_list(b, 4, meter)?;
            if count != 4 || vals.iter().any(|v| !v.is_finite() || *v < 0. || *v > 1.) {
                diagnostic(d, DiagnosticKind::InvalidTrack, spl.offset);
                return None;
            }
            t.splines.push([vals[0], vals[1], vals[2], vals[3]])
        }
    }
    Some(t)
}

fn spans_equal(
    source: &[u8],
    span: vexel_xml::XmlSpan,
    other: &[u8],
    meter: &mut Meter<'_>,
) -> Option<bool> {
    let bytes = span.slice(source)?;
    if bytes.len() != other.len() {
        return Some(false);
    }
    for (a, b) in bytes.iter().zip(other) {
        if !meter.charge(WorkKind::SourceRead, 1) {
            return None;
        }
        if a != b {
            return Some(false);
        }
    }
    Some(true)
}

pub(super) fn contains_controlled(
    bytes: &[u8],
    needle: &[u8],
    meter: &mut Meter<'_>,
) -> Option<bool> {
    if needle.is_empty() {
        return Some(true);
    }
    if needle.len() > bytes.len() {
        return Some(false);
    }
    for start in 0..=bytes.len() - needle.len() {
        let mut matched = true;
        for (offset, expected) in needle.iter().enumerate() {
            if !meter.charge(WorkKind::SourceRead, 1) {
                return None;
            }
            if bytes[start + offset] != *expected {
                matched = false;
                break;
            }
        }
        if matched {
            return Some(true);
        }
    }
    Some(false)
}

fn parse_inner<'a>(
    source: &'a [u8],
    tokens: &[XmlToken],
    meter: &mut Meter<'_>,
) -> Result<Document<'a>, ParseError> {
    if !meter.checkpoint() {
        return Err(ParseError::Allocation);
    }
    let mut d = Document {
        source,
        tracks: Vec::new(),
        diagnostics: Vec::new(),
    };
    d.tracks
        .try_reserve_exact(MAX_TRACKS)
        .map_err(|_| ParseError::Allocation)?;
    d.diagnostics
        .try_reserve_exact(MAX_DIAGNOSTICS)
        .map_err(|_| ParseError::Allocation)?;
    let mut allocation_failed = false;
    for (i, t) in tokens.iter().enumerate() {
        if !meter.charge(WorkKind::TokenVisit, 1) {
            break;
        }
        if !matches!(t.kind, TokenKind::StartElement | TokenKind::EmptyElement) {
            continue;
        }
        let name = span(source, t.local_name).unwrap_or_default();
        if t.prefix.length == 0 && name == b"style" {
            let mut child = i + 1;
            while child < tokens.len() {
                if !meter.charge(WorkKind::TokenVisit, 1) {
                    break;
                }
                let x = &tokens[child];
                if x.kind == TokenKind::EndElement && x.depth == t.depth {
                    break;
                }
                if x.kind == TokenKind::Text {
                    if let Some(b) = span(source, x.value) {
                        let mut matched = false;
                        for needle in [b"@keyframes".as_slice(), b"animation:", b"transition:"] {
                            match contains_controlled(b, needle, meter) {
                                Some(true) => {
                                    matched = true;
                                    break;
                                }
                                Some(false) => (),
                                None => break,
                            }
                        }
                        if meter.stop().is_some() {
                            break;
                        }
                        if matched {
                            diagnostic(&mut d, DiagnosticKind::CssAnimation, x.value.offset)
                        }
                    }
                }
                child += 1;
            }
        }
        if t.prefix.length != 0
            || !matches!(
                name,
                b"animate" | b"animateTransform" | b"set" | b"animateMotion" | b"animateColor"
            )
        {
            continue;
        }
        if d.tracks.len() >= MAX_TRACKS {
            return Err(ParseError::TrackLimit {
                offset: t.name.offset,
            });
        }
        let mut parent = None;
        for j in (0..i).rev() {
            if !meter.charge(WorkKind::TokenVisit, 1) {
                break;
            }
            if matches!(
                tokens[j].kind,
                TokenKind::StartElement | TokenKind::EmptyElement
            ) && tokens[j].depth + 1 == t.depth
            {
                parent = Some(j);
                break;
            }
        }
        if meter.stop().is_some() {
            break;
        }
        let Some(parent) = parent else {
            diagnostic(&mut d, DiagnosticKind::InvalidTrack, t.name.offset);
            continue;
        };
        if let Some(track) = parse_track(
            source,
            tokens,
            i,
            parent,
            &mut d,
            &mut allocation_failed,
            meter,
        ) {
            d.tracks.push(track)
        }
        if allocation_failed {
            return Err(ParseError::Allocation);
        }
    }
    // Resolve syncbase identifiers only after every supported track has been collected.
    for i in 0..d.tracks.len() {
        if !meter.charge(WorkKind::AnimationTransform, 1) {
            break;
        }
        for begin in [true, false] {
            let id = if begin {
                d.tracks[i].begin_sync_id
            } else {
                d.tracks[i].end_sync_id
            };
            let Some(id) = id else { continue };
            let needle = span(source, id).unwrap_or_default();
            let mut found = None;
            let mut match_count = 0;
            for (j, track) in d.tracks.iter().enumerate() {
                if !meter.charge(WorkKind::ReferenceProbe, 1) {
                    break;
                }
                if track.id.length > 0
                    && spans_equal(source, track.id, needle, meter).unwrap_or(false)
                {
                    found = Some(j);
                    match_count += 1;
                }
            }
            if match_count != 1 {
                diagnostic(&mut d, DiagnosticKind::UnsupportedTiming, id.offset);
                d.tracks[i].supported = false
            } else if begin {
                d.tracks[i].begin_sync_track = found
            } else {
                d.tracks[i].end_sync_track = found
            }
        }
    }
    if meter.stop().is_some() {
        return Err(ParseError::Allocation);
    }
    // Cycles are retained as unsupported timing and resolved by evaluator to inactive tracks.
    Ok(d)
}

pub fn parse<'a>(source: &'a [u8], tokens: &[XmlToken]) -> Result<Document<'a>, ParseError> {
    parse_inner(source, tokens, &mut Meter::unlimited())
}

fn controlled<E>(stop: Stop) -> ControlledError<E> {
    match stop {
        Stop::Canceled => ControlledError::Canceled,
        Stop::Limit => ControlledError::Limit,
        Stop::Overflow => ControlledError::Overflow,
    }
}

pub fn parse_controlled<'a>(
    source: &'a [u8],
    tokens: &[XmlToken],
    control: &mut dyn WorkControl,
) -> Result<Document<'a>, ControlledError<ParseError>> {
    let mut meter = Meter::controlled(control);
    let result = parse_inner(source, tokens, &mut meter);
    if let Some(stop) = meter.stop() {
        return Err(controlled(stop));
    }
    match result {
        Ok(document) => Ok(document),
        Err(ParseError::Allocation) => Err(ControlledError::OutOfMemory),
        Err(error) => Err(ControlledError::Original(error)),
    }
}

/// Parse with the accepted parser, then retain the caller's source owner on success.
pub fn parse_owned(
    source: &Arc<Vec<u8>>,
    tokens: &[XmlToken],
) -> Result<OwnedDocument, ParseError> {
    let parsed = parse(source.as_slice(), tokens)?;
    let Document {
        tracks,
        diagnostics,
        ..
    } = parsed;
    Ok(OwnedDocument {
        source_owner: Arc::clone(source),
        tracks,
        diagnostics,
    })
}

pub fn parse_owned_controlled(
    source: &Arc<Vec<u8>>,
    tokens: &[XmlToken],
    control: &mut dyn WorkControl,
) -> Result<OwnedDocument, ControlledError<ParseError>> {
    let parsed = parse_controlled(source.as_slice(), tokens, control)?;
    let Document {
        tracks,
        diagnostics,
        ..
    } = parsed;
    Ok(OwnedDocument {
        source_owner: Arc::clone(source),
        tracks,
        diagnostics,
    })
}
