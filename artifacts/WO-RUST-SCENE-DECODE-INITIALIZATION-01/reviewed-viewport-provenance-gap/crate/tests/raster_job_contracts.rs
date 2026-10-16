mod support;
use std::sync::Arc;
use support::*;
use vexel_scene::*;

fn job(budget: usize, leases: usize) -> RasterJob {
    RasterJob::new(ChargeLedger::new(&owner(), budget, leases).unwrap()).unwrap()
}
fn output(job: &mut RasterJob) -> JobBuffer<u8> {
    job.buffer(RasterPool::Output, usize::MAX, &mut Run).unwrap()
}
#[test]
fn source_once_owned_drop_and_no_self_reference() {
    let source = owner();
    let weak = Arc::downgrade(&source);
    let mut j = RasterJob::new(ChargeLedger::new(&source, 1 << 20, 64).unwrap()).unwrap();
    assert_eq!(Arc::strong_count(&source), 2);
    let a = output(&mut j);
    let b = output(&mut j);
    j.extend(a, &[1, 2], &mut Run).unwrap();
    j.extend(b, &[3, 4], &mut Run).unwrap();
    assert_eq!(j.stats().source_bytes, source.capacity());
    assert_eq!(Arc::strong_count(&source), 2);
    drop(source);
    // A handle retained after job destruction cannot retain opaque owned data.
    std::thread::spawn(move || {
        assert_eq!(j.slice(a).unwrap(), &[1, 2]);
        drop(j);
    })
    .join()
    .unwrap();
    assert!(weak.upgrade().is_none());
}
#[test]
fn actual_spare_capacity_reuse_growth_clear_release_history() {
    let mut j = job(1 << 20, 64);
    let a = output(&mut j);
    let baseline = j.stats();
    j.reserve(a, 137, &mut Run).unwrap();
    let first = j.stats();
    assert_eq!(first.output_bytes, j.capacity(a).unwrap());
    assert_eq!(first.live_bytes - baseline.live_bytes, 137);
    assert_eq!(first.total_allocated_bytes - baseline.total_allocated_bytes, 137);
    j.extend(a, &[1, 2], &mut Run).unwrap();
    assert_eq!(j.stats(), first);
    j.truncate(a, 0).unwrap();
    assert_eq!(j.stats(), first);
    j.extend(a, &[3], &mut Run).unwrap();
    j.reserve(a, 250, &mut Run).unwrap();
    let grown = j.stats();
    assert_eq!(j.slice(a).unwrap(), &[3]);
    assert_eq!(grown.output_bytes, 250);
    assert!(grown.peak_live_bytes >= baseline.live_bytes + 137 + 250);
    assert_eq!(grown.total_allocated_bytes, first.total_allocated_bytes + 250);
    j.clear_capacity(a, &mut Run).unwrap();
    assert_eq!(j.capacity(a).unwrap(), 0);
    assert_eq!(j.stats().output_bytes, 0);
    assert_eq!(j.stats().total_allocated_bytes, grown.total_allocated_bytes);
    j.extend(a, &[8], &mut Run).unwrap();
    j.release(a).unwrap();
    assert!(j.slice(a).is_err());
    assert_eq!(j.stats().output_bytes, 0);
    let final_stats = j.finish().unwrap();
    assert_eq!(final_stats.live_bytes, 0);
    assert!(final_stats.total_allocated_bytes > 0);
}
#[test]
fn overlap_rejected_before_alloc_and_old_buffer_unchanged() {
    // Measure fixed metadata from an identical job, avoiding implementation sizes.
    let mut measure = job(1 << 20, 64);
    let _ = output(&mut measure);
    let fixed = measure.stats().live_bytes;
    let mut j = job(fixed + 300, 64);
    let a = output(&mut j);
    j.extend(a, &[7; 100], &mut Run).unwrap();
    let before = j.stats();
    // Final capacity 250 fits, but old(100)+new(250) exceeds 300.
    assert_eq!(
        j.reserve(a, 250, &mut Run).unwrap_err().kind,
        ErrorKind::Limit(LimitKind::ReservedBytes)
    );
    assert_eq!(j.slice(a).unwrap(), &[7; 100]);
    assert_eq!(j.capacity(a).unwrap(), 100);
    assert_eq!(j.stats(), before);
}
#[test]
fn stale_double_wrong_job_and_equal_bytes_wrong_source_rejected() {
    let mut a = job(1 << 20, 64);
    let mut b = job(1 << 20, 64);
    let old = output(&mut a);
    a.extend(old, &[2], &mut Run).unwrap();
    let before = b.stats();
    assert!(b.release(old).is_err());
    assert!(b.reserve(old, 30, &mut Run).is_err());
    assert_eq!(b.stats(), before);
    let source = a.source_owner().clone();
    a.validate_source(&source).unwrap();
    assert!(a.validate_source(&Arc::new(source.as_slice().to_vec())).is_err());
    a.release(old).unwrap();
    assert!(a.release(old).is_err());
    let new = output(&mut a);
    assert_ne!(old, new);
    assert!(a.slice(old).is_err());
    a.extend(new, &[4], &mut Run).unwrap();
    assert_eq!(a.slice(new).unwrap(), &[4]);
}
#[test]
fn metadata_pressure_leaves_no_published_buffers_or_external_lease_escape() {
    let mut j = job(1 << 20, 1);
    assert!(j.buffer::<u8>(RasterPool::Output, 100, &mut Run).is_err());
    // The actual arena metadata is retained and counted, not an uncharged leak.
    assert!(j.stats().live_bytes > j.stats().source_bytes);
    assert_eq!(j.stats().output_bytes, 0);
    assert_eq!(j.finish().unwrap().live_bytes, 0);
    let source = owner();
    let mut ledger = ChargeLedger::new(&source, 1 << 20, 32).unwrap();
    let lease = ledger.admit(AllocationTag::Output, 5).unwrap();
    let error = RasterJob::new(ledger).err().unwrap();
    assert_eq!(error.diagnostic.kind, ErrorKind::InvalidArgument);
    let mut ledger = error.into_ledger();
    ledger.release(lease).unwrap();
    assert!(RasterJob::new(ledger).is_ok());
}
#[test]
fn real_try_reserve_failure_and_checked_byte_overflow_preserve_state() {
    let mut j = job(usize::MAX, 64);
    let a = j.buffer::<u8>(RasterPool::Scratch, usize::MAX, &mut Run).unwrap();
    j.extend(a, &[11], &mut Run).unwrap();
    let before = j.stats();
    // Calls the real Vec::try_reserve_exact, which rejects an object larger
    // than isize::MAX. This is capacity failure, NOT injected global OOM.
    assert_eq!(
        j.reserve(a, isize::MAX as usize + 1, &mut Run).unwrap_err().kind,
        ErrorKind::OutOfMemory
    );
    assert_eq!(j.stats(), before);
    assert_eq!(j.slice(a).unwrap(), &[11]);
    let b = j
        .buffer::<u32>(RasterPool::Scratch, usize::MAX, &mut Run)
        .unwrap();
    let before = j.stats();
    assert_eq!(
        j.reserve(b, usize::MAX, &mut Run).unwrap_err().kind,
        ErrorKind::Limit(LimitKind::Arithmetic)
    );
    assert_eq!(j.stats(), before);
    assert!(j.slice(b).unwrap().is_empty());
}
struct CancelAt {
    checkpoints: usize,
    charges: usize,
}
impl WorkControl for CancelAt {
    fn checkpoint(&mut self) -> std::result::Result<(), Stop> {
        if self.checkpoints == 0 {
            return Err(Stop::Canceled);
        }
        self.checkpoints -= 1;
        Ok(())
    }
    fn charge(&mut self, _: WorkKind, _: u64) -> std::result::Result<(), Stop> {
        if self.charges == 0 {
            return Err(Stop::Canceled);
        }
        self.charges -= 1;
        Ok(())
    }
}
#[test]
fn cancel_before_publication_retains_real_allocation_history_not_contents() {
    let mut j = job(1 << 20, 64);
    let a = output(&mut j);
    j.extend(a, &[7], &mut Run).unwrap();
    let before = j.stats();
    // reserve entry checkpoint, allocation entry checkpoint, then stop after
    // actual allocation and capacity accounting, before the old buffer moves.
    let mut cancel = CancelAt {
        checkpoints: 2,
        charges: 100,
    };
    assert_eq!(
        j.reserve(a, 200, &mut cancel).unwrap_err().kind,
        ErrorKind::Canceled
    );
    assert_eq!(j.slice(a).unwrap(), &[7]);
    assert_eq!(j.capacity(a).unwrap(), 1);
    assert_eq!(j.stats().live_bytes, before.live_bytes);
    assert_eq!(
        j.stats().total_allocated_bytes,
        before.total_allocated_bytes + 200
    );
    assert!(j.stats().peak_live_bytes >= before.live_bytes + 200);
    j.release(a).unwrap(); // cleanup is unconditional after cancellation
}
#[test]
fn cancellation_inside_append_preserves_old_capacity_and_contents() {
    let mut j = job(1 << 20, 64);
    let a = output(&mut j);
    j.extend(a, &[9], &mut Run).unwrap();
    let mut cancel = CancelAt {
        checkpoints: 100,
        charges: 2,
    };
    assert_eq!(
        j.extend(a, &[1, 2, 3, 4], &mut cancel).unwrap_err().kind,
        ErrorKind::Canceled
    );
    assert_eq!(j.slice(a).unwrap(), &[9]);
    assert_eq!(j.capacity(a).unwrap(), 1);
    let mut cancel = CancelAt {
        checkpoints: 0,
        charges: 100,
    };
    assert!(j.clear_capacity(a, &mut cancel).is_err());
    assert_eq!(j.capacity(a).unwrap(), 1);
}
#[test]
fn scratch_registration_failure_rolls_back_tokens_not_physical_history() {
    let mut j = job(1 << 20, 10);
    let keep = output(&mut j);
    j.extend(keep, &[5], &mut Run).unwrap();
    assert!(JobRasterScratch::new(&mut j, &mut Run).is_err());
    assert_eq!(j.slice(keep).unwrap(), &[5]);
    // Released failed registrations can be reused and no opaque live lease leaks.
    for _ in 0..30 {
        let h = output(&mut j);
        j.release(h).unwrap();
    }
    assert_eq!(j.finish().unwrap().live_bytes, 0);
}
#[test]
fn nested_grids_share_source_budget_actual_capacity_and_stale_checks() {
    let mut j = job(1 << 20, 128);
    let scratch = JobRasterScratch::new(&mut j, &mut Run).unwrap();
    let cells = j.buffer(RasterPool::Grid, 65536, &mut Run).unwrap();
    let refs = j.buffer(RasterPool::Grid, 1_000_000, &mut Run).unwrap();
    let edge = Edge {
        a: Point { x: 0., y: 0. },
        b: Point { x: 1., y: 1. },
    };
    j.extend(
        scratch.dash.segments,
        &[DashSegment {
            edge,
            start_cap: Linecap::Butt,
            end_cap: Linecap::Butt,
        }],
        &mut Run,
    )
    .unwrap();
    j.reserve(cells, 7, &mut Run).unwrap();
    j.extend(cells, &[GridCell { first: 0, count: 1 }], &mut Run)
        .unwrap();
    j.reserve(refs, 17, &mut Run).unwrap();
    j.extend(refs, &[0], &mut Run).unwrap();
    let range = JobDashRange {
        segments: TableRange { first: 0, count: 1 },
        joins: TableRange::default(),
        grid_cells: cells,
        grid_refs: refs,
        grid_min_x: 0.,
        grid_min_y: 0.,
        grid_cell_size: 1.,
        grid_columns: 1,
        grid_rows: 1,
    };
    j.extend(scratch.dash.shapes, &[range], &mut Run).unwrap();
    let live = j.stats().live_bytes;
    let cache = j.dash_view(scratch.dash, &mut Run).unwrap();
    assert_eq!(cache.grid(&range).unwrap().1, &[0]);
    assert_eq!(j.stats().live_bytes, live); // borrowed input creates no copied cache
    j.release(refs).unwrap();
    assert_eq!(live - j.stats().live_bytes, 17 * std::mem::size_of::<u32>());
    assert!(j.dash_view(scratch.dash, &mut Run).is_err());
    j.release(cells).unwrap();
    scratch.release(&mut j).unwrap();
    assert_eq!(j.finish().unwrap().live_bytes, 0);
}
#[test]
fn combined_dash_grid_cap_boundary_and_group_caps_are_original() {
    let mut j = job(100 << 20, 128);
    let a = j
        .buffer::<DashSegment>(RasterPool::Dash, usize::MAX, &mut Run)
        .unwrap();
    let b = j.buffer::<u32>(RasterPool::Grid, usize::MAX, &mut Run).unwrap();
    j.reserve(a, (16 << 20) / std::mem::size_of::<DashSegment>(), &mut Run)
        .unwrap();
    assert_eq!(
        j.reserve(b, 100, &mut Run).unwrap_err().kind,
        ErrorKind::Limit(LimitKind::DashCache)
    );
    let boundary = j.buffer::<u8>(RasterPool::Boundary, 123, &mut Run).unwrap();
    assert!(j.reserve(boundary, 124, &mut Run).is_err());
    let group = j
        .buffer::<u8>(RasterPool::GroupScratch, usize::MAX, &mut Run)
        .unwrap();
    assert!(j.reserve(group, (64 << 20) + 1, &mut Run).is_err());
}
#[test]
fn duplicate_or_wrong_scratch_descriptor_cannot_partially_release() {
    let mut j = job(1 << 20, 128);
    let scratch = JobRasterScratch::new(&mut j, &mut Run).unwrap();
    let mut bad = scratch;
    bad.subpixel_samples = bad.center_samples;
    let before = j.stats();
    assert!(bad.validate(&j, &mut Run).is_err());
    assert!(bad.release(&mut j).is_err());
    assert_eq!(j.stats(), before);
    let unrelated = j.buffer::<u8>(RasterPool::Scratch, 10, &mut Run).unwrap();
    let mut wrong = scratch;
    wrong.boundary_bits = unrelated;
    let before = j.stats();
    assert!(wrong.release(&mut j).is_err());
    assert_eq!(j.stats(), before);
    assert!(j.slice(unrelated).is_ok());
    scratch.release(&mut j).unwrap();
    assert!(scratch.release(&mut j).is_err());
}

// Explicit TEST provider. This bounded stop-selection/clip-bounds fixture is
// neither a gradient engine nor a registered production renderer.
struct FixtureResources {
    visits: std::cell::Cell<usize>,
}
fn stop_error(s: Stop) -> Diagnostic {
    Diagnostic {
        kind: match s {
            Stop::Canceled => ErrorKind::Canceled,
            Stop::Limit => ErrorKind::Limit(LimitKind::Work),
            Stop::Overflow => ErrorKind::Limit(LimitKind::Arithmetic),
        },
        offset: 0,
    }
}
fn bad() -> Diagnostic {
    Diagnostic {
        kind: ErrorKind::InvalidArgument,
        offset: 0,
    }
}
impl ControlledKernelResources for FixtureResources {
    fn paint_sample(
        &self,
        _: &Paint,
        stops: &[PaintStop],
        x: f64,
        _: f64,
        control: &mut dyn WorkControl,
    ) -> Result<[u8; 4]> {
        let mut color = [0; 4];
        for stop in stops {
            control.charge(WorkKind::PaintVisit, 1).map_err(stop_error)?;
            self.visits.set(self.visits.get() + 1);
            if x >= stop.offset {
                color = stop.rgba;
            }
        }
        Ok(color)
    }
    fn clip_contains(
        &self,
        view: SceneView<'_>,
        id: ClipId,
        x: f64,
        y: f64,
        control: &mut dyn WorkControl,
    ) -> Result<bool> {
        control.charge(WorkKind::ClipChain, 1).map_err(stop_error)?;
        let c = view.clip(id).ok_or_else(bad)?;
        Ok(x >= c.bounds[0] && y >= c.bounds[1] && x < c.bounds[2] && y < c.bounds[3])
    }
    fn clip_boundary_edges(
        &self,
        view: SceneView<'_>,
        id: ClipId,
        visitor: &mut dyn FnMut(Edge) -> bool,
        control: &mut dyn WorkControl,
    ) -> Result<bool> {
        for edge in &view.clip(id).ok_or_else(bad)?.geometry.edges {
            control.charge(WorkKind::ClipEdge, 1).map_err(stop_error)?;
            if !visitor(*edge) {
                return Ok(false);
            }
        }
        Ok(true)
    }
    fn clip_work_counts(
        &self,
        view: SceneView<'_>,
        id: ClipId,
        control: &mut dyn WorkControl,
    ) -> Result<ClipWorkCounts> {
        let c = view.clip(id).ok_or_else(bad)?;
        let mut result = ClipWorkCounts {
            chain_elements: 1,
            child_shapes: 0,
            edges: 0,
        };
        for _ in &c.geometry.shapes {
            control.charge(WorkKind::ClipChild, 1).map_err(stop_error)?;
            result.child_shapes += 1;
        }
        for _ in &c.geometry.edges {
            control.charge(WorkKind::ClipEdge, 1).map_err(stop_error)?;
            result.edges += 1;
        }
        Ok(result)
    }
    fn stroke_sample_work(
        &self,
        view: SceneView<'_>,
        shape: ShapeId,
        cache: JobDashView<'_>,
        _: f64,
        _: f64,
        control: &mut dyn WorkControl,
    ) -> Result<(usize, bool, bool)> {
        view.shape(shape).ok_or_else(bad)?;
        let mut count = 0;
        for _ in cache.segments {
            control.charge(WorkKind::StrokeTest, 1).map_err(stop_error)?;
            count += 1;
        }
        Ok((count, false, false))
    }
}
struct FixtureIndex;
impl ControlledIndexKernel for FixtureIndex {
    fn build_index(
        &mut self,
        view: SceneView<'_>,
        job: &mut RasterJob,
        mapping: ViewMapping,
        control: &mut dyn WorkControl,
    ) -> Result<JobRasterScratch> {
        job.validate_source(view.source_owner())?;
        mapping.validate()?;
        let scratch = JobRasterScratch::new(job, control)?;
        // Real per-edge work and real owned row storage on the same job.
        for index in 0..view.edges.len() {
            control
                .charge(WorkKind::ScanlineEdgeEntry, 1)
                .map_err(stop_error)?;
            job.extend(scratch.active_edges, &[index as u32], control)?;
        }
        job.extend(scratch.shape_offsets, &[0, view.edges.len()], control)?;
        Ok(scratch)
    }
}
#[test]
fn actual_lower_index_sample_and_child_output_share_one_job() {
    let mut b = builder();
    let id = b
        .append_shape(b.root_id(), shape(b.source_owner()), &mut Run)
        .unwrap();
    let graph = b.freeze().unwrap();
    let mut j = RasterJob::new(ChargeLedger::new(graph.source_owner(), 1 << 20, 256).unwrap()).unwrap();
    let mapping = ViewMapping {
        scale_x: 1.,
        scale_y: 1.,
        offset_x: 0.,
        offset_y: 0.,
    };
    let scratch = FixtureIndex
        .build_index(graph.root(), &mut j, mapping, &mut Run)
        .unwrap();
    let child = JobRasterScratch::new(&mut j, &mut Run).unwrap();
    let pixels = output(&mut j);
    j.extend(pixels, &[1, 2, 3, 255], &mut Run).unwrap();
    let result = JobRenderOutput {
        generation: 7,
        pixels,
        width: 1,
        height: 1,
        stride: 4,
    };
    result.validate(&j).unwrap();
    let resources = FixtureResources { visits: 0.into() };
    let cache = j.dash_view(scratch.dash, &mut Run).unwrap();
    let input = ControlledSampleInput {
        resources: &resources,
        active_edges: j.slice(scratch.active_edges).unwrap(),
        local_edges: &[],
        dash_seams: &[],
        dash_cache: cache,
        px: 0.5,
        py: 0.5,
        paint_x: 0.5,
        paint_y: 0.5,
    };
    input.validate(graph.root(), &mut Run).unwrap();
    assert_eq!(input.active_edges.len(), graph.root().edges.len());
    assert_eq!(
        resources
            .stroke_sample_work(graph.root(), id, cache, 0.5, 0.5, &mut Run)
            .unwrap()
            .0,
        0
    );
    let stops = [PaintStop {
        offset: 0.,
        rgba: [11, 22, 33, 255],
    }];
    assert_eq!(
        input.paint_sample(&paint(), &stops, &mut Run).unwrap(),
        stops[0].rgba
    );
    // Both output and child remain live in the same source/job ledger.
    assert_eq!(j.stats().source_bytes, graph.source_owner().capacity());
    assert_eq!(j.stats().output_bytes, 4);
    child.release(&mut j).unwrap();
    j.release(pixels).unwrap();
    scratch.release(&mut j).unwrap();
    assert_eq!(j.finish().unwrap().live_bytes, 0);
}
#[test]
fn controlled_paint_receives_actual_control_inside_stop_loop_and_propagates_errors() {
    let mut j = job(1 << 20, 128);
    let scratch = JobRasterScratch::new(&mut j, &mut Run).unwrap();
    let resources = FixtureResources { visits: 0.into() };
    let input = ControlledSampleInput {
        resources: &resources,
        active_edges: &[],
        local_edges: &[],
        dash_seams: &[],
        dash_cache: j.dash_view(scratch.dash, &mut Run).unwrap(),
        px: 0.,
        py: 0.,
        paint_x: 0.5,
        paint_y: 0.5,
    };
    let stops = [PaintStop {
        offset: 0.,
        rgba: [1, 2, 3, 255],
    }; 17];
    let mut cancel = CancelAt {
        checkpoints: 100,
        charges: 7,
    };
    assert_eq!(
        input
            .paint_sample(&paint(), &stops, &mut cancel)
            .unwrap_err()
            .kind,
        ErrorKind::Canceled
    );
    assert_eq!(resources.visits.get(), 7); // actual loop interrupted, no synthetic precharge
    let flag = std::sync::atomic::AtomicBool::new(false);
    let mut budget = vexel_base::BudgetControl::new(&flag, 3);
    assert_eq!(
        input
            .paint_sample(&paint(), &stops, &mut budget)
            .unwrap_err()
            .kind,
        ErrorKind::Limit(LimitKind::Work)
    );
    assert_eq!(budget.stats().units(WorkKind::PaintVisit), 3);
    assert_eq!(resources.visits.get(), 10);
}
#[test]
fn original_four_by_four_lattice_and_stops_data_pass_without_reinterpretation() {
    let mut j = job(1 << 20, 128);
    let scratch = JobRasterScratch::new(&mut j, &mut Run).unwrap();
    let cache = j.dash_view(scratch.dash, &mut Run).unwrap();
    let resources = FixtureResources { visits: 0.into() };
    let stops = [
        PaintStop {
            offset: 0.,
            rgba: [0, 0, 0, 255],
        },
        PaintStop {
            offset: 0.5,
            rgba: [200, 100, 50, 255],
        },
    ];
    let mut sum = [0u32; 4];
    let flag = std::sync::atomic::AtomicBool::new(false);
    let mut control = vexel_base::BudgetControl::new(&flag, u64::MAX);
    for sy in 0..4 {
        for sx in 0..4 {
            let x = (sx as f64 + 0.5) / 4.;
            let y = (sy as f64 + 0.5) / 4.;
            let input = ControlledSampleInput {
                resources: &resources,
                active_edges: &[],
                local_edges: &[],
                dash_seams: &[],
                dash_cache: cache,
                px: x,
                py: y,
                paint_x: x,
                paint_y: y,
            };
            let color = input.paint_sample(&paint(), &stops, &mut control).unwrap();
            for i in 0..4 {
                sum[i] += color[i] as u32;
            }
        }
    }
    assert_eq!(sum.map(|v| ((v + 8) / 16) as u8), [100, 50, 25, 255]);
    assert_eq!(control.stats().units(WorkKind::PaintVisit), 32);
    assert_eq!(resources.visits.get(), 32);
}
#[test]
fn wrong_pool_output_dash_and_grid_are_rejected() {
    let mut j = job(1 << 20, 128);
    let mut scratch = JobRasterScratch::new(&mut j, &mut Run).unwrap();
    let bad_segments = j.buffer(RasterPool::Scratch, 100, &mut Run).unwrap();
    let good_segments = scratch.dash.segments;
    scratch.dash.segments = bad_segments;
    assert!(j.dash_view(scratch.dash, &mut Run).is_err());
    scratch.dash.segments = good_segments;
    let pixels = j.buffer(RasterPool::Scratch, 100, &mut Run).unwrap();
    j.extend(pixels, &[0; 4], &mut Run).unwrap();
    assert!(JobRenderOutput {
        generation: 1,
        pixels,
        width: 1,
        height: 1,
        stride: 4
    }
    .validate(&j)
    .is_err());
    let cells = j.buffer(RasterPool::Scratch, 65536, &mut Run).unwrap();
    let refs = j.buffer(RasterPool::Grid, 1_000_000, &mut Run).unwrap();
    let range = JobDashRange {
        segments: TableRange::EMPTY,
        joins: TableRange::EMPTY,
        grid_cells: cells,
        grid_refs: refs,
        grid_min_x: 0.,
        grid_min_y: 0.,
        grid_cell_size: 1.,
        grid_columns: 0,
        grid_rows: 0,
    };
    j.extend(scratch.dash.shapes, &[range], &mut Run).unwrap();
    assert!(j.dash_view(scratch.dash, &mut Run).is_err());
}
#[test]
fn growth_copy_loop_cancellation_and_in_capacity_append_are_atomic() {
    let mut j = job(1 << 20, 64);
    let a = output(&mut j);
    j.extend(a, &[5; 16], &mut Run).unwrap();
    let before = j.stats();
    let mut cancel = CancelAt {
        checkpoints: 100,
        charges: 7,
    };
    assert!(j.reserve(a, 64, &mut cancel).is_err());
    assert_eq!(j.capacity(a).unwrap(), 16);
    assert_eq!(j.slice(a).unwrap(), &[5; 16]);
    assert_eq!(j.stats().live_bytes, before.live_bytes);
    assert_eq!(j.stats().total_allocated_bytes, before.total_allocated_bytes + 64);
    j.reserve(a, 64, &mut Run).unwrap();
    let before = j.stats();
    let mut cancel = CancelAt {
        checkpoints: 100,
        charges: 2,
    };
    assert!(j.extend(a, &[1, 2, 3], &mut cancel).is_err());
    assert_eq!(j.slice(a).unwrap(), &[5; 16]);
    assert_eq!(j.stats(), before);
}
#[test]
fn source_witness_on_scene_view_handles_empty_sources_and_frame_overlays() {
    let b = builder();
    let graph = b.freeze().unwrap();
    assert!(Arc::ptr_eq(graph.root().source_owner(), graph.source_owner()));
    let empty_a = Arc::new(Vec::new());
    let empty_b = Arc::new(Vec::new());
    let j = RasterJob::new(ChargeLedger::new(&empty_a, 4096, 32).unwrap()).unwrap();
    assert!(j.validate_source(&empty_b).is_err());
    j.validate_source(&empty_a).unwrap();
}
#[test]
fn physical_shrink_reserves_overlap_and_preserves_history_and_values() {
    let mut j = job(1 << 20, 64);
    let h = output(&mut j);
    j.reserve(h, 1024, &mut Run).unwrap();
    j.extend(h, &[3, 4], &mut Run).unwrap();
    let before = j.stats();
    j.shrink_to(h, 2, &mut Run).unwrap();
    assert_eq!(j.slice(h).unwrap(), &[3, 4]);
    assert_eq!(j.capacity(h).unwrap(), 2);
    assert_eq!(j.stats().live_bytes, before.live_bytes - 1024 + 2);
    assert_eq!(j.stats().total_allocated_bytes, before.total_allocated_bytes + 2);
    let before = j.stats();
    assert!(j.shrink_to(h, 1, &mut Run).is_err());
    assert_eq!(j.stats(), before);
    j.truncate(h, 0).unwrap();
    j.shrink_to(h, 0, &mut Run).unwrap();
    assert_eq!(j.capacity(h).unwrap(), 0);
}
#[test]
fn output_layout_caps_and_wrong_scratch_roles_are_rejected() {
    let mut j = job(1 << 20, 128);
    let mut s = JobRasterScratch::new(&mut j, &mut Run).unwrap();
    s.validate(&j, &mut Run).unwrap();
    let fake = output(&mut j);
    s.center_samples = fake;
    assert!(s.validate(&j, &mut Run).is_err());
    assert!(j
        .reserve(fake, vexel_raster::MAX_ALLOCATED_BYTES + 1, &mut Run)
        .is_err());
    for (width, height, stride) in [(0, 1, 0), (4097, 1, 16388), (1, 0, 4), (1, 1, 8)] {
        assert!(JobRenderOutput {
            generation: 0,
            pixels: fake,
            width,
            height,
            stride
        }
        .validate(&j)
        .is_err());
    }
}
#[test]
fn global_grid_capacity_limits_admit_before_allocation_and_release_for_reuse() {
    let mut j = job(16 << 20, 64);
    let a = j.buffer::<GridCell>(RasterPool::Grid, 65536, &mut Run).unwrap();
    let b = j.buffer::<GridCell>(RasterPool::Grid, 65536, &mut Run).unwrap();
    j.reserve(a, 65536, &mut Run).unwrap();
    let before = j.stats();
    assert_eq!(
        j.reserve(b, 1, &mut Run).unwrap_err().kind,
        ErrorKind::Limit(LimitKind::Grid)
    );
    assert_eq!(j.stats(), before);
    j.clear_capacity(a, &mut Run).unwrap();
    j.reserve(b, 65536, &mut Run).unwrap();
    j.release(b).unwrap();
    j.reserve(a, 65536, &mut Run).unwrap();
}
#[test]
fn release_all_reclaims_discarded_descriptors_and_invalidates_saved_tokens() {
    let mut j = job(1 << 20, 128);
    let s = JobRasterScratch::new(&mut j, &mut Run).unwrap();
    j.extend(s.center_samples, &[1, 2], &mut Run).unwrap();
    let h = output(&mut j);
    j.extend(h, &[9], &mut Run).unwrap();
    let before = j.stats();
    j.release_all().unwrap();
    assert!(j.slice(h).is_err());
    assert!(j.slice(s.center_samples).is_err());
    assert_eq!(j.stats().live_bytes, before.live_bytes - 3);
    assert_eq!(j.stats().total_allocated_bytes, before.total_allocated_bytes);
    let new = output(&mut j);
    assert!(j.slice(h).is_err());
    j.extend(new, &[8], &mut Run).unwrap();
    assert_eq!(j.slice(new).unwrap(), &[8]);
}

#[test]
fn boundary_limit_counts_marking_work_and_failure_preserves_counter() {
    let mut j = job(1 << 20, 64);
    let h = j.buffer::<u8>(RasterPool::Boundary, 100, &mut Run).unwrap();
    let alias = h;
    j.charge_boundary(h, RASTER_BOUNDARY_WORK_LIMIT - 1, &mut Run)
        .unwrap();
    let before = j.boundary_work(h).unwrap();
    assert!(j.charge_boundary(alias, 2, &mut Run).is_err());
    assert_eq!(j.boundary_work(h).unwrap(), before);
    assert!(j.charge_boundary(h, usize::MAX, &mut Run).is_err());
    assert_eq!(j.boundary_work(h).unwrap(), before);
    j.charge_boundary(alias, 1, &mut Run).unwrap();
    assert_eq!(j.boundary_work(h).unwrap(), 200_000_000);
    j.clear_capacity(h, &mut Run).unwrap();
    assert_eq!(j.boundary_work(alias).unwrap(), 200_000_000);
    let h = j.buffer::<u8>(RasterPool::Boundary, 100, &mut Run).unwrap();
    let mut cancel = CancelAt {
        checkpoints: 0,
        charges: 0,
    };
    assert!(j.charge_boundary(h, 1, &mut cancel).is_err());
    assert_eq!(j.boundary_work(h).unwrap(), 0);
}
