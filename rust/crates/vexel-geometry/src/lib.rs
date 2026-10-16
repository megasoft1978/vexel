#![forbid(unsafe_code)]
//! Double precision SVG geometry with the validation limits of the C reference.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Edge {
    pub a: Point,
    pub b: Point,
}

/// SVG affine matrix `[a c e; b d f; 0 0 1]`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Affine {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
    pub e: f64,
    pub f: f64,
}

/// Returns whether both coordinates are finite.
pub fn finite_point(point: Point) -> bool {
    point.x.is_finite() && point.y.is_finite()
}

/// Returns the identity transform.
pub const fn affine_identity() -> Affine {
    Affine { a: 1.0, b: 0.0, c: 0.0, d: 1.0, e: 0.0, f: 0.0 }
}

/// Applies the reference coefficient and translation limits.
pub fn affine_valid(matrix: Affine) -> bool {
    matrix.a.is_finite() && matrix.b.is_finite() && matrix.c.is_finite()
        && matrix.d.is_finite() && matrix.e.is_finite() && matrix.f.is_finite()
        && matrix.a.abs() <= 1.0e12 && matrix.b.abs() <= 1.0e12
        && matrix.c.abs() <= 1.0e12 && matrix.d.abs() <= 1.0e12
        && matrix.e.abs() <= 1.0e6 && matrix.f.abs() <= 1.0e6
}

/// Composes transforms so points pass through `right` first, then `left`.
pub fn affine_compose(left: Affine, right: Affine) -> Option<Affine> {
    if !affine_valid(left) || !affine_valid(right) {
        return None;
    }
    let result = Affine {
        a: left.a * right.a + left.c * right.b,
        b: left.b * right.a + left.d * right.b,
        c: left.a * right.c + left.c * right.d,
        d: left.b * right.c + left.d * right.d,
        e: left.a * right.e + left.c * right.f + left.e,
        f: left.b * right.e + left.d * right.f + left.f,
    };
    affine_valid(result).then_some(result)
}

/// Maps a finite point, rejecting invalid transforms, overflow, and excessive output.
pub fn affine_map_point(matrix: Affine, point: Point) -> Option<Point> {
    if !affine_valid(matrix) || !finite_point(point) {
        return None;
    }
    let mapped = Point {
        x: matrix.a * point.x + matrix.c * point.y + matrix.e,
        y: matrix.b * point.x + matrix.d * point.y + matrix.f,
    };
    (finite_point(mapped) && mapped.x.abs() <= 1.0e6 && mapped.y.abs() <= 1.0e6)
        .then_some(mapped)
}

/// Maps all four rectangle corners and returns `[min_x, min_y, width, height]`.
pub fn affine_map_bounds(
    matrix: Affine,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Option<[f64; 4]> {
    if !x.is_finite() || !y.is_finite() || !width.is_finite() || !height.is_finite()
        || width < 0.0 || height < 0.0
    {
        return None;
    }
    let corners = [
        Point { x, y },
        Point { x: x + width, y },
        Point { x: x + width, y: y + height },
        Point { x, y: y + height },
    ];
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for corner in corners {
        let point = affine_map_point(matrix, corner)?;
        min_x = min_x.min(point.x);
        min_y = min_y.min(point.y);
        max_x = max_x.max(point.x);
        max_y = max_y.max(point.y);
    }
    Some([min_x, min_y, max_x - min_x, max_y - min_y])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(left: f64, right: f64, tolerance: f64) {
        assert!((left - right).abs() <= tolerance, "{left} != {right}");
    }

    #[test]
    fn original_composition_point_and_bounds_assertions() {
        let translate = Affine { a: 1.0, b: 0.0, c: 0.0, d: 1.0, e: 10.0, f: 20.0 };
        let scale = Affine { a: 2.0, b: 0.0, c: 0.0, d: 3.0, e: 0.0, f: 0.0 };
        let composed = affine_compose(translate, scale).unwrap();
        let mapped = affine_map_point(composed, Point { x: 1.0, y: 2.0 }).unwrap();
        close(mapped.x, 12.0, 1e-12);
        close(mapped.y, 26.0, 1e-12);
        let bounds = affine_map_bounds(composed, 0.0, 0.0, 2.0, 1.0).unwrap();
        for (actual, expected) in bounds.into_iter().zip([10.0, 20.0, 4.0, 3.0]) {
            close(actual, expected, 1e-12);
        }
        let singular = Affine { a: 0.0, b: 0.0, c: 0.0, d: 0.0, e: 1.0, f: 1.0 };
        assert_eq!(affine_map_point(singular, Point { x: 3.0, y: 4.0 }), Some(Point { x: 1.0, y: 1.0 }));
        let turn = Affine { a: 0.0, b: 1.0, c: -1.0, d: 0.0, e: 0.0, f: 0.0 };
        let rotated = affine_map_bounds(turn, 0.0, 0.0, 2.0, 1.0).unwrap();
        for (actual, expected) in rotated.into_iter().zip([-1.0, 0.0, 1.0, 2.0]) {
            close(actual, expected, 1e-12);
        }
    }

    #[test]
    fn original_iterative_right_first_composition_assertion() {
        for index in 1..=64 {
            let k = f64::from(index);
            let left = Affine { a: 1.0 + k / 100.0, b: 0.25, c: -0.125, d: 0.75 + k / 200.0, e: k, f: -k / 2.0 };
            let right = Affine { a: 0.5 + k / 300.0, b: -0.2, c: 0.4, d: 1.0 + k / 400.0, e: -k / 3.0, f: k / 4.0 };
            let point = Point { x: k / 5.0, y: -k / 7.0 };
            let inner = affine_map_point(right, point).unwrap();
            let expected = Point {
                x: left.a * inner.x + left.c * inner.y + left.e,
                y: left.b * inner.x + left.d * inner.y + left.f,
            };
            let actual = affine_map_point(affine_compose(left, right).unwrap(), point).unwrap();
            close(actual.x, expected.x, 1e-10);
            close(actual.y, expected.y, 1e-10);
        }
    }

    #[test]
    fn finite_identity_and_affine_limits() {
        let edge = Edge { a: Point { x: 0.0, y: 1.0 }, b: Point { x: 2.0, y: 3.0 } };
        assert_eq!(edge.a, Point { x: 0.0, y: 1.0 });
        assert_eq!(edge.b, Point { x: 2.0, y: 3.0 });
        assert!(finite_point(Point { x: 0.0, y: -1.0 }));
        assert!(!finite_point(Point { x: f64::NAN, y: 0.0 }));
        assert!(!finite_point(Point { x: f64::INFINITY, y: 0.0 }));
        assert_eq!(affine_identity(), Affine { a: 1.0, b: 0.0, c: 0.0, d: 1.0, e: 0.0, f: 0.0 });
        assert!(affine_valid(Affine { e: 1e6, f: -1e6, ..affine_identity() }));
        assert!(!affine_valid(Affine { a: 1e12_f64.next_up(), ..affine_identity() }));
        assert!(!affine_valid(Affine { e: f64::INFINITY, ..affine_identity() }));
        assert!(affine_compose(Affine { a: f64::NAN, ..affine_identity() }, affine_identity()).is_none());
        assert!(affine_compose(Affine { a: 1e12, ..affine_identity() }, Affine { a: 2.0, ..affine_identity() }).is_none());
    }

    #[test]
    fn point_mapping_rejects_nonfinite_and_overflow() {
        assert!(affine_map_point(affine_identity(), Point { x: f64::INFINITY, y: 0.0 }).is_none());
        assert!(affine_map_point(affine_identity(), Point { x: f64::MAX, y: 0.0 }).is_none());
        assert!(affine_map_point(Affine { a: 1e12, ..affine_identity() }, Point { x: 1e6, y: 0.0 }).is_none());
        assert!(affine_map_point(Affine { a: f64::NAN, ..affine_identity() }, Point { x: 0.0, y: 0.0 }).is_none());
    }

    #[test]
    fn bounds_validate_dimensions_corners_and_intermediate_overflow() {
        assert_eq!(affine_map_bounds(affine_identity(), 2.0, 3.0, 0.0, 0.0), Some([2.0, 3.0, 0.0, 0.0]));
        assert_eq!(affine_map_bounds(affine_identity(), 2.0, 3.0, 4.0, 5.0), Some([2.0, 3.0, 4.0, 5.0]));
        assert!(affine_map_bounds(affine_identity(), 0.0, 0.0, -1.0, 1.0).is_none());
        assert!(affine_map_bounds(affine_identity(), 0.0, 0.0, 1.0, -1.0).is_none());
        assert!(affine_map_bounds(affine_identity(), f64::NAN, 0.0, 1.0, 1.0).is_none());
        assert!(affine_map_bounds(affine_identity(), 0.0, 0.0, f64::INFINITY, 1.0).is_none());
        assert!(affine_map_bounds(affine_identity(), f64::MAX, 0.0, f64::MAX, 1.0).is_none());
        assert!(affine_map_bounds(Affine { a: f64::INFINITY, ..affine_identity() }, 0.0, 0.0, 1.0, 1.0).is_none());
    }
}
