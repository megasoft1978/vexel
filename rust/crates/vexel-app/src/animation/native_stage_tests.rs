use crate::animation::native_frame::NativeFrameError;
use crate::animation::native_stage::{NativeStage, OnionSettings, StageUpdate};
use crate::compose::Frame;
use crate::native_animation::{NativeFrameResult, NativeFrameTag};

fn tag(revision: u64, tick: u32) -> NativeFrameTag {
    NativeFrameTag {
        state_revision: revision,
        tick,
        width: 1,
        height: 1,
    }
}

fn frame(value: u8) -> Frame {
    Frame {
        width: 1,
        height: 1,
        rgba: vec![value; 4],
        disclosures: Vec::new(),
    }
}

#[test]
fn stale_and_failed_frames_keep_the_last_complete_frame() {
    let mut stage = NativeStage::new();
    let onion = OnionSettings {
        before: 2,
        after: 3,
        opacity: 0.25,
    };
    stage.expect(tag(1, 0), onion);
    assert_eq!(
        stage.accept(NativeFrameResult {
            tag: tag(1, 0),
            result: Ok(frame(7)),
        }),
        StageUpdate::Published
    );
    stage.expect(tag(2, 1), onion);
    assert_eq!(
        stage.accept(NativeFrameResult {
            tag: tag(1, 0),
            result: Ok(frame(8)),
        }),
        StageUpdate::Stale
    );
    assert_eq!(
        stage.accept(NativeFrameResult {
            tag: tag(2, 1),
            result: Err(NativeFrameError::InvalidTick),
        }),
        StageUpdate::Failed(NativeFrameError::InvalidTick)
    );
    assert_eq!(
        stage.frame().map(|current| current.rgba.as_slice()),
        Some([7, 7, 7, 7].as_slice())
    );
    assert_eq!(stage.onion_settings(), Some(onion));
    assert!(!stage.onion_preview_available());
    assert_eq!(
        stage.onion_preview_status(),
        "Onion-skin preview unavailable"
    );
}

#[test]
fn refuses_a_frame_with_geometry_that_does_not_match_its_tag() {
    let mut stage = NativeStage::new();
    stage.expect(
        tag(4, 2),
        OnionSettings {
            before: 0,
            after: 0,
            opacity: 0.5,
        },
    );
    assert_eq!(
        stage.accept(NativeFrameResult {
            tag: tag(4, 2),
            result: Ok(Frame {
                width: 2,
                height: 1,
                rgba: vec![0; 8],
                disclosures: Vec::new(),
            }),
        }),
        StageUpdate::Failed(NativeFrameError::Compose(
            crate::compose::ComposeError::InvalidSize
        ))
    );
    assert!(stage.frame().is_none());
}
