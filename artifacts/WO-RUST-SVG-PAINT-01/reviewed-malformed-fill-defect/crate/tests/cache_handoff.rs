use std::sync::Arc;
use vexel_geometry::affine_identity;
use vexel_scene::*;
use vexel_svg_paint::{
    CachedResolveContext, resolve_cached_context, resolve_cached_fill, resolve_fill,
};
use vexel_xml::XmlSpan;

const SOURCE: &[u8] = b"<svg><linearGradient id='g'><stop stop-color='red'/><stop offset='1' stop-color='blue'/></linearGradient><rect fill='url(#g)'/></svg>";
struct Run;
impl WorkControl for Run {
    fn charge(&mut self, _: WorkKind, _: u64) -> std::result::Result<(), Stop> {
        Ok(())
    }
    fn checkpoint(&mut self) -> std::result::Result<(), Stop> {
        Ok(())
    }
}
fn fill() -> XmlSpan {
    XmlSpan {
        offset: SOURCE.windows(7).position(|v| v == b"url(#g)").unwrap(),
        length: 7,
    }
}
fn edges() -> [Edge; 1] {
    [Edge {
        a: Point { x: 0., y: 0. },
        b: Point { x: 10., y: 10. },
    }]
}

#[test]
fn temporary_context_ends_but_cache_borrow_remains_and_40000_hits_allocate_no_stops() {
    let source = Arc::new(SOURCE.to_vec());
    let doc = LexicalDocument::parse(source.clone(), 1 << 20, &mut Run).unwrap();
    let mut shared = DecodeSource::new(&doc, &mut Run).unwrap();
    let mut cache = PaintCache::default();
    let mut ledger = ChargeLedger::new(&source, 1 << 24, 64).unwrap();
    let cached = {
        let ctx = CachedResolveContext {
            shared: &mut shared,
            cache: &mut cache,
            ledger: &mut ledger,
        };
        resolve_cached_context(
            ctx,
            fill(),
            &edges(),
            affine_identity(),
            [10., 10.],
            &mut Run,
        )
        .unwrap()
    };
    // These independent mutable borrows compile while cached stops remain live.
    ledger.work.charge(WorkKind::PaintVisit, 1).unwrap();
    let _ = shared.source();
    let pointer = cached.stops.as_ptr();
    let identity = cached.template.clone();
    assert_eq!(cached.stops.len(), 2);
    assert_eq!(cached.paint.bounds, [0., 0., 10., 10.]);
    let before = ledger.stats();
    let capacities = (
        cache.entries.capacity(),
        cache.slots.capacity(),
        cache.entries[0].stops.capacity(),
    );
    assert_eq!(pointer, cache.entries[0].stops.as_ptr());
    assert!(before.live_bytes >= source.capacity() + cache.storage_bytes().unwrap());
    for n in 0..40_000 {
        let inverse = Affine {
            a: 0.5,
            d: 0.5,
            e: (n % 3) as f64,
            ..affine_identity()
        };
        let cached = resolve_cached_fill(
            &mut shared,
            &mut cache,
            &mut ledger,
            fill(),
            &edges(),
            inverse,
            [10., 10.],
            &mut Run,
        )
        .unwrap();
        assert_eq!(cached.stops.as_ptr(), pointer);
        assert!(identity.same_template(&cached.template));
        assert_eq!(cached.paint.user_inverse, inverse);
        assert_eq!(cached.paint.bounds, [inverse.e, 0., 5. + inverse.e, 5.]);
    }
    assert_eq!(ledger.stats(), before);
    assert_eq!(
        capacities,
        (
            cache.entries.capacity(),
            cache.slots.capacity(),
            cache.entries[0].stops.capacity()
        )
    );
    assert_eq!(cache.entries.len(), 1);
}

#[test]
fn owned_conversion_has_real_payload_distinct_allocation_and_releasable_capacity() {
    let source = Arc::new(SOURCE.to_vec());
    let doc = LexicalDocument::parse(source.clone(), 1 << 20, &mut Run).unwrap();
    let mut shared = DecodeSource::new(&doc, &mut Run).unwrap();
    let mut cache = PaintCache::default();
    let mut ledger = ChargeLedger::new(&source, 1 << 24, 64).unwrap();
    let cached = resolve_cached_fill(
        &mut shared,
        &mut cache,
        &mut ledger,
        fill(),
        &edges(),
        affine_identity(),
        [10., 10.],
        &mut Run,
    )
    .unwrap();
    let before = ledger.stats();
    let owned = cached.to_owned(&mut ledger, &mut Run).unwrap();
    assert_eq!(cached.stops, owned.as_resolved().stops);
    assert_ne!(cached.stops.as_ptr(), owned.as_resolved().stops.as_ptr());
    let bytes = owned.as_resolved().stops.capacity() * std::mem::size_of::<PaintStop>();
    assert_eq!(ledger.stats().live_bytes - before.live_bytes, bytes);
    assert_eq!(ledger.stats().allocation_count, before.allocation_count + 1);
    let mut wrong_job = ChargeLedger::new(&source, 1 << 24, 64).unwrap();
    let (error, owned) = owned.release(&mut wrong_job).unwrap_err();
    assert_eq!(error.kind, ErrorKind::InvalidArgument);
    assert_eq!(ledger.stats().live_bytes - before.live_bytes, bytes);
    owned.release(&mut ledger).unwrap();
    assert_eq!(ledger.stats().live_bytes, before.live_bytes);
    // Frozen owned ResolveContext producer still returns a complete payload,
    // with capacity explicitly retained in job accounting.
    let mut ctx = ResolveContext {
        shared: &mut shared,
        cache: &mut cache,
        ledger: &mut ledger,
    };
    let result = resolve_fill(
        &mut ctx,
        fill(),
        &edges(),
        affine_identity(),
        [10., 10.],
        &mut Run,
    )
    .unwrap();
    assert_eq!(result.stops.len(), 2);
    assert!(result.template.is_some());
    assert_eq!(
        ctx.ledger.stats().live_bytes - before.live_bytes,
        result.stops.capacity() * std::mem::size_of::<PaintStop>()
    );
}

#[test]
fn wrong_job_source_and_cold_budget_rejection_do_not_publish_or_leak_cache_buffers() {
    let source = Arc::new(SOURCE.to_vec());
    let doc = LexicalDocument::parse(source.clone(), 1 << 20, &mut Run).unwrap();
    let mut shared = DecodeSource::new(&doc, &mut Run).unwrap();
    let other = Arc::new(SOURCE.to_vec());
    let mut wrong = ChargeLedger::new(&other, 1 << 24, 64).unwrap();
    let mut cache = PaintCache::default();
    let error = resolve_cached_fill(
        &mut shared,
        &mut cache,
        &mut wrong,
        fill(),
        &edges(),
        affine_identity(),
        [10., 10.],
        &mut Run,
    )
    .unwrap_err();
    assert_eq!(error.kind, ErrorKind::InvalidArgument);
    let mut ledger = ChargeLedger::new(&source, source.capacity() + 4096, 64).unwrap();
    let error = resolve_cached_fill(
        &mut shared,
        &mut cache,
        &mut ledger,
        fill(),
        &edges(),
        affine_identity(),
        [10., 10.],
        &mut Run,
    )
    .unwrap_err();
    assert_eq!(error.kind, ErrorKind::Limit(LimitKind::ReservedBytes));
    assert_eq!(error.offset, fill().offset);
    assert_eq!(cache.storage_bytes().unwrap(), 0);
    assert!(cache.entries.is_empty());
    assert_eq!(ledger.stats().source_bytes, source.capacity());
}

#[test]
fn cold_cache_has_one_stop_allocation_and_metadata_growth_accounts_actual_overlap() {
    let source = Arc::new(SOURCE.to_vec());
    let doc = LexicalDocument::parse(source.clone(), 1 << 20, &mut Run).unwrap();
    let mut shared = DecodeSource::new(&doc, &mut Run).unwrap();
    let mut cache = PaintCache::default();
    let mut ledger = ChargeLedger::new(&source, 1 << 24, 128).unwrap();
    // Establish ledger-registry capacity independently of cache buffers so its
    // own metadata is excluded from the measured cache allocation deltas.
    let mut registry = [None; 128];
    for item in &mut registry {
        *item = Some(ledger.admit(AllocationTag::None, 0).unwrap());
    }
    for item in registry {
        ledger.release(item.unwrap()).unwrap();
    }
    let initial = ledger.stats();
    let cached = resolve_cached_fill(
        &mut shared,
        &mut cache,
        &mut ledger,
        fill(),
        &edges(),
        affine_identity(),
        [10., 10.],
        &mut Run,
    )
    .unwrap();
    assert_eq!(cached.stops.len(), 2);
    assert_eq!(
        ledger.stats().allocation_count - initial.allocation_count,
        3
    );
    assert_eq!(
        ledger.stats().live_bytes - initial.live_bytes,
        cache.storage_bytes().unwrap()
    );
    assert_eq!(
        ledger.stats().total_allocated_bytes - initial.total_allocated_bytes,
        cache.storage_bytes().unwrap()
    );
    for index in 1..16 {
        resolve_cached_fill(
            &mut shared,
            &mut cache,
            &mut ledger,
            fill(),
            &edges(),
            affine_identity(),
            [10. + index as f64, 10.],
            &mut Run,
        )
        .unwrap();
    }
    let before_growth = ledger.stats();
    let new_entries = 32 * std::mem::size_of::<GradientCacheEntry>();
    resolve_cached_fill(
        &mut shared,
        &mut cache,
        &mut ledger,
        fill(),
        &edges(),
        affine_identity(),
        [100., 10.],
        &mut Run,
    )
    .unwrap();
    let stop_bytes =
        cache.entries.last().unwrap().stops.capacity() * std::mem::size_of::<PaintStop>();
    assert!(ledger.stats().peak_live_bytes >= before_growth.live_bytes + new_entries + stop_bytes);
    assert_eq!(
        ledger.stats().live_bytes - initial.live_bytes,
        cache.storage_bytes().unwrap()
    );
    assert_eq!(cache.entries.capacity(), 32);
}

struct StopAt {
    remaining: usize,
    polls: usize,
}
impl StopAt {
    fn poll(&mut self) -> std::result::Result<(), Stop> {
        self.polls += 1;
        if self.remaining == 0 {
            Err(Stop::Canceled)
        } else {
            self.remaining -= 1;
            Ok(())
        }
    }
}
impl WorkControl for StopAt {
    fn charge(&mut self, _: WorkKind, _: u64) -> std::result::Result<(), Stop> {
        self.poll()
    }
    fn checkpoint(&mut self) -> std::result::Result<(), Stop> {
        self.poll()
    }
}

#[test]
fn cancellation_at_every_cold_resolver_poll_keeps_only_actual_retained_cache_capacity() {
    let source = Arc::new(SOURCE.to_vec());
    let doc = LexicalDocument::parse(source.clone(), 1 << 20, &mut Run).unwrap();
    let mut shared = DecodeSource::new(&doc, &mut Run).unwrap();
    let mut cache = PaintCache::default();
    let mut ledger = ChargeLedger::new(&source, 1 << 24, 64).unwrap();
    let mut baseline = StopAt {
        remaining: usize::MAX,
        polls: 0,
    };
    resolve_cached_fill(
        &mut shared,
        &mut cache,
        &mut ledger,
        fill(),
        &edges(),
        affine_identity(),
        [10., 10.],
        &mut baseline,
    )
    .unwrap();
    for at in 0..baseline.polls {
        let mut shared = DecodeSource::new(&doc, &mut Run).unwrap();
        let mut cache = PaintCache::default();
        let mut ledger = ChargeLedger::new(&source, 1 << 24, 64).unwrap();
        let registry = ledger.admit(AllocationTag::None, 0).unwrap();
        ledger.release(registry).unwrap();
        let before = ledger.stats().live_bytes;
        let mut cancel = StopAt {
            remaining: at,
            polls: 0,
        };
        let error = resolve_cached_fill(
            &mut shared,
            &mut cache,
            &mut ledger,
            fill(),
            &edges(),
            affine_identity(),
            [10., 10.],
            &mut cancel,
        )
        .unwrap_err();
        assert_eq!(error.kind, ErrorKind::Canceled, "poll {at}");
        assert_eq!(error.offset, fill().offset);
        assert_eq!(
            ledger.stats().live_bytes - before,
            cache.storage_bytes().unwrap(),
            "poll {at}"
        );
        assert!(cache.entries.len() <= 1);
        resolve_cached_fill(
            &mut shared,
            &mut cache,
            &mut ledger,
            fill(),
            &edges(),
            affine_identity(),
            [10., 10.],
            &mut Run,
        )
        .unwrap();
        assert_eq!(cache.entries.len(), 1);
    }
}

#[test]
fn explicit_owned_copy_cancellation_releases_every_temporary_stop_capacity() {
    let source = Arc::new(SOURCE.to_vec());
    let doc = LexicalDocument::parse(source.clone(), 1 << 20, &mut Run).unwrap();
    let mut shared = DecodeSource::new(&doc, &mut Run).unwrap();
    let mut cache = PaintCache::default();
    let mut ledger = ChargeLedger::new(&source, 1 << 24, 64).unwrap();
    let cached = resolve_cached_fill(
        &mut shared,
        &mut cache,
        &mut ledger,
        fill(),
        &edges(),
        affine_identity(),
        [10., 10.],
        &mut Run,
    )
    .unwrap();
    let before = ledger.stats().live_bytes;
    let mut baseline = StopAt {
        remaining: usize::MAX,
        polls: 0,
    };
    cached
        .to_owned(&mut ledger, &mut baseline)
        .unwrap()
        .release(&mut ledger)
        .unwrap();
    for at in 0..baseline.polls {
        let mut cancel = StopAt {
            remaining: at,
            polls: 0,
        };
        let error = cached.to_owned(&mut ledger, &mut cancel).unwrap_err();
        assert_eq!(error.kind, ErrorKind::Canceled);
        assert_eq!(error.offset, fill().offset);
        assert_eq!(ledger.stats().live_bytes, before);
    }
}
