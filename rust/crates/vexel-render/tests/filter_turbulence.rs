//! `feTurbulence` through the filter chain, the grain recipe, the extra
//! `feColorMatrix` types and a release micro-benchmark.
//! Bench: `cargo test -p vexel-render --release --test filter_turbulence -- --ignored --nocapture`
mod common;

use common::{NEVER, unit_view};
use std::time::Instant;
use vexel_render::filter::apply_filter_chain;
use vexel_scene::{Filter, FilterInput, FilterKind, FilterPrimitive, TableRange};

fn prim(kind: FilterKind) -> FilterPrimitive {
    FilterPrimitive {
        kind,
        input: FilterInput::Previous,
        input2: FilterInput::Graphic,
        merge_inputs: [FilterInput::Graphic; 8],
        merge_count: 0,
        mode: 0,
        rgba: [0, 0, 0, 255],
        values: [0.0; 20],
        source_offset: 0,
    }
}

fn turbulence(base: f64, octaves: f64, seed: f64, mode: u8) -> FilterPrimitive {
    let mut p = prim(FilterKind::Turbulence);
    p.values[0] = base;
    p.values[1] = base;
    p.values[2] = octaves;
    p.values[3] = seed;
    p.mode = mode;
    p
}

fn matrix(mode: u8, values: &[f64]) -> FilterPrimitive {
    let mut p = prim(FilterKind::ColorMatrix);
    p.mode = mode;
    p.values[..values.len()].copy_from_slice(values);
    p
}

fn whole(count: usize, linear_rgb: bool) -> Filter {
    Filter {
        primitives: TableRange { first: 0, count },
        linear_rgb,
        filter_units_object_bounding_box: false,
        primitive_units_object_bounding_box: false,
        custom_region: true,
        region: [0.0, 0.0, 1.0e4, 1.0e4],
    }
}

fn run(size: u32, chain: &[FilterPrimitive], linear_rgb: bool, fill: [u8; 4]) -> Vec<u8> {
    let mut data: Vec<u8> = fill.iter().copied().cycle().take((size * size * 4) as usize).collect();
    apply_filter_chain(&mut data, &unit_view(size), &whole(chain.len(), linear_rgb), chain, 64 << 20, NEVER)
        .unwrap();
    data
}

fn fnv(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, b| (h ^ u64::from(*b)).wrapping_mul(0x0000_0100_0000_01b3))
}

#[test]
fn noise_is_pinned_by_hash_and_seed_changes_it() {
    let a = run(32, &[turbulence(0.1, 2.0, 3.0, 0)], false, [255; 4]);
    let b = run(32, &[turbulence(0.1, 2.0, 3.0, 0)], false, [255; 4]);
    let c = run(32, &[turbulence(0.1, 2.0, 4.0, 0)], false, [255; 4]);
    assert_eq!(a, b);
    assert_ne!(a, c);
    assert_eq!(fnv(&a), PINNED_FRACTAL_HASH, "regenerate only for an intentional algorithm change");
    let t = run(32, &[turbulence(0.1, 2.0, 3.0, 1)], false, [255; 4]);
    assert_eq!(fnv(&t), PINNED_TURBULENCE_HASH);
}

const PINNED_FRACTAL_HASH: u64 = 10_635_293_578_497_428_112;
const PINNED_TURBULENCE_HASH: u64 = 13_902_043_368_308_938_837;

#[test]
fn grain_recipe_keeps_content_and_adds_alpha_noise() {
    // noise -> luminanceToAlpha -> composite "in" the source: grain only on content.
    let mut over = prim(FilterKind::Composite);
    over.mode = 1; // in
    over.input = FilterInput::Previous;
    over.input2 = FilterInput::Graphic;
    let chain = [turbulence(0.9, 2.0, 5.0, 0), matrix(3, &[]), over];
    let out = run(64, &chain, true, [200, 200, 200, 255]);
    let alphas: std::collections::HashSet<u8> = out.chunks_exact(4).map(|p| p[3]).collect();
    assert!(alphas.len() > 20, "grain alpha varies ({})", alphas.len());
    assert!(out.chunks_exact(4).all(|p| p[..3] == [0, 0, 0] || p[3] == 0 || p[0] == p[1]));
}

#[test]
fn matrix_types_saturate_hue_rotate_and_luminance_to_alpha() {
    let one = |p: FilterPrimitive, px: [u8; 4]| -> [u8; 4] {
        let out = run(2, &[p], false, px);
        out[..4].try_into().unwrap()
    };
    assert_eq!(one(matrix(1, &[0.0]), [255, 0, 0, 255]), [54, 54, 54, 255]);
    assert_eq!(one(matrix(1, &[1.0]), [200, 100, 50, 255]), [200, 100, 50, 255]);
    assert_eq!(one(matrix(2, &[0.0]), [200, 100, 50, 255]), [200, 100, 50, 255]);
    assert_eq!(one(matrix(2, &[180.0]), [255, 0, 0, 255]), [0, 109, 109, 255]);
    assert_eq!(one(matrix(3, &[]), [255, 255, 255, 255]), [0, 0, 0, 255]);
    assert_eq!(one(matrix(3, &[]), [255, 0, 0, 255]), [0, 0, 0, 54]);
}

#[test]
fn turbulence_scales_with_the_view() {
    // Same user-space noise at 2x resolution: the 2x layer's even pixels see the
    // 1x layer's pixel positions, so the coarse structure agrees.
    let one = run(16, &[turbulence(0.05, 1.0, 9.0, 0)], false, [255; 4]);
    let big = vexel_render::ViewMap::from_viewbox(
        vexel_render::ViewBox { x: 0.0, y: 0.0, width: 16.0, height: 16.0 },
        32,
        32,
        None,
    )
    .unwrap();
    let mut data = vec![255u8; 32 * 32 * 4];
    let chain = [turbulence(0.05, 1.0, 9.0, 0)];
    apply_filter_chain(&mut data, &big, &whole(1, false), &chain, 64 << 20, NEVER).unwrap();
    for (x, y) in [(0usize, 0usize), (4, 4), (8, 12)] {
        assert_eq!(data[((y * 2) * 32 + x * 2) * 4..][..4], one[(y * 16 + x) * 4..][..4]);
    }
}

fn grain_ms(runs: usize) -> f64 {
    let mut over = prim(FilterKind::Composite);
    over.mode = 1;
    let chain = [turbulence(0.8, 3.0, 5.0, 0), matrix(3, &[]), over];
    let mut times = Vec::new();
    for _ in 0..runs {
        let mut data = vec![200u8; 1024 * 1024 * 4];
        let start = Instant::now();
        apply_filter_chain(&mut data, &unit_view(1024), &whole(3, true), &chain, 64 << 20, NEVER).unwrap();
        times.push(start.elapsed().as_secs_f64() * 1000.0);
    }
    times.sort_by(f64::total_cmp);
    times[runs / 2]
}

#[test]
#[ignore]
fn grain_1024_p50_ms() {
    let ms = grain_ms(7);
    println!("BENCH grain 1024x1024 (3 octaves + matrix + composite) p50_ms {ms:.1}");
    assert!(ms <= 60.0, "grain budget 60 ms, got {ms:.1}");
}
