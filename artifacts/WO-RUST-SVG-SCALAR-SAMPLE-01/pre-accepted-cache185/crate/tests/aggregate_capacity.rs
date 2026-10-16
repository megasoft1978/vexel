//! Public builder proofs; geometry is an explicitly constructed fixture admitted
//! through the real SceneBuilder. No cache-policy wrapper or fake allocator.
use std::{mem::size_of, sync::Arc};
use vexel_geometry::{Edge, Point, affine_identity};
use vexel_scene::*;
use vexel_svg_scalar_sample::{DASH_CACHE_BYTE_LIMIT as CAP, build_dash_cache};

struct Run;
impl WorkControl for Run {
    fn charge(&mut self, _: WorkKind, _: u64) -> std::result::Result<(), Stop> {
        Ok(())
    }
    fn checkpoint(&mut self) -> std::result::Result<(), Stop> {
        Ok(())
    }
}
fn fixture(counts: &[usize]) -> (SceneGraph, SourceOwner) {
    let source: SourceOwner = Arc::new(b"<svg/>".to_vec());
    let lexical = LexicalDocument::parse(source.clone(), 1024, &mut Run).unwrap();
    let animation = vexel_svg_animation::parse_owned(&source, lexical.tokens()).unwrap();
    let mut builder = SceneBuilder::new(
        source.clone(),
        animation,
        1,
        Viewport {
            width: 16,
            height: 16,
            view_x: 0.0,
            view_y: 0.0,
            view_width: 16.0,
            view_height: 16.0,
            aspect: Default::default(),
        },
        Limits::default(),
    )
    .unwrap();
    let root = builder.root_id();
    let draft = builder.begin_draft(root).unwrap();
    for &count in counts {
        let mut shape = Shape::new(SourceMap {
            element: Span {
                start: 0,
                end: source.len(),
            },
            transform: affine_identity(),
            transform_node: None,
        });
        shape.filled = false;
        shape.stroke.rgba = [255; 4];
        shape.stroke.width = 0.25;
        shape.stroke.join = Linejoin::Round;
        let edge = Edge {
            a: Point { x: 0.25, y: 0.25 },
            b: Point { x: 0.5, y: 0.5 },
        };
        builder
            .append_shape(
                root,
                ShapeInput {
                    shape,
                    path: vexel_svg_path::PathDocument {
                        edges: vec![edge; count],
                        subpaths: vec![],
                        segments: vec![],
                        numbers: vec![],
                    },
                    edge_colors: vec![[255; 4]; count],
                },
                &mut Run,
            )
            .unwrap();
    }
    builder.commit_draft(&draft, &mut Run).unwrap();
    (builder.freeze().unwrap(), source)
}
fn job(source: &SourceOwner) -> (RasterJob, JobRasterScratch) {
    let mut job = RasterJob::new(ChargeLedger::new(source, 256 << 20, 4096).unwrap()).unwrap();
    let scratch = JobRasterScratch::new(&mut job, &mut Run).unwrap();
    (job, scratch)
}
fn retained(job: &RasterJob, cache: JobDashCache) -> usize {
    let mut n = job.capacity(cache.shapes).unwrap() * size_of::<JobDashRange>()
        + job.capacity(cache.segments).unwrap() * size_of::<DashSegment>()
        + job.capacity(cache.joins).unwrap() * size_of::<DashJoin>();
    for r in job.slice(cache.shapes).unwrap() {
        n += job.capacity(r.grid_cells).unwrap() * size_of::<GridCell>()
            + job.capacity(r.grid_refs).unwrap() * size_of::<u32>();
    }
    n
}
// At this count both geometric buffers have capacity 176371. Two 1-cell
// grids have exactly 2*n references in total; solve for equality, not tolerance.
fn exact_count() -> usize {
    let fixed = 176_371 * (size_of::<DashSegment>() + size_of::<DashJoin>())
        + 2 * size_of::<JobDashRange>()
        + 2 * size_of::<GridCell>();
    assert_eq!((CAP - fixed) % 8, 0);
    let n = (CAP - fixed) / 8;
    assert!((156_777..176_371).contains(&n));
    n
}
#[test]
fn exact_16mib_cache_succeeds_with_real_grids_and_submillion_dash_work() {
    let n = exact_count();
    let (graph, source) = fixture(&[n, 1]);
    let view = graph.root();
    let (mut job, mut scratch) = job(&source);
    let cache = build_dash_cache(view, view.edges, &[], &mut job, &mut scratch, &mut Run).unwrap();
    assert_eq!(retained(&job, cache), CAP);
    assert_eq!(cache.build_steps, 2 * n + 2);
    assert!(cache.build_steps < 1_000_000);
    assert_eq!(job.capacity(cache.segments).unwrap(), 176_371);
    assert_eq!(job.capacity(cache.joins).unwrap(), 176_371);
    assert!(
        job.slice(cache.shapes)
            .unwrap()
            .iter()
            .all(|r| r.grid_columns == 1 && r.grid_rows == 1)
    );
    job.dash_view(cache, &mut Run).unwrap();
    eprintln!(
        "exact cache: n={n}, retained={}, dash_steps={}",
        retained(&job, cache),
        cache.build_steps
    );
}
#[test]
fn aggregate_grid_cell_and_reference_crossings_fall_back_below_individual_caps() {
    for extra in [1, 3] {
        let n = exact_count() + extra;
        let (graph, source) = fixture(&[n, 1]);
        let view = graph.root();
        let (mut job, mut scratch) = job(&source);
        let cache =
            build_dash_cache(view, view.edges, &[], &mut job, &mut scratch, &mut Run).unwrap();
        let ranges = job.slice(cache.shapes).unwrap();
        let fallback = if extra == 1 { 1 } else { 0 };
        assert_eq!(ranges[fallback].grid_columns, 0);
        assert_eq!(ranges[fallback].grid_rows, 0);
        assert_eq!(job.capacity(ranges[fallback].grid_cells).unwrap(), 0);
        assert_eq!(job.capacity(ranges[fallback].grid_refs).unwrap(), 0);
        assert_eq!(ranges[1 - fallback].grid_columns, 1);
        // Every allocation fits individually; only the aggregate blocks it.
        assert!(176_371 * size_of::<DashSegment>() < CAP);
        assert!(176_371 * size_of::<DashJoin>() < CAP);
        assert!((2 * n - 1) * 4 < CAP);
        assert!(2 * n - 1 < 1_000_000);
        assert!(cache.build_steps < 1_000_000);
        assert_eq!(
            retained(&job, cache),
            if extra == 1 {
                CAP - 12
            } else {
                176_371 * (size_of::<DashSegment>() + size_of::<DashJoin>())
                    + 2 * size_of::<JobDashRange>()
                    + size_of::<GridCell>()
                    + 4
            }
        );
        job.dash_view(cache, &mut Run).unwrap();
    }
}
#[test]
fn aggregate_dash_growth_rejects_and_cleans_up_under_step_cap() {
    let (graph, source) = fixture(&[176_373]);
    let view = graph.root();
    let (mut job, mut scratch) = job(&source);
    let before = job.stats();
    let error =
        build_dash_cache(view, view.edges, &[], &mut job, &mut scratch, &mut Run).unwrap_err();
    assert_eq!(
        error,
        Diagnostic {
            kind: ErrorKind::Limit(LimitKind::DashCache),
            offset: 0
        }
    );
    assert!(198_417 * size_of::<DashSegment>() < CAP);
    assert!(198_417 * size_of::<DashJoin>() < CAP);
    assert!(2 * 176_373 < 1_000_000);
    assert_eq!(
        job.stats().reference_index_bytes,
        before.reference_index_bytes
    );
    assert!(job.stats().total_allocated_bytes > before.total_allocated_bytes);
    job.dash_view(scratch.dash, &mut Run).unwrap();
}
#[test]
fn shared_pool_currently_blocks_valid_near_cap_rebuild_without_losing_old_cache() {
    // This is a reproducer of the protected shared-contract blocker, not a
    // claim that the required near-cap rebuild semantics have been repaired.
    let (graph, source) = fixture(&[exact_count(), 1]);
    let view = graph.root();
    let (mut job, mut scratch) = job(&source);
    let cache = build_dash_cache(view, view.edges, &[], &mut job, &mut scratch, &mut Run).unwrap();
    let before = job.stats();
    for _ in 0..2 {
        let error =
            build_dash_cache(view, view.edges, &[], &mut job, &mut scratch, &mut Run).unwrap_err();
        assert_eq!(
            error,
            Diagnostic {
                kind: ErrorKind::Limit(LimitKind::DashCache),
                offset: 0
            }
        );
        assert_eq!(
            job.stats().reference_index_bytes,
            before.reference_index_bytes
        );
        assert_eq!(retained(&job, scratch.dash), CAP);
        job.dash_view(cache, &mut Run).unwrap();
    }
    // The shared aggregate check rejects the very first shape allocation.
    // No physical payload allocation occurred; history must say exactly that.
    assert_eq!(
        job.stats().total_allocated_bytes,
        before.total_allocated_bytes
    );
    assert_eq!(job.stats().peak_live_bytes, before.peak_live_bytes);
}

struct StopStroke {
    at: usize,
    calls: usize,
    stop: Stop,
}
impl WorkControl for StopStroke {
    fn charge(&mut self, kind: WorkKind, _: u64) -> std::result::Result<(), Stop> {
        if kind == WorkKind::StrokeTest {
            self.calls += 1;
            if self.calls == self.at {
                return Err(self.stop);
            }
        }
        Ok(())
    }
    fn checkpoint(&mut self) -> std::result::Result<(), Stop> {
        Ok(())
    }
}
#[test]
fn every_dash_and_partial_grid_stop_restores_cache_and_repeated_rebuilds_release_handles() {
    let (graph, source) = fixture(&[3, 5]);
    let view = graph.root();
    let (mut job, mut scratch) = job(&source);
    let mut meter = StopStroke {
        at: usize::MAX,
        calls: 0,
        stop: Stop::Canceled,
    };
    let cache =
        build_dash_cache(view, view.edges, &[], &mut job, &mut scratch, &mut meter).unwrap();
    let calls = meter.calls;
    assert_eq!(calls, 88);
    let segments = job.slice(cache.segments).unwrap().to_vec();
    let joins = job.slice(cache.joins).unwrap().to_vec();
    let ranges = job.slice(cache.shapes).unwrap().to_vec();
    let grids: Vec<_> = ranges
        .iter()
        .map(|r| {
            (
                job.slice(r.grid_cells).unwrap().to_vec(),
                job.slice(r.grid_refs).unwrap().to_vec(),
            )
        })
        .collect();
    let before_payload = job.stats().reference_index_bytes;
    let mut history = job.stats().total_allocated_bytes;
    for stop in [Stop::Canceled, Stop::Limit] {
        for at in 1..=calls {
            let error = build_dash_cache(
                view,
                view.edges,
                &[],
                &mut job,
                &mut scratch,
                &mut StopStroke { at, calls: 0, stop },
            )
            .unwrap_err();
            assert_eq!(
                error,
                Diagnostic {
                    kind: if stop == Stop::Canceled {
                        ErrorKind::Canceled
                    } else {
                        ErrorKind::Limit(LimitKind::Work)
                    },
                    offset: 0
                }
            );
            assert_eq!(job.stats().reference_index_bytes, before_payload);
            assert!(job.stats().total_allocated_bytes >= history);
            history = job.stats().total_allocated_bytes;
            assert_eq!(job.slice(cache.segments).unwrap(), segments);
            assert_eq!(job.slice(cache.joins).unwrap(), joins);
            for (i, r) in job.slice(scratch.dash.shapes).unwrap().iter().enumerate() {
                assert_eq!(r.grid_columns, ranges[i].grid_columns);
                assert_eq!(r.grid_rows, ranges[i].grid_rows);
                assert_eq!(job.slice(r.grid_cells).unwrap(), grids[i].0);
                assert_eq!(job.slice(r.grid_refs).unwrap(), grids[i].1);
            }
            job.dash_view(cache, &mut Run).unwrap();
        }
    }
    let mut live = None;
    for _ in 0..100 {
        let old = scratch.dash;
        let next =
            build_dash_cache(view, view.edges, &[], &mut job, &mut scratch, &mut Run).unwrap();
        assert!(job.slice(old.shapes).is_err());
        assert_eq!(job.stats().reference_index_bytes, retained(&job, next));
        if let Some(bytes) = live {
            assert_eq!(job.stats().live_bytes, bytes);
        }
        live = Some(job.stats().live_bytes);
        job.dash_view(next, &mut Run).unwrap();
    }
}
