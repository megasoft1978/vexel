//! Filter region (C `find_alpha_bounds` and `clip_filter_region`).

use crate::ViewMap;
use vexel_scene::Filter;

/// Pixel bounds `[min_x, min_y, max_x, max_y)` of non-zero alpha, as the C
/// object bounding box of the layer; `None` for a fully transparent layer.
pub(super) fn alpha_bounds(rgba: &[u8], width: usize, height: usize) -> Option<[f64; 4]> {
    let (mut min_x, mut min_y, mut max_x, mut max_y) = (width, height, 0usize, 0usize);
    let mut found = false;
    for y in 0..height {
        for x in 0..width {
            if rgba[(y * width + x) * 4 + 3] == 0 {
                continue;
            }
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x + 1);
            max_y = max_y.max(y + 1);
            found = true;
        }
    }
    found.then(|| [min_x as f64, min_y as f64, max_x as f64, max_y as f64])
}

/// Filter region in device pixels `[min_x, min_y, max_x, max_y]`; `source` is
/// the pre-filter alpha bounds.
pub(super) fn device_region(view: &ViewMap, filter: &Filter, source: [f64; 4]) -> [f64; 4] {
    let mut region = [0.0f64; 4];
    if !filter.custom_region || filter.filter_units_object_bounding_box {
        let box_width = source[2] - source[0];
        let box_height = source[3] - source[1];
        region[0] = source[0] + filter.region[0] * box_width;
        region[1] = source[1] + filter.region[1] * box_height;
        region[2] = region[0] + filter.region[2] * box_width;
        region[3] = region[1] + filter.region[3] * box_height;
    } else {
        region[0] = view.offset_x + (filter.region[0] - view.view.x) * view.scale_x;
        region[1] = view.offset_y + (filter.region[1] - view.view.y) * view.scale_y;
        region[2] = region[0] + filter.region[2] * view.scale_x;
        region[3] = region[1] + filter.region[3] * view.scale_y;
    }
    region
}

/// Filter region as user-space `[x, y, width, height]` (stitchTiles tile).
pub(super) fn user_region(view: &ViewMap, filter: &Filter, source: Option<[f64; 4]>) -> [f64; 4] {
    let Some(source) = source else {
        return [view.view.x, view.view.y, view.view.width, view.view.height];
    };
    let r = device_region(view, filter, source);
    [
        view.view.x + (r[0] - view.offset_x) / view.scale_x,
        view.view.y + (r[1] - view.offset_y) / view.scale_y,
        (r[2] - r[0]) / view.scale_x,
        (r[3] - r[1]) / view.scale_y,
    ]
}

/// Clears every pixel whose centre lies outside `region` (device pixels of
/// this buffer, from [`device_region`]); `None` clears everything.
pub(super) fn clip(rgba: &mut [u8], width: usize, height: usize, region: Option<[f64; 4]>) {
    let Some(region) = region else {
        rgba.fill(0);
        return;
    };
    for y in 0..height {
        let sample_y = y as f64 + 0.5;
        for x in 0..width {
            let sample_x = x as f64 + 0.5;
            if sample_x >= region[0]
                && sample_y >= region[1]
                && sample_x < region[2]
                && sample_y < region[3]
            {
                continue;
            }
            let at = (y * width + x) * 4;
            rgba[at..at + 4].fill(0);
        }
    }
}
