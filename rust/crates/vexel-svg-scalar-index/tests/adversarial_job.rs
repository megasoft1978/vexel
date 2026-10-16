mod support;
use support::*;
use vexel_scene::*;
use vexel_svg_scalar_index::*;
fn rect() -> String {
    "M1 1 L6 1 L6 6 L1 6 Z".into()
}
#[test]
fn boundary_200m_is_work_not_bytes_and_rejects_before_mask_publication() {
    let mut path = "M0 0".to_string();
    for i in 0..20 {
        path.push_str(if i % 2 == 0 { " L4096 4096" } else { " L0 0" });
    }
    path.push_str(" Z");
    let graph = support::graph(&[path], 4096, 4096, false, None, true);
    let view = graph.root();
    let mut job = support::job(&graph, 8 << 20);
    let s = JobRasterScratch::new(&mut job, &mut Run).unwrap();
    let error = mark_boundaries_job(view, map(), &mut job, &s, &mut Run).unwrap_err();
    assert_eq!(error.kind, ErrorKind::Limit(LimitKind::Work));
    assert_eq!(error.offset, 0);
    assert!(job.slice(s.boundary_bits).unwrap().is_empty());
    assert_eq!(job.capacity(s.boundary_bits).unwrap(), 0);
    let h = job
        .buffer::<u8>(RasterPool::Boundary, usize::MAX, &mut Run)
        .unwrap();
    job.charge_boundary(h, 200_000_000, &mut Run).unwrap();
    assert_eq!(
        job.charge_boundary(h, 1, &mut Run).unwrap_err().kind,
        ErrorKind::Limit(LimitKind::Work)
    );
    assert_eq!(job.boundary_work(h).unwrap(), 200_000_000);
}
#[test]
fn invalid_stroke_inverse_keeps_full_bounds_but_original_miter_marking_error() {
    let graph = support::graph(&[rect()], 8, 8, false, None, true);
    let view = graph.root();
    let id = view.edge_shape(0).unwrap();
    let mut frame = FrameBuilder::new(view, 1 << 20).unwrap();
    let mut shape = *view.shape(id).unwrap();
    shape.stroke.rgba = [255, 0, 0, 255];
    shape.stroke.width = 2.;
    shape.stroke_inverse = Affine {
        a: 0.,
        b: 0.,
        c: 0.,
        d: 0.,
        e: 0.,
        f: 0.,
    };
    frame.set_shape(id, shape, &mut Run).unwrap();
    let frame = frame.finish(&mut Run).unwrap();
    let animated = frame.view();
    let mut job = support::job(&graph, 8 << 20);
    let s = JobRasterScratch::new(&mut job, &mut Run).unwrap();
    assert_eq!(
        build_bounds_job(animated, map(), &mut job, &s, &mut Run).unwrap(),
        64
    );
    let error = mark_boundaries_job(animated, map(), &mut job, &s, &mut Run).unwrap_err();
    assert_eq!(error.kind, ErrorKind::Limit(LimitKind::Work));
    assert_eq!(error.offset, 0);
    assert!(job.slice(s.boundary_bits).unwrap().is_empty());
}
#[test]
fn controlled_provider_cancel_releases_unpublished_engine_and_preserves_source() {
    let graph = support::graph(
        &[rect()],
        8,
        8,
        false,
        Some(vexel_geometry::affine_identity()),
        true,
    );
    let view = graph.root();
    let mut job = support::job(&graph, 8 << 20);
    let mut control = StopOn {
        kind: WorkKind::ClipEdge,
        left: 1,
        stop: Stop::Canceled,
    };
    let resources = Resources::default();
    let error = ScalarIndex::build(view, map(), &mut job, &resources, &mut control).unwrap_err();
    assert_eq!(error.kind, ErrorKind::Canceled);
    assert_eq!(resources.clip_visits.get(), 1);
    assert_eq!(
        graph.source_owner().as_slice(),
        view.source_owner().as_slice()
    );
    assert!(job.stats().allocation_count > 0);
    assert_eq!(job.finish().unwrap().live_bytes, 0);
}
#[test]
fn admission_failure_wrong_source_alias_rows_and_malformed_storage_are_errors() {
    let graph = support::graph(&[rect()], 8, 8, false, None, true);
    let view = graph.root();
    let mut small = support::job(&graph, view.source_owner().capacity() + 128);
    assert!(ScalarIndex::build(view, map(), &mut small, &Resources::default(), &mut Run).is_err());
    assert_eq!(small.finish().unwrap().live_bytes, 0);
    let mut job = support::job(&graph, 8 << 20);
    let mut index =
        ScalarIndex::build(view, map(), &mut job, &Resources::default(), &mut Run).unwrap();
    let stats = job.stats();
    let other = support::graph(&[rect()], 8, 8, false, None, true);
    assert_eq!(
        index
            .build_row(other.root(), 0, &mut job, &mut Run)
            .unwrap_err()
            .kind,
        ErrorKind::InvalidArgument
    );
    assert_eq!(job.stats(), stats);
    let mut rows = index.rows;
    let mut state = index.scanlines;
    assert!(
        build_row_job(
            &mut state,
            &mut rows,
            view,
            0,
            &mut job,
            &index.scratch,
            &index.scratch,
            &mut Run
        )
        .is_err()
    );
    job.truncate(index.scratch.shape_offsets, 0).unwrap();
    assert_eq!(
        index
            .build_row(view, 0, &mut job, &mut Run)
            .unwrap_err()
            .kind,
        ErrorKind::Limit(LimitKind::Scratch)
    );
}
#[test]
fn one_edge_uniform_fill_and_valid_empty_scene_have_source_defined_results() {
    let graph = support::graph(&["M0 0 Z".into()], 8, 8, false, None, true);
    let view = graph.root();
    assert_eq!(view.edges.len(), 1);
    let mut job = support::job(&graph, 8 << 20);
    let index = ScalarIndex::build(view, map(), &mut job, &Resources::default(), &mut Run).unwrap();
    assert!(eligible_job(&job, &index.scratch, view.edge_shape(0).unwrap()).unwrap());
    let empty = support::graph(&[], 8, 8, false, None, true);
    let view = empty.root();
    let mut job = support::job(&empty, 8 << 20);
    let mut index =
        ScalarIndex::build(view, map(), &mut job, &Resources::default(), &mut Run).unwrap();
    let mut stats = RenderWorkStats::default();
    preflight_job(
        view,
        &mut index,
        map(),
        &mut job,
        &mut stats,
        &Resources::default(),
        &mut Run,
    )
    .unwrap();
    assert_eq!(stats.total_work, 64);
    assert_eq!(stats.shape_pixels, 0);
    assert_eq!(stats.shape_samples, 0);
}
#[test]
fn parsed_animation_sample_and_frame_geometry_flow_into_same_source_job() {
    let graph = support::graph(&[rect()], 8, 8, false, None, true);
    let overlay =
        vexel_svg_animation::evaluate_view_controlled(graph.animation(), 0.5, &mut Run).unwrap();
    assert_eq!(overlay.items.len(), 1);
    assert_eq!(overlay.items[0].value.number[0], 0.5);
    let view = graph.root();
    let mut frame = FrameBuilder::new(view, 1 << 20).unwrap();
    let mut edges = view.edges.to_vec();
    for edge in &mut edges {
        edge.a.x += 1.;
        edge.b.x += 1.;
    }
    frame.replace_edges(edges, &mut Run).unwrap();
    let mut colors = view.edge_colors.to_vec();
    for color in &mut colors {
        color[3] = (color[3] as f64 * overlay.items[0].value.number[0]).round() as u8;
    }
    frame.replace_colors(colors, &mut Run).unwrap();
    let frame = frame.finish(&mut Run).unwrap();
    let mut job = support::job(&graph, 8 << 20);
    let index = ScalarIndex::build(
        frame.view(),
        map(),
        &mut job,
        &Resources::default(),
        &mut Run,
    )
    .unwrap();
    assert_eq!(
        job.slice(index.scratch.bounds).unwrap()[0].pixels.x_first,
        1
    );
    assert!(std::sync::Arc::ptr_eq(
        frame.view().source_owner(),
        job.source_owner()
    ));
}

#[test]
fn second_clip_pass_invalid_inverse_still_marks_conservatively_full() {
    use std::cell::Cell;
    struct SecondInvalid {
        inner: Resources,
        visits: Cell<usize>,
    }
    impl ControlledKernelResources for SecondInvalid {
        fn paint_sample(
            &self,
            p: &Paint,
            s: &[PaintStop],
            x: f64,
            y: f64,
            c: &mut dyn WorkControl,
        ) -> Result<[u8; 4]> {
            self.inner.paint_sample(p, s, x, y, c)
        }
        fn clip_contains(
            &self,
            v: SceneView<'_>,
            id: ClipId,
            x: f64,
            y: f64,
            c: &mut dyn WorkControl,
        ) -> Result<bool> {
            self.inner.clip_contains(v, id, x, y, c)
        }
        fn clip_work_counts(
            &self,
            v: SceneView<'_>,
            id: ClipId,
            c: &mut dyn WorkControl,
        ) -> Result<ClipWorkCounts> {
            self.inner.clip_work_counts(v, id, c)
        }
        fn stroke_sample_work(
            &self,
            v: SceneView<'_>,
            id: ShapeId,
            d: JobDashView<'_>,
            x: f64,
            y: f64,
            c: &mut dyn WorkControl,
        ) -> Result<(usize, bool, bool)> {
            self.inner.stroke_sample_work(v, id, d, x, y, c)
        }
        fn clip_boundary_edges(
            &self,
            v: SceneView<'_>,
            id: ClipId,
            visit: &mut dyn FnMut(Edge) -> bool,
            c: &mut dyn WorkControl,
        ) -> Result<bool> {
            let n = self.visits.get();
            self.visits.set(n + 1);
            if n == 1 {
                c.charge(WorkKind::ClipChild, 1).map_err(|_| Diagnostic {
                    kind: ErrorKind::Canceled,
                    offset: 0,
                })?;
                return Ok(false);
            }
            self.inner.clip_boundary_edges(v, id, visit, c)
        }
    }
    let graph = graph(
        &[rect()],
        8,
        8,
        false,
        Some(vexel_geometry::affine_identity()),
        true,
    );
    let view = graph.root();
    let mut job = job(&graph, 8 << 20);
    let provider = SecondInvalid {
        inner: Resources::default(),
        visits: Cell::new(0),
    };
    let index = ScalarIndex::build(view, map(), &mut job, &provider, &mut Run).unwrap();
    assert_eq!(
        job.boundary_work(index.scratch.boundary_bits).unwrap(),
        242 + 64
    );
    assert_eq!(index.marked_pixel_count, 64);
    for pixel in 0..64 {
        assert!(boundary_test_job(&job, &index.scratch, 64, pixel).unwrap());
    }
}

#[test]
fn fixture_paint_provider_receives_actual_control_inside_stop_iteration() {
    let identity = vexel_geometry::affine_identity();
    let paint = Paint {
        kind: PaintKind::Linear,
        spread: Spread::Pad,
        object_bounding_box: false,
        coordinates: [0., 0., 1., 0., 0., 0.],
        bounds: [0., 0., 1., 1.],
        user_inverse: identity,
        gradient_inverse: identity,
        stops: TableRange {
            first: 0,
            count: 17,
        },
        pattern: None,
        pattern_inverse: identity,
        pattern_opacity: 1.,
    };
    let stops = (0..17)
        .map(|i| PaintStop {
            offset: i as f64 / 16.,
            rgba: [i, 0, 0, 255],
        })
        .collect::<Vec<_>>();
    let resources = Resources::default();
    let mut stop = StopOn {
        kind: WorkKind::PaintVisit,
        left: 3,
        stop: Stop::Canceled,
    };
    assert_eq!(
        resources
            .paint_sample(&paint, &stops, 1., 0., &mut stop)
            .unwrap_err()
            .kind,
        ErrorKind::Canceled
    );
    assert_eq!(resources.paint_visits.get(), 3);
    assert_eq!(
        resources
            .paint_sample(&paint, &stops, 0.5, 0., &mut Run)
            .unwrap(),
        [8, 0, 0, 255]
    );
    assert_eq!(resources.paint_visits.get(), 20);
}

#[test]
fn every_engine_descriptor_uses_one_source_charge_and_partial_bytes_count_once() {
    let graph = graph(&[rect()], 8, 8, false, None, true);
    let owners = std::sync::Arc::strong_count(graph.source_owner());
    let mut job = job(&graph, 8 << 20);
    assert_eq!(
        std::sync::Arc::strong_count(graph.source_owner()),
        owners + 1
    );
    let index = ScalarIndex::build(
        graph.root(),
        map(),
        &mut job,
        &Resources::default(),
        &mut Run,
    )
    .unwrap();
    assert_eq!(
        std::sync::Arc::strong_count(graph.source_owner()),
        owners + 1
    );
    assert_eq!(job.stats().source_bytes, graph.source_owner().capacity());
    let s = JobRasterScratch::new(&mut job, &mut Run).unwrap();
    job.extend(s.boundary_bits, &[255, 255], &mut Run).unwrap();
    assert_eq!(marked_pixel_count_job(&job, &s, 13, &mut Run).unwrap(), 13);
    assert!(boundary_test_job(&job, &s, 13, 13).unwrap());
    index.release(&mut job).unwrap();
    s.release(&mut job).unwrap();
    assert_eq!(job.finish().unwrap().live_bytes, 0);
    assert_eq!(std::sync::Arc::strong_count(graph.source_owner()), owners);
}

#[test]
fn fill_run_limit_checks_retained_actual_capacity_not_only_requested_width() {
    let graph = graph(&[rect()], 8, 8, false, None, true);
    let view = graph.root();
    let mut job = job(&graph, 32 << 20);
    let s = JobRasterScratch::new(&mut job, &mut Run).unwrap();
    let backup = JobRasterScratch::new(&mut job, &mut Run).unwrap();
    let cap = (8 * 1024 * 1024) / std::mem::size_of::<PixelRun>() + 1;
    job.reserve(s.pixel_runs, cap, &mut Run).unwrap();
    assert!(
        job.capacity(s.pixel_runs).unwrap() * std::mem::size_of::<PixelRun>() > 8 * 1024 * 1024
    );
    let error = create_fill_cache_job(view, &mut job, &s, &backup, &mut Run).unwrap_err();
    assert_eq!(error.kind, ErrorKind::Limit(LimitKind::PixelRuns));
    assert!(job.slice(s.eligible_shapes).unwrap().is_empty());
    assert_eq!(job.finish().unwrap().live_bytes, 0);
}
