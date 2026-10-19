//! Complete-frame publication for the native timeline canvas.

use crate::compose::Frame;
use crate::native_animation::{NativeFrameResult, NativeFrameTag};

/// Persisted onion preferences remain visible while onion compositing is not
/// available in this playback stage.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OnionSettings {
    pub before: u32,
    pub after: u32,
    pub opacity: f64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StageUpdate {
    Published,
    Failed(super::native_frame::NativeFrameError),
    Stale,
}

/// Holds the last complete frame and accepts only the active render identity.
#[derive(Debug, Default)]
pub struct NativeStage {
    expected: Option<NativeFrameTag>,
    frame: Option<Frame>,
    onion: Option<OnionSettings>,
}

impl NativeStage {
    pub fn new() -> Self {
        Self::default()
    }

    /// Start expecting a frame for this state/tick/size. The previous complete
    /// frame stays available until a replacement succeeds.
    pub fn expect(&mut self, tag: NativeFrameTag, onion: OnionSettings) {
        self.expected = Some(tag);
        self.onion = Some(onion);
    }

    pub fn accept(&mut self, result: NativeFrameResult) -> StageUpdate {
        if self.expected != Some(result.tag) {
            return StageUpdate::Stale;
        }
        match result.result {
            Ok(frame) if frame.width == result.tag.width && frame.height == result.tag.height => {
                self.frame = Some(frame);
                StageUpdate::Published
            }
            Ok(_) => StageUpdate::Failed(super::native_frame::NativeFrameError::Compose(
                crate::compose::ComposeError::InvalidSize,
            )),
            Err(error) => StageUpdate::Failed(error),
        }
    }

    pub fn frame(&self) -> Option<&Frame> {
        self.frame.as_ref()
    }

    pub fn expected(&self) -> Option<NativeFrameTag> {
        self.expected
    }

    pub fn onion_settings(&self) -> Option<OnionSettings> {
        self.onion
    }

    pub const fn onion_preview_available(&self) -> bool {
        false
    }

    pub const fn onion_preview_status(&self) -> &'static str {
        "Onion-skin preview unavailable"
    }
}

#[cfg(test)]
#[path = "native_stage_tests.rs"]
mod tests;
