use crate::control::Meter;
use crate::*;
use vexel_base::{Stop, WorkControl, WorkKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvaluationError {
    InvalidTime,
    InvalidTrack,
    OutOfMemory,
    Canceled,
    Limit,
    Overflow,
}

fn stop_error(stop: Stop) -> EvaluationError {
    match stop {
        Stop::Canceled => EvaluationError::Canceled,
        Stop::Limit => EvaluationError::Limit,
        Stop::Overflow => EvaluationError::Overflow,
    }
}

fn duration(t: &Track) -> f64 {
    let mut d = if t.duration_indefinite {
        f64::INFINITY
    } else {
        t.duration_seconds
    };
    if t.repeat_indefinite && d.is_finite() {
        d = f64::INFINITY
    } else if d.is_finite() {
        d *= t.repeat_count
    }
    if !t.repeat_duration_indefinite && t.repeat_duration < d {
        d = t.repeat_duration
    }
    if !d.is_finite() || d > MAX_TIMELINE_SECONDS {
        MAX_TIMELINE_SECONDS
    } else {
        d
    }
}
fn resolve(
    tracks: &[Track],
    i: usize,
    state: &mut [u8; MAX_TRACKS],
    starts: &mut [f64; MAX_TRACKS],
    meter: &mut Meter<'_>,
) -> Result<bool, Stop> {
    meter
        .charge(WorkKind::AnimationTransform, 1)
        .then_some(())
        .ok_or_else(|| meter.stop().unwrap_or(Stop::Canceled))?;
    if i >= tracks.len() {
        return Ok(false);
    }
    if state[i] == 2 {
        return Ok(true);
    }
    if state[i] == 1 {
        starts[i] = f64::INFINITY;
        state[i] = 2;
        return Ok(true);
    }
    state[i] = 1;
    let t = &tracks[i];
    if !t.supported || t.begin_indefinite {
        starts[i] = f64::INFINITY
    } else if !t.has_begin_sync {
        starts[i] = t.begin_offset
    } else {
        let Some(r) = t.begin_sync_track else {
            starts[i] = f64::INFINITY;
            state[i] = 2;
            return Ok(true);
        };
        if !resolve(tracks, r, state, starts, meter)? {
            return Ok(false);
        }
        let mut s = starts[r];
        if t.begin_sync_end {
            if !s.is_finite() {
                starts[i] = f64::INFINITY;
                state[i] = 2;
                return Ok(true);
            }
            s += duration(&tracks[r])
        }
        starts[i] = s + t.begin_offset
    }
    if starts[i].is_finite() && (starts[i] < 0. || starts[i] > MAX_TIMELINE_SECONDS) {
        starts[i] = f64::INFINITY
    }
    state[i] = 2;
    Ok(true)
}
fn active_duration(
    tracks: &[Track],
    i: usize,
    state: &[u8; MAX_TRACKS],
    starts: &[f64; MAX_TRACKS],
) -> f64 {
    let t = &tracks[i];
    let mut d = duration(t);
    if t.has_end {
        let mut end = t.end_seconds;
        if t.end_is_sync {
            let Some(r) = t.end_sync_track else { return 0. };
            if r >= tracks.len() || state[r] != 2 || !starts[r].is_finite() {
                return 0.;
            }
            end += starts[r] + duration(&tracks[r])
        }
        let e = end - starts[i];
        if e < d {
            d = e
        }
    }
    if d < 0. {
        0.
    } else {
        d.min(MAX_TIMELINE_SECONDS)
    }
}
fn distance(a: Value, b: Value) -> f64 {
    if a.kind != b.kind {
        return 0.;
    }
    match a.kind {
        ValueKind::Color => a
            .rgba
            .iter()
            .zip(b.rgba)
            .map(|(x, y)| (*x as f64 - y as f64).powi(2))
            .sum::<f64>()
            .sqrt(),
        ValueKind::Number => (b.number[0] - a.number[0]).abs(),
        ValueKind::Transform => {
            let n = (a.number[3] as usize).min(3);
            a.number[..n]
                .iter()
                .zip(&b.number[..n])
                .map(|(x, y)| (y - x).powi(2))
                .sum::<f64>()
                .sqrt()
        }
        _ => 0.,
    }
}
fn bezier(t: f64, a: f64, b: f64) -> f64 {
    let u = 1. - t;
    3. * u * u * t * a + 3. * u * t * t * b + t * t * t
}
fn spline(p: f64, c: [f64; 4], meter: &mut Meter<'_>) -> Result<f64, Stop> {
    let (mut l, mut h) = (0., 1.);
    let mut x = p;
    for _ in 0..48 {
        if !meter.charge(WorkKind::AnimationTransform, 1) {
            return Err(meter.stop().unwrap_or(Stop::Canceled));
        }
        x = (l + h) * 0.5;
        if bezier(x, c[0], c[2]) < p {
            l = x
        } else {
            h = x
        }
    }
    Ok(bezier(x, c[1], c[3]))
}
fn interpolate(t: &Track, p: f64, meter: &mut Meter<'_>) -> Result<Option<Value>, Stop> {
    let n = t.keys.len();
    if n == 0 {
        return Ok(None);
    }
    if n == 1 {
        return Ok(Some(t.keys[0].value));
    }
    if p <= 0. {
        return Ok(Some(t.keys[0].value));
    }
    if p >= 1. {
        return Ok(Some(t.keys[n - 1].value));
    }
    let (mut l, mut r, mut local) = (0, 1, 0.);
    if t.calc_mode == CalcMode::Paced {
        let mut total = 0.0;
        for i in 0..n - 1 {
            if !meter.charge(WorkKind::AnimationTransform, 1) {
                return Err(meter.stop().unwrap_or(Stop::Canceled));
            }
            total += distance(t.keys[i].value, t.keys[i + 1].value);
        }
        if total <= 0. {
            local = p
        } else {
            let wanted = p * total;
            let mut traversed = 0.;
            for i in 0..n - 1 {
                if !meter.charge(WorkKind::AnimationTransform, 1) {
                    return Err(meter.stop().unwrap_or(Stop::Canceled));
                }
                let d = distance(t.keys[i].value, t.keys[i + 1].value);
                if wanted <= traversed + d || i + 2 == n {
                    l = i;
                    r = i + 1;
                    local = if d > 0. { (wanted - traversed) / d } else { 0. };
                    break;
                }
                traversed += d
            }
        }
    } else {
        while r + 1 < n && p >= t.keys[r].time {
            if !meter.charge(WorkKind::AnimationTransform, 1) {
                return Err(meter.stop().unwrap_or(Stop::Canceled));
            }
            l += 1;
            r += 1
        }
        let dt = t.keys[r].time - t.keys[l].time;
        local = if dt > 0. {
            (p - t.keys[l].time) / dt
        } else {
            0.
        }
    }
    let a = t.keys[l].value;
    let b = t.keys[r].value;
    if t.calc_mode == CalcMode::Discrete || matches!(a.kind, ValueKind::Path | ValueKind::Motion) {
        return Ok(Some(if local < 1. { a } else { b }));
    }
    if t.calc_mode == CalcMode::Spline {
        local = spline(local, *t.splines.get(l).ok_or(Stop::Overflow)?, meter)?
    }
    if a.kind != b.kind {
        return Ok(None);
    }
    let mut v = a;
    match a.kind {
        ValueKind::Color => {
            for i in 0..4 {
                v.rgba[i] = (a.rgba[i] as f64 + (b.rgba[i] as f64 - a.rgba[i] as f64) * local)
                    .clamp(0., 255.)
                    .round() as u8
            }
            v.raw = Default::default()
        }
        ValueKind::Number => {
            v.number[0] = a.number[0] + (b.number[0] - a.number[0]) * local;
            if !v.number[0].is_finite() {
                return Ok(None);
            }
        }
        ValueKind::Transform => {
            let count = a.number[3] as usize;
            if count > 3 || count != b.number[3] as usize {
                return Ok(None);
            }
            for i in 0..count {
                v.number[i] = a.number[i] + (b.number[i] - a.number[i]) * local
            }
        }
        _ => (),
    }
    Ok(Some(v))
}
fn motion(t: &Track, p: f64, meter: &mut Meter<'_>) -> Result<Option<Value>, Stop> {
    if t.motion_points.len() < 2 || t.motion_length <= 0. {
        return Ok(None);
    }
    let wanted = p * t.motion_length;
    let mut r = 1;
    while r + 1 < t.motion_points.len() && wanted > t.motion_points[r].distance {
        if !meter.charge(WorkKind::AnimationTransform, 1) {
            return Err(meter.stop().unwrap_or(Stop::Canceled));
        }
        r += 1
    }
    let a = t.motion_points[r - 1];
    let b = t.motion_points[r];
    let d = b.distance - a.distance;
    let q = if d > 0. {
        ((wanted - a.distance) / d).clamp(0., 1.)
    } else {
        0.
    };
    let mut v = Value {
        kind: ValueKind::Motion,
        ..Value::default()
    };
    v.number[0] = a.x + (b.x - a.x) * q;
    v.number[1] = a.y + (b.y - a.y) * q;
    v.number[2] = if t.motion_rotate_auto {
        (b.y - a.y).atan2(b.x - a.x).to_degrees() + if t.motion_rotate_reverse { 180. } else { 0. }
    } else {
        t.motion_fixed_rotation
    };
    v.number[3] = 1.;
    [v.number[0], v.number[1], v.number[2]]
        .iter()
        .all(|x| x.is_finite())
        .then_some(v)
        .map_or(Ok(None), |value| Ok(Some(value)))
}
fn accumulate(t: &Track, iteration: usize, v: &mut Value) {
    if !t.accumulate || iteration == 0 || t.keys.len() < 2 {
        return;
    }
    let a = t.keys[0].value;
    let b = t.keys[t.keys.len() - 1].value;
    match v.kind {
        ValueKind::Number if a.kind == v.kind && b.kind == v.kind => {
            v.number[0] += iteration as f64 * (b.number[0] - a.number[0])
        }
        ValueKind::Color if a.kind == v.kind && b.kind == v.kind => {
            for c in 0..4 {
                v.rgba[c] = (v.rgba[c] as f64
                    + iteration as f64 * (b.rgba[c] as f64 - a.rgba[c] as f64))
                    .clamp(0., 255.)
                    .round() as u8
            }
        }
        _ => (),
    }
}
fn one(
    tracks: &[Track],
    i: usize,
    state: &[u8; MAX_TRACKS],
    starts: &[f64; MAX_TRACKS],
    now: f64,
    meter: &mut Meter<'_>,
) -> Result<Option<OverlayItem>, EvaluationError> {
    let t = &tracks[i];
    let mut out = OverlayItem {
        track_index: i,
        value: Value::default(),
        iteration: 0,
        active: false,
    };
    if !t.supported || !starts[i].is_finite() || now < starts[i] {
        return Ok(Some(out));
    }
    let dur = active_duration(tracks, i, state, starts);
    let elapsed = now - starts[i];
    let finished = elapsed >= dur;
    if finished && t.fill_mode != FillMode::Freeze {
        return Ok(Some(out));
    }
    let e = if finished { dur } else { elapsed };
    let (p, it) = if t.kind == Kind::Set || t.duration_indefinite || t.duration_seconds <= 0. {
        (1., 0)
    } else if finished {
        let repeats = dur / t.duration_seconds;
        let fract = repeats - repeats.floor();
        if fract > 0. && t.fill_mode == FillMode::Freeze {
            (fract, repeats.floor() as usize)
        } else {
            let iteration = if repeats > 0. {
                (repeats - 1e-12).floor() as usize
            } else {
                0
            };
            (1., iteration)
        }
    } else {
        (
            (e % t.duration_seconds) / t.duration_seconds,
            (e / t.duration_seconds).floor() as usize,
        )
    };
    let mut value = if t.kind == Kind::Motion {
        motion(t, p, meter)
            .map_err(stop_error)?
            .ok_or(EvaluationError::InvalidTrack)?
    } else {
        let mut v = interpolate(t, p, meter)
            .map_err(stop_error)?
            .ok_or(EvaluationError::InvalidTrack)?;
        accumulate(t, it, &mut v);
        v
    };
    if t.additive_sum && value.kind == ValueKind::Number && t.base_value.kind == ValueKind::Number {
        value.number[0] += t.base_value.number[0]
    }
    out.value = value;
    out.iteration = it;
    out.active = true;
    Ok(Some(out))
}

/// Evaluate every parsed track at a finite time in the supported timeline.
/// On invalid input or any failed evaluation, no partial overlay is returned.
pub fn evaluate(doc: &Document<'_>, time_seconds: f64) -> Option<Overlay> {
    evaluate_view(doc.view(), time_seconds)
}

/// Evaluate a borrowed document view using the same engine as `evaluate`.
pub fn evaluate_view(doc: DocumentView<'_>, time_seconds: f64) -> Option<Overlay> {
    evaluate_inner(doc, time_seconds, &mut Meter::unlimited()).ok()
}

pub fn evaluate_controlled(
    doc: &Document<'_>,
    time_seconds: f64,
    control: &mut dyn WorkControl,
) -> Result<Overlay, EvaluationError> {
    evaluate_view_controlled(doc.view(), time_seconds, control)
}

pub fn evaluate_view_controlled(
    doc: DocumentView<'_>,
    time_seconds: f64,
    control: &mut dyn WorkControl,
) -> Result<Overlay, EvaluationError> {
    evaluate_inner(doc, time_seconds, &mut Meter::controlled(control))
}

fn valid_value(value: Value) -> bool {
    match value.kind {
        ValueKind::Number => value.number[0].is_finite(),
        ValueKind::Transform => {
            let count = value.number[3];
            count.is_finite()
                && count.fract() == 0.0
                && (1.0..=3.0).contains(&count)
                && value.number[..count as usize].iter().all(|n| n.is_finite())
        }
        ValueKind::Motion => value.number.iter().all(|n| n.is_finite()),
        ValueKind::Color | ValueKind::Path => true,
    }
}

fn valid_track(t: &Track, meter: &mut Meter<'_>) -> Result<bool, Stop> {
    if t.keys.is_empty()
        || t.keys.len() > MAX_VALUES
        || t.motion_points.len() > 4096
        || t.splines.len() > MAX_VALUES
    {
        return Ok(false);
    }
    if t.calc_mode == CalcMode::Spline && t.splines.len() != t.keys.len().saturating_sub(1) {
        return Ok(false);
    }
    if t.kind == Kind::Motion
        && (t.motion_points.len() < 2 || !t.motion_length.is_finite() || t.motion_length <= 0.0)
    {
        return Ok(false);
    }
    if !valid_value(t.base_value) {
        return Ok(false);
    }
    if !t.duration_indefinite
        && t.kind != Kind::Set
        && (!t.duration_seconds.is_finite() || t.duration_seconds <= 0.0)
    {
        return Ok(false);
    }
    if !t.repeat_indefinite && (!t.repeat_count.is_finite() || t.repeat_count <= 0.0) {
        return Ok(false);
    }
    if t.repeat_duration.is_nan() || t.repeat_duration < 0.0 {
        return Ok(false);
    }
    if t.has_end && !t.end_seconds.is_finite() {
        return Ok(false);
    }
    let mut previous_distance = -1.0;
    for key in &t.keys {
        if !meter.charge(WorkKind::AnimationTransform, 1) {
            return Err(meter.stop().unwrap_or(Stop::Canceled));
        }
        if !key.time.is_finite() || !(0.0..=1.0).contains(&key.time) || !valid_value(key.value) {
            return Ok(false);
        }
    }
    for point in &t.motion_points {
        if !meter.charge(WorkKind::AnimationTransform, 1) {
            return Err(meter.stop().unwrap_or(Stop::Canceled));
        }
        if !point.x.is_finite()
            || !point.y.is_finite()
            || !point.distance.is_finite()
            || point.distance < previous_distance
        {
            return Ok(false);
        }
        previous_distance = point.distance;
    }
    for spline in &t.splines {
        if !meter.charge(WorkKind::AnimationTransform, 1) {
            return Err(meter.stop().unwrap_or(Stop::Canceled));
        }
        if spline
            .iter()
            .any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
        {
            return Ok(false);
        }
    }
    Ok(true)
}

fn evaluate_inner(
    doc: DocumentView<'_>,
    time_seconds: f64,
    meter: &mut Meter<'_>,
) -> Result<Overlay, EvaluationError> {
    if !time_seconds.is_finite() || !(0.0..=MAX_TIMELINE_SECONDS).contains(&time_seconds) {
        return Err(EvaluationError::InvalidTime);
    }
    if !meter.checkpoint() {
        return Err(stop_error(meter.stop().unwrap_or(Stop::Canceled)));
    }
    if doc.tracks.len() > MAX_TRACKS {
        return Err(EvaluationError::InvalidTrack);
    }
    for track in doc.tracks {
        if !meter.charge(WorkKind::AnimationTransform, 1) {
            return Err(stop_error(meter.stop().unwrap_or(Stop::Canceled)));
        }
        if !valid_track(track, meter).map_err(stop_error)? {
            return Err(EvaluationError::InvalidTrack);
        }
    }
    let mut items = Vec::new();
    items
        .try_reserve_exact(doc.tracks.len())
        .map_err(|_| EvaluationError::OutOfMemory)?;
    let mut state = [0u8; MAX_TRACKS];
    let mut starts = [0.0; MAX_TRACKS];
    for i in 0..doc.tracks.len() {
        if !resolve(doc.tracks, i, &mut state, &mut starts, meter).map_err(stop_error)? {
            return Err(EvaluationError::InvalidTrack);
        }
    }
    for i in 0..doc.tracks.len() {
        if !meter.charge(WorkKind::AnimationTransform, 1) {
            return Err(stop_error(meter.stop().unwrap_or(Stop::Canceled)));
        }
        items.push(
            one(doc.tracks, i, &state, &starts, time_seconds, meter)?
                .ok_or(EvaluationError::InvalidTrack)?,
        );
    }
    Ok(Overlay {
        items,
        time_seconds,
    })
}
