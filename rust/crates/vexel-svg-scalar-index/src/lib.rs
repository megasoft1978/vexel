//! Conservative shape bounds, boundary masks, scanline rows, fill intervals,
//! and deterministic scalar work preflight.
#![forbid(unsafe_code)]

mod bounds;
mod fill;
mod job_bounds;
mod job_clip;
mod job_engine;
mod job_fill;
mod job_preflight;
mod job_rows;
mod job_storage;
mod preflight;
mod scanline;
mod source_limits;
mod util;

// Historical Vec interfaces are compiled only for the original unit fixtures.
// Production callers reach owning-job engines and allocation-free scene helpers.
#[cfg(test)]
mod legacy {
    pub use crate::bounds::{boundary_test, build_bounds, mark_boundaries, mark_clip_boundaries};
    pub use crate::fill::{
        build_fill_row, build_fill_runs, build_runs, color, create_fill_cache, eligible,
        fill_contains, interval_pixel_range, pixel_runs,
    };
    pub use crate::preflight::preflight;
    pub use crate::scanline::{build_row, create_scanlines, prepare_rows, reset_scanlines};
}
pub use fill::fill_color;
pub use job_bounds::{build_bounds_job, mark_boundaries_job, marked_pixel_count_job};
#[cfg(test)]
use legacy::*;
pub use preflight::work_budget;
pub use scanline::{JobScanlineState, create_scanlines_job};

#[cfg(test)]
mod tests;

pub use job_clip::mark_clip_boundaries_job;
pub use job_fill::*;
pub use job_rows::*;

pub use job_engine::ScalarIndex;
pub use job_preflight::preflight_job;

pub use source_limits::validate_source_length;
