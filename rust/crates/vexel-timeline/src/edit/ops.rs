use super::{
    EditFailure, EditResult, TimelineEdit, VALUE_JSON_LIMIT, fail, valid_asset, valid_id,
    valid_target, validate_utf8,
};
use crate::json::{Field, array_spans, decode_string, object_fields};
use crate::schema::{field, string, uint};
use crate::{
    Error, LoopMode, MAX_ITEMS, MAX_ITEMS_PER_TRACK, MAX_SOURCE_BYTES, MAX_TRACKS, TimelineContext,
    decode,
};
use vexel_base::Span;

struct Track {
    items: Span,
    item_spans: Vec<Span>,
    drawing: bool,
    duration: u32,
}
fn at(error: Error, offset: usize) -> EditFailure {
    fail(error, offset)
}
fn fields(source: &[u8], span: Span) -> Result<Vec<Field>, EditFailure> {
    object_fields(source, span, None).map_err(Into::into)
}
fn root_span(source: &[u8]) -> Result<Span, EditFailure> {
    let mut r = crate::json::Reader::new(source, None);
    r.scan(0).map_err(EditFailure::from)?;
    Ok(Span {
        start: 0,
        end: r.cursor,
    })
}
fn selected(
    source: &[u8],
    context: &TimelineContext,
    id: &[u8],
) -> Result<(Track, usize), EditFailure> {
    selected_view(source, context, id).map(|(track, index, _)| (track, index))
}
fn selected_view<'a>(
    source: &'a [u8],
    context: &TimelineContext,
    id: &[u8],
) -> Result<(Track, usize, crate::TimelineView<'a>), EditFailure> {
    let view = decode(source, context).map_err(EditFailure::from)?;
    let root_fields = fields(source, root_span(source)?)?;
    let tracks = field(&root_fields, b"tracks")
        .ok_or_else(|| at(Error::InvalidSchema, 0))?
        .value;
    let track_spans = array_spans(source, tracks, MAX_TRACKS, None).map_err(EditFailure::from)?;
    let mut found = None;
    for (index, span) in track_spans.iter().enumerate() {
        let f = fields(source, *span)?;
        if string(
            source,
            field(&f, b"id")
                .ok_or_else(|| at(Error::InvalidSchema, span.start))?
                .value,
        )
        .map_err(EditFailure::from)?
        .as_slice()
            == id
        {
            found = Some((index, *span, f));
            break;
        }
    }
    let (index, span, f) = found.ok_or_else(|| at(Error::InvalidReference, 0))?;
    let drawing = string(
        source,
        field(&f, b"kind")
            .ok_or_else(|| at(Error::InvalidSchema, span.start))?
            .value,
    )
    .map_err(EditFailure::from)?
        == b"drawing";
    let item_field = field(&f, if drawing { b"cels" } else { b"keys" })
        .ok_or_else(|| at(Error::InvalidSchema, span.start))?;
    let item_spans = array_spans(source, item_field.value, MAX_ITEMS_PER_TRACK, None)
        .map_err(EditFailure::from)?;
    Ok((
        Track {
            items: item_field.value,
            item_spans,
            drawing,
            duration: view.duration_ticks(),
        },
        index,
        view,
    ))
}
fn splice(source: &[u8], start: usize, remove: usize, insert: &[u8]) -> EditResult<Vec<u8>> {
    if start > source.len() || remove > source.len() - start {
        return Err(at(Error::InvalidSchema, start));
    }
    let len = source
        .len()
        .checked_sub(remove)
        .and_then(|n| n.checked_add(insert.len()))
        .ok_or_else(|| at(Error::Limit, start))?;
    if len > MAX_SOURCE_BYTES {
        return Err(at(Error::Limit, start));
    }
    let mut out = Vec::new();
    out.try_reserve_exact(len)
        .map_err(|_| at(Error::Limit, start))?;
    out.extend_from_slice(&source[..start]);
    out.extend_from_slice(insert);
    out.extend_from_slice(&source[start + remove..]);
    Ok(out)
}
fn append(out: &mut Vec<u8>, part: &[u8]) -> EditResult {
    out.try_reserve(part.len())
        .map_err(|_| at(Error::Limit, 0))?;
    out.extend_from_slice(part);
    Ok(())
}
pub(super) fn json_string(raw: &[u8]) -> EditResult<Vec<u8>> {
    if !validate_utf8(raw) || raw.len() > crate::MAX_STRING_BYTES {
        return Err(at(Error::InvalidArgument, 0));
    }
    let cap = raw
        .len()
        .checked_mul(6)
        .and_then(|n| n.checked_add(2))
        .ok_or_else(|| at(Error::Limit, 0))?;
    let mut out = Vec::new();
    out.try_reserve_exact(cap)
        .map_err(|_| at(Error::Limit, 0))?;
    out.push(b'"');
    const HEX: &[u8] = b"0123456789abcdef";
    for &b in raw {
        match b {
            b'"' | b'\\' => {
                out.push(b'\\');
                out.push(b)
            }
            0..=31 => {
                out.extend_from_slice(b"\\u00");
                out.push(HEX[(b >> 4) as usize]);
                out.push(HEX[(b & 15) as usize]);
            }
            _ => out.push(b),
        }
    }
    out.push(b'"');
    Ok(out)
}
pub(super) fn value_json(raw: &[u8]) -> Result<(), EditFailure> {
    if raw.is_empty() {
        return Err(at(Error::InvalidArgument, 0));
    }
    // Path morph keys are JSON strings of up to MAX_STRING_BYTES.
    let limit = if raw[0] == b'"' {
        crate::MAX_STRING_BYTES + 64
    } else {
        VALUE_JSON_LIMIT
    };
    if raw.len() > limit {
        return Err(at(Error::Limit, 0));
    }
    // Syntax and typed-value errors are reported after splicing, with offsets
    // in the candidate document as in the C contract.
    Ok(())
}
fn track_candidate(
    edit: &TimelineEdit,
    context: &TimelineContext,
    id: &[u8],
    payload: &[u8],
    target: Option<&[u8]>,
) -> EditResult<Vec<u8>> {
    let source = edit.source();
    let view = decode(source, context).map_err(EditFailure::from)?;
    if view.track_count() >= MAX_TRACKS {
        return Err(at(Error::Limit, view.tracks_span().start));
    }
    if !valid_id(id) {
        return Err(at(Error::InvalidArgument, 0));
    }
    for i in 0..view.track_count() {
        let t = view
            .track_at(i)
            .ok_or_else(|| at(Error::InvalidSchema, 0))?;
        let decoded = decode_string(t.id).ok_or_else(|| at(Error::Limit, 0))?;
        if decoded == id {
            return Err(at(Error::Duplicate, view.tracks_span().start));
        }
    }
    if target.is_some_and(|target| !valid_target(context, target)) {
        return Err(at(Error::InvalidReference, 0));
    }
    let tracks = view.tracks_span();
    let spans = array_spans(source, tracks, MAX_TRACKS, None).map_err(EditFailure::from)?;
    let insert_at = spans.last().map(|s| s.end).unwrap_or(tracks.end - 1);
    let comma = if spans.is_empty() {
        &b""[..]
    } else {
        &b","[..]
    };
    let insertion_len = comma
        .len()
        .checked_add(payload.len())
        .ok_or_else(|| at(Error::Limit, insert_at))?;
    let mut insertion = Vec::new();
    insertion
        .try_reserve_exact(insertion_len)
        .map_err(|_| at(Error::Limit, insert_at))?;
    insertion.extend_from_slice(comma);
    insertion.extend_from_slice(payload);
    splice(source, insert_at, 0, &insertion)
}
fn track_json_add(
    edit: &mut TimelineEdit,
    context: &TimelineContext,
    id: &[u8],
    payload: &[u8],
) -> EditResult {
    let candidate = track_candidate(edit, context, id, payload, None)?;
    edit.commit(context, candidate)
}
fn make_drawing(id: &[u8], target: &[u8], asset: &[u8], name: &[u8]) -> EditResult<Vec<u8>> {
    let (i, t, a, n) = (
        json_string(id)?,
        json_string(target)?,
        json_string(asset)?,
        json_string(name)?,
    );
    let mut out = Vec::new();
    out.try_reserve(i.len() + t.len() + a.len() + n.len() + 96)
        .map_err(|_| at(Error::Limit, 0))?;
    out.extend_from_slice(b"{\"id\":");
    out.extend(i);
    out.extend_from_slice(b",\"kind\":\"drawing\",\"target\":");
    out.extend(t);
    out.extend_from_slice(b",\"visible\":true,\"cels\":[{\"tick\":0,\"asset\":");
    out.extend(a);
    out.extend_from_slice(b"}]");
    if !name.is_empty() {
        out.extend_from_slice(b",\"name\":");
        out.extend(n);
    }
    out.push(b'}');
    Ok(out)
}
pub(super) fn add_drawing_track(
    edit: &mut TimelineEdit,
    context: &TimelineContext,
    id: &[u8],
    target: &[u8],
    asset: &[u8],
    name: &[u8],
) -> EditResult {
    if id.is_empty()
        || id.len() > 128
        || !validate_utf8(name)
        || name.len() > crate::MAX_STRING_BYTES
    {
        return Err(at(Error::InvalidArgument, 0));
    }
    if let Some(offset) = id
        .iter()
        .position(|b| !b.is_ascii_alphanumeric() && !matches!(*b, b'_' | b'-'))
    {
        return Err(at(Error::InvalidValue, offset));
    }
    let _view = decode(edit.source(), context).map_err(EditFailure::from)?;
    if !valid_target(context, target) || !valid_asset(context, asset) {
        return Err(at(Error::InvalidReference, 0));
    }
    let payload = make_drawing(id, target, asset, name)?;
    track_json_add(edit, context, id, &payload)
}
fn make_property(
    id: &[u8],
    target: &[u8],
    property: &[u8],
    ty: &[u8],
    interp: &[u8],
    value: &[u8],
    extra: Option<(u32, &[u8])>,
    name: &[u8],
) -> EditResult<Vec<u8>> {
    for s in [id, target, property, ty, interp, name] {
        if !validate_utf8(s) || s.len() > crate::MAX_STRING_BYTES {
            return Err(at(Error::InvalidArgument, 0));
        }
    }
    if property.is_empty() || ty.is_empty() || interp.is_empty() {
        return Err(at(Error::InvalidArgument, 0));
    }
    value_json(value)?;
    if let Some((_, raw)) = extra {
        value_json(raw)?;
    }
    let (i, t, p, y, n) = (
        json_string(id)?,
        json_string(target)?,
        json_string(property)?,
        json_string(ty)?,
        json_string(interp)?,
    );
    let extra_len = extra.map_or(0, |(_, raw)| raw.len().saturating_add(40));
    let cap = i
        .len()
        .checked_add(t.len())
        .and_then(|v| v.checked_add(p.len()))
        .and_then(|v| v.checked_add(y.len()))
        .and_then(|v| v.checked_add(n.len()))
        .and_then(|v| v.checked_add(value.len()))
        .and_then(|v| v.checked_add(extra_len))
        .and_then(|v| v.checked_add(name.len().saturating_mul(6)))
        .and_then(|v| v.checked_add(180))
        .ok_or_else(|| at(Error::Limit, 0))?;
    let mut out = Vec::new();
    out.try_reserve_exact(cap)
        .map_err(|_| at(Error::Limit, 0))?;
    out.extend_from_slice(b"{\"id\":");
    out.extend(i);
    out.extend_from_slice(b",\"kind\":\"property\",\"target\":");
    out.extend(t);
    out.extend_from_slice(b",\"visible\":true,\"property\":");
    out.extend(p);
    out.extend_from_slice(b",\"value_type\":");
    out.extend(y);
    out.extend_from_slice(b",\"interpolation\":");
    out.extend(n);
    out.extend_from_slice(b",\"keys\":[{\"tick\":0,\"value\":");
    out.extend_from_slice(value);
    out.push(b'}');
    if let Some((tick, raw)) = extra {
        out.extend_from_slice(b",{\"tick\":");
        out.extend_from_slice(tick.to_string().as_bytes());
        out.extend_from_slice(b",\"value\":");
        out.extend_from_slice(raw);
        out.push(b'}');
    }
    out.push(b']');
    if !name.is_empty() {
        out.extend_from_slice(b",\"name\":");
        out.extend(json_string(name)?);
    }
    out.push(b'}');
    Ok(out)
}
pub(super) fn add_property_track(
    edit: &mut TimelineEdit,
    context: &TimelineContext,
    id: &[u8],
    target: &[u8],
    property: &[u8],
    ty: &[u8],
    interp: &[u8],
    value: &[u8],
    name: &[u8],
) -> EditResult {
    if !valid_id(id) || target.is_empty() || target.len() > crate::MAX_STRING_BYTES {
        return Err(at(Error::InvalidArgument, 0));
    }
    let payload = make_property(id, target, property, ty, interp, value, None, name)?;
    let candidate = track_candidate(edit, context, id, &payload, Some(target))?;
    edit.commit(context, candidate)
}
pub(super) fn add_property_track_key(
    edit: &mut TimelineEdit,
    context: &TimelineContext,
    id: &[u8],
    target: &[u8],
    property: &[u8],
    ty: &[u8],
    interp: &[u8],
    initial: &[u8],
    tick: u32,
    value: &[u8],
    name: &[u8],
) -> EditResult {
    if tick == 0 {
        return Err(at(Error::InvalidArgument, 0));
    }
    decode(edit.source(), context).map_err(EditFailure::from)?;
    if !valid_id(id) || target.is_empty() || target.len() > crate::MAX_STRING_BYTES {
        return Err(at(Error::InvalidArgument, 0));
    }
    let initial_payload = make_property(id, target, property, ty, interp, initial, None, name)?;
    let initial_candidate = track_candidate(edit, context, id, &initial_payload, Some(target))?;
    let current = decode(edit.source(), context).map_err(EditFailure::from)?;
    decode(&initial_candidate, context).map_err(EditFailure::from)?;
    if tick >= current.duration_ticks() {
        return Err(at(Error::InvalidValue, 0));
    }
    let payload = make_property(
        id,
        target,
        property,
        ty,
        interp,
        initial,
        Some((tick, value)),
        name,
    )?;
    let candidate = track_candidate(edit, context, id, &payload, Some(target))?;
    edit.commit(context, candidate)
}
fn insert_item(
    source: &[u8],
    array: Span,
    spans: &[Span],
    index: Option<usize>,
    replace: bool,
    item: &[u8],
) -> EditResult<Vec<u8>> {
    let (insert_at, remove, insert) = if let Some(i) = index {
        let len = item
            .len()
            .checked_add(usize::from(!replace))
            .ok_or_else(|| at(Error::Limit, spans[i].start))?;
        let mut b = Vec::new();
        b.try_reserve_exact(len)
            .map_err(|_| at(Error::Limit, spans[i].start))?;
        b.extend_from_slice(item);
        if !replace {
            b.push(b',');
        }
        (
            spans[i].start,
            if replace {
                spans[i].end - spans[i].start
            } else {
                0
            },
            b,
        )
    } else {
        let insert_at = spans.last().map(|s| s.end).unwrap_or(array.end - 1);
        let len = item
            .len()
            .checked_add(usize::from(!spans.is_empty()))
            .ok_or_else(|| at(Error::Limit, insert_at))?;
        let mut b = Vec::new();
        b.try_reserve_exact(len)
            .map_err(|_| at(Error::Limit, insert_at))?;
        if !spans.is_empty() {
            b.push(b',');
        }
        b.extend_from_slice(item);
        (insert_at, 0, b)
    };
    splice(source, insert_at, remove, &insert)
}
pub(super) fn make_key(tick: u32, value: &[u8], ease: Option<&[u8]>) -> EditResult<Vec<u8>> {
    value_json(value)?;
    let mut out = Vec::new();
    out.try_reserve(value.len() + ease.map_or(40, |e| e.len() + 48))
        .map_err(|_| at(Error::Limit, 0))?;
    out.extend_from_slice(b"{\"tick\":");
    out.extend_from_slice(tick.to_string().as_bytes());
    out.extend_from_slice(b",\"value\":");
    out.extend_from_slice(value);
    if let Some(ease) = ease {
        if ease.is_empty() || ease.len() > VALUE_JSON_LIMIT { return Err(at(Error::InvalidArgument, 0)); }
        out.extend_from_slice(b",\"ease\":");
        out.extend_from_slice(ease);
    }
    out.push(b'}');
    Ok(out)
}
fn make_cel(tick: u32, asset: &[u8], label: &[u8]) -> EditResult<Vec<u8>> {
    let (a, l) = (json_string(asset)?, json_string(label)?);
    let mut out = Vec::new();
    out.try_reserve(a.len() + l.len() + 48)
        .map_err(|_| at(Error::Limit, 0))?;
    out.extend_from_slice(b"{\"tick\":");
    out.extend_from_slice(tick.to_string().as_bytes());
    out.extend_from_slice(b",\"asset\":");
    out.extend(a);
    if !label.is_empty() {
        out.extend_from_slice(b",\"label\":");
        out.extend(l);
    }
    out.push(b'}');
    Ok(out)
}
fn ease_json(raw: &[u8]) -> Option<&[u8]> {
    let marker = b",\"ease\":";
    let start = raw.windows(marker.len()).position(|w| w == marker)? + marker.len();
    raw.get(start..raw.len().checked_sub(1)?)
}
fn change_item(
    edit: &mut TimelineEdit,
    context: &TimelineContext,
    id: &[u8],
    tick: u32,
    raw: &[u8],
    drawing: bool,
    preserve_ease: bool,
) -> EditResult {
    let source = edit.source();
    let (track, _, view) = selected_view(source, context, id)?;
    if track.drawing != drawing {
        return Err(at(Error::InvalidArgument, 0));
    }
    if tick >= track.duration {
        return Err(at(Error::InvalidValue, 0));
    }
    let mut replace = None;
    let mut insert = None;
    let mut total = 0usize;
    for i in 0..view.track_count() {
        total = total
            .checked_add(
                view.track_at(i)
                    .ok_or_else(|| at(Error::InvalidSchema, 0))?
                    .item_count,
            )
            .ok_or_else(|| at(Error::Limit, 0))?;
    }
    for (i, span) in track.item_spans.iter().enumerate() {
        let f = fields(source, *span)?;
        let atick = uint(
            source,
            field(&f, b"tick")
                .ok_or_else(|| at(Error::InvalidSchema, span.start))?
                .value,
        )
        .map_err(EditFailure::from)?;
        if atick == tick {
            replace = Some(i);
            break;
        }
        if atick > tick {
            insert = Some(i);
            break;
        }
    }
    if replace.is_none() && (track.item_spans.len() >= MAX_ITEMS_PER_TRACK || total >= MAX_ITEMS) {
        return Err(at(Error::Limit, track.items.start));
    }
    let mut replacement = raw.to_vec();
    if preserve_ease && ease_json(raw).is_none() {
        if let Some(index) = replace {
            let old_fields = fields(source, track.item_spans[index])?;
            if let Some(old) = old_fields.iter().find(|field| field.key == b"ease") {
                if replacement.last() == Some(&b'}') {
                    replacement.pop();
                    replacement.extend_from_slice(b",\"ease\":");
                    replacement.extend_from_slice(&source[old.value.start..old.value.end]);
                    replacement.push(b'}');
                }
            }
        }
    }
    let candidate = insert_item(source, track.items, &track.item_spans, replace.or(insert), replace.is_some(), &replacement)?;
    edit.commit(context, candidate)
}
pub(super) fn set_property_key(
    edit: &mut TimelineEdit,
    context: &TimelineContext,
    id: &[u8],
    tick: u32,
    value: &[u8],
    ease: Option<&[u8]>,
) -> EditResult {
    if id.is_empty() || value.is_empty() {
        return Err(at(Error::InvalidArgument, 0));
    }
    let (track, _) = selected(edit.source(), context, id)?;
    if track.drawing {
        return Err(at(Error::InvalidArgument, 0));
    }
    if tick >= track.duration {
        return Err(at(Error::InvalidValue, 0));
    }
    let key = make_key(tick, value, ease)?;
    change_item(edit, context, id, tick, &key, false, true)
}
pub(super) fn set_cel(
    edit: &mut TimelineEdit,
    context: &TimelineContext,
    id: &[u8],
    tick: u32,
    asset: &[u8],
    label: &[u8],
) -> EditResult {
    if !valid_asset(context, asset) {
        return Err(at(Error::InvalidReference, 0));
    }
    if !validate_utf8(label) || label.len() > crate::MAX_STRING_BYTES {
        return Err(at(Error::InvalidArgument, 0));
    }
    change_item(
        edit,
        context,
        id,
        tick,
        &make_cel(tick, asset, label)?,
        true,
        false,
    )
}
pub(super) fn remove_cel(
    edit: &mut TimelineEdit,
    context: &TimelineContext,
    id: &[u8],
    tick: u32,
) -> EditResult {
    let source = edit.source();
    let (track, _) = selected(source, context, id)?;
    if !track.drawing {
        return Err(at(Error::InvalidArgument, 0));
    }
    let idx = track
        .item_spans
        .iter()
        .enumerate()
        .find_map(|(i, s)| {
            let f = fields(source, *s).ok()?;
            let t = uint(source, field(&f, b"tick")?.value).ok()?;
            (t == tick).then_some(i)
        })
        .ok_or_else(|| at(Error::InvalidReference, 0))?;
    if track.item_spans.len() <= 1 {
        return Err(at(Error::InvalidValue, 0));
    }
    let span = track.item_spans[idx];
    let (start, end) = if idx == 0 {
        (span.start, track.item_spans[1].start)
    } else {
        let mut comma = track.item_spans[idx - 1].end;
        while comma < span.start && source[comma] != b',' {
            comma += 1;
        }
        if comma == span.start {
            return Err(at(Error::InvalidSchema, span.start));
        }
        (comma, span.end)
    };
    let candidate = splice(source, start, end - start, &[])?;
    edit.commit(context, candidate)
}

pub(super) fn move_item(
    edit: &mut TimelineEdit,
    context: &TimelineContext,
    id: &[u8],
    from: u32,
    to: u32,
) -> EditResult {
    if id.is_empty() {
        return Err(at(Error::InvalidArgument, 0));
    }
    let source = edit.source();
    let (track, _) = selected(source, context, id)?;
    let index = track
        .item_spans
        .iter()
        .enumerate()
        .find_map(|(i, s)| {
            let f = fields(source, *s).ok()?;
            let t = uint(source, field(&f, b"tick")?.value).ok()?;
            (t == from).then_some(i)
        })
        .ok_or_else(|| at(Error::InvalidReference, 0))?;
    if from == to {
        return Ok(());
    }
    if from == 0 || to >= track.duration {
        return Err(at(Error::InvalidValue, 0));
    }
    for (i, s) in track.item_spans.iter().enumerate() {
        if i != index {
            let f = fields(source, *s)?;
            let t = uint(
                source,
                field(&f, b"tick")
                    .ok_or_else(|| at(Error::InvalidSchema, s.start))?
                    .value,
            )
            .map_err(EditFailure::from)?;
            if t == to {
                return Err(at(Error::Duplicate, s.start));
            }
        }
    }
    let count = track.item_spans.len();
    let mut order = Vec::new();
    order
        .try_reserve_exact(count)
        .map_err(|_| at(Error::Limit, track.items.start))?;
    for i in 0..count {
        if i != index {
            let s = track.item_spans[i];
            let f = fields(source, s)?;
            let t = uint(
                source,
                field(&f, b"tick")
                    .ok_or_else(|| at(Error::InvalidSchema, s.start))?
                    .value,
            )
            .map_err(EditFailure::from)?;
            order.push((t, i));
        }
    }
    order.push((to, index));
    order.sort_by_key(|entry| entry.0);
    let mut out = Vec::new();
    out.try_reserve_exact(
        source
            .len()
            .checked_add(10)
            .ok_or_else(|| at(Error::Limit, 0))?,
    )
    .map_err(|_| at(Error::Limit, 0))?;
    append(&mut out, &source[..track.item_spans[0].start])?;
    for (position, (_, i)) in order.iter().enumerate() {
        let span = track.item_spans[*i];
        if *i == index {
            let f = fields(source, span)?;
            let tick_span = field(&f, b"tick")
                .ok_or_else(|| at(Error::InvalidSchema, span.start))?
                .value;
            append(&mut out, &source[span.start..tick_span.start])?;
            append(&mut out, to.to_string().as_bytes())?;
            append(&mut out, &source[tick_span.end..span.end])?;
        } else {
            append(&mut out, &source[span.start..span.end])?;
        }
        if position + 1 < count {
            let left = track.item_spans[position];
            let right = track.item_spans[position + 1];
            append(&mut out, &source[left.end..right.start])?;
        }
    }
    append(&mut out, &source[track.item_spans[count - 1].end..])?;
    edit.commit(context, out)
}

pub(super) fn set_loop(
    edit: &mut TimelineEdit,
    context: &TimelineContext,
    mode: LoopMode,
) -> EditResult {
    let source = edit.source();
    let view = decode(source, context).map_err(EditFailure::from)?;
    if view.loop_mode() == mode {
        return Ok(());
    }
    let root = fields(source, root_span(source)?)?;
    let f = field(&root, b"loop").ok_or_else(|| at(Error::InvalidSchema, 0))?;
    let name = match mode {
        LoopMode::Once => b"once".as_slice(),
        LoopMode::Repeat => b"loop",
        LoopMode::PingPong => b"ping_pong",
    };
    let encoded = json_string(name)?;
    let candidate = splice(source, f.value.start, f.value.end - f.value.start, &encoded)?;
    edit.commit(context, candidate)
}
pub(super) fn set_onion(
    edit: &mut TimelineEdit,
    context: &TimelineContext,
    before: u32,
    after: u32,
    opacity: f64,
) -> EditResult {
    if before > 8 || after > 8 || !opacity.is_finite() || !(0.0..=1.0).contains(&opacity) {
        return Err(at(Error::InvalidArgument, 0));
    }
    let source = edit.source();
    let view = decode(source, context).map_err(EditFailure::from)?;
    let current = view.onion();
    if current.0 == before && current.1 == after && current.2 == opacity {
        return Ok(());
    }
    let root = fields(source, root_span(source)?)?;
    let onion = field(&root, b"onion").ok_or_else(|| at(Error::InvalidSchema, 0))?;
    let f = fields(source, onion.value)?;
    let reserve = source
        .len()
        .checked_add(64)
        .ok_or_else(|| at(Error::Limit, onion.value.start))?;
    let mut patches = Vec::new();
    patches
        .try_reserve_exact(3)
        .map_err(|_| at(Error::Limit, onion.value.start))?;
    for (key, value) in [
        (b"before".as_slice(), before.to_string()),
        (b"after".as_slice(), after.to_string()),
        (b"opacity".as_slice(), opacity.to_string()),
    ] {
        let s = field(&f, key)
            .ok_or_else(|| at(Error::InvalidSchema, onion.value.start))?
            .value;
        patches.push((s, value.into_bytes()));
    }
    patches.sort_by_key(|p| p.0.start);
    let mut out = Vec::new();
    out.try_reserve_exact(reserve)
        .map_err(|_| at(Error::Limit, onion.value.start))?;
    let mut cursor = 0;
    for (span, value) in patches {
        append(&mut out, &source[cursor..span.start])?;
        append(&mut out, &value)?;
        cursor = span.end;
    }
    append(&mut out, &source[cursor..])?;
    edit.commit(context, out)
}

pub(super) fn set_track_visibility(
    edit: &mut TimelineEdit,
    context: &TimelineContext,
    id: &[u8],
    visible: bool,
) -> EditResult {
    if !valid_id(id) {
        return Err(at(Error::InvalidArgument, 0));
    }
    let source = edit.source();
    let (_, index) = selected(source, context, id)?;
    let root = fields(source, root_span(source)?)?;
    let tracks = field(&root, b"tracks")
        .ok_or_else(|| at(Error::InvalidSchema, 0))?
        .value;
    let spans = array_spans(source, tracks, MAX_TRACKS, None).map_err(EditFailure::from)?;
    let track_span = *spans
        .get(index)
        .ok_or_else(|| at(Error::InvalidSchema, tracks.start))?;
    let track_fields = fields(source, track_span)?;
    let value = field(&track_fields, b"visible")
        .ok_or_else(|| at(Error::InvalidSchema, track_span.start))?
        .value;
    let replacement = if visible {
        b"true".as_slice()
    } else {
        b"false".as_slice()
    };
    if &source[value.start..value.end] == replacement {
        return Ok(());
    }
    let candidate = splice(source, value.start, value.end - value.start, replacement)?;
    edit.commit(context, candidate)
}
