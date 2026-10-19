//! Application contracts shared by native timeline control, rendering and UI.

mod commands;
mod controller;
mod snapshot;

pub mod host;
pub mod import;

pub use controller::NativeAnimationController;
pub use host::NativeAnimationHost;

use std::sync::Arc;

use crate::animation::native_frame::NativeFrameError;
use crate::compose::{ComposeError, Frame, SvgViewport};
use crate::native_timeline::{NativeTimelineAction, NativeTimelineSnapshot};
use crate::project_session::{
    NativeCommandError, ProjectTimelineCandidate, ProjectTimelineError, SceneIdsError, SessionError,
};

/// User-selected timeline surface. Native tracks and SVG SMIL stay separate.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum TimelineSurface {
    #[default]
    Smil,
    Native,
}

/// An effect that must be completed by the application host.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NativeTimelineEffect {
    ImportSvg { target_id: String, tick: u32 },
}

/// Result of processing one native timeline intent.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct NativeActionOutcome {
    pub changed: bool,
    pub effect: Option<NativeTimelineEffect>,
}

/// Typed failure while reading or applying native timeline state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NativeAnimationError {
    Session(SessionError),
    Timeline(ProjectTimelineError),
    Operation(ProjectTimelineError),
    SceneIds(SceneIdsError),
    Compose(ComposeError),
    InvalidAction,
}

impl From<NativeCommandError<()>> for NativeAnimationError {
    fn from(error: NativeCommandError<()>) -> Self {
        match error {
            NativeCommandError::Session(error) => Self::Session(error),
            NativeCommandError::Timeline(error) => Self::Timeline(error),
            NativeCommandError::SceneIds(error) => Self::SceneIds(error),
            NativeCommandError::CandidateLineageChanged
            | NativeCommandError::ExistingAssetsChanged
            | NativeCommandError::Operation(()) => Self::InvalidAction,
        }
    }
}

impl From<NativeCommandError<ProjectTimelineError>> for NativeAnimationError {
    fn from(error: NativeCommandError<ProjectTimelineError>) -> Self {
        match error {
            NativeCommandError::Session(error) => Self::Session(error),
            NativeCommandError::Timeline(error) => Self::Timeline(error),
            NativeCommandError::SceneIds(error) => Self::SceneIds(error),
            NativeCommandError::Operation(error) => Self::Operation(error),
            NativeCommandError::CandidateLineageChanged
            | NativeCommandError::ExistingAssetsChanged => Self::InvalidAction,
        }
    }
}

/// All immutable inputs needed to render one native integer tick.
#[derive(Clone, Debug)]
pub struct NativeRenderRequest {
    pub state_revision: u64,
    pub tick: u32,
    pub output_width: u32,
    pub output_height: u32,
    pub source_svg: Arc<Vec<u8>>,
    pub timeline: ProjectTimelineCandidate,
    pub viewport: SvgViewport,
}

/// Identity attached to a render job and its completed frame.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NativeFrameTag {
    pub state_revision: u64,
    pub tick: u32,
    pub width: u32,
    pub height: u32,
}

/// A complete native frame or a typed error; never a partial frame.
#[derive(Debug)]
pub struct NativeFrameResult {
    pub tag: NativeFrameTag,
    pub result: Result<Frame, NativeFrameError>,
}

// Keep the UI types visible in the module-level contract for consumers.
pub type NativeIntent = NativeTimelineAction;
pub type NativePresentation = NativeTimelineSnapshot;
