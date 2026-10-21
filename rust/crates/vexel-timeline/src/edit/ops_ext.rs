use super::{fail, valid_id, EditFailure, EditResult, TimelineEdit};
use crate::json::{array_spans, object_fields, Field};
use crate::schema::{field, string, uint};
use crate::{decode, Error, TimelineContext, MAX_TRACKS};
use vexel_base::Span;

fn fields(source: &[u8], span: Span) -> EditResult<Vec<Field>> {
    object_fields(source, span, None).map_err(EditFailure::from)
}
fn root(source: &[u8]) -> EditResult<Vec<Field>> {
    let mut reader = crate::json::Reader::new(source, None);
    reader.scan(0).map_err(EditFailure::from)?;
    fields(
        source,
        Span {
            start: 0,
            end: reader.cursor,
        },
    )
}
fn tracks(source: &[u8]) -> EditResult<(Span, Vec<Span>)> {
    let root = root(source)?;
    let span = field(&root, b"tracks")
        .ok_or_else(|| fail(Error::InvalidSchema, 0))?
        .value;
    let spans = array_spans(source, span, MAX_TRACKS, None).map_err(EditFailure::from)?;
    Ok((span, spans))
}
fn track_index(source: &[u8], spans: &[Span], id: &[u8]) -> EditResult<usize> {
    spans
        .iter()
        .position(|span| {
            fields(source, *span)
                .ok()
                .and_then(|f| field(&f, b"id").and_then(|v| string(source, v.value).ok()))
                .is_some_and(|value| value == id)
        })
        .ok_or_else(|| fail(Error::InvalidReference, 0))
}
fn commit_tracks(
    edit: &mut TimelineEdit,
    context: &TimelineContext,
    span: Span,
    spans: &[Span],
    order: &[usize],
    replacement: Option<(usize, &[u8])>,
) -> EditResult {
    let source = edit.source();
    let mut out = Vec::new();
    out.try_reserve_exact(
        source
            .len()
            .saturating_add(replacement.map_or(0, |(_, b)| b.len())),
    )
    .map_err(|_| fail(Error::Limit, span.start))?;
    out.extend_from_slice(&source[..spans.first().map_or(span.end - 1, |s| s.start)]);
    for (position, index) in order.iter().enumerate() {
        if position > 0 {
            out.push(b',');
        }
        if replacement.is_some_and(|(at, _)| at == *index) {
            out.extend_from_slice(replacement.unwrap().1);
        } else {
            out.extend_from_slice(&source[spans[*index].start..spans[*index].end]);
        }
    }
    out.extend_from_slice(&source[spans.last().map_or(span.end - 1, |s| s.end)..]);
    edit.commit(context, out)
}
pub(super) fn remove_property_key(
    edit: &mut TimelineEdit,
    context: &TimelineContext,
    id: &[u8],
    tick: u32,
) -> EditResult {
    let source = edit.source();
    let index = selected_property(source, context, id)?;
    let (items, spans) = property_items(source, index)?;
    let key = spans
        .iter()
        .position(|s| {
            fields(source, *s)
                .ok()
                .and_then(|f| field(&f, b"tick").and_then(|v| uint(source, v.value).ok()))
                == Some(tick)
        })
        .ok_or_else(|| fail(Error::InvalidReference, 0))?;
    if tick == 0 {
        return Err(fail(Error::InvalidValue, spans[key].start));
    }
    let mut out = Vec::new();
    out.try_reserve_exact(source.len())
        .map_err(|_| fail(Error::Limit, items.start))?;
    let s = spans[key];
    let (start, end) = if key == 0 {
        (s.start, spans[1].start)
    } else {
        (spans[key - 1].end, s.end)
    };
    out.extend_from_slice(&source[..start]);
    out.extend_from_slice(&source[end..]);
    edit.commit(context, out)
}
pub(super) fn move_property_key(
    edit: &mut TimelineEdit,
    context: &TimelineContext,
    id: &[u8],
    from: u32,
    to: u32,
) -> EditResult {
    if from == to {
        return Ok(());
    }
    let source = edit.source();
    let view = decode(source, context).map_err(EditFailure::from)?;
    let index = selected_property(source, context, id)?;
    if to >= view.duration_ticks() || from == 0 {
        return Err(fail(Error::InvalidValue, 0));
    }
    let (items, spans) = property_items(source, index)?;
    let idx = spans
        .iter()
        .position(|s| {
            fields(source, *s)
                .ok()
                .and_then(|f| field(&f, b"tick").and_then(|v| uint(source, v.value).ok()))
                == Some(from)
        })
        .ok_or_else(|| fail(Error::InvalidReference, 0))?;
    if spans.iter().enumerate().any(|(i, s)| {
        i != idx
            && fields(source, *s)
                .ok()
                .and_then(|f| field(&f, b"tick").and_then(|v| uint(source, v.value).ok()))
                == Some(to)
    }) {
        return Err(fail(Error::Duplicate, 0));
    }
    let span = spans[idx];
    let f = fields(source, span)?;
    let tick = field(&f, b"tick").unwrap().value;
    let mut item = Vec::new();
    item.extend_from_slice(&source[span.start..tick.start]);
    item.extend_from_slice(to.to_string().as_bytes());
    item.extend_from_slice(&source[tick.end..span.end]);
    let mut entries: Vec<(u32, Vec<u8>)> = Vec::new();
    for s in &spans {
        let t = fields(source, *s)?;
        let old = uint(source, field(&t, b"tick").unwrap().value).map_err(EditFailure::from)?;
        entries.push((
            if old == from { to } else { old },
            if old == from {
                item.clone()
            } else {
                source[s.start..s.end].to_vec()
            },
        ));
    }
    entries.sort_by_key(|e| e.0);
    let mut keys = Vec::new();
    keys.try_reserve_exact(entries.iter().map(|e| e.1.len() + 1).sum())
        .map_err(|_| fail(Error::Limit, items.start))?;
    for (i, (_, b)) in entries.iter().enumerate() {
        if i > 0 {
            keys.push(b',')
        }
        keys.extend_from_slice(b)
    }
    let mut out = Vec::new();
    out.extend_from_slice(&source[..spans[0].start]);
    out.extend_from_slice(&keys);
    out.extend_from_slice(&source[spans.last().unwrap().end..]);
    edit.commit(context, out)
}
fn selected_property(source: &[u8], _context: &TimelineContext, id: &[u8]) -> EditResult<usize> {
    let (_, spans) = tracks(source)?;
    let i = track_index(source, &spans, id)?;
    let f = fields(source, spans[i])?;
    if string(
        source,
        field(&f, b"kind")
            .ok_or_else(|| fail(Error::InvalidSchema, 0))?
            .value,
    )
    .map_err(EditFailure::from)?
        != b"property"
    {
        return Err(fail(Error::InvalidArgument, 0));
    }
    array_spans(
        source,
        field(&f, b"keys")
            .ok_or_else(|| fail(Error::InvalidSchema, 0))?
            .value,
        crate::MAX_ITEMS_PER_TRACK,
        None,
    )
    .map_err(EditFailure::from)?;
    Ok(i)
}
fn property_items(source: &[u8], index: usize) -> EditResult<(Span, Vec<Span>)> {
    let (_, spans) = tracks(source)?;
    let f = fields(source, spans[index])?;
    let items = field(&f, b"keys")
        .ok_or_else(|| fail(Error::InvalidSchema, 0))?
        .value;
    let keys =
        array_spans(source, items, crate::MAX_ITEMS_PER_TRACK, None).map_err(EditFailure::from)?;
    Ok((items, keys))
}

pub(super) fn remove_track(
    edit: &mut TimelineEdit,
    context: &TimelineContext,
    id: &[u8],
) -> EditResult {
    let source = edit.source();
    let (span, spans) = tracks(source)?;
    let idx = track_index(source, &spans, id)?;
    let order: Vec<_> = (0..spans.len()).filter(|i| *i != idx).collect();
    commit_tracks(edit, context, span, &spans, &order, None)
}
pub(super) fn duplicate_track(
    edit: &mut TimelineEdit,
    context: &TimelineContext,
    id: &[u8],
    new_id: &[u8],
) -> EditResult {
    if !valid_id(new_id) {
        return Err(fail(Error::InvalidArgument, 0));
    }
    let source = edit.source();
    let (span, spans) = tracks(source)?;
    if spans.len() >= MAX_TRACKS {
        return Err(fail(Error::Limit, span.start));
    }
    if spans.iter().any(|s| {
        fields(source, *s)
            .ok()
            .and_then(|f| field(&f, b"id").and_then(|v| string(source, v.value).ok()))
            .as_deref()
            == Some(new_id)
    }) {
        return Err(fail(Error::Duplicate, span.start));
    }
    let idx = track_index(source, &spans, id)?;
    let f = fields(source, spans[idx])?;
    let idspan = field(&f, b"id").unwrap().value;
    let mut copy = source[spans[idx].start..spans[idx].end].to_vec();
    let offset = idspan.start - spans[idx].start;
    let old_len = idspan.end - idspan.start;
    let mut encoded = Vec::new();
    encoded.push(b'"');
    encoded.extend_from_slice(new_id);
    encoded.push(b'"');
    copy.splice(offset..offset + old_len, encoded);
    let mut payload = Vec::new();
    payload
        .try_reserve_exact(copy.len() + 1)
        .map_err(|_| fail(Error::Limit, span.start))?;
    payload.push(b',');
    payload.extend_from_slice(&copy);
    let mut out = Vec::new();
    let at = spans.last().map_or(span.end - 1, |s| s.end);
    out.extend_from_slice(&source[..at]);
    out.extend_from_slice(&payload);
    out.extend_from_slice(&source[at..]);
    edit.commit(context, out)
}
pub(super) fn rename_track(
    edit: &mut TimelineEdit,
    context: &TimelineContext,
    id: &[u8],
    name: &[u8],
) -> EditResult {
    if std::str::from_utf8(name).is_err() || name.len() > crate::MAX_STRING_BYTES {
        return Err(fail(Error::InvalidArgument, 0));
    }
    let source = edit.source();
    let (_, spans) = tracks(source)?;
    let i = track_index(source, &spans, id)?;
    let f = fields(source, spans[i])?;
    if let Some(n) = field(&f, b"name") {
        let mut encoded = vec![b'"'];
        const HEX: &[u8] = b"0123456789abcdef";
        for b in name {
            match *b {
                b'"' | b'\\' => {
                    encoded.push(b'\\');
                    encoded.push(*b);
                }
                0..=31 => {
                    encoded.extend_from_slice(b"\\u00");
                    encoded.push(HEX[(b >> 4) as usize]);
                    encoded.push(HEX[(b & 15) as usize]);
                }
                _ => encoded.push(*b),
            }
        }
        encoded.push(b'"');
        let mut raw = source.to_vec();
        let start = n.value.start;
        let end = n.value.end;
        raw.splice(start..end, encoded);
        edit.commit(context, raw)
    } else {
        Ok(())
    }
}
pub(super) fn set_track_interpolation(
    edit: &mut TimelineEdit,
    context: &TimelineContext,
    id: &[u8],
    value: &[u8],
) -> EditResult {
    if ![b"step".as_slice(), b"linear", b"cubic"].contains(&value) {
        return Err(fail(Error::InvalidArgument, 0));
    }
    let source = edit.source();
    let (_, spans) = tracks(source)?;
    let i = track_index(source, &spans, id)?;
    let f = fields(source, spans[i])?;
    if string(
        source,
        field(&f, b"kind")
            .ok_or_else(|| fail(Error::InvalidSchema, 0))?
            .value,
    )
    .map_err(EditFailure::from)?
        != b"property"
    {
        return Err(fail(Error::InvalidArgument, 0));
    }
    let v = field(&f, b"interpolation").unwrap().value;
    let mut encoded = vec![b'"'];
    encoded.extend_from_slice(value);
    encoded.push(b'"');
    let mut out = source.to_vec();
    out.splice(v.start..v.end, encoded);
    edit.commit(context, out)
}
pub(super) fn reorder_track(
    edit: &mut TimelineEdit,
    context: &TimelineContext,
    id: &[u8],
    new_index: usize,
) -> EditResult {
    let source = edit.source();
    let (span, spans) = tracks(source)?;
    if new_index >= spans.len() {
        return Err(fail(Error::InvalidValue, span.start));
    }
    let i = track_index(source, &spans, id)?;
    if i == new_index {
        return Ok(());
    }
    let mut order: Vec<_> = (0..spans.len()).collect();
    let item = order.remove(i);
    order.insert(new_index, item);
    commit_tracks(edit, context, span, &spans, &order, None)
}

#[cfg(test)]
#[path = "ops_ext_tests.rs"]
mod tests;
