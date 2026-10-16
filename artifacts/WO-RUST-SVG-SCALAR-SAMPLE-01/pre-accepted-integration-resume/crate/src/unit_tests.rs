use super::*;
use vexel_geometry::{Edge, Point};
use vexel_scene::{Linecap, Stroke};

struct Control;
impl WorkControl for Control {
    fn charge(
        &mut self,
        _: vexel_scene::WorkKind,
        _: u64,
    ) -> std::result::Result<(), vexel_scene::Stop> {
        Ok(())
    }
    fn checkpoint(&mut self) -> std::result::Result<(), vexel_scene::Stop> {
        Ok(())
    }
}
struct LimitedControl(u64);
impl WorkControl for LimitedControl {
    fn charge(
        &mut self,
        _: vexel_scene::WorkKind,
        units: u64,
    ) -> std::result::Result<(), vexel_scene::Stop> {
        if units > self.0 {
            return Err(vexel_scene::Stop::Limit);
        }
        self.0 -= units;
        Ok(())
    }
    fn checkpoint(&mut self) -> std::result::Result<(), vexel_scene::Stop> {
        Ok(())
    }
}
fn p(x: f64, y: f64) -> Point {
    Point { x, y }
}
fn has_values(actual: Option<&[f64]>, expected: &[f64], count: usize) -> bool {
    let Some(actual) = actual else { return false };
    count <= expected.len()
        && (0..count).all(|i| {
            actual
                .get(i)
                .is_some_and(|value| value.is_finite() && *value == expected[i])
        })
}
#[test]
fn original_has_values_assertion_predicate() {
    assert!(has_values(Some(&[1.0, 2.0, 3.0]), &[1.0, 2.0, 3.0], 3));
    assert!(!has_values(None, &[1.0], 1));
    assert!(!has_values(Some(&[f64::NAN]), &[f64::NAN], 1));
    assert!(!has_values(Some(&[1.0]), &[2.0], 1));
    assert!(!has_values(Some(&[]), &[1.0], 1));
}
#[test]
fn original_dash_storage_and_seam_predicates() {
    let edges = [Edge {
        a: p(0.0, 0.0),
        b: p(10.0, 0.0),
    }];
    let mut stroke = Stroke::initial();
    stroke.dash_offset = 0.0;
    stroke.dash_values = vexel_scene::TableRange { first: 0, count: 3 };
    let d = [1.0, 2.0, 3.0];
    assert!(!dash_range_joins_seam(&stroke, &d, &edges, 0, 1, &mut Control).unwrap());
    stroke.dash_values.count = 2;
    let alternating = [4.0, 5.0];
    assert!(dash_range_joins_seam(&stroke, &alternating, &edges, 0, 1, &mut Control).unwrap());
    stroke.dash_offset = -1.0;
    assert!(!dash_range_joins_seam(&stroke, &alternating, &edges, 0, 1, &mut Control).unwrap());
}
#[test]
fn dash_seam_walk_propagates_work_limits() {
    let edges = [Edge {
        a: p(0.0, 0.0),
        b: p(100.0, 0.0),
    }];
    let mut stroke = Stroke::initial();
    let result = dash_range_joins_seam(&stroke, &[1.0, 1.0], &edges, 0, 1, &mut LimitedControl(8));
    assert_eq!(
        result.unwrap_err().kind,
        vexel_scene::ErrorKind::Limit(vexel_scene::LimitKind::Work)
    );
    stroke.dash_offset = f64::NAN;
    assert!(!dash_range_joins_seam(&stroke, &[1.0, 1.0], &edges, 0, 1, &mut Control).unwrap());
}
#[test]
fn original_stroke_cap_and_join_geometry_predicates() {
    use crate::stroke::{join_hit, segment_caps};
    let a = p(4.0, 8.0);
    let b = p(12.0, 8.0);
    assert!(!segment_caps(
        a,
        b,
        3.0,
        7.0,
        1.0,
        Linecap::Butt,
        Linecap::Butt
    ));
    assert!(segment_caps(
        a,
        b,
        3.0,
        7.0,
        1.0,
        Linecap::Square,
        Linecap::Square
    ));
    assert!(segment_caps(
        a,
        b,
        3.0,
        8.0,
        1.0,
        Linecap::Round,
        Linecap::Round
    ));
    let prev = p(4.0, 10.0);
    let v = p(8.0, 6.0);
    let next = p(12.0, 10.0);
    assert!(join_hit(
        prev,
        v,
        next,
        8.0,
        4.0,
        2.0,
        vexel_scene::Linejoin::Miter,
        4.0
    ));
    assert!(!join_hit(
        prev,
        v,
        next,
        8.0,
        3.0,
        2.0,
        vexel_scene::Linejoin::Bevel,
        4.0
    ));
    assert!(!join_hit(
        prev,
        v,
        next,
        8.0,
        3.0,
        2.0,
        vexel_scene::Linejoin::Miter,
        1.0
    ));
}
#[test]
fn original_source_over_rounding_predicate() {
    use crate::sample::over;
    let mut dst = [0, 0, 255, 255];
    over(&mut dst, [255, 0, 0, 128]);
    assert_eq!(dst, [128, 0, 127, 255]);
    let mut transparent = [45, 80, 190, 0];
    over(&mut transparent, [1, 2, 3, 0]);
    assert_eq!(transparent, [45, 80, 190, 0]);
}
#[test]
fn fill_sampling_lattice_and_size_arithmetic() {
    assert_eq!(FILL_SAMPLES_PER_AXIS, 4);
    assert_eq!(FILL_SAMPLE_COUNT, 16);
    assert_eq!(
        4usize
            .checked_mul(FILL_SAMPLE_COUNT)
            .and_then(|n| n.checked_mul(4)),
        Some(256)
    );
    assert_eq!(usize::MAX.checked_mul(FILL_SAMPLE_COUNT), None);
}
