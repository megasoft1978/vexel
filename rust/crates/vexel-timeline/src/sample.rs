use crate::DecodeFailure;
use crate::json::array_spans;
use crate::schema::{Error, ItemView, TimelineView, TrackView};
use vexel_base::Span;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PropertyValue {
    Scalar(f64),
    Transform2d([f64; 6]),
    Rgba([f64; 4]),
    Boolean(bool),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PropertySample<'a> {
    pub track_index: usize,
    pub target: &'a [u8],
    pub property: &'a [u8],
    pub value: PropertyValue,
}

/// Samples all visible property tracks at an integer timeline tick.
/// Results retain serialized track order. Drawing cels are evaluated by the
/// separate drawing composition path.
pub fn sample_properties<'a>(
    view: &TimelineView<'a>,
    tick: u32,
) -> Result<Vec<PropertySample<'a>>, DecodeFailure> {
    if tick >= view.duration_ticks() {
        return Err(failure(Error::InvalidArgument, 0));
    }
    let mut samples = Vec::new();
    samples
        .try_reserve(view.track_count())
        .map_err(|_| failure(Error::Limit, 0))?;
    for track_index in 0..view.track_count() {
        let track = view
            .track_at(track_index)
            .ok_or_else(|| failure(Error::InvalidSchema, 0))?;
        if track.kind != b"property" || !track.visible {
            continue;
        }
        samples.push(sample_track(view, track_index, track, tick)?);
    }
    Ok(samples)
}

fn sample_track<'a>(
    view: &TimelineView<'a>,
    track_index: usize,
    track: TrackView<'a>,
    tick: u32,
) -> Result<PropertySample<'a>, DecodeFailure> {
    let first = view
        .item_at(track_index, 0)
        .ok_or_else(|| failure(Error::InvalidSchema, 0))?;
    let mut lower = first;
    let mut upper = first;
    for index in 1..track.item_count {
        let next = view
            .item_at(track_index, index)
            .ok_or_else(|| failure(Error::InvalidSchema, 0))?;
        if next.tick > tick {
            upper = next;
            break;
        }
        lower = next;
        upper = next;
    }
    let value = if lower.tick == upper.tick || tick <= lower.tick {
        parse_value(
            view,
            lower,
            track
                .value_type
                .ok_or_else(|| failure(Error::InvalidSchema, lower.source.start))?,
        )?
    } else {
        let start = parse_value(
            view,
            lower,
            track
                .value_type
                .ok_or_else(|| failure(Error::InvalidSchema, lower.source.start))?,
        )?;
        let end = parse_value(
            view,
            upper,
            track
                .value_type
                .ok_or_else(|| failure(Error::InvalidSchema, upper.source.start))?,
        )?;
        let t = f64::from(tick - lower.tick) / f64::from(upper.tick - lower.tick);
        let t = match track.interpolation {
            Some(mode) if mode == b"step" => 0.0,
            Some(mode) if mode == b"linear" => t,
            Some(mode) if mode == b"cubic" => 3.0 * t * t - 2.0 * t * t * t,
            _ => return Err(failure(Error::InvalidValue, lower.source.start)),
        };
        interpolate(
            start,
            end,
            t,
            track.interpolation.is_some_and(|mode| mode == b"step"),
            lower.source.start,
        )?
    };
    Ok(PropertySample {
        track_index,
        target: track.target,
        property: track
            .property
            .ok_or_else(|| failure(Error::InvalidSchema, 0))?,
        value,
    })
}

fn parse_value(
    view: &TimelineView<'_>,
    item: ItemView<'_>,
    kind: &[u8],
) -> Result<PropertyValue, DecodeFailure> {
    let span = item.source;
    match kind {
        b"boolean" => match item.value_json {
            Some(value) if value == b"true" => Ok(PropertyValue::Boolean(true)),
            Some(value) if value == b"false" => Ok(PropertyValue::Boolean(false)),
            _ => Err(failure(Error::InvalidValue, span.start)),
        },
        b"scalar" => {
            let value = item
                .value_json
                .ok_or_else(|| failure(Error::InvalidSchema, span.start))?;
            let value_span = value_span(view, value)?;
            Ok(PropertyValue::Scalar(number(view.source(), value_span)?))
        }
        b"transform2d" => Ok(PropertyValue::Transform2d(numbers::<6>(view, item)?)),
        b"rgba" => Ok(PropertyValue::Rgba(numbers::<4>(view, item)?)),
        _ => Err(failure(Error::InvalidValue, span.start)),
    }
}

fn numbers<const N: usize>(
    view: &TimelineView<'_>,
    item: ItemView<'_>,
) -> Result<[f64; N], DecodeFailure> {
    let value = item
        .value_json
        .ok_or_else(|| failure(Error::InvalidSchema, item.source.start))?;
    let value_span = value_span(view, value)?;
    let spans = array_spans(view.source(), value_span, N, None)?;
    if spans.len() != N {
        return Err(failure(Error::InvalidValue, value_span.start));
    }
    let mut result = [0.0; N];
    for (slot, span) in result.iter_mut().zip(spans) {
        *slot = number(view.source(), span)?;
    }
    Ok(result)
}

fn value_span(view: &TimelineView<'_>, value: &[u8]) -> Result<Span, DecodeFailure> {
    let source = view.source();
    let source_start = source.as_ptr() as usize;
    let value_start = value.as_ptr() as usize;
    let source_end = source_start
        .checked_add(source.len())
        .ok_or_else(|| failure(Error::InvalidSchema, 0))?;
    let value_end = value_start
        .checked_add(value.len())
        .ok_or_else(|| failure(Error::InvalidSchema, 0))?;
    if value_start < source_start || value_end > source_end {
        return Err(failure(Error::InvalidSchema, 0));
    }
    let start = value_start - source_start;
    let end = value_end - source_start;
    if source.get(start..end) != Some(value) {
        return Err(failure(Error::InvalidSchema, start));
    }
    Ok(Span { start, end })
}

fn number(source: &[u8], span: vexel_base::Span) -> Result<f64, DecodeFailure> {
    let raw = source
        .get(span.start..span.end)
        .ok_or_else(|| failure(Error::InvalidValue, span.start))?;
    let text = std::str::from_utf8(raw).map_err(|_| failure(Error::InvalidValue, span.start))?;
    let value = text
        .parse::<f64>()
        .map_err(|_| failure(Error::InvalidValue, span.start))?;
    if value.is_finite() {
        Ok(value)
    } else {
        Err(failure(Error::InvalidValue, span.start))
    }
}

fn interpolate(
    start: PropertyValue,
    end: PropertyValue,
    t: f64,
    step: bool,
    offset: usize,
) -> Result<PropertyValue, DecodeFailure> {
    if step {
        return Ok(start);
    }
    let mix = |a: f64, b: f64| a + (b - a) * t;
    match (start, end) {
        (PropertyValue::Scalar(a), PropertyValue::Scalar(b)) => {
            Ok(PropertyValue::Scalar(mix(a, b)))
        }
        (PropertyValue::Transform2d(a), PropertyValue::Transform2d(b)) => {
            Ok(PropertyValue::Transform2d(std::array::from_fn(|i| {
                mix(a[i], b[i])
            })))
        }
        (PropertyValue::Rgba(a), PropertyValue::Rgba(b)) => {
            Ok(PropertyValue::Rgba(std::array::from_fn(|i| {
                mix(a[i], b[i])
            })))
        }
        (PropertyValue::Boolean(_), PropertyValue::Boolean(_)) => {
            Err(failure(Error::InvalidValue, offset))
        }
        _ => Err(failure(Error::InvalidValue, offset)),
    }
}

fn failure(error: Error, offset: usize) -> DecodeFailure {
    DecodeFailure {
        error,
        offset,
        canceled: false,
    }
}
