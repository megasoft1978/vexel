use super::super::{DiffCache, compose_frame_cached, compose_svg};
use std::sync::Arc;

fn svg(x: u32, opacity: &str, extra: &str) -> Vec<u8> {
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="96" height="64" viewBox="0 0 96 64"><defs><linearGradient id="g"><stop offset="0" stop-color="#f00"/><stop offset="1" stop-color="#00f"/></linearGradient>{extra}</defs><rect width="96" height="64" fill="#fff"/><rect x="2" y="2" width="30" height="30" fill="#36c" fill-opacity="0.7" stroke="#000"/><rect x="40" y="30" width="30" height="30" fill="url(#g)"/><circle cx="{x}" cy="20" r="10" fill="#c33" opacity="{opacity}" stroke="#000" stroke-width="2"/><rect x="60" y="40" width="30" height="20" fill="#393" fill-opacity="0.5"/></svg>"##
    )
    .into_bytes()
}

fn run(frames: &[Vec<u8>]) -> DiffCache {
    let mut cache = DiffCache::default();
    for source in frames {
        let want = compose_svg(source, 96, 64).unwrap();
        let got = compose_frame_cached(Arc::new(source.clone()), 96, 64, &mut cache).unwrap();
        assert_eq!(got.rgba, want.rgba);
    }
    cache
}

#[test]
fn moved_shape_repaints_partially_with_identical_pixels() {
    let frames: Vec<_> = [50, 52, 54, 54, 80]
        .iter()
        .map(|x| svg(*x, "0.6", ""))
        .collect();
    let cache = run(&frames);
    assert!(!cache.disabled());
    assert!(cache.partial_frames() >= 3, "{}", cache.partial_frames());
}

#[test]
fn opacity_group_appearing_and_vanishing_is_identical() {
    let frames: Vec<_> = [
        ("0.5", 40),
        ("0.5", 44),
        ("1", 44),
        ("1", 48),
        ("0.25", 48),
        ("1", 20),
    ]
    .iter()
    .map(|(o, x)| svg(*x, o, ""))
    .collect();
    run(&frames);
}

#[test]
fn undiffable_scene_falls_back_to_full_renders() {
    let mask = r##"<mask id="m"><rect width="96" height="40" fill="#fff"/></mask>"##;
    let frames: Vec<_> = [50, 52, 54]
        .iter()
        .map(|x| {
            String::from_utf8(svg(*x, "0.6", mask))
                .unwrap()
                .replace("<circle", "<circle mask=\"url(#m)\"")
                .into_bytes()
        })
        .collect();
    let cache = run(&frames);
    assert!(cache.disabled());
    assert_eq!(cache.partial_frames(), 0);
}

#[test]
fn resized_output_restarts_from_a_full_render() {
    let mut cache = DiffCache::default();
    let source = svg(50, "0.6", "");
    for (w, h) in [(96, 64), (48, 32), (96, 64)] {
        let want = compose_svg(&source, w, h).unwrap();
        let got = compose_frame_cached(Arc::new(source.clone()), w, h, &mut cache).unwrap();
        assert_eq!(got.rgba, want.rgba);
    }
}
