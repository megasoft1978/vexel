use super::*;
use crate::animation::frame;
use std::sync::atomic::AtomicBool;

/// Renders `layers` (plus a background) with the app's own SVG renderer.
fn render(layers: &[Layer], w: usize, h: usize, bg: &str) -> Vec<u8> {
    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\"><rect width=\"{w}\" height=\"{h}\" fill=\"{bg}\"/>"
    );
    for l in layers {
        svg.push_str(&format!("<path d=\"{}\" fill=\"{}\"/>", l.d, l.hex()));
    }
    svg.push_str("</svg>");
    let scene = frame::decode(svg.as_bytes()).expect("traced svg decodes");
    scene
        .render_sized(0.0, (w as u32, h as u32), &AtomicBool::new(false))
        .expect("render")
        .rgba()
        .to_vec()
}

/// Mean absolute RGB difference per channel (0..255).
fn mean_err(a: &[u8], b: &[u8]) -> f64 {
    let mut s = 0u64;
    let mut n = 0u64;
    for (p, q) in a.chunks_exact(4).zip(b.chunks_exact(4)) {
        for c in 0..3 {
            s += u64::from(p[c].abs_diff(q[c]));
            n += 1;
        }
    }
    s as f64 / n as f64
}

fn canvas(w: usize, h: usize, bg: [u8; 3]) -> Vec<u8> {
    (0..w * h)
        .flat_map(|_| [bg[0], bg[1], bg[2], 255])
        .collect()
}

/// Paints `f(x, y) -> coverage 0..1` of `col` over the canvas (anti-aliased by
/// 4x4 supersampling).
fn paint(px: &mut [u8], w: usize, col: [u8; 3], f: impl Fn(f64, f64) -> bool) {
    for (i, p) in px.chunks_exact_mut(4).enumerate() {
        let (x, y) = ((i % w) as f64, (i / w) as f64);
        let mut hit = 0;
        for sy in 0..4 {
            for sx in 0..4 {
                if f(
                    x + (f64::from(sx) + 0.5) / 4.0,
                    y + (f64::from(sy) + 0.5) / 4.0,
                ) {
                    hit += 1;
                }
            }
        }
        for c in 0..3 {
            let mix = f64::from(p[c]) * f64::from(16 - hit) / 16.0
                + f64::from(col[c]) * f64::from(hit) / 16.0;
            p[c] = mix.round() as u8;
        }
    }
}

/// Error of the flat-quantised image itself: the floor any flat-colour trace
/// can reach, since anti-aliased edge pixels cannot be represented.
fn flat_err(px: &[u8], w: usize, h: usize, k: usize) -> f64 {
    let q = quantize(&image(px, w, h), k, 1).unwrap();
    let flat: Vec<u8> = q
        .labels
        .iter()
        .flat_map(|&l| {
            let c = q.palette[usize::from(l)];
            [c[0], c[1], c[2], 255]
        })
        .collect();
    mean_err(px, &flat)
}

fn image(px: &[u8], w: usize, h: usize) -> Rgba<'_> {
    Rgba::new(w, h, px).unwrap()
}

#[test]
fn circle_area_error_is_small() {
    let (w, h) = (120, 120);
    let mut px = canvas(w, h, [255, 255, 255]);
    paint(&mut px, w, [200, 30, 30], |x, y| {
        (x - 60.0).powi(2) + (y - 60.0).powi(2) <= 45.0 * 45.0
    });
    let layers = trace_image(&image(&px, w, h), 2, &TraceOptions::default(), None).unwrap();
    assert_eq!(layers.len(), 2);
    let out = render(&layers, w, h, "#ffffff");
    // Coverage of the red disc from the green channel (white 255 .. red 30).
    let coverage = |px: &[u8]| -> f64 {
        px.chunks_exact(4)
            .map(|p| (255.0 - f64::from(p[1])) / 225.0)
            .sum::<f64>()
    };
    let (traced, source) = (coverage(&out), coverage(&px));
    let truth = std::f64::consts::PI * 45.0 * 45.0;
    assert!((source - truth).abs() / truth < 0.01, "source {source}");
    assert!(
        (traced - truth).abs() / truth < 0.02,
        "traced {traced} truth {truth}"
    );
    let excess = mean_err(&px, &out) - flat_err(&px, w, h, 2);
    assert!(
        excess < 1.3,
        "curve fitting adds {excess} to the flat-colour floor"
    );
    // A circle fits in a handful of curves, not hundreds of segments.
    assert!(layers[1].d.len() < 400, "{}", layers[1].d.len());
}

#[test]
fn ring_keeps_its_hole() {
    let (w, h) = (100, 100);
    let mut px = canvas(w, h, [250, 250, 250]);
    paint(&mut px, w, [20, 60, 200], |x, y| {
        let r2 = (x - 50.0).powi(2) + (y - 50.0).powi(2);
        (18.0 * 18.0..=40.0 * 40.0).contains(&r2)
    });
    let layers = trace_image(&image(&px, w, h), 2, &TraceOptions::default(), None).unwrap();
    let top = layers.last().unwrap();
    assert_eq!(top.contours, 2, "outer ring and hole: {}", top.d);
    let out = render(&layers, w, h, "#fafafa");
    let centre = &out[(50 * w + 50) * 4..(50 * w + 50) * 4 + 3];
    assert!(centre[0] > 230, "hole must show the background: {centre:?}");
    let excess = mean_err(&px, &out) - flat_err(&px, w, h, 2);
    assert!(
        excess < 1.5,
        "curve fitting adds {excess} to the flat-colour floor"
    );
}

#[test]
fn two_colours_and_exact_rectangles() {
    let (w, h) = (80, 60);
    let mut px = canvas(w, h, [10, 10, 10]);
    paint(&mut px, w, [240, 200, 0], |x, y| {
        x >= 10.0 && x < 40.0 && y >= 10.0 && y < 50.0
    });
    paint(&mut px, w, [0, 200, 120], |x, y| {
        x >= 45.0 && x < 70.0 && y >= 5.0 && y < 30.0
    });
    let layers = trace_image(&image(&px, w, h), 3, &TraceOptions::default(), None).unwrap();
    assert_eq!(layers.len(), 3);
    let out = render(&layers, w, h, "#000000");
    assert!(
        mean_err(&px, &out) < 1.0,
        "mean err {}",
        mean_err(&px, &out)
    );
    // Straight edges stay straight: no curve commands for rectangles.
    assert!(layers.iter().all(|l| !l.d.contains('C')), "{layers:?}");
}

#[test]
fn antialiased_edges_do_not_add_speckle() {
    let (w, h) = (96, 96);
    let mut px = canvas(w, h, [255, 255, 255]);
    paint(&mut px, w, [0, 0, 0], |x, y| {
        (x - 48.0).powi(2) / 900.0 + (y - 48.0).powi(2) / 400.0 <= 1.0
    });
    let layers = trace_image(&image(&px, w, h), 4, &TraceOptions::default(), None).unwrap();
    let contours: usize = layers.iter().map(|l| l.contours).sum();
    assert!(contours <= 8, "{contours} contours");
    let out = render(&layers, w, h, "#ffffff");
    assert!(
        mean_err(&px, &out) < 3.0,
        "mean err {}",
        mean_err(&px, &out)
    );
}

#[test]
fn deterministic_for_a_seed() {
    let (w, h) = (64, 64);
    let mut px = canvas(w, h, [30, 30, 90]);
    paint(&mut px, w, [250, 120, 20], |x, y| {
        (x - 30.0).powi(2) + (y - 34.0).powi(2) < 400.0
    });
    paint(&mut px, w, [20, 200, 90], |x, y| x > 40.0 && y < 20.0);
    let o = TraceOptions::default();
    let a = trace_image(&image(&px, w, h), 5, &o, None).unwrap();
    let b = trace_image(&image(&px, w, h), 5, &o, None).unwrap();
    assert_eq!(a, b);
    let q1 = quantize(&image(&px, w, h), 5, 7).unwrap();
    let q2 = quantize(&image(&px, w, h), 5, 7).unwrap();
    assert_eq!((q1.palette, q1.labels), (q2.palette, q2.labels));
}

#[test]
fn limits_are_enforced_before_allocation() {
    assert_eq!(Rgba::new(0, 4, &[]).unwrap_err(), TraceError::BadImage);
    assert_eq!(
        Rgba::new(5000, 5000, &[]).unwrap_err(),
        TraceError::BadImage
    );
    assert_eq!(
        Rgba::new(usize::MAX, 2, &[]).unwrap_err(),
        TraceError::BadImage
    );
    let px = canvas(4, 4, [0, 0, 0]);
    let img = image(&px, 4, 4);
    assert_eq!(quantize(&img, 0, 1).unwrap_err(), TraceError::BadColours);
    assert_eq!(quantize(&img, 65, 1).unwrap_err(), TraceError::BadColours);
    assert_eq!(
        trace_image(&img, 2, &TraceOptions::default(), Some([2, 2, 9, 9])).unwrap_err(),
        TraceError::BadRegion
    );
    // Checkerboard: far more contours than allowed with min_area 1.
    let mut mask = vec![false; 600 * 600];
    for (i, m) in mask.iter_mut().enumerate() {
        *m = ((i % 600) + (i / 600)) % 2 == 0;
    }
    let o = TraceOptions {
        min_area: 1,
        ..TraceOptions::default()
    };
    assert_eq!(
        trace(&mask, 600, 600, &o).unwrap_err(),
        TraceError::TooManyContours
    );
}

#[test]
fn region_offsets_the_output() {
    let (w, h) = (60, 60);
    let mut px = canvas(w, h, [255, 255, 255]);
    paint(&mut px, w, [0, 0, 0], |x, y| {
        x >= 30.0 && x < 50.0 && y >= 30.0 && y < 50.0
    });
    let layers = trace_image(
        &image(&px, w, h),
        2,
        &TraceOptions::default(),
        Some([20, 20, 60, 60]),
    )
    .unwrap();
    assert_eq!(layers.last().unwrap().d, "M30 30L50 30L50 50L30 50Z");
}
