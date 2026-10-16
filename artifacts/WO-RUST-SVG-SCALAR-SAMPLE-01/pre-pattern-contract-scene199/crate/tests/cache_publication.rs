use std::{mem::size_of, sync::Arc};
use vexel_scene::*;
struct Run;
impl WorkControl for Run {
    fn checkpoint(&mut self) -> std::result::Result<(), Stop> {
        Ok(())
    }
    fn charge(&mut self, _: WorkKind, _: u64) -> std::result::Result<(), Stop> {
        Ok(())
    }
}
struct Cancel;
impl WorkControl for Cancel {
    fn checkpoint(&mut self) -> std::result::Result<(), Stop> {
        Err(Stop::Canceled)
    }
    fn charge(&mut self, _: WorkKind, _: u64) -> std::result::Result<(), Stop> {
        Err(Stop::Canceled)
    }
}
fn job(source: &SourceOwner, budget: usize) -> RasterJob {
    RasterJob::new(ChargeLedger::new(source, budget, 1024).unwrap()).unwrap()
}
fn segment() -> DashSegment {
    DashSegment {
        edge: Edge {
            a: Point { x: 0., y: 0. },
            b: Point { x: 1., y: 1. },
        },
        start_cap: Linecap::Butt,
        end_cap: Linecap::Butt,
    }
}
// A complete lower-builder example: real typed buffers, actual retained native
// capacities, mutable accepted API, original global grid indices, no Vec/copies
// at commit. Every one of the near-cap segment records is initialized and read.
fn build(tx: &mut CacheTransaction<'_>, target: usize) -> Result<JobDashCache> {
    let segments = tx.buffer(RasterPool::Dash, usize::MAX, &mut Run)?;
    let joins = tx.buffer(RasterPool::Dash, usize::MAX, &mut Run)?;
    let shapes = tx.buffer(RasterPool::Dash, 65536, &mut Run)?;
    let cells = tx.buffer(RasterPool::Grid, 65536, &mut Run)?;
    let refs = tx.buffer(RasterPool::Grid, 1_000_000, &mut Run)?;
    let base = size_of::<JobDashRange>() + size_of::<GridCell>() + size_of::<DashJoin>();
    let n = (target - base - 4) / size_of::<DashSegment>();
    let ref_capacity = (target - base - n * size_of::<DashSegment>()) / 4;
    tx.reserve(segments, n, &mut Run)?;
    let chunk = [segment(); 1024];
    for _ in 0..n / chunk.len() {
        tx.extend(segments, &chunk, &mut Run)?;
    }
    tx.extend(segments, &chunk[..n % chunk.len()], &mut Run)?;
    let join = DashJoin {
        previous: Point { x: 0., y: 0. },
        vertex: Point { x: 1., y: 1. },
        next: Point { x: 2., y: 0. },
    };
    tx.extend(joins, &[join], &mut Run)?;
    tx.reserve(refs, ref_capacity, &mut Run)?;
    tx.extend(refs, &[0, STROKE_GRID_JOIN_BIT], &mut Run)?;
    tx.extend(cells, &[GridCell { first: 0, count: 2 }], &mut Run)?;
    tx.extend(
        shapes,
        &[JobDashRange {
            segments: TableRange { first: 0, count: n },
            joins: TableRange { first: 0, count: 1 },
            grid_cells: cells,
            grid_refs: refs,
            grid_min_x: 0.,
            grid_min_y: 0.,
            grid_cell_size: 1.,
            grid_columns: 1,
            grid_rows: 1,
        }],
        &mut Run,
    )?;
    tx.slice_mut(segments)?[0] = segment();
    assert_eq!(tx.capacity(segments)?, n);
    Ok(JobDashCache {
        segments,
        joins,
        shapes,
        build_steps: 1,
    })
}
fn publish(
    j: &mut RasterJob,
    source: &SourceOwner,
    old: Option<CacheGeneration>,
    bytes: usize,
) -> CacheGeneration {
    let mut tx = j.begin_cache(source, old, &mut Run).unwrap();
    let cache = build(&mut tx, bytes).unwrap();
    tx.commit(cache, &mut Run).unwrap()
}
#[test]
fn exact_cap_payload_replacement_overlap_and_composition() {
    let source = Arc::new(Vec::new());
    let mut j = job(&source, 256 << 20);
    let old = publish(&mut j, &source, None, 16 << 20);
    let old_descriptor = j.cache_descriptor(old).unwrap();
    let baseline = j.stats();
    let mut tx = j.begin_cache(&source, Some(old), &mut Run).unwrap();
    let next = build(&mut tx, 16 << 20).unwrap();
    let range = tx.slice(next.shapes).unwrap()[0];
    let retained = tx.capacity(next.segments).unwrap() * size_of::<DashSegment>()
        + tx.capacity(next.joins).unwrap() * size_of::<DashJoin>()
        + tx.capacity(next.shapes).unwrap() * size_of::<JobDashRange>()
        + tx.capacity(range.grid_cells).unwrap() * size_of::<GridCell>()
        + tx.capacity(range.grid_refs).unwrap() * size_of::<u32>();
    assert_eq!(retained, 16 << 20);
    // Metadata registry growth is also live and admitted, in addition to payload.
    assert!(tx.stats().live_bytes - baseline.live_bytes >= retained);
    let address = tx.slice(next.segments).unwrap().as_ptr();
    assert!(tx.stats().peak_live_bytes >= (32 << 20));
    let next = tx.commit(next, &mut Run).unwrap();
    assert!(j.cache_descriptor(old).is_err());
    assert!(j.dash_view(old_descriptor, &mut Run).is_err());
    assert!(j.begin_cache(&source, Some(old), &mut Run).is_err());
    let descriptor = j.cache_descriptor(next).unwrap();
    assert_eq!(j.slice(descriptor.segments).unwrap().as_ptr(), address);
    let a = j.buffer(RasterPool::Output, 4, &mut Run).unwrap();
    let b = j.buffer(RasterPool::Scratch, 1, &mut Run).unwrap();
    j.extend(a, &[0u8; 4], &mut Run).unwrap();
    j.extend(b, &[0usize], &mut Run).unwrap();
    let before = j.stats();
    {
        let mut composition = j.compose(&source, descriptor, &mut Run).unwrap();
        for _ in 0..16 {
            composition
                .with_buffers(a, b, &mut Run, |view, pixels, count, _| {
                    assert!(view.segments().iter().all(|v| v.edge.b.x == 1.));
                    assert_eq!(view.joins().len(), 1);
                    assert_eq!(view.grid_at(0)?.2, &[0, STROKE_GRID_JOIN_BIT]);
                    pixels.copy_from_slice(&[1, 2, 3, 255]);
                    count[0] = view.segments().len();
                    Ok(())
                })
                .unwrap();
        }
        assert_eq!(composition.stats(), before);
    }
    assert_eq!(j.slice(a).unwrap(), &[1, 2, 3, 255]);
    assert!(j.slice(b).unwrap()[0] > 100_000);
    assert_eq!(j.finish().unwrap().live_bytes, 0);
}
#[test]
fn inadequate_physical_budget_repeated_failure_preserves_old() {
    let source = Arc::new(Vec::new());
    let mut j = job(&source, 24 << 20);
    let old = publish(&mut j, &source, None, 16 << 20);
    let baseline = j.stats();
    for _ in 0..8 {
        let mut tx = j.begin_cache(&source, Some(old), &mut Run).unwrap();
        assert_eq!(
            build(&mut tx, 16 << 20).unwrap_err().kind,
            ErrorKind::Limit(LimitKind::ReservedBytes)
        );
        tx.abort();
        assert_eq!(
            j.cache_view(old, &mut Run).unwrap().grid_at(0).unwrap().2,
            &[0, STROKE_GRID_JOIN_BIT]
        );
    }
    let stable = j.stats();
    for _ in 0..8 {
        let mut tx = j.begin_cache(&source, Some(old), &mut Run).unwrap();
        assert!(build(&mut tx, 16 << 20).is_err());
    }
    assert_eq!(j.stats().live_bytes, stable.live_bytes);
    assert!(j.stats().total_allocated_bytes >= baseline.total_allocated_bytes);
    assert!(j.stats().peak_live_bytes <= 24 << 20);
    assert_eq!(j.finish().unwrap().live_bytes, 0);
}
#[test]
fn individual_aggregate_and_grid_caps_are_generation_local() {
    let source = Arc::new(Vec::new());
    let mut j = job(&source, 256 << 20);
    let old = publish(&mut j, &source, None, 4096);
    for _ in 0..3 {
        let mut tx = j.begin_cache(&source, Some(old), &mut Run).unwrap();
        let a = tx
            .buffer::<DashSegment>(RasterPool::Dash, usize::MAX, &mut Run)
            .unwrap();
        let b = tx
            .buffer::<DashJoin>(RasterPool::Dash, usize::MAX, &mut Run)
            .unwrap();
        let before = tx.stats();
        assert_eq!(
            tx.reserve(a, (16 << 20) / size_of::<DashSegment>() + 1, &mut Run)
                .unwrap_err()
                .kind,
            ErrorKind::Limit(LimitKind::DashCache)
        );
        assert_eq!(tx.stats(), before);
        tx.reserve(a, (16 << 20) / size_of::<DashSegment>(), &mut Run)
            .unwrap();
        assert_eq!(
            tx.reserve(b, 1, &mut Run).unwrap_err().kind,
            ErrorKind::Limit(LimitKind::DashCache)
        );
    }
    // Real populated grids: four 128x128 arrays and 1M references, retained
    // simultaneously in old and staged generations without shared count caps.
    fn grid_cache(tx: &mut CacheTransaction<'_>) -> JobDashCache {
        let s = tx.buffer(RasterPool::Dash, 1, &mut Run).unwrap();
        let j = tx.buffer(RasterPool::Dash, 1, &mut Run).unwrap();
        let shapes = tx.buffer(RasterPool::Dash, 4, &mut Run).unwrap();
        tx.extend(s, &[segment()], &mut Run).unwrap();
        for _ in 0..4 {
            let cells = tx
                .buffer::<GridCell>(RasterPool::Grid, usize::MAX, &mut Run)
                .unwrap();
            let refs = tx
                .buffer::<u32>(RasterPool::Grid, usize::MAX, &mut Run)
                .unwrap();
            tx.reserve(cells, 16384, &mut Run).unwrap();
            tx.reserve(refs, 250_000, &mut Run).unwrap();
            for _ in 0..16 {
                tx.extend(cells, &[GridCell { first: 0, count: 1 }; 1024], &mut Run)
                    .unwrap();
            }
            for _ in 0..250 {
                tx.extend(refs, &[0; 1000], &mut Run).unwrap();
            }
            tx.slice_mut(cells).unwrap()[0].count = 250_000;
            tx.extend(
                shapes,
                &[JobDashRange {
                    segments: TableRange { first: 0, count: 1 },
                    joins: TableRange::EMPTY,
                    grid_cells: cells,
                    grid_refs: refs,
                    grid_min_x: 0.,
                    grid_min_y: 0.,
                    grid_cell_size: 1.,
                    grid_columns: 128,
                    grid_rows: 128,
                }],
                &mut Run,
            )
            .unwrap();
        }
        let extra_cells = tx
            .buffer::<GridCell>(RasterPool::Grid, 65536, &mut Run)
            .unwrap();
        let extra_refs = tx
            .buffer::<u32>(RasterPool::Grid, 1_000_000, &mut Run)
            .unwrap();
        let before = tx.stats();
        for (h, n) in [(extra_cells, 65537), (extra_cells, 1)] {
            assert_eq!(
                tx.reserve(h, n, &mut Run).unwrap_err().kind,
                ErrorKind::Limit(LimitKind::Grid)
            );
        }
        for (h, n) in [(extra_refs, 1_000_001), (extra_refs, 1)] {
            assert_eq!(
                tx.reserve(h, n, &mut Run).unwrap_err().kind,
                ErrorKind::Limit(LimitKind::Grid)
            );
        }
        assert_eq!(tx.stats(), before);
        JobDashCache {
            segments: s,
            joins: j,
            shapes,
            build_steps: 0,
        }
    }
    let mut tx = j.begin_cache(&source, Some(old), &mut Run).unwrap();
    let cache = grid_cache(&mut tx);
    let first = tx.commit(cache, &mut Run).unwrap();
    let mut tx = j.begin_cache(&source, Some(first), &mut Run).unwrap();
    let cache = grid_cache(&mut tx);
    let second = tx.commit(cache, &mut Run).unwrap();
    let view = j.cache_view(second, &mut Run).unwrap();
    let totals = (0..4)
        .map(|i| {
            let (_, cells, refs) = view.grid_at(i).unwrap();
            assert_eq!(cells[0].count, 250_000);
            assert!(refs.iter().all(|r| *r == 0));
            (cells.len(), refs.len())
        })
        .fold((0, 0), |a, b| (a.0 + b.0, a.1 + b.1));
    assert_eq!(totals, (65536, 1_000_000));
    assert!(j.cache_view(first, &mut Run).is_err());
}
#[test]
fn mixed_foreign_stale_wrong_role_validation_abort_cancel_and_drop() {
    let source = Arc::new(Vec::new());
    let mut j = job(&source, 64 << 20);
    let old = publish(&mut j, &source, None, 4096);
    let old_cache = j.cache_descriptor(old).unwrap();
    let old_cells = j.cache_view(old, &mut Run).unwrap().shapes[0].grid_cells;
    let mut other = job(&source, 64 << 20);
    let foreign = publish(&mut other, &source, None, 4096);
    assert!(j.begin_cache(&source, Some(foreign), &mut Run).is_err());
    assert!(j
        .begin_cache(&Arc::new(Vec::new()), Some(old), &mut Run)
        .is_err());
    assert!(j.begin_cache(&source, Some(old), &mut Cancel).is_err());
    for mode in 0..6 {
        let mut tx = j.begin_cache(&source, Some(old), &mut Run).unwrap();
        assert!(tx.reserve(old_cache.segments, 1, &mut Run).is_err());
        assert!(tx.buffer::<u8>(RasterPool::Output, 4, &mut Run).is_err());
        assert!(tx.buffer::<u8>(RasterPool::Dash, 4, &mut Run).is_err());
        let mut staged = build(&mut tx, 4096).unwrap();
        let saved = staged;
        match mode {
            0 => {
                staged.joins = old_cache.joins;
                assert!(tx.commit(staged, &mut Run).is_err());
            }
            1 => {
                let range = tx.slice_mut(staged.shapes).unwrap();
                range[0].grid_refs =
                    other.cache_view(foreign, &mut Run).unwrap().shapes[0].grid_refs;
                assert!(tx.commit(staged, &mut Run).is_err());
            }
            2 => {
                tx.slice_mut(staged.shapes).unwrap()[0].grid_cells = old_cells;
                assert!(tx.commit(staged, &mut Run).is_err());
            }
            3 => {
                tx.slice_mut(staged.segments).unwrap()[0].edge.a.x = f64::NAN;
                assert!(tx.commit(staged, &mut Run).is_err());
            }
            4 => {
                assert!(tx.commit(staged, &mut Cancel).is_err());
            }
            _ => tx.abort(),
        }
        assert!(j.slice(saved.segments).is_err());
        assert!(j.dash_view(saved, &mut Run).is_err());
        assert!(j.cache_view(old, &mut Run).is_ok());
    }
    let escaped;
    {
        let mut tx = j.begin_cache(&source, Some(old), &mut Run).unwrap();
        escaped = build(&mut tx, 4096).unwrap();
    }
    assert!(j.slice(escaped.segments).is_err());
    assert!(j.cache_view(old, &mut Run).is_ok());
    assert_eq!(j.finish().unwrap().live_bytes, 0);
}
#[test]
fn forgotten_transaction_cannot_finish_or_begin_again() {
    let source = Arc::new(Vec::new());
    let mut j = job(&source, 1 << 20);
    let tx = j.begin_cache(&source, None, &mut Run).unwrap();
    std::mem::forget(tx);
    assert!(j.begin_cache(&source, None, &mut Run).is_err());
    assert!(j.finish().is_err());
    let mut j = job(&source, 1 << 20);
    let mut tx = j.begin_cache(&source, None, &mut Run).unwrap();
    let descriptor = build(&mut tx, 4096).unwrap();
    std::mem::forget(tx);
    assert!(j.dash_view(descriptor, &mut Run).is_err());
    j.release_all().unwrap();
    assert_eq!(j.finish().unwrap().live_bytes, 0);
}

struct StopAfter(usize);
impl WorkControl for StopAfter {
    fn checkpoint(&mut self) -> std::result::Result<(), Stop> {
        if self.0 == 0 {
            Err(Stop::Canceled)
        } else {
            self.0 -= 1;
            Ok(())
        }
    }
    fn charge(&mut self, _: WorkKind, _: u64) -> std::result::Result<(), Stop> {
        self.checkpoint()
    }
}
#[test]
fn cancellation_after_real_allocation_and_validation_keeps_old_and_history() {
    let source = Arc::new(Vec::new());
    let mut j = job(&source, 64 << 20);
    let old = publish(&mut j, &source, None, 4096);
    // Failed validation after allocation must preserve the history of storage
    // that really existed; cleanup releases only the unpublished payload.
    for _ in 0..4 {
        let before = j.stats();
        let mut tx = j.begin_cache(&source, Some(old), &mut Run).unwrap();
        let cache = build(&mut tx, 4096).unwrap();
        let after_build = tx.stats();
        assert!(after_build.total_allocated_bytes > before.total_allocated_bytes);
        assert!(tx.commit(cache, &mut StopAfter(3)).is_err());
        assert_eq!(
            j.stats().total_allocated_bytes,
            after_build.total_allocated_bytes
        );
        assert!(j.stats().live_bytes < after_build.live_bytes);
        assert!(j.cache_view(old, &mut Run).is_ok());
    }
    let mut tx = j.begin_cache(&source, Some(old), &mut Run).unwrap();
    let h = tx
        .buffer::<DashSegment>(RasterPool::Dash, 10000, &mut Run)
        .unwrap();
    tx.extend(h, &[segment()], &mut Run).unwrap();
    let before = tx.stats();
    assert!(tx.reserve(h, 10000, &mut CancelCopy).is_err());
    assert_eq!(tx.slice(h).unwrap().len(), 1);
    assert_eq!(tx.capacity(h).unwrap(), 1);
    assert!(tx.stats().total_allocated_bytes > before.total_allocated_bytes);
    tx.abort();
    assert!(j.cache_view(old, &mut Run).is_ok());
}

#[test]
fn scoped_mutation_cleanup_stale_handles_and_build_step_limit() {
    let source = Arc::new(Vec::new());
    let mut j = job(&source, 64 << 20);
    let old = publish(&mut j, &source, None, 4096);
    let mut tx = j.begin_cache(&source, Some(old), &mut Run).unwrap();
    let h = tx
        .buffer::<u32>(RasterPool::Grid, 1_000_000, &mut Run)
        .unwrap();
    tx.reserve(h, 100, &mut Run).unwrap();
    tx.extend(h, &[1, 2, 3], &mut Run).unwrap();
    tx.truncate(h, 1).unwrap();
    tx.shrink_to(h, 1, &mut Run).unwrap();
    assert_eq!(tx.capacity(h).unwrap(), 1);
    assert_eq!(tx.slice(h).unwrap(), &[1]);
    tx.clear_capacity(h, &mut Run).unwrap();
    assert_eq!(tx.capacity(h).unwrap(), 0);
    tx.release(h).unwrap();
    let fresh = tx.buffer::<u32>(RasterPool::Grid, 4, &mut Run).unwrap();
    assert!(tx.extend(h, &[0], &mut Run).is_err());
    assert!(tx.slice_mut(h).is_err());
    tx.extend(fresh, &[0], &mut Run).unwrap();
    let mut cache = build(&mut tx, 4096).unwrap();
    cache.build_steps = 1_000_001;
    assert_eq!(
        tx.commit(cache, &mut Run).unwrap_err().kind,
        ErrorKind::Limit(LimitKind::DashCache)
    );
    assert!(j.slice(fresh).is_err());
    assert!(j.cache_view(old, &mut Run).is_ok());
}

struct CancelCopy;
impl WorkControl for CancelCopy {
    fn checkpoint(&mut self) -> std::result::Result<(), Stop> {
        Ok(())
    }
    fn charge(&mut self, kind: WorkKind, _: u64) -> std::result::Result<(), Stop> {
        if kind == WorkKind::SourceRead {
            Err(Stop::Canceled)
        } else {
            Ok(())
        }
    }
}
