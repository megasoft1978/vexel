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
mod size;
mod state;
mod surface;
mod view;

pub use controls::{Control, ControlKind, Controls, controls, paint_cluster};
pub use events::{EventOutcome, handle_event};
pub use map::{Bounds, ViewMap};
pub use natural::natural_size;
pub use overlay::{
    HANDLE, HOVER_OPACITY, HOVER_WIDTH, SELECTION_WIDTH, handle_points, handle_rect,
};
pub use paint::{CHECKER, SHADOW_LAYERS, StagePaint, artboard_device_rect, over, paint_stage};
pub use size::{artboard_pixels, frame_matches, target_compose_size};
pub use state::CanvasState;
pub use view::{FIT_MARGIN, MAX_ZOOM, MIN_ZOOM, View, ZOOM_STEP};
