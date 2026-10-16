//! Conservative shape bounds, boundary masks, scanline rows, fill intervals,
//! and deterministic scalar work preflight.
#![forbid(unsafe_code)]

mod bounds;
mod fill;
mod preflight;
mod scanline;
mod util;

pub use bounds::{boundary_test, build_bounds, mark_boundaries, mark_clip_boundaries};
pub use fill::{
    build_fill_row, build_fill_runs, build_runs, color, create_fill_cache, eligible, fill_color,
    fill_contains, interval_pixel_range, pixel_runs,
};
pub use preflight::{preflight, work_budget};
pub use scanline::{build_row, create_scanlines, prepare_rows, reset_scanlines};

#[cfg(test)]
mod tests;
