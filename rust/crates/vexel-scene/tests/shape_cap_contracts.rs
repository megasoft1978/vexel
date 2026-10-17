//! WO-RUST-SCENE-SHAPES-01: the shape cap is `MAX_SHAPES` at every raster-job and
//! scene site, and the dash-cache boundary is exact.
mod support;
use support::*;
use vexel_scene::*;

#[test]
fn default_limits_use_the_named_shape_cap() {
    assert_eq!(Limits::default().shapes, MAX_SHAPES);
}

fn dash_cache_with(shapes: usize) -> DashCache {
    DashCache {
        segments: Vec::new(),
        joins: Vec::new(),
        shapes: (0..shapes)
            .map(|_| DashRange {
                segments: TableRange::default(),
                joins: TableRange::default(),
                grid_cells: Vec::new(),
                grid_refs: Vec::new(),
                grid_min_x: 0.0,
                grid_min_y: 0.0,
                grid_cell_size: 0.0,
                grid_columns: 0,
                grid_rows: 0,
            })
            .collect(),
        build_steps: 0,
        grid_cell_count: 0,
        grid_ref_count: 0,
        grid_bytes: 0,
    }
}

#[test]
fn builder_append_admits_max_shapes_and_refuses_the_next() {
    let mut b = builder();
    let mut c = Run;
    let draft = b.begin_draft(b.root_id()).unwrap();
    let _request = b.begin_request(&draft, &mut c).unwrap();
    let root = b.root_id();
    for i in 0..MAX_SHAPES {
        b.append_shape(root, shape(b.source_owner()), &mut c)
            .unwrap_or_else(|e| panic!("shape {i} refused: {e:?}"));
    }
    let error = b
        .append_shape(root, shape(b.source_owner()), &mut c)
        .unwrap_err();
    assert_eq!(error.kind, ErrorKind::Limit(LimitKind::Shapes));
}

#[test]
fn shape_record_bytes_at_the_cap_are_measured() {
    let record = std::mem::size_of::<Shape>();
    eprintln!(
        "MEASURE size_of_shape={record} shape_record_bytes_at_cap={}",
        MAX_SHAPES * record
    );
    assert!(MAX_SHAPES * record <= 512 * 1024 * 1024);
}

#[test]
fn dash_cache_shape_boundary_is_max_shapes() {
    assert!(dash_cache_with(MAX_SHAPES).validate(&mut Run).is_ok());
    assert_eq!(
        dash_cache_with(MAX_SHAPES + 1)
            .validate(&mut Run)
            .unwrap_err()
            .kind,
        ErrorKind::Limit(LimitKind::Shapes)
    );
}
