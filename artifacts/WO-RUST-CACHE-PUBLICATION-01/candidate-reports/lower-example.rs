use vexel_scene::*;

fn lower(tx: &mut CacheTransaction<'_>, c: &mut dyn WorkControl) -> Result<JobDashCache> {
    let segments = tx.buffer(RasterPool::Dash, usize::MAX, c)?;
    let joins = tx.buffer(RasterPool::Dash, usize::MAX, c)?;
    let shapes = tx.buffer(RasterPool::Dash, 65536, c)?;
    let cells = tx.buffer(RasterPool::Grid, 65536, c)?;
    let refs = tx.buffer(RasterPool::Grid, 1_000_000, c)?;
    tx.reserve(segments, 1, c)?;
    tx.extend(segments, &[DashSegment {
        edge: Edge { a: Point { x: 0., y: 0. }, b: Point { x: 1., y: 1. } },
        start_cap: Linecap::Butt, end_cap: Linecap::Butt,
    }], c)?;
    tx.extend(joins, &[DashJoin {
        previous: Point { x: 0., y: 0. }, vertex: Point { x: 1., y: 1. },
        next: Point { x: 2., y: 0. },
    }], c)?;
    tx.extend(refs, &[0, STROKE_GRID_JOIN_BIT], c)?;
    tx.extend(cells, &[GridCell { first: 0, count: 2 }], c)?;
    tx.extend(shapes, &[JobDashRange {
        segments: TableRange { first: 0, count: 1 },
        joins: TableRange { first: 0, count: 1 }, grid_cells: cells, grid_refs: refs,
        grid_min_x: 0., grid_min_y: 0., grid_cell_size: 1.,
        grid_columns: 1, grid_rows: 1,
    }], c)?;
    tx.slice_mut(segments)?[0].end_cap = Linecap::Round;
    Ok(JobDashCache { segments, joins, shapes, build_steps: 1 })
}
fn replace(job: &mut RasterJob, source: &SourceOwner,
           old: Option<CacheGeneration>, c: &mut dyn WorkControl) -> Result<CacheGeneration> {
    let mut tx = job.begin_cache(source, old, c)?;
    let cache = lower(&mut tx, c)?; // any error drops tx and leaves old intact
    tx.commit(cache, c)
}
fn run(source: &SourceOwner, c: &mut dyn WorkControl) -> Result<()> {
    let mut job = RasterJob::new(ChargeLedger::new(source, 64 << 20, 1024)?)
        .map_err(|e| e.diagnostic)?;
    let first = replace(&mut job, source, None, c)?;
    let current = replace(&mut job, source, Some(first), c)?;
    assert!(job.cache_descriptor(first).is_err());
    let descriptor = job.cache_descriptor(current)?;
    let out = job.buffer(RasterPool::Output, 4, c)?;
    let count = job.buffer(RasterPool::Scratch, 1, c)?;
    job.extend(out, &[0u8; 4], c)?;
    job.extend(count, &[0usize], c)?;
    {
        let mut scope = job.compose(source, descriptor, c)?;
        scope.with_buffers(out, count, c, |cache, pixels, count, _| {
            count[0] = cache.grid_at(0)?.2.len();
            pixels.copy_from_slice(&[1, 2, 3, 255]);
            Ok(())
        })?;
    }
    assert_eq!(job.slice(count)?, &[2]);
    assert_eq!(job.finish()?.live_bytes, 0);
    Ok(())
}

struct Run;
impl WorkControl for Run {
    fn checkpoint(&mut self) -> std::result::Result<(), Stop> { Ok(()) }
    fn charge(&mut self, _: WorkKind, _: u64) -> std::result::Result<(), Stop> { Ok(()) }
}
fn main() { run(&std::sync::Arc::new(Vec::new()), &mut Run).unwrap(); }
