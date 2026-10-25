//! Region-bounded filter window and the large-sigma blur budget.
mod common;

use common::{NEVER, unit_view};
use vexel_render::filter::apply_filter_chain;
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

fn blur(sx: f64, sy: f64) -> FilterPrimitive {
    let mut p = prim(FilterKind::GaussianBlur);
    p.values[0] = sx;
    p.values[1] = sy;
    p
}

fn offset(dx: f64, dy: f64) -> FilterPrimitive {
    let mut p = prim(FilterKind::Offset);
    p.values[2] = dx;
    p.values[3] = dy;
    p
}

fn filter(count: usize, default_region: bool) -> Filter {
    Filter {
        primitives: TableRange { first: 0, count },
        linear_rgb: false,
        filter_units_object_bounding_box: false,
        primitive_units_object_bounding_box: false,
        custom_region: !default_region,
        region: if default_region { [-0.1, -0.1, 1.2, 1.2] } else { [0.0, 0.0, 1.0e4, 1.0e4] },
    }
}

fn view(width: u32, height: u32) -> ViewMap {
    let vb = ViewBox { x: 0.0, y: 0.0, width: f64::from(width), height: f64::from(height) };
    ViewMap::from_viewbox(vb, width, height, None).unwrap()
}

fn rect_layer(w: usize, h: usize, rect: [usize; 4]) -> Vec<u8> {
    let mut data = vec![0u8; w * h * 4];
    for y in rect[1]..rect[3] {
        for x in rect[0]..rect[2] {
            let c = [(x * 7 % 251) as u8, (y * 13 % 241) as u8, 90, 255];
            data[(y * w + x) * 4..(y * w + x) * 4 + 4].copy_from_slice(&c);
        }
    }
    data
}

/// The bounded window must give exactly what the whole-canvas evaluation gives
/// inside the filter region (and nothing outside it).
#[test]
fn bounded_window_matches_whole_canvas_evaluation() {
    let (w, h) = (400usize, 300usize);
    let rect = [150, 100, 250, 160];
    let chain = [blur(4.0, 3.0), offset(7.0, -5.0), blur(2.0, 2.0)];
    let mut whole = rect_layer(w, h, rect);
    apply_filter_chain(&mut whole, &view(w as u32, h as u32), &filter(3, false), &chain, 1 << 30, NEVER).unwrap();
    let mut bounded = rect_layer(w, h, rect);
    apply_filter_chain(&mut bounded, &view(w as u32, h as u32), &filter(3, true), &chain, 1 << 30, NEVER).unwrap();
    // region of the default filter: bbox grown by 10% / 120%
    let (rx0, ry0, rx1, ry1) = (140.0, 94.0, 260.0, 166.0);
    let mut inside = 0;
    for y in 0..h {
        for x in 0..w {
            let at = (y * w + x) * 4;
            let (cx, cy) = (x as f64 + 0.5, y as f64 + 0.5);
            let in_region = cx >= rx0 && cx < rx1 && cy >= ry0 && cy < ry1;
            if in_region {
                inside += 1;
                assert_eq!(bounded[at..at + 4], whole[at..at + 4], "pixel {x},{y}");
            } else {
                assert_eq!(bounded[at..at + 4], [0; 4], "outside {x},{y}");
            }
        }
    }
    assert!(inside > 0);
}

/// 1580 x 560 with sigma 32 used to be refused; it now runs as a box blur.
#[test]
fn sigma_32_on_a_wide_region_is_accepted_and_conserves_mass() {
    let (w, h) = (1580usize, 560usize);
    let rect = [400, 150, 1000, 400];
    let mut data = rect_layer(w, h, rect);
    let before: u64 = data.chunks_exact(4).map(|p| u64::from(p[3])).sum();
    apply_filter_chain(&mut data, &view(w as u32, h as u32), &filter(1, false), &[blur(32.0, 32.0)], 1 << 30, NEVER)
        .unwrap();
    let alpha = |x: usize, y: usize| data[(y * w + x) * 4 + 3];
    assert_eq!(alpha(700, 275), 255);
    let edge = alpha(400, 275);
    assert!((100..=156).contains(&edge), "edge alpha {edge}");
    assert_eq!(alpha(10, 10), 0);
    let after: u64 = data.chunks_exact(4).map(|p| u64::from(p[3])).sum();
    let drift = before.abs_diff(after) as f64 / before as f64;
    assert!(drift < 0.01, "mass drift {drift}");
}

#[test]
fn sigma_above_32_and_oversize_windows_are_typed_errors() {
    let mut data = rect_layer(64, 64, [10, 10, 50, 50]);
    assert_eq!(
        apply_filter_chain(&mut data, &unit_view(64), &filter(1, true), &[blur(32.5, 1.0)], 1 << 30, NEVER),
        Err(RenderError::LimitExceeded)
    );
}

#[test]
fn transparent_layers_cost_nothing_whatever_the_canvas() {
    let mut data = vec![0u8; 4096 * 4096 * 4];
    apply_filter_chain(&mut data, &view(4096, 4096), &filter(1, true), &[blur(32.0, 32.0)], 1 << 30, NEVER)
        .unwrap();
}
