//! WO-RUST-LIMITS-03: the shape cap is 524,288 at the literal boundary. The literal
//! targets are frozen here so that a regression of the constant fails by name.
mod support;
use support::*;
use vexel_scene::*;

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
fn limit03_shape_cap_is_524288() {
    assert_eq!(MAX_SHAPES, 524_288);
    assert_eq!(Limits::default().shapes, 524_288);
}

#[test]
fn limit03_dash_cache_accepts_524288_and_refuses_524289() {
    assert!(dash_cache_with(524_288).validate(&mut Run).is_ok());
    assert_eq!(
        dash_cache_with(524_289).validate(&mut Run).unwrap_err().kind,
        ErrorKind::Limit(LimitKind::Shapes)
    );
}
