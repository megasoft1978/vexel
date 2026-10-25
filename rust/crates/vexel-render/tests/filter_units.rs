//! Unit tests of the filter primitives (`vexel_render::filter`), one behaviour
//! each. Expected values follow the C `shape_filter.c` arithmetic; layers are
//! straight RGBA8 and, unless stated, `linear_rgb` is off so that 0 and 255
//! stay fixed points and the expectations are plain 8-bit arithmetic.
mod common;

use common::{NEVER, unit_view};
use std::cell::Cell;
use vexel_render::filter::{MAX_FILTER_PIXELS, apply_filter_chain};
use vexel_render::{RenderError, ViewBox, ViewMap};
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

fn blur(sigma_x: f64, sigma_y: f64) -> FilterPrimitive {
    let mut p = prim(FilterKind::GaussianBlur);
    p.values[0] = sigma_x;
    p.values[1] = sigma_y;
    p
}

fn offset(dx: f64, dy: f64) -> FilterPrimitive {
    let mut p = prim(FilterKind::Offset);
    p.values[2] = dx;
    p.values[3] = dy;
    p
}

fn flood(rgba: [u8; 4]) -> FilterPrimitive {
    let mut p = prim(FilterKind::Flood);
    p.rgba = rgba;
    p
}

fn two_input(kind: FilterKind, mode: u8, second: usize) -> FilterPrimitive {
    let mut p = prim(kind);
    p.input = FilterInput::Graphic;
    p.input2 = FilterInput::Result(second);
    p.mode = mode;
    p
}

fn matrix(mode: u8, values: &[f64]) -> FilterPrimitive {
    let mut p = prim(FilterKind::ColorMatrix);
    p.mode = mode;
    p.values[..values.len()].copy_from_slice(values);
    p
}

fn merge(inputs: &[FilterInput]) -> FilterPrimitive {
    let mut p = prim(FilterKind::Merge);
    p.merge_inputs[..inputs.len()].copy_from_slice(inputs);
    p.merge_count = inputs.len() as u8;
    p
}

/// Region covering the whole `size` x `size` layer in user space.
fn whole(count: usize) -> Filter {
    Filter {
        primitives: TableRange { first: 0, count },
        linear_rgb: false,
        filter_units_object_bounding_box: false,
        primitive_units_object_bounding_box: false,
        custom_region: true,
        region: [0.0, 0.0, 1.0e4, 1.0e4],
    }
}

fn layer(size: usize, fill: [u8; 4]) -> Vec<u8> {
    fill.iter().copied().cycle().take(size * size * 4).collect()
}

fn set(buffer: &mut [u8], size: usize, x: usize, y: usize, rgba: [u8; 4]) {
    buffer[(y * size + x) * 4..(y * size + x) * 4 + 4].copy_from_slice(&rgba);
}

fn get(buffer: &[u8], size: usize, x: usize, y: usize) -> [u8; 4] {
    buffer[(y * size + x) * 4..(y * size + x) * 4 + 4]
        .try_into()
        .unwrap()
}

fn run(
    buffer: &mut [u8],
    size: u32,
    filter: &Filter,
    chain: &[FilterPrimitive],
) -> Result<(), RenderError> {
    apply_filter_chain(buffer, &unit_view(size), filter, chain, 64 << 20, NEVER)
}

fn close(actual: [u8; 4], expected: [u8; 4], tolerance: i32) -> bool {
    actual
        .iter()
        .zip(expected)
        .all(|(a, e)| (i32::from(*a) - i32::from(e)).abs() <= tolerance)
}

#[test]
fn blur_radius_zero_is_identity() {
    let mut data = layer(8, [0; 4]);
    set(&mut data, 8, 1, 1, [200, 100, 50, 255]);
    set(&mut data, 8, 2, 1, [10, 250, 90, 77]);
    set(&mut data, 8, 5, 6, [255, 255, 255, 1]);
    let before = data.clone();
    run(&mut data, 8, &whole(1), &[blur(0.0, 0.0)]).unwrap();
    assert_eq!(data, before);
}

#[test]
fn blur_of_a_single_pixel_preserves_energy_within_rounding() {
    let size = 21;
    let mut data = layer(size, [0; 4]);
    set(&mut data, size, 10, 10, [255, 255, 255, 255]);
    run(&mut data, size as u32, &whole(1), &[blur(1.5, 1.5)]).unwrap();
    let energy: i32 = data.chunks_exact(4).map(|p| i32::from(p[3])).sum();
    // Each of the 11 x 11 kernel pixels (radius ceil(3 * 1.5) = 5) rounds to a
    // byte, so the C rounding can move the sum by at most half a byte per pixel;
    // the many sub-half-byte tail pixels round down, hence a deficit.
    assert!((energy - 255).abs() <= 121 / 2, "alpha energy {energy}");
    assert!(
        energy >= 255 * 9 / 10 && energy <= 255,
        "alpha energy {energy}"
    );
    assert_eq!(
        get(&data, size, 9, 10),
        get(&data, size, 11, 10),
        "symmetric in x"
    );
    assert_eq!(
        get(&data, size, 10, 9),
        get(&data, size, 10, 11),
        "symmetric in y"
    );
    assert!(data[(10 * size + 10) * 4 + 3] > get(&data, size, 9, 10)[3]);
    assert_eq!(get(&data, size, 10, 10)[..3], [255, 255, 255]);
}

#[test]
fn blur_radius_above_the_limit_is_limit_exceeded() {
    let mut data = layer(8, [255; 4]);
    let before = data.clone();
    assert_eq!(
        run(&mut data, 8, &whole(1), &[blur(32.5, 1.0)]),
        Err(RenderError::LimitExceeded)
    );
    assert_eq!(data, before, "layer untouched on error");
}

#[test]
fn offset_shifts_by_whole_negative_and_fractional_amounts() {
    let size = 8;
    let mut base = layer(size, [0; 4]);
    set(&mut base, size, 3, 3, [255, 0, 0, 255]);
    let cases = [
        ((-2.0, 0.0), (1, 3)),
        ((0.0, -1.0), (3, 2)),
        ((1.4, 0.0), (4, 3)),
        ((1.5, 0.0), (5, 3)),
        ((-1.5, 0.0), (1, 3)),
        ((0.0, 2.6), (3, 6)),
    ];
    for ((dx, dy), (x, y)) in cases {
        let mut data = base.clone();
        run(&mut data, size as u32, &whole(1), &[offset(dx, dy)]).unwrap();
        let lit: Vec<usize> = (0..size * size).filter(|i| data[i * 4 + 3] != 0).collect();
        assert_eq!(lit, vec![y * size + x], "dx={dx} dy={dy}");
        assert_eq!(get(&data, size, x, y), [255, 0, 0, 255]);
    }
}

#[test]
fn offset_out_of_range_is_limit_exceeded() {
    let mut data = layer(8, [255; 4]);
    assert_eq!(
        run(&mut data, 8, &whole(1), &[offset(5000.0, 0.0)]),
        Err(RenderError::LimitExceeded)
    );
}

#[test]
fn flood_fills_with_color_and_opacity_over_the_region() {
    let mut data = layer(8, [0; 4]);
    set(&mut data, 8, 0, 0, [1, 2, 3, 255]);
    let mut filter = whole(1);
    run(&mut data, 8, &filter, &[flood([10, 20, 30, 128])]).unwrap();
    assert!(data.chunks_exact(4).all(|p| p == [10, 20, 30, 128]));
    // linear light round trip keeps alpha and nearly keeps colour
    let mut data = layer(8, [0; 4]);
    set(&mut data, 8, 0, 0, [1, 2, 3, 255]);
    filter.linear_rgb = true;
    run(&mut data, 8, &filter, &[flood([10, 20, 30, 128])]).unwrap();
    let pixel = get(&data, 8, 4, 4);
    assert_eq!(pixel[3], 128);
    assert!(close(pixel, [10, 20, 30, 128], 3), "{pixel:?}");
}

#[test]
fn blend_modes_follow_the_c_formulas() {
    let source = [200u8, 100, 50, 255];
    let backdrop = [100u8, 150, 250, 255];
    let expected = [
        (0u8, [200, 100, 50, 255]),
        (1, [78, 59, 49, 255]),
        (2, [222, 191, 251, 255]),
        (3, [100, 100, 50, 255]),
        (4, [200, 150, 250, 255]),
    ];
    for (mode, want) in expected {
        let mut data = layer(4, source);
        let chain = [flood(backdrop), two_input(FilterKind::Blend, mode, 0)];
        run(&mut data, 4, &whole(2), &chain).unwrap();
        let got = get(&data, 4, 1, 1);
        assert!(close(got, want, 1), "mode {mode}: {got:?} vs {want:?}");
    }
}

#[test]
fn composite_operators_follow_the_c_formulas() {
    let a = [255u8, 0, 0, 128];
    let b = [0u8, 0, 255, 128];
    let (alpha_a, alpha_b) = (128.0 / 255.0, 128.0 / 255.0);
    let q = |v: f64| (v.clamp(0.0, 1.0) * 255.0 + 0.5).floor() as i32;
    // (mode, alpha, red-channel straight value)
    let over = alpha_a + alpha_b * (1.0 - alpha_a);
    let expected = [
        (0u8, q(over), q(alpha_a / over)),
        (1, q(alpha_a * alpha_b), 255),
        (2, q(alpha_a * (1.0 - alpha_b)), 255),
        (3, q(alpha_b), q(alpha_a * alpha_b / alpha_b)),
        (
            4,
            q(alpha_a * (1.0 - alpha_b) + alpha_b * (1.0 - alpha_a)),
            128,
        ),
        (5, 255, q((alpha_a / 1.0).min(1.0))),
    ];
    for (mode, alpha, red) in expected {
        let mut data = layer(4, a);
        let chain = [flood(b), two_input(FilterKind::Composite, mode, 0)];
        run(&mut data, 4, &whole(2), &chain).unwrap();
        let got = get(&data, 4, 2, 2);
        assert!(
            (i32::from(got[3]) - alpha).abs() <= 1,
            "mode {mode} alpha {got:?} vs {alpha}"
        );
        assert!(
            (i32::from(got[0]) - red).abs() <= 1 || mode == 4,
            "mode {mode} red {got:?} vs {red}"
        );
    }
    // arithmetic: k2*a + k3*b per channel including alpha, clamped
    let mut data = layer(4, a);
    let mut arithmetic = two_input(FilterKind::Composite, 6, 0);
    arithmetic.values[1] = 0.5;
    arithmetic.values[2] = 0.25;
    arithmetic.values[3] = 0.1;
    run(&mut data, 4, &whole(2), &[flood(b), arithmetic]).unwrap();
    let got = get(&data, 4, 0, 0);
    let channel = |av: f64, bv: f64| q(0.5 * av + 0.25 * bv + 0.1);
    let want = [
        channel(1.0, 0.0),
        channel(0.0, 0.0),
        channel(0.0, 1.0),
        channel(alpha_a, alpha_b),
    ];
    for c in 0..4 {
        assert!(
            (i32::from(got[c]) - want[c]).abs() <= 1,
            "arithmetic ch{c} {got:?} {want:?}"
        );
    }
}

#[test]
fn merge_stacks_inputs_in_order() {
    let red = [255, 0, 0, 255];
    let blue_half = [0, 0, 255, 128];
    let results = [FilterInput::Result(0), FilterInput::Result(1)];
    let mut data = layer(4, [0, 0, 0, 255]);
    run(
        &mut data,
        4,
        &whole(3),
        &[flood(red), flood(blue_half), merge(&results)],
    )
    .unwrap();
    let pixel = get(&data, 4, 1, 1);
    assert_eq!(pixel[3], 255);
    assert!(
        close(pixel, [127, 0, 128, 255], 1),
        "blue over red: {pixel:?}"
    );
    let reversed = [FilterInput::Result(1), FilterInput::Result(0)];
    let mut data = layer(4, [0, 0, 0, 255]);
    run(
        &mut data,
        4,
        &whole(3),
        &[flood(red), flood(blue_half), merge(&reversed)],
    )
    .unwrap();
    assert_eq!(
        get(&data, 4, 1, 1),
        [255, 0, 0, 255],
        "red on top hides blue"
    );
}

#[test]
fn color_matrix_types() {
    let source = [200u8, 100, 50, 255];
    // matrix: swap red and blue
    let swap = [
        0., 0., 1., 0., 0., 0., 1., 0., 0., 0., 1., 0., 0., 0., 0., 0., 0., 0., 1., 0.,
    ];
    let mut data = layer(4, source);
    run(&mut data, 4, &whole(1), &[matrix(0, &swap)]).unwrap();
    assert!(close(get(&data, 4, 0, 0), [50, 100, 200, 255], 1));
    // saturate 0 -> luminance grey (0.213, 0.715, 0.072)
    let mut data = layer(4, source);
    run(&mut data, 4, &whole(1), &[matrix(1, &[0.0])]).unwrap();
    assert!(
        close(get(&data, 4, 0, 0), [118, 118, 118, 255], 1),
        "{:?}",
        get(&data, 4, 0, 0)
    );
    // hueRotate 0 -> identity
    let mut data = layer(4, source);
    run(&mut data, 4, &whole(1), &[matrix(2, &[0.0])]).unwrap();
    assert!(close(get(&data, 4, 0, 0), source, 1));
    // hueRotate 90 changes the colour
    let mut data = layer(4, source);
    run(&mut data, 4, &whole(1), &[matrix(2, &[90.0])]).unwrap();
    assert!(!close(get(&data, 4, 0, 0), source, 10));
    // luminanceToAlpha: rgb 0, alpha 0.2126 r + 0.7152 g + 0.0722 b
    let mut data = layer(4, source);
    run(&mut data, 4, &whole(1), &[matrix(3, &[])]).unwrap();
    assert!(
        close(get(&data, 4, 0, 0), [0, 0, 0, 118], 1),
        "{:?}",
        get(&data, 4, 0, 0)
    );
    // fully transparent input stays transparent for matrix modes
    let mut data = layer(4, [0; 4]);
    run(&mut data, 4, &whole(1), &[matrix(0, &swap)]).unwrap();
    assert!(data.iter().all(|b| *b == 0));
}

#[test]
fn drop_shadow_offsets_tints_and_sits_under_the_source() {
    let size = 12;
    let mut data = layer(size, [0; 4]);
    set(&mut data, size, 5, 5, [255, 255, 255, 255]);
    let mut shadow = prim(FilterKind::DropShadow);
    shadow.values[2] = 3.0;
    shadow.values[3] = 2.0;
    shadow.rgba = [255, 0, 0, 128];
    run(&mut data, size as u32, &whole(1), &[shadow]).unwrap();
    assert_eq!(
        get(&data, size, 5, 5),
        [255, 255, 255, 255],
        "source on top"
    );
    assert_eq!(
        get(&data, size, 8, 7),
        [255, 0, 0, 128],
        "shadow at the offset"
    );
    let lit = data.chunks_exact(4).filter(|p| p[3] != 0).count();
    assert_eq!(lit, 2, "std 0: one source pixel, one shadow pixel");
    // blurred shadow spreads but stays tinted
    let mut data = layer(size, [0; 4]);
    set(&mut data, size, 5, 5, [255, 255, 255, 255]);
    shadow.values[0] = 1.0;
    shadow.values[1] = 1.0;
    run(&mut data, size as u32, &whole(1), &[shadow]).unwrap();
    let edge = get(&data, size, 9, 7);
    assert!(
        edge[3] > 0 && edge[3] < 128 && edge[..3] == [255, 0, 0],
        "{edge:?}"
    );
}

#[test]
fn region_clips_user_space_and_bounding_box_regions() {
    let size = 8;
    let mut data = layer(size, [255, 255, 255, 255]);
    let mut filter = whole(1);
    filter.region = [2.0, 2.0, 4.0, 4.0];
    run(&mut data, size as u32, &filter, &[offset(0.0, 0.0)]).unwrap();
    for y in 0..size {
        for x in 0..size {
            let inside = (2..6).contains(&x) && (2..6).contains(&y);
            assert_eq!(get(&data, size, x, y)[3] == 255, inside, "({x},{y})");
        }
    }
    // default (bounding box) region is the alpha bounds grown by 10% per side
    let mut data = layer(size, [0; 4]);
    for y in 2..6 {
        for x in 2..6 {
            set(&mut data, size, x, y, [255, 255, 255, 255]);
        }
    }
    let bbox = Filter {
        custom_region: false,
        filter_units_object_bounding_box: true,
        region: [-0.1, -0.1, 1.2, 1.2],
        ..whole(1)
    };
    run(&mut data, size as u32, &bbox, &[blur(1.0, 1.0)]).unwrap();
    assert_ne!(get(&data, size, 2, 3)[3], 0);
    assert_eq!(
        get(&data, size, 1, 3),
        [0; 4],
        "left spill clipped (centre 1.5 < 1.6)"
    );
    assert_eq!(
        get(&data, size, 6, 3),
        [0; 4],
        "right spill clipped (centre 6.5 >= 6.4)"
    );
    // empty source: nothing survives, even a flood
    let mut data = layer(size, [0; 4]);
    run(&mut data, size as u32, &bbox, &[flood([9, 9, 9, 255])]).unwrap();
    assert!(data.iter().all(|b| *b == 0));
}

#[test]
fn source_alpha_input_drops_color() {
    let mut data = layer(4, [10, 200, 30, 90]);
    let mut copy = prim(FilterKind::Offset);
    copy.input = FilterInput::Alpha;
    run(&mut data, 4, &whole(1), &[copy]).unwrap();
    assert_eq!(get(&data, 4, 1, 1), [0, 0, 0, 90]);
}

#[test]
fn previous_input_of_the_first_primitive_is_the_source_graphic() {
    let mut data = layer(4, [10, 20, 30, 255]);
    run(&mut data, 4, &whole(1), &[offset(0.0, 0.0)]).unwrap();
    assert_eq!(get(&data, 4, 1, 1), [10, 20, 30, 255]);
}

#[test]
fn dangling_or_empty_chains_are_invalid() {
    let mut data = layer(4, [255; 4]);
    let mut forward = offset(0.0, 0.0);
    forward.input = FilterInput::Result(3);
    assert_eq!(
        run(&mut data, 4, &whole(1), &[forward]),
        Err(RenderError::InvalidGeometry)
    );
    assert_eq!(
        run(&mut data, 4, &whole(0), &[]),
        Err(RenderError::InvalidGeometry)
    );
    assert_eq!(
        run(&mut data, 4, &whole(2), &[offset(0.0, 0.0)]),
        Err(RenderError::InvalidGeometry)
    );
}

#[test]
fn oversize_work_and_scratch_are_limit_exceeded() {
    // 4100 x 4100 opaque pixels exceed the 16 Mpx window cap.
    let size = 4100u32;
    let mut data = vec![255u8; size as usize * size as usize * 4];
    let chain = [offset(1.0, 0.0)];
    assert!(size as usize * size as usize > MAX_FILTER_PIXELS);
    let view = ViewMap::from_viewbox(
        ViewBox {
            x: 0.0,
            y: 0.0,
            width: f64::from(size),
            height: f64::from(size),
        },
        size,
        size,
        None,
    )
    .unwrap();
    assert_eq!(
        apply_filter_chain(&mut data, &view, &whole(1), &chain, 1 << 40, NEVER),
        Err(RenderError::LimitExceeded)
    );
    // scratch smaller than the surfaces it needs
    let mut small = layer(8, [255; 4]);
    assert_eq!(
        apply_filter_chain(
            &mut small,
            &unit_view(8),
            &whole(1),
            &[offset(1.0, 0.0)],
            100,
            NEVER
        ),
        Err(RenderError::LimitExceeded)
    );
}

#[test]
fn cancellation_returns_cancelled_and_keeps_the_layer() {
    let mut data = layer(16, [255, 0, 0, 255]);
    let before = data.clone();
    let chain = [blur(2.0, 2.0), offset(1.0, 1.0)];
    let always = || true;
    assert_eq!(
        apply_filter_chain(
            &mut data,
            &unit_view(16),
            &whole(2),
            &chain,
            64 << 20,
            &always
        ),
        Err(RenderError::Cancelled)
    );
    assert_eq!(data, before);
    // cancel part way through the pixel rows of the blur
    let calls = Cell::new(0u32);
    let later = || {
        calls.set(calls.get() + 1);
        calls.get() > 5
    };
    assert_eq!(
        apply_filter_chain(
            &mut data,
            &unit_view(16),
            &whole(2),
            &chain,
            64 << 20,
            &later
        ),
        Err(RenderError::Cancelled)
    );
    assert_eq!(data, before);
}

#[test]
fn linear_rgb_blur_of_a_gray_edge_differs_from_srgb_blur() {
    let size = 12;
    let mut base = layer(size, [0, 0, 0, 255]);
    for y in 0..size {
        for x in 6..size {
            set(&mut base, size, x, y, [255, 255, 255, 255]);
        }
    }
    let (mut linear, mut srgb) = (base.clone(), base);
    let mut filter = whole(1);
    run(&mut srgb, size as u32, &filter, &[blur(2.0, 0.0)]).unwrap();
    filter.linear_rgb = true;
    run(&mut linear, size as u32, &filter, &[blur(2.0, 0.0)]).unwrap();
    // averaging in linear light and re-encoding is brighter at the edge
    assert!(get(&linear, size, 5, 4)[0] > get(&srgb, size, 5, 4)[0]);
}
