//! Clip rendering of the reference rasterizer against the real `vexel-render`
//! CPU output on seeded clip scenes: rect and path clips, even-odd and union
//! children, nested chains, clipPath transforms, bounding-box units, clips on
//! groups (layered and not) and on strokes. Bounds are the ADR-0035
//! tolerances from `policy`, never loosened. `-- --nocapture` prints metrics.
#![allow(dead_code)]

use vexel_gpu_scene::policy::{
    PARITY_MEAN_ABS_ERROR_MAX, PARITY_OVER_2_MAX_FRACTION, PARITY_OVER_16_MAX_FRACTION,
};
use vexel_gpu_scene::{encode, reference};

include!("common/strokes.rs");
include!("common/clips.rs");

fn compare(a: &[u8], b: &[u8]) -> (f64, f64, f64, u8) {
    assert_eq!(a.len(), b.len());
    let pixels = a.len() / 4;
    let (mut over_2, mut over_16, mut sum, mut max_diff) = (0usize, 0usize, 0u64, 0u8);
    for (pa, pb) in a.chunks_exact(4).zip(b.chunks_exact(4)) {
        let worst = (0..4).map(|c| pa[c].abs_diff(pb[c])).max().unwrap();
        over_2 += usize::from(worst > 2);
        over_16 += usize::from(worst > 16);
        max_diff = max_diff.max(worst);
        sum += (0..4).map(|c| u64::from(pa[c].abs_diff(pb[c]))).sum::<u64>();
    }
    (
        over_2 as f64 / pixels as f64,
        over_16 as f64 / pixels as f64,
        sum as f64 / (pixels * 4) as f64 / 255.0,
        max_diff,
    )
}

#[test]
fn clip_scenes_match_cpu_within_adr() {
    for (name, seed, count, features) in CLIP_CONFIGS {
        for (k, (w, h)) in [(480u32, 360u32), (700, 500)].into_iter().enumerate() {
            let scene = clip_scene(seed + 16 * k as u64, count, f64::from(w), f64::from(h), features);
            let gpu_scene = clip_gpu_scene(&scene, w, h);
            assert!(vexel_gpu_scene::classify(&gpu_scene).is_supported(), "{name}");
            let encoded = encode(&gpu_scene).unwrap();
            let got = reference::render_straight(&encoded);
            let cpu = clip_cpu_render(&scene, w, h);
            let (o2, o16, mae, max) = compare(&got, &cpu);
            println!(
                "{name} {w}x{h}: clips={} records={} >2/255={:.5}% >16/255={:.5}% mae={mae:.6} max={max}",
                scene.specs.len(),
                encoded.clips.len(),
                o2 * 100.0,
                o16 * 100.0
            );
            assert!(o2 <= PARITY_OVER_2_MAX_FRACTION, "{name} over 2: {o2}");
            assert!(o16 <= PARITY_OVER_16_MAX_FRACTION, "{name} over 16: {o16}");
            assert!(mae <= PARITY_MEAN_ABS_ERROR_MAX, "{name} mae: {mae}");
        }
    }
}

/// The scenes really clip: without the clips the picture is far from the CPU.
#[test]
fn clips_change_the_picture() {
    fn strip(nodes: &mut [GpuNode]) {
        for n in nodes {
            match n {
                GpuNode::Shape(s) => s.clips.clear(),
                GpuNode::Group(g) => {
                    g.clips.clear();
                    strip(&mut g.children);
                }
            }
        }
    }
    let (w, h) = (480u32, 360u32);
    let (name, seed, count, features) = CLIP_CONFIGS[6];
    let scene = clip_scene(seed, count, f64::from(w), f64::from(h), features);
    let mut gpu_scene = clip_gpu_scene(&scene, w, h);
    strip(&mut gpu_scene.nodes);
    let got = reference::render_straight(&encode(&gpu_scene).unwrap());
    let (o2, ..) = compare(&got, &clip_cpu_render(&scene, w, h));
    println!("{name} unclipped differs on {:.2}% of pixels", o2 * 100.0);
    assert!(o2 > 0.02, "clips must matter in the generated scenes");
}
