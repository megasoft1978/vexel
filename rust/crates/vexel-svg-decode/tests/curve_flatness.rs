//! Circle and ellipse flattening bound: the emitted polygon stays within 0.1
//! device px of the analytic curve at any scale (the implementation targets
//! 0.08 px), keeps the 12-step floor and the 4096 edge cap.
mod support;
use support::*;
use vexel_scene::{ErrorKind, LimitKind};

const CENTER: f64 = 100.0;

/// Distance from `(px, py)` to the dense reference polyline of the ellipse.
fn distance_to_ellipse(dense: &[(f64, f64)], px: f64, py: f64) -> f64 {
    dense
        .windows(2)
        .map(|w| {
            let (dx, dy) = (w[1].0 - w[0].0, w[1].1 - w[0].1);
            let len2 = dx * dx + dy * dy;
            let t = (((px - w[0].0) * dx + (py - w[0].1) * dy) / len2).clamp(0.0, 1.0);
            (px - (w[0].0 + t * dx)).hypot(py - (w[0].1 + t * dy))
        })
        .fold(f64::INFINITY, f64::min)
}

/// Decodes one shape and returns (edge count, max radial error in device px).
fn flatten_error(element: &str, rx: f64, ry: f64, scale: f64) -> (usize, f64) {
    let source = format!(
        r#"<svg width="8" height="8"><{element} transform="scale({scale})"/></svg>"#,
        element = element
    );
    let graph = decode(&source).unwrap();
    let edges = graph.root().edges;
    let dense: Vec<(f64, f64)> = (0..=20_000)
        .map(|i| {
            let a = std::f64::consts::TAU * f64::from(i) / 20_000.0;
            (
                (CENTER + rx * a.cos()) * scale,
                (CENTER + ry * a.sin()) * scale,
            )
        })
        .collect();
    let mut worst = 0.0f64;
    for e in edges {
        for k in 0..=8 {
            let t = f64::from(k) / 8.0;
            let (x, y) = (e.a.x + t * (e.b.x - e.a.x), e.a.y + t * (e.b.y - e.a.y));
            worst = worst.max(distance_to_ellipse(&dense, x, y));
        }
    }
    (edges.len(), worst)
}

#[test]
fn circle_radial_error_is_within_a_tenth_of_a_device_pixel() {
    for scale in [1.0, 2.0] {
        for r in [1.0, 5.0, 40.0, 300.0] {
            let element = format!(r#"circle cx="{CENTER}" cy="{CENTER}" r="{r}""#);
            let (count, error) = flatten_error(&element, r, r, scale);
            assert!(count >= 12, "r={r} scale={scale}: {count} edges");
            assert!(error <= 0.1, "r={r} scale={scale}: error {error}");
        }
    }
}

#[test]
fn ellipse_radial_error_is_within_a_tenth_of_a_device_pixel() {
    for scale in [1.0, 2.0] {
        for ry in [1.0, 5.0, 40.0, 100.0] {
            let rx = 3.0 * ry;
            let element = format!(r#"ellipse cx="{CENTER}" cy="{CENTER}" rx="{rx}" ry="{ry}""#);
            let (count, error) = flatten_error(&element, rx, ry, scale);
            assert!(count >= 12, "rx={rx} scale={scale}: {count} edges");
            assert!(error <= 0.1, "rx={rx} ry={ry} scale={scale}: error {error}");
        }
    }
}

#[test]
fn radius_forty_at_one_x_has_at_least_48_vertices() {
    let element = format!(r#"circle cx="{CENTER}" cy="{CENTER}" r="40""#);
    let (count, _) = flatten_error(&element, 40.0, 40.0, 1.0);
    assert!(count >= 48, "{count} edges");
}

#[test]
fn tiny_circle_keeps_the_twelve_step_floor() {
    let element = format!(r#"circle cx="{CENTER}" cy="{CENTER}" r="0.5""#);
    let (count, _) = flatten_error(&element, 0.5, 0.5, 1.0);
    assert_eq!(count, 12);
}

#[test]
fn over_cap_radius_still_returns_the_edge_limit_error() {
    let source = r#"<svg width="8" height="8"><circle cx="1" cy="1" r="1000000"/></svg>"#;
    let error = decode(source).unwrap_err();
    assert_eq!(error.kind, ErrorKind::Limit(LimitKind::Edges));
}
