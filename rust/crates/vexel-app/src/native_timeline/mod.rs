//! Native animation timeline UI contracts.
//!
//! This panel is separate from `timeline`, which displays source-based SVG
//! SMIL animation. The coordinator maps emitted intents to app commands.

pub mod interaction;
pub mod layout;
pub mod model;
pub mod paint;
pub mod presentation;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeTrackKind {
    Drawing,
    Property,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NativeTimelineItem {
    pub tick: u32,
    pub label: String,
    pub selected: bool,
    pub ease: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NativeTimelineTrack {
    pub id: String,
    pub name: String,
    pub target: String,
    pub kind: NativeTrackKind,
    pub visible: bool,
    pub selected: bool,
    pub items: Vec<NativeTimelineItem>,
}

/// Immutable display state built by the application for a single paint/input
/// pass. The UI never retains project or renderer objects.
#[derive(Clone, Debug, PartialEq)]
pub struct NativeTimelineSnapshot {
    pub ticks_per_second: u32,
    pub duration_ticks: u32,
    pub current_tick: u32,
    pub tracks: Vec<NativeTimelineTrack>,
    /// Stable SVG id of the canvas selection, when one exists. This lets the
    /// panel create the first timeline track before any timeline row exists.
    pub selected_target: Option<String>,
    pub playing: bool,
    pub loop_enabled: bool,
    pub onion_before: u32,
    pub onion_after: u32,
    pub onion_opacity: f64,
    pub can_add_drawing_track: bool,
    pub can_import_asset: bool,
    pub can_edit_selected_item: bool,
    pub status: Option<String>,
}

/// User intent only. The application validates and commits every request.
#[derive(Clone, Debug, PartialEq)]
pub enum NativeTimelineAction {
    SelectTrack {
        track_id: String,
    },
    SelectItem {
        track_id: String,
        tick: u32,
    },
    Seek {
        tick: u32,
    },
    SetViewport {
        first_tick: u32,
        last_tick: u32,
    },
    SetTrackVisibility {
        track_id: String,
        visible: bool,
    },
    RequestAddDrawingTrack {
        target_id: String,
    },
    RequestImportSvg {
        target_id: String,
        tick: u32,
    },
    SetCel {
        track_id: String,
        tick: u32,
        asset_name: String,
    },
    RemoveCel {
        track_id: String,
        tick: u32,
    },
    MoveCel {
        track_id: String,
        from_tick: u32,
        to_tick: u32,
    },
    MovePropertyKey {
        track_id: String,
        from_tick: u32,
        to_tick: u32,
    },
    RemovePropertyKey {
        track_id: String,
        tick: u32,
    },
    SetPropertyKeyEase {
        track_id: String,
        tick: u32,
        value: String,
        ease: String,
    },
    DuplicateTrack {
        track_id: String,
    },
    RemoveTrack {
        track_id: String,
    },
    AddOpacityKey {
        target_id: String,
        tick: u32,
        value: f64,
    },
    TogglePlayback,
    SetLoop {
        enabled: bool,
    },
    SetOnion {
        before: u32,
        after: u32,
        opacity: f64,
    },
}
