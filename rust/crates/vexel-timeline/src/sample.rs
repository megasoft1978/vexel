use crate::DecodeFailure;
use crate::json::{array_spans, object_fields};
use crate::schema::{Error, ItemView, TimelineView, TrackView};
use vexel_base::Span;

pub use crate::easing::{ease_value, Ease};

/// Translate/rotate/scale/skew transform with a pivot; components interpolate independently.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Trs {
    pub tx: f64,
    pub ty: f64,
    pub rotation_deg: f64,
    pub sx: f64,
    pub sy: f64,
    pub pivot_x: f64,
    pub pivot_y: f64,
    pub skew_x_deg: f64,
}

impl Trs {
    /// SVG style matrix `[a, b, c, d, e, f]` for
    /// `M = T(tx,ty) * T(pivot) * R * SkewX * S * T(-pivot)`.
    pub fn to_matrix(&self) -> [f64; 6] {
        let (sin, cos) = self.rotation_deg.to_radians().sin_cos();
        let tan = self.skew_x_deg.to_radians().tan();
        let a = cos * self.sx;
        let b = sin * self.sx;
        let c = (cos * tan - sin) * self.sy;
        let d = (sin * tan + cos) * self.sy;
        let e = self.tx + self.pivot_x - (a * self.pivot_x + c * self.pivot_y);
        let f = self.ty + self.pivot_y - (b * self.pivot_x + d * self.pivot_y);
        [a, b, c, d, e, f]
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PropertyValue {
    Trs(Trs),
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
    let mut cursor = crate::schema::TimelineIterator::default();
    let first = view
        .item_next(track_index, &mut cursor)
        .ok_or_else(|| failure(Error::InvalidSchema, 0))?;
    let mut lower = first;
    let mut upper = first;
    for _ in 1..track.item_count {
        let next = view
            .item_next(track_index, &mut cursor)
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
        let (t, step) = match lower.ease {
            Some(ease) => (ease.apply(t), ease.is_step()),
            None => {
                let t = match track.interpolation {
                    Some(mode) if mode == b"step" => 0.0,
                    Some(mode) if mode == b"linear" => t,
                    Some(mode) if mode == b"cubic" => 3.0 * t * t - 2.0 * t * t * t,
                    _ => return Err(failure(Error::InvalidValue, lower.source.start)),
                };
                (t, track.interpolation.is_some_and(|mode| mode == b"step"))
            }
        };
        interpolate(start, end, t, step, lower.source.start)?
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
        b"trs" => parse_trs(view, item),
        _ => Err(failure(Error::InvalidValue, span.start)),
    }
}

fn parse_trs(view: &TimelineView<'_>, item: ItemView<'_>) -> Result<PropertyValue, DecodeFailure> {
    let value = item
        .value_json
        .ok_or_else(|| failure(Error::InvalidSchema, item.source.start))?;
    let span = value_span(view, value)?;
    let fields = object_fields(view.source(), span, None)?;
    let get = |key: &[u8]| -> Result<Option<f64>, DecodeFailure> {
        match fields.iter().find(|f| f.key == key) {
            Some(f) => number(view.source(), f.value).map(Some),
            None => Ok(None),
        }
    };
    let need = |key: &[u8]| get(key)?.ok_or_else(|| failure(Error::InvalidValue, span.start));
    Ok(PropertyValue::Trs(Trs {
        tx: need(b"tx")?,
        ty: need(b"ty")?,
        rotation_deg: need(b"rotation_deg")?,
        sx: need(b"sx")?,
        sy: need(b"sy")?,
        pivot_x: need(b"pivot_x")?,
        pivot_y: need(b"pivot_y")?,
        skew_x_deg: get(b"skew_x_deg")?.unwrap_or(0.0),
    }))
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
        (PropertyValue::Trs(a), PropertyValue::Trs(b)) => Ok(PropertyValue::Trs(Trs {
            tx: mix(a.tx, b.tx),
            ty: mix(a.ty, b.ty),
            rotation_deg: mix(a.rotation_deg, b.rotation_deg),
            sx: mix(a.sx, b.sx),
            sy: mix(a.sy, b.sy),
            pivot_x: mix(a.pivot_x, b.pivot_x),
            pivot_y: mix(a.pivot_y, b.pivot_y),
            skew_x_deg: mix(a.skew_x_deg, b.skew_x_deg),
        })),
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
