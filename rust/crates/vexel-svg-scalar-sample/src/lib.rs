//! Scalar SVG sampling and stroke acceleration primitives.
#![forbid(unsafe_code)]

mod cache_capacity;
mod cache_storage;
mod cache_builder;
mod dash;
mod fill;
mod grid;
mod job_dash;
mod job_grid;
mod sample;
mod stroke;

pub use dash::{dash_range_joins_seam, shape_closed_seam, subpath_closed_seam};
pub use fill::{FillRowLayout, fill_row, tile_extent_ceil};
pub use grid::grid_lookup;
pub use cache_builder::{CacheState, rebuild_dash_cache, build_dash_cache};
pub use sample::{RasterSampleInput, paint_sample, paint_sample_inside};
pub use stroke::{sample_work, stroke_contains};

pub const DASH_CACHE_BYTE_LIMIT: usize = 16 * 1024 * 1024;
pub const DASH_CACHE_MAX_STEPS: usize = 1_000_000;
pub const STROKE_GRID_MAX_CELLS: usize = 65_536;
pub const STROKE_GRID_MAX_REFS: usize = 1_000_000;
pub const STROKE_GRID_MAX_AXIS: usize = 128;
pub const STROKE_GRID_JOIN_BIT: u32 = 0x8000_0000;
pub const FILL_SAMPLES_PER_AXIS: usize = 4;
pub const FILL_SAMPLE_COUNT: usize = FILL_SAMPLES_PER_AXIS * FILL_SAMPLES_PER_AXIS;
const DASH_EPSILON: f64 = 1.0e-12;

fn stopped(stop: vexel_scene::Stop) -> vexel_scene::Diagnostic {
    vexel_scene::Diagnostic {
        kind: match stop {
            vexel_scene::Stop::Canceled => vexel_scene::ErrorKind::Canceled,
            vexel_scene::Stop::Limit => vexel_scene::ErrorKind::Limit(vexel_scene::LimitKind::Work),
            vexel_scene::Stop::Overflow => {
                vexel_scene::ErrorKind::Limit(vexel_scene::LimitKind::Arithmetic)
            }
        },
        offset: 0,
    }
}
#[cfg(test)]
mod unit_tests;
