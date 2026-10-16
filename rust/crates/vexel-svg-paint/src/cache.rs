use crate::control::PaintControl;
use crate::{MAX_GRADIENT_STOPS, MAX_STOPS, MAX_TOKEN_VISITS};
use std::sync::Arc;
use vexel_base::{WorkControl, WorkKind};
use vexel_geometry::{affine_identity, affine_valid};
use vexel_scene::{
    AllocationTag, Diagnostic, ElementId, ErrorKind, GradientCacheContext, GradientCacheEntry,
    LimitKind, Paint, PaintCache, PaintKind, PaintStop, Result, SourceOwner, TableRange,
};

pub(crate) const MAX_GRADIENTS: usize = 32_768;
pub(crate) const CACHE_SLOTS: usize = MAX_GRADIENTS * 2;

pub(crate) fn invalid(offset: usize) -> Diagnostic {
    Diagnostic {
        kind: ErrorKind::InvalidArgument,
        offset,
    }
}
pub(crate) fn unsupported(offset: usize) -> Diagnostic {
    Diagnostic {
        kind: ErrorKind::Unsupported,
        offset,
    }
}
pub(crate) fn limit(kind: LimitKind, offset: usize) -> Diagnostic {
    Diagnostic {
        kind: ErrorKind::Limit(kind),
        offset,
    }
}
pub(crate) fn oom(offset: usize) -> Diagnostic {
    Diagnostic {
        kind: ErrorKind::OutOfMemory,
        offset,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Key {
    definition: u32,
    width: u64,
    height: u64,
}

fn key(definition: ElementId, viewport: [f64; 2], offset: usize) -> Result<Key> {
    if definition.0 > u32::MAX as usize {
        return Err(limit(LimitKind::PaintVisits, offset));
    }
    if !viewport
        .into_iter()
        .all(|value| value.is_finite() && value > 0.0)
    {
        return Err(unsupported(offset));
    }
    Ok(Key {
        definition: definition.0 as u32,
        width: viewport[0].to_bits(),
        height: viewport[1].to_bits(),
    })
}

fn hash(key: Key) -> usize {
    let mut value =
        u64::from(key.definition) ^ key.width ^ key.height.wrapping_add(0x9e37_79b9_7f4a_7c15);
    value ^= value >> 30;
    value = value.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^= value >> 31;
    value as usize & (CACHE_SLOTS - 1)
}

pub(crate) fn token_visit(
    cache: &mut PaintCache,
    control: &mut PaintControl<'_>,
    offset: usize,
) -> Result<()> {
    let next = cache
        .token_visits
        .checked_add(1)
        .ok_or_else(|| limit(LimitKind::Arithmetic, offset))?;
    if next > MAX_TOKEN_VISITS {
        return Err(limit(LimitKind::PaintVisits, offset));
    }
    control
        .charge(WorkKind::TokenVisit, 1)
        .map_err(|_| limit(LimitKind::Work, offset))?;
    cache.token_visits = next;
    Ok(())
}

pub(crate) fn probe(control: &mut PaintControl<'_>, offset: usize) -> Result<()> {
    control
        .charge(WorkKind::ReferenceProbe, 1)
        .map_err(|_| limit(LimitKind::Work, offset))
}

fn template_context(viewport: [f64; 2]) -> GradientCacheContext {
    GradientCacheContext {
        viewport,
        user_inverse: affine_identity(),
        bounds: [0.0; 4],
    }
}

fn entry_valid(
    entry: &GradientCacheEntry,
    source: &SourceOwner,
    key: Key,
    kind: PaintKind,
) -> bool {
    let context = entry.context;
    entry.definition.0 == key.definition as usize
        && Arc::ptr_eq(&entry.source, source)
        && context.viewport.map(f64::to_bits) == [key.width, key.height]
        && context.user_inverse == affine_identity()
        && context.bounds == [0.0; 4]
        && entry.stop_range == entry.paint.stops
        && entry.paint.user_inverse == affine_identity()
        && entry.paint.bounds == [0.0; 4]
        && affine_valid(entry.paint.gradient_inverse)
        && entry.paint.coordinates.into_iter().all(f64::is_finite)
        && (entry.paint.kind != PaintKind::Radial || entry.paint.coordinates[2] > 0.0)
        && entry.paint.pattern.is_none()
        && entry
            .stop_range
            .slice(&entry.stops)
            .is_some_and(valid_stops)
        && entry.stops.len() <= MAX_GRADIENT_STOPS
        && entry.paint.kind == kind
}

fn valid_stops(stops: &[PaintStop]) -> bool {
    stops.len() >= 2
        && stops
            .iter()
            .all(|stop| stop.offset.is_finite() && (0.0..=1.0).contains(&stop.offset))
        && stops
            .windows(2)
            .all(|pair| pair[1].offset >= pair[0].offset)
}

/// Find a source-owned template, validating every slot before indexing. The
/// cache is the accepted scene contract; there is no paint-local replacement.
pub(crate) fn find_template(
    cache: &PaintCache,
    source: &SourceOwner,
    definition: ElementId,
    viewport: [f64; 2],
    kind: PaintKind,
    control: &mut PaintControl<'_>,
    offset: usize,
) -> Result<Option<usize>> {
    if cache.entries.len() > MAX_GRADIENTS
        || cache.slots.len() > CACHE_SLOTS
        || cache.token_visits > MAX_TOKEN_VISITS
        || (!cache.entries.is_empty() && cache.slots.len() != CACHE_SLOTS)
    {
        return Err(limit(LimitKind::PaintVisits, offset));
    }
    let wanted = key(definition, viewport, offset)?;
    if cache.slots.is_empty() {
        return Ok(None);
    }
    let mut slot = hash(wanted);
    for _ in 0..CACHE_SLOTS {
        probe(control, offset)?;
        match cache.slots.get(slot).copied().flatten() {
            None => return Ok(None),
            Some(index) => {
                let entry = cache.entries.get(index).ok_or_else(|| invalid(offset))?;
                if !Arc::ptr_eq(&entry.source, source) {
                    return Err(invalid(offset));
                }
                let candidate = key(entry.definition, entry.context.viewport, offset)
                    .map_err(|_| invalid(offset))?;
                if candidate == wanted && Arc::ptr_eq(&entry.source, source) {
                    if !entry_valid(entry, source, wanted, kind) {
                        return Err(invalid(offset));
                    }
                    return Ok(Some(index));
                }
            }
        }
        slot = (slot + 1) & (CACHE_SLOTS - 1);
    }
    Ok(None)
}

/// Transfer an admitted cold stop vector into the shared cache without copying.
/// Capacity leases remain in this source job until its cache and ledger end.
pub(crate) fn insert_template(
    cache: &mut PaintCache,
    source: &SourceOwner,
    definition: ElementId,
    viewport: [f64; 2],
    paint: Paint,
    stops: Vec<PaintStop>,
    stop_lease: vexel_scene::AllocationLease,
    control: &mut PaintControl<'_>,
    offset: usize,
) -> Result<usize> {
    let result = insert_inner(
        cache, source, definition, viewport, paint, stops, control, offset,
    );
    if result.is_err() {
        control.ledger.release(stop_lease)?;
    }
    result
}

fn insert_inner(
    cache: &mut PaintCache,
    source: &SourceOwner,
    definition: ElementId,
    viewport: [f64; 2],
    paint: Paint,
    stops: Vec<PaintStop>,
    control: &mut PaintControl<'_>,
    offset: usize,
) -> Result<usize> {
    let wanted = key(definition, viewport, offset)?;
    if cache.entries.len() >= MAX_GRADIENTS {
        return Err(limit(LimitKind::PaintVisits, offset));
    }
    let mut old_stops = 0usize;
    for entry in &cache.entries {
        control
            .charge(WorkKind::PaintVisit, 1)
            .map_err(|_| limit(LimitKind::Work, offset))?;
        old_stops = old_stops
            .checked_add(entry.stops.len())
            .ok_or_else(|| limit(LimitKind::Arithmetic, offset))?;
    }
    if stops.len() > MAX_GRADIENT_STOPS
        || old_stops > MAX_STOPS
        || stops.len() > MAX_STOPS - old_stops
    {
        return Err(limit(LimitKind::PaintStops, offset));
    }
    if !valid_stops(&stops)
        || paint.stops
            != (TableRange {
                first: 0,
                count: stops.len(),
            })
        || (!cache.slots.is_empty() && cache.slots.len() != CACHE_SLOTS)
    {
        return Err(invalid(offset));
    }
    let mut slot = hash(wanted);
    if !cache.slots.is_empty() {
        let mut available = false;
        for _ in 0..CACHE_SLOTS {
            probe(control, offset)?;
            if cache.slots[slot].is_none() {
                available = true;
                break;
            }
            slot = (slot + 1) & (CACHE_SLOTS - 1);
        }
        if !available {
            return Err(limit(LimitKind::PaintVisits, offset));
        }
    }
    let target_entries = if cache.entries.len() < cache.entries.capacity() {
        cache.entries.capacity()
    } else if cache.entries.capacity() == 0 {
        16
    } else {
        cache
            .entries
            .capacity()
            .checked_mul(2)
            .ok_or_else(|| limit(LimitKind::Arithmetic, offset))?
            .min(MAX_GRADIENTS)
    };
    let mut slots = replacement(&cache.slots, CACHE_SLOTS, control, offset)?;
    let mut entries = match replacement(&cache.entries, target_entries, control, offset) {
        Ok(value) => value,
        Err(error) => {
            if let Some(buffer) = slots {
                control.ledger.release(buffer.lease)?;
            }
            return Err(error);
        }
    };
    // Every fallible control operation precedes cache mutation.
    let prepared = (|| {
        for bytes in [
            if slots.is_some() {
                cache
                    .slots
                    .len()
                    .checked_mul(std::mem::size_of::<Option<usize>>())
            } else {
                Some(0)
            },
            if entries.is_some() {
                cache
                    .entries
                    .len()
                    .checked_mul(std::mem::size_of::<GradientCacheEntry>())
            } else {
                Some(0)
            },
        ] {
            let mut bytes = bytes.ok_or_else(|| limit(LimitKind::Arithmetic, offset))?;
            while bytes != 0 {
                let batch = bytes.min(256);
                control
                    .charge(WorkKind::SourceRead, batch as u64)
                    .map_err(|_| limit(LimitKind::Work, offset))?;
                bytes -= batch;
            }
        }
        control
            .checkpoint()
            .map_err(|_| limit(LimitKind::Work, offset))
    })();
    if let Err(error) = prepared {
        if let Some(buffer) = slots {
            control.ledger.release(buffer.lease)?;
        }
        if let Some(buffer) = entries {
            control.ledger.release(buffer.lease)?;
        }
        return Err(error);
    }
    // The full replacements were admitted during overlap. Retain only their
    // growth after old metadata is dropped; previously admitted job capacities
    // remain represented by the old job leases.
    if let Some(buffer) = &slots {
        reconcile(buffer, control)?;
    }
    if let Some(buffer) = &entries {
        reconcile(buffer, control)?;
    }
    if let Some(mut buffer) = slots.take() {
        buffer.values.append(&mut cache.slots);
        cache.slots = buffer.values;
    }
    if let Some(mut buffer) = entries.take() {
        buffer.values.append(&mut cache.entries);
        cache.entries = buffer.values;
    }
    cache.slots.resize(CACHE_SLOTS, None);
    let index = cache.entries.len();
    cache.entries.push(GradientCacheEntry {
        definition,
        context: template_context(viewport),
        paint,
        stop_range: paint.stops,
        stops,
        source: Arc::clone(source),
    });
    cache.slots[slot] = Some(index);
    Ok(index)
}

struct Replacement<T> {
    values: Vec<T>,
    lease: vexel_scene::AllocationLease,
    old_bytes: usize,
    new_bytes: usize,
}

fn replacement<T>(
    old: &Vec<T>,
    target: usize,
    control: &mut PaintControl<'_>,
    offset: usize,
) -> Result<Option<Replacement<T>>> {
    if old.capacity() >= target {
        return Ok(None);
    }
    let size = std::mem::size_of::<T>();
    let requested = target
        .checked_mul(size)
        .ok_or_else(|| limit(LimitKind::Arithmetic, offset))?;
    if requested > control.ledger.available() {
        return Err(limit(LimitKind::ReservedBytes, offset));
    }
    let mut values = Vec::new();
    values.try_reserve_exact(target).map_err(|_| oom(offset))?;
    let new_bytes = values
        .capacity()
        .checked_mul(size)
        .ok_or_else(|| limit(LimitKind::Arithmetic, offset))?;
    let old_bytes = old
        .capacity()
        .checked_mul(size)
        .ok_or_else(|| limit(LimitKind::Arithmetic, offset))?;
    let lease = control.ledger.admit(AllocationTag::None, new_bytes)?;
    Ok(Some(Replacement {
        values,
        lease,
        old_bytes,
        new_bytes,
    }))
}

fn reconcile<T>(buffer: &Replacement<T>, control: &mut PaintControl<'_>) -> Result<()> {
    if buffer.old_bytes == 0 {
        return Ok(());
    }
    control.ledger.release(buffer.lease)?;
    let _job_lease = control
        .ledger
        .admit(AllocationTag::None, buffer.new_bytes - buffer.old_bytes)?;
    Ok(())
}
