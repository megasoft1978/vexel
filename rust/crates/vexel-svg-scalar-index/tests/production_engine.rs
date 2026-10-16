mod support;
use support::*;
use vexel_scene::*;
use vexel_svg_scalar_index::*;
fn rectangle() -> String {
    "M1 1 L6 1 L6 6 L1 6 Z".into()
}

#[test]
fn rectangle_preflight_matches_original_exact_cost_terms_and_has_no_row_allocation() {
    let graph = support::graph(&[rectangle()], 8, 8, false, None, true);
    let view = graph.root();
    let mut job = support::job(&graph, 8 << 20);
    let resources = Resources::default();
    let mut index = ScalarIndex::build(view, map(), &mut job, &resources, &mut Run).unwrap();
    assert_eq!(index.pixel_count, 49);
    assert_eq!(job.boundary_work(index.scratch.boundary_bits).unwrap(), 112);
    setup_cost(&index, &mut job, 1);
    let memory = job.stats();
    let mut stats = RenderWorkStats::default();
    preflight_job(
        view,
        &mut index,
        map(),
        &mut job,
        &mut stats,
        &resources,
        &mut Run,
    )
    .unwrap();
    // C formulas: 230 base + 156 event + 60 lookup + 50 listed + 42
    // shape-row visits + 500 interval setup + 49 runs + 1014 tests + 385 samples.
    assert_eq!(stats.shape_pixels, 49);
    assert_eq!(stats.fill_edge_tests, 50);
    assert_eq!(stats.shape_row_visits, 7);
    assert_eq!(stats.shape_samples, 385);
    assert_eq!(stats.fill_interval_tests, 1014);
    assert_eq!(stats.total_work, 2486);
    assert_eq!(job.stats(), memory);
    index.reset(&mut job, &mut Run).unwrap();
    for y in 0..8 {
        index.build_row(view, y, &mut job, &mut Run).unwrap();
        assert_eq!(job.stats(), memory);
    }
    index.release(&mut job).unwrap();
    assert_eq!(job.finish().unwrap().live_bytes, 0);
}

#[test]
fn all_five_rows_match_brute_edges_and_painter_order_through_two_child_removal() {
    let paths = (0..9)
        .map(|i| {
            format!(
                "M1 {} L6 {} L6 {} L1 {} Z",
                i % 3,
                i % 3,
                3 + i % 5,
                3 + i % 5
            )
        })
        .collect::<Vec<_>>();
    let graph = support::graph(&paths, 8, 10, false, None, true);
    let view = graph.root();
    let mut job = support::job(&graph, 8 << 20);
    let mut index =
        ScalarIndex::build(view, map(), &mut job, &Resources::default(), &mut Run).unwrap();
    let memory = job.stats();
    let events = job.slice(index.scratch.scanline_events).unwrap();
    assert!(
        events
            .windows(2)
            .all(|w| (w[0].slot, w[0].edge, w[0].add) <= (w[1].slot, w[1].edge, w[1].add))
    );
    let events = job.slice(index.scratch.shape_events).unwrap();
    assert!(
        events
            .windows(2)
            .all(|w| (w[0].row, w[0].add, w[0].shape) <= (w[1].row, w[1].add, w[1].shape))
    );
    for y in 0..10 {
        index.build_row(view, y, &mut job, &mut Run).unwrap();
        let expected = job
            .slice(index.scratch.bounds)
            .unwrap()
            .iter()
            .enumerate()
            .filter(|(_, b)| b.pixels.visible && y >= b.pixels.y_first && y <= b.pixels.y_last)
            .map(|(i, _)| i as u32)
            .collect::<Vec<_>>();
        assert_eq!(index.active_shapes(&job).unwrap(), expected);
        let stride = view.shapes.len() + 1;
        for (sample, dy) in [0.125, 0.375, 0.5, 0.625, 0.875].iter().enumerate() {
            for &shape in &expected {
                let offsets = job.slice(index.scratch.shape_offsets).unwrap();
                let a = offsets[sample * stride + shape as usize];
                let b = offsets[sample * stride + shape as usize + 1];
                let sy = y as f64 + dy;
                let brute = view
                    .edges
                    .iter()
                    .enumerate()
                    .filter(|(i, e)| {
                        view.edge_shape(*i).unwrap().index() == shape as usize
                            && e.a.y.min(e.b.y) <= sy
                            && sy < e.a.y.max(e.b.y)
                    })
                    .map(|(i, _)| i as u32)
                    .collect::<Vec<_>>();
                assert_eq!(&index.active_edges(&job).unwrap()[a..b], brute);
            }
        }
        assert_eq!(job.stats(), memory);
    }
}

#[test]
fn nonzero_evenodd_coincident_crossings_and_half_open_intervals() {
    for evenodd in [false, true] {
        let path = "M0 0 L6 0 L6 6 L0 6 Z M2 2 L4 2 L4 4 L2 4 Z".to_string();
        let graph = support::graph(&[path], 8, 8, evenodd, None, true);
        let view = graph.root();
        let mut job = support::job(&graph, 8 << 20);
        let mut index =
            ScalarIndex::build(view, map(), &mut job, &Resources::default(), &mut Run).unwrap();
        for y in 0..=3 {
            index.build_row(view, y, &mut job, &mut Run).unwrap();
        }
        let id = view.edge_shape(0).unwrap();
        let work = index
            .build_fill_row(view, id, 3, map(), &mut job, &mut Run)
            .unwrap();
        assert_eq!(work.edge_visits, 20);
        assert_eq!(work.work_units, 240);
        assert_eq!(index.fill.interval_counts, [if evenodd { 2 } else { 3 }; 5]);
        for row in 0..5 {
            assert_eq!(
                fill_contains_job(&index.fill, &job, &index.scratch, row, 3., &mut Run)
                    .unwrap()
                    .0,
                !evenodd
            );
            assert!(
                fill_contains_job(&index.fill, &job, &index.scratch, row, 1., &mut Run)
                    .unwrap()
                    .0
            );
            assert!(
                !fill_contains_job(&index.fill, &job, &index.scratch, row, 6., &mut Run)
                    .unwrap()
                    .0
            );
        }
    }
    // Coincident reversed subpaths cancel winding and parity at equal x ties.
    let graph = support::graph(
        &["M0 0 L6 0 L6 6 L0 6 Z M0 0 L0 6 L6 6 L6 0 Z".into()],
        8,
        8,
        false,
        None,
        true,
    );
    let view = graph.root();
    let mut job = support::job(&graph, 8 << 20);
    let mut index =
        ScalarIndex::build(view, map(), &mut job, &Resources::default(), &mut Run).unwrap();
    for y in 0..=2 {
        index.build_row(view, y, &mut job, &mut Run).unwrap();
    }
    index
        .build_fill_row(
            view,
            view.edge_shape(0).unwrap(),
            2,
            map(),
            &mut job,
            &mut Run,
        )
        .unwrap();
    assert_eq!(index.fill.interval_counts, [0; 5]);
}

#[test]
fn exactly_4096_edges_fast_and_4097_edges_scalar_fallback() {
    for edges in [4096, 4097] {
        let mut path = "M0 0".to_string();
        for i in 0..edges - 1 {
            path.push_str(&format!(" L{} {}", i % 8, if i % 2 == 0 { 8 } else { 0 }));
        }
        path.push_str(" Z");
        let graph = support::graph(&[path], 8, 8, false, None, true);
        let view = graph.root();
        assert_eq!(view.edges.len(), edges);
        let mut job = support::job(&graph, 32 << 20);
        let index =
            ScalarIndex::build(view, map(), &mut job, &Resources::default(), &mut Run).unwrap();
        assert_eq!(
            eligible_job(&job, &index.scratch, view.edge_shape(0).unwrap()).unwrap(),
            edges == 4096
        );
        if edges == 4097 {
            assert_eq!(index.fill.max_edge_count, 0);
            assert!(job.slice(index.scratch.crossings).unwrap().is_empty());
            let mut index = index;
            setup_cost(&index, &mut job, 1);
            let mut stats = RenderWorkStats::default();
            let memory = job.stats();
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
            assert_eq!(stats.shape_samples, 1024);
            assert!(stats.fill_edge_tests > 4_000_000);
            assert_eq!(stats.fill_interval_tests, 0);
            assert_eq!(job.stats(), memory);
        }
    }
}

#[test]
fn controlled_clip_two_visits_actual_filled_counts_and_conservative_invalid_inverse() {
    let graph = support::graph(
        &[rectangle()],
        8,
        8,
        false,
        Some(vexel_geometry::affine_identity()),
        true,
    );
    let view = graph.root();
    let mut job = support::job(&graph, 8 << 20);
    let resources = Resources::default();
    let index = ScalarIndex::build(view, map(), &mut job, &resources, &mut Run).unwrap();
    assert_eq!(resources.clip_visits.get(), 8);
    // Original 3 outer scans + chain1 + 2*(chain1+child2) + 2*4 edges
    // plus the four box areas (each 28). Both filled/unfilled child counts stay.
    assert_eq!(
        job.boundary_work(index.scratch.boundary_bits).unwrap(),
        112 + 3 + 7 + 8 + 112
    );
    let singular = Affine {
        a: 0.,
        b: 0.,
        c: 0.,
        d: 0.,
        e: 0.,
        f: 0.,
    };
    let graph = support::graph(&[rectangle()], 8, 8, false, Some(singular), true);
    let view = graph.root();
    let mut job = support::job(&graph, 8 << 20);
    let index = ScalarIndex::build(view, map(), &mut job, &Resources::default(), &mut Run).unwrap();
    assert_eq!(
        job.boundary_work(index.scratch.boundary_bits).unwrap(),
        112 + 3 + 7 + 64
    );
    for p in 0..64 {
        assert!(boundary_test_job(&job, &index.scratch, 64, p).unwrap());
    }
}

#[test]
fn row_fill_and_whole_preflight_cancel_restore_payload_and_scalar_state() {
    let graph = support::graph(&[rectangle()], 8, 8, false, None, true);
    let view = graph.root();
    let mut job = support::job(&graph, 8 << 20);
    let resources = Resources::default();
    let mut index = ScalarIndex::build(view, map(), &mut job, &resources, &mut Run).unwrap();
    setup_cost(&index, &mut job, 1);
    let initial = index.scanlines;
    let nodes = job.slice(index.scratch.scanline_nodes).unwrap().to_vec();
    let memory = job.stats();
    let mut stop = StopOn {
        kind: WorkKind::ScanlineEdgeEntry,
        left: 11,
        stop: Stop::Canceled,
    };
    assert_eq!(
        index
            .build_row(view, 0, &mut job, &mut stop)
            .unwrap_err()
            .kind,
        ErrorKind::Canceled
    );
    assert_eq!(index.scanlines.event_cursor, initial.event_cursor);
    assert_eq!(job.slice(index.scratch.scanline_nodes).unwrap(), nodes);
    assert_eq!(job.stats(), memory);
    for y in 0..=2 {
        index.build_row(view, y, &mut job, &mut Run).unwrap();
    }
    let old_fill = index.fill;
    let intervals = job.slice(index.scratch.intervals).unwrap().to_vec();
    let mut stop = StopOn {
        kind: WorkKind::FillEdgeTest,
        left: 2,
        stop: Stop::Canceled,
    };
    assert_eq!(
        index
            .build_fill_row(
                view,
                view.edge_shape(0).unwrap(),
                2,
                map(),
                &mut job,
                &mut stop
            )
            .unwrap_err()
            .kind,
        ErrorKind::Canceled
    );
    assert_eq!(index.fill, old_fill);
    assert_eq!(job.slice(index.scratch.intervals).unwrap(), intervals);
    let before = index.scanlines;
    let rows = index.rows;
    let offsets = job.slice(index.scratch.shape_offsets).unwrap().to_vec();
    let mut stats = RenderWorkStats::default();
    let old_stats = stats;
    let mut stop = StopOn {
        kind: WorkKind::FillIntervalTest,
        left: 25,
        stop: Stop::Limit,
    };
    assert_eq!(
        preflight_job(
            view,
            &mut index,
            map(),
            &mut job,
            &mut stats,
            &resources,
            &mut stop
        )
        .unwrap_err()
        .kind,
        ErrorKind::Limit(LimitKind::Work)
    );
    assert_eq!(stats, old_stats);
    assert_eq!(index.scanlines.event_cursor, before.event_cursor);
    assert_eq!(index.scanlines.listed_edge_count, before.listed_edge_count);
    assert_eq!(index.rows, rows);
    assert_eq!(job.slice(index.scratch.shape_offsets).unwrap(), offsets);
    assert_eq!(job.stats(), memory);
}
