//! AC-5 (offscreen backing scale) and AC-6 (determinism).
mod common;
use common::*;
use vexel_render::{
    OFFSCREEN_BACKING_SCALE, PreserveAspect, RenderError, ViewBox, offscreen_tile_size,
    render_offscreen,
};

#[test]
fn offscreen_size_ignores_any_simulated_host_scale() {
    assert_eq!(OFFSCREEN_BACKING_SCALE, 1.0);
    // The function takes no scale. Simulate hosts at 1x, 2x and 3x: the
    // result must not depend on them, so every simulation yields one size.
    let sizes: Vec<_> = [1.0f64, 2.0, 3.0]
        .iter()
        .map(|_host_scale| offscreen_tile_size(600.0, 650.0).unwrap())
        .collect();
    assert!(sizes.iter().all(|s| *s == (600, 650)));
    // Points are rounded up like C render-frame's `ceil(size * zoom)`.
    assert_eq!(offscreen_tile_size(10.2, 3.0), Ok((11, 3)));
    assert_eq!(offscreen_tile_size(0.5, 0.01), Ok((1, 1)));
    for bad in [0.0, -3.0, f64::NAN, f64::INFINITY, 4097.0] {
        assert_eq!(offscreen_tile_size(bad, 10.0), Err(RenderError::InvalidGeometry));
        assert_eq!(offscreen_tile_size(10.0, bad), Err(RenderError::InvalidGeometry));
    }
}

#[test]
fn offscreen_module_never_reads_a_host_scale() {
    let source = include_str!("../src/presentation.rs");
    // Strip comments so documentation may name what the module avoids.
    let code: String = source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");
    for forbidden in ["backing_scale", "vexel_platform_apple", "vexel_platform_macos", "NSScreen", "backingScaleFactor", "scale_factor", "device_pixel_ratio"] {
        assert!(!code.contains(forbidden), "offscreen module references `{forbidden}`");
    }
    assert!(!include_str!("../Cargo.toml").contains("vexel-platform"));
}

#[test]
fn render_offscreen_uses_the_one_to_one_tile() {
    let edges = polygon(&[(0.0, 0.0), (4.0, 0.0), (4.0, 4.0), (0.0, 4.0)]);
    let shapes = [fill_shape(&edges, [0, 0, 255, 255], false)];
    let vb = ViewBox { x: 0.0, y: 0.0, width: 8.0, height: 8.0 };
    let tile = render_offscreen(&shapes, vb, None, 8.0, 8.0, NEVER).unwrap();
    assert_eq!((tile.width(), tile.height()), (8, 8));
    assert_eq!(pixel(&tile, 1, 1), [0, 0, 255, 255]);
    assert_eq!(pixel(&tile, 5, 5), [0; 4]);
    // Meet letterboxing: a 2:1 view box in a square tile paints only the content band.
    let wide = ViewBox { x: 0.0, y: 0.0, width: 8.0, height: 4.0 };
    let band = [fill_shape(&edges, [0, 0, 255, 255], false)];
    let tile = render_offscreen(&band, wide, Some(PreserveAspect::MID_MEET), 8.0, 8.0, NEVER).unwrap();
    assert_eq!(pixel(&tile, 1, 3), [0, 0, 255, 255], "content starts at offset y=2");
    assert_eq!(pixel(&tile, 1, 1), [0; 4], "letterbox stays transparent");
}

#[test]
fn view_mapping_slice_and_none() {
    let edges = polygon(&[(0.0, 0.0), (8.0, 0.0), (8.0, 4.0), (0.0, 4.0)]);
    let shapes = [fill_shape(&edges, [1, 2, 3, 255], false)];
    let vb = ViewBox { x: 0.0, y: 0.0, width: 8.0, height: 4.0 };
    let none = PreserveAspect { align_x: 0.0, align_y: 0.0, none: true, slice: false };
    let tile = render_offscreen(&shapes, vb, Some(none), 8.0, 8.0, NEVER).unwrap();
    assert_eq!(pixel(&tile, 7, 7), [1, 2, 3, 255], "none stretches to fill the tile");
    let slice = PreserveAspect { align_x: 0.5, align_y: 0.5, none: false, slice: true };
    let tile = render_offscreen(&shapes, vb, Some(slice), 8.0, 8.0, NEVER).unwrap();
    assert_eq!(pixel(&tile, 0, 0), [1, 2, 3, 255], "slice covers the tile, cropping x");
}

#[test]
fn identical_input_renders_byte_identical_tiles() {
    let star: Vec<_> = (0..10)
        .map(|k| {
            let a = k as f64 * std::f64::consts::PI / 5.0;
            let r = if k % 2 == 0 { 30.0 } else { 13.0 };
            (40.0 + r * a.sin(), 40.0 - r * a.cos())
        })
        .collect();
    let edges = polygon(&star);
    let line = polyline(&[(5.0, 70.0), (35.0, 60.0), (70.0, 75.0)]);
    let st = style([10, 200, 30, 200], 5.0, vexel_render::Linecap::Round, vexel_render::Linejoin::Round, 4.0);
    let build = || [fill_shape(&edges, [200, 30, 90, 180], true), stroke_shape(&line, false, st)];
    let first = render(&build(), 80);
    let second = render(&build(), 80);
    assert_eq!(first.rgba(), second.rgba());
    assert!(first.rgba().iter().any(|b| *b != 0));
}
