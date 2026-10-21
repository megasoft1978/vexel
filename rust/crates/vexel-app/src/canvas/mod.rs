//! The canvas well: artboard presentation, zoom/pan/fit, and the hover and
//! selection overlays, painted inside the stage (design direction section 5).
//!
//! The document is composed once as the artboard at the wanted resolution
//! (`size`) and presented through a [`View`] (`paint`); only a zoom that
//! leaves the cached resolution band recomposes. `ViewMap` is the single
//! view <-> frame <-> document mapping used by pointer, tools and overlays.

mod controls;
mod events;
mod map;
mod natural;
mod overlay;
mod paint;
mod rotate;
mod size;
mod state;
mod surface;
mod view;

pub use controls::{controls, paint_cluster, Control, ControlKind, Controls};
pub use events::{handle_event, EventOutcome};
pub use map::{Bounds, ViewMap};
pub use natural::natural_size;
pub use overlay::{
    handle_points, handle_rect, HANDLE, HOVER_OPACITY, HOVER_WIDTH, SELECTION_WIDTH,
};
pub use paint::{
    artboard_device_rect, over, paint_stage, well_fill, StagePaint, CHECKER, SHADOW_LAYERS,
};
pub use rotate::{rotation_targets, snapped_rotation};
pub use size::{artboard_pixels, frame_matches, target_compose_size};
pub use state::CanvasState;
pub use view::{percent_label, View, FIT_MARGIN, FIT_MAX_ZOOM, MAX_ZOOM, MIN_ZOOM, ZOOM_STEP};
