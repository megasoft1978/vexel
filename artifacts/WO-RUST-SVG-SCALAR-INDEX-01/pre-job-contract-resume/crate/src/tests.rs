use crate::*;
use vexel_scene::{
    BoundaryMask, FillCache, FillCrossing, FillInterval, ScanlineIndex, ScanlineNode, Stop,
    WorkControl, WorkKind,
};

struct Run;
impl WorkControl for Run {
    fn charge(&mut self, _: WorkKind, _: u64) -> std::result::Result<(), Stop> {
        Ok(())
    }
    fn checkpoint(&mut self) -> std::result::Result<(), Stop> {
        Ok(())
    }
}
struct Cancel;
impl WorkControl for Cancel {
    fn charge(&mut self, _: WorkKind, _: u64) -> std::result::Result<(), Stop> {
        Err(Stop::Canceled)
    }
    fn checkpoint(&mut self) -> std::result::Result<(), Stop> {
        Err(Stop::Canceled)
    }
}

#[test]
fn original_raster_budget_assertions() {
    assert_eq!(work_budget(1, 1), 200_000_000);
    assert_eq!(work_budget(1024, 1024), 200_000_000);
    assert_eq!(work_budget(1056, 1000), 382_452_224);
    assert_eq!(work_budget(1600, 1000), 13_751_796_224);
    assert_eq!(work_budget(8192, 8192), 16_000_000_000);
    assert_eq!(work_budget(u32::MAX, u32::MAX), 16_000_000_000);
}

#[test]
fn original_pixel_range_margin_assertions() {
    assert_eq!(crate::util::pixel_range(1.0, 7.0, 8), Some((0, 7)));
    assert_eq!(crate::util::pixel_range(-4.0, -2.0, 8), None);
    assert_eq!(crate::util::pixel_range(f64::NAN, 1.0, 8), None);
    assert_eq!(crate::util::boundary_pixel_range(2.0, 3.0, 8), Some((0, 4)));
    assert_eq!(crate::util::boundary_pixel_range(0.0, 0.0, 8), Some((0, 1)));
}

#[test]
fn boundary_mask_out_of_range_is_conservative() {
    let mask = BoundaryMask {
        bits: vec![0b0000_0100],
        bit_count: 8,
        marked_pixel_count: 1,
        mark_work: 1,
    };
    assert!(!boundary_test(&mask, 0));
    assert!(boundary_test(&mask, 2));
    assert!(boundary_test(&mask, 8));
    let malformed = BoundaryMask {
        bits: Vec::new(),
        bit_count: 8,
        ..BoundaryMask::default()
    };
    assert!(boundary_test(&malformed, 0));
}

#[test]
fn checked_size_and_cancellation_errors_are_preserved() {
    assert!(crate::util::add(usize::MAX, 1).is_err());
    assert!(crate::util::mul(usize::MAX, 2).is_err());
    let mut canceled = Cancel;
    let error = crate::util::poll(&mut canceled, WorkKind::BoundaryMark).unwrap_err();
    assert_eq!(error.kind, vexel_scene::ErrorKind::Canceled);
}

#[test]
fn work_budget_cap_rejects_without_changing_accumulator() {
    let budget = work_budget(1, 1);
    let mut work = 0usize;
    crate::preflight::work_add(&mut work, budget as usize, budget).unwrap();
    assert_eq!(work, budget as usize);
    assert!(crate::preflight::work_add(&mut work, 1, budget).is_err());
    assert_eq!(work, budget as usize);
}

#[test]
fn crossing_heapsort_preserves_x_then_edge_tiebreak() {
    let mut values = vec![
        FillCrossing {
            x: 4.0,
            winding_delta: 1,
            edge_index: 8,
        },
        FillCrossing {
            x: 2.0,
            winding_delta: -1,
            edge_index: 3,
        },
        FillCrossing {
            x: 2.0,
            winding_delta: 1,
            edge_index: 1,
        },
        FillCrossing {
            x: -1.0,
            winding_delta: 1,
            edge_index: 9,
        },
    ];
    let mut run = Run;
    crate::fill::heap_sort(&mut values, &mut run).unwrap();
    assert_eq!(
        values
            .iter()
            .map(|c| (c.x, c.edge_index))
            .collect::<Vec<_>>(),
        vec![(-1.0, 9), (2.0, 1), (2.0, 3), (4.0, 8)]
    );
}

#[test]
fn interval_binary_search_keeps_half_open_edges_and_comparison_count() {
    let mut cache = FillCache::default();
    cache.interval_stride = 3;
    cache.intervals = vec![
        FillInterval {
            left: 1.0,
            right: 2.0,
        },
        FillInterval {
            left: 4.0,
            right: 7.0,
        },
        FillInterval::default(),
    ];
    cache.interval_counts[0] = 2;
    assert_eq!(fill_contains(&cache, 0, 1.0), (true, 3));
    assert_eq!(fill_contains(&cache, 0, 2.0), (false, 3));
    assert_eq!(fill_contains(&cache, 0, 6.9), (true, 3));
    assert_eq!(fill_contains(&cache, 5, 1.0), (false, 0));
}

#[test]
fn avl_events_remain_sorted_and_removal_releases_all_nodes() {
    let mut nodes = vec![
        ScanlineNode {
            left: -1,
            right: -1,
            height: 1
        };
        128
    ];
    let mut root = -1;
    for key in 0..128 {
        root = crate::scanline::insert(&mut nodes, root, key, 0).unwrap();
    }
    let mut out = Vec::new();
    let mut run = Run;
    crate::scanline::append_order(&nodes, root, &mut out, 128, &mut run).unwrap();
    assert_eq!(out, (0..128).collect::<Vec<u32>>());
    assert!(nodes[root as usize].height <= 8);
    for key in (0..128).rev() {
        root = crate::scanline::remove(&mut nodes, root, key, 0).unwrap();
    }
    assert_eq!(root, -1);
    assert!(
        crate::scanline::insert(&mut nodes, -1, 0, crate::util::SCANLINE_MAX_TREE_DEPTH).is_none()
    );
    assert!(crate::scanline::remove(&mut nodes, -1, 0, 0).is_none());
}

#[test]
fn row_buffers_reserve_five_sample_offsets_and_active_edge_capacity() {
    let index = ScanlineIndex {
        shape_count: 7,
        edge_count: 11,
        ..ScanlineIndex::default()
    };
    let mut rows = vexel_scene::RowBuffers::default();
    prepare_rows(&index, &mut rows).unwrap();
    assert_eq!(rows.shape_offsets.len(), 40);
    assert!(rows.active_edges.capacity() >= 55);
    assert!(rows.active_shapes.capacity() >= 7);
}

#[test]
fn scanline_tree_polling_propagates_cancellation_before_output() {
    let nodes = vec![ScanlineNode {
        left: -1,
        right: -1,
        height: 1,
    }];
    let mut out = Vec::new();
    let mut cancel = Cancel;
    let error = crate::scanline::append_order(&nodes, 0, &mut out, 1, &mut cancel).unwrap_err();
    assert_eq!(error.kind, vexel_scene::ErrorKind::Canceled);
    assert!(out.is_empty());
}

#[test]
fn malformed_scanline_storage_is_rejected_before_indexing() {
    let mut index = ScanlineIndex::default();
    index.shape_count = 1;
    index.edge_count = 1;
    index.nodes.push(ScanlineNode {
        left: 9,
        right: -1,
        height: 1,
    });
    index.roots.push(0);
    index.shape_nodes.push(ScanlineNode {
        left: -1,
        right: -1,
        height: 1,
    });
    assert!(crate::scanline::validate_index(&index).is_err());
}
