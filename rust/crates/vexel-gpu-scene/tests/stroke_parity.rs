//! Stroke rendering of the reference rasterizer against the real `vexel-render`
//! CPU output, on seeded stroke scenes (widths 0.5-40, all joins and caps,
//! dashes, transforms, fill under stroke). Bounds are the ADR-0035 tolerances
//! from `policy`; they are never loosened here. `-- --nocapture` prints metrics.
#![allow(dead_code)]

use vexel_gpu_scene::policy::{
    PARITY_MEAN_ABS_ERROR_MAX, PARITY_OVER_2_MAX_FRACTION, PARITY_OVER_16_MAX_FRACTION,
};
use vexel_gpu_scene::{encode, reference};

include!("common/strokes.rs");

fn compare(a: &[u8], b: &[u8]) -> (f64, f64, f64, u8) {
    assert_eq!(a.len(), b.len());
    let pixels = a.len() / 4;
    let (mut over_2, mut over_16, mut sum, mut max_diff) = (0usize, 0usize, 0u64, 0u8);
    for (pa, pb) in a.chunks_exact(4).zip(b.chunks_exact(4)) {
        let worst = (0..4).map(|c| pa[c].abs_diff(pb[c])).max().unwrap();
        over_2 += usize::from(worst > 2);
        over_16 += usize::from(worst > 16);
        max_diff = max_diff.max(worst);
        sum += (0..4)
            .map(|c| u64::from(pa[c].abs_diff(pb[c])))
            .sum::<u64>();
    }
    (
        over_2 as f64 / pixels as f64,
        over_16 as f64 / pixels as f64,
        sum as f64 / (pixels * 4) as f64 / 255.0,
        max_diff,
    )
}

fn check(name: &str, seed: u64, count: usize, size: u32, min_w: f64, max_w: f64) {
    let side = f64::from(size);
    let cases = stroke_cases(seed, count, side, side, min_w, max_w);
    let (scene, cpu) = stroke_scene(&cases, size, size);
    assert!(vexel_gpu_scene::classify(&scene).is_supported(), "{name}");
    let encoded = encode(&scene).unwrap();
    let gpu = reference::render_straight(&encoded);
    let (o2, o16, mae, max) = compare(&gpu, &cpu);
    println!(
        "{name}: {size}px shapes={} prims={} >2/255={:.5}% >16/255={:.5}% mae={:.6} max={max}",
        cases.len(),
        encoded.prims.len(),
        o2 * 100.0,
        o16 * 100.0,
        mae
    );
    assert!(o2 <= PARITY_OVER_2_MAX_FRACTION, "{name} over 2: {o2}");
    assert!(o16 <= PARITY_OVER_16_MAX_FRACTION, "{name} over 16: {o16}");
    assert!(mae <= PARITY_MEAN_ABS_ERROR_MAX, "{name} mae: {mae}");
}

#[test]
fn thin_strokes_match_cpu() {
    check("thin", 0x51, 60, 480, 0.5, 3.0);
}

#[test]
fn medium_strokes_match_cpu() {
    check("medium", 0x52, 50, 480, 3.0, 14.0);
}

#[test]
fn wide_strokes_match_cpu() {
    check("wide", 0x53, 40, 480, 14.0, 40.0);
}

#[test]
fn mixed_strokes_match_cpu_large_canvas() {
    check("mixed-900", 0x54, 120, 900, 0.5, 40.0);
}

#[test]
fn many_seeds_match_cpu() {
    for seed in 0..6u64 {
        check(&format!("seed{seed}"), 0x1000 + seed, 40, 480, 0.5, 40.0);
    }
}

/// Zoomed view: the stroke space is the device space scaled by 6.4 (as under
/// a 100-unit view box on a 640 px canvas), dashes in local units.
#[test]
fn zoomed_dashed_strokes_match_cpu() {
    let size = 640u32;
    let mut cases = stroke_cases(0x77, 60, f64::from(size), f64::from(size), 0.4, 5.0);
    for c in &mut cases {
        c.transform = Affine {
            a: 6.4,
            b: 0.0,
            c: 0.0,
            d: 6.4,
            e: 0.0,
            f: 0.0,
        };
        c.dashes = vec![3.0, 2.0];
    }
    let (scene, cpu) = stroke_scene(&cases, size, size);
    let encoded = encode(&scene).unwrap();
    let gpu = reference::render_straight(&encoded);
    let (o2, o16, mae, max) = compare(&gpu, &cpu);
    println!("zoomed: >2={o2:.6} >16={o16:.6} mae={mae:.7} max={max}");
    assert!(o2 <= PARITY_OVER_2_MAX_FRACTION && o16 <= PARITY_OVER_16_MAX_FRACTION);
}

/// Shapes the app pipeline once rendered differently from the CPU (bisected):
/// dashed closed triangles with square caps and wide strokes under a
/// letterboxed view (user -> device: x * 6.4 + 130, y * 6.4).
fn app_triangle(points: [(f64, f64); 3], width: f64, join: Linejoin) -> StrokeCase {
    let dev: Vec<(f64, f64)> = points
        .iter()
        .map(|p| (130.0 + p.0 * 6.4, p.1 * 6.4))
        .collect();
    StrokeCase {
        subpaths: vec![(dev, true)],
        style: StrokeStyle {
            rgba: [34, 170, 102, 255],
            width,
            miter_limit: 4.0,
            dash_offset: 0.0,
            cap: Linecap::Square,
            join,
            crisp_edges: false,
        },
        transform: Affine {
            a: 6.4,
            b: 0.0,
            c: 0.0,
            d: 6.4,
            e: 130.0,
            f: 0.0,
        },
        dashes: vec![3.0, 2.0],
        fill: None,
        evenodd: false,
    }
}

#[test]
fn app_triangle_square_caps_dashes() {
    for (points, width, join) in [
        (
            [(46.1, 54.6), (70.4, 69.3), (53.4, 62.4)],
            2.76,
            Linejoin::Bevel,
        ),
        (
            [(74.5, 16.4), (83.9, 24.7), (85.2, 31.8)],
            5.33,
            Linejoin::Round,
        ),
        (
            [(22.9, 51.5), (49.1, 56.4), (33.9, 79.8)],
            5.15,
            Linejoin::Bevel,
        ),
    ] {
        let (scene, cpu) = stroke_scene(&[app_triangle(points, width, join)], 900, 640);
        if let Some(GpuNode::Shape(sh)) = scene.nodes.first() {
            let st = sh.stroke.as_ref().unwrap();
            println!(
                "HARNESS segs={} first={:?} last={:?} xf={:?} r={}",
                st.segments.len(),
                st.segments.first(),
                st.segments.last(),
                st.transform,
                st.radius
            );
        }
        let gpu = reference::render_straight(&encode(&scene).unwrap());
        let (o2, o16, mae, max) = compare(&gpu, &cpu);
        println!("triangle w={width}: >2={o2:.6} >16={o16:.6} mae={mae:.7} max={max}");
        assert!(o16 <= PARITY_OVER_16_MAX_FRACTION);
    }
}

/// The same triangles in USER space under a letterboxed 100-unit view box
/// (exactly the app's geometry): the CPU must agree with the reference.
#[test]
fn letterboxed_user_space_strokes_match_cpu() {
    for (points, width, join) in [
        (
            [(46.1, 54.6), (70.4, 69.3), (53.4, 62.4)],
            2.76,
            Linejoin::Bevel,
        ),
        (
            [(74.5, 16.4), (83.9, 24.7), (85.2, 31.8)],
            5.33,
            Linejoin::Round,
        ),
    ] {
        let mut case = app_triangle(points, width, join);
        case.subpaths = vec![(points.to_vec(), true)];
        case.transform = identity_affine();
        let (scene, cpu) = stroke_scene_viewed(&[case], 900, 640, (100.0, 100.0));
        let gpu = reference::render_straight(&encode(&scene).unwrap());
        let (o2, o16, mae, max) = compare(&gpu, &cpu);
        println!("letterboxed w={width}: >2={o2:.6} >16={o16:.6} mae={mae:.7} max={max}");
        assert!(o16 <= PARITY_OVER_16_MAX_FRACTION);
    }
}
