//! Scalar SVG sampling and stroke acceleration primitives.
#![forbid(unsafe_code)]

mod dash;
mod fill;
mod grid;
mod sample;
mod stroke;

pub use dash::{build_dash_cache, dash_range_joins_seam, shape_closed_seam, subpath_closed_seam};
pub use fill::fill_row;
pub use grid::{build_stroke_grid, grid_lookup};
pub use sample::{paint_sample, paint_sample_inside};
pub use stroke::{sample_work, stroke_contains};

use vexel_scene::{
    AlphaMode, DashCache, RasterScratch, Result, SampleInput, ShapeId, ViewMapping, WorkControl,
};

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
fn oom() -> vexel_scene::Diagnostic {
    vexel_scene::Diagnostic {
        kind: vexel_scene::ErrorKind::OutOfMemory,
        offset: 0,
    }
}
fn limit() -> vexel_scene::Diagnostic {
    vexel_scene::Diagnostic {
        kind: vexel_scene::ErrorKind::Limit(vexel_scene::LimitKind::DashCache),
        offset: 0,
    }
}
fn reserve<T>(v: &mut Vec<T>, n: usize) -> Result<()> {
    v.try_reserve_exact(n).map_err(|_| oom())
}
fn bytes<T>(v: &Vec<T>) -> Option<usize> {
    v.capacity().checked_mul(std::mem::size_of::<T>())
}
fn cache_used(cache: &DashCache) -> Option<usize> {
    bytes(&cache.shapes)?
        .checked_add(bytes(&cache.segments)?)?
        .checked_add(bytes(&cache.joins)?)?
        .checked_add(cache.grid_bytes)
}

#[cfg(test)]
mod unit_tests;
