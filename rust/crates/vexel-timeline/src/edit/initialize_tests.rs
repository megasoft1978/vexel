use super::{EditLimits, EmptyTimelineSettings, TimelineEdit, initialize_empty_timeline};
use crate::{Error, LoopMode, TimelineContext, decode};

fn settings() -> EmptyTimelineSettings {
    EmptyTimelineSettings {
        ticks_per_second: 24,
        duration_ticks: 240,
        fps_numerator: 24,
        fps_denominator: 1,
        loop_mode: LoopMode::PingPong,
        onion_before: 2,
        onion_after: 3,
        onion_opacity: 0.375,
    }
}

#[test]
fn initializer_builds_valid_empty_v1_payload_from_caller_settings() {
    let context = TimelineContext::default();
    let bytes = initialize_empty_timeline(settings(), &context).unwrap();
    let view = decode(&bytes, &context).unwrap();
    assert!(!view.is_empty());
    assert_eq!(view.track_count(), 0);
    assert_eq!(view.ticks_per_second(), 24);
    assert_eq!(view.duration_ticks(), 240);
    assert_eq!(view.fps(), (24, 1));
    assert_eq!(view.loop_mode(), LoopMode::PingPong);
    assert_eq!(view.onion(), (2, 3, 0.375));
}

#[test]
fn initializer_rejects_invalid_settings_and_invalid_context() {
    let context = TimelineContext::default();
    let mut invalid = settings();
    invalid.fps_numerator = 7;
    assert_eq!(
        initialize_empty_timeline(invalid, &context)
            .unwrap_err()
            .error,
        Error::InvalidArgument
    );
    invalid = settings();
    invalid.onion_after = 9;
    assert_eq!(
        initialize_empty_timeline(invalid, &context)
            .unwrap_err()
            .error,
        Error::InvalidArgument
    );

    let invalid_context = TimelineContext {
        scene_ids: vec![b"z".to_vec(), b"a".to_vec()],
        assets: Vec::new(),
    };
    assert_eq!(
        initialize_empty_timeline(settings(), &invalid_context)
            .unwrap_err()
            .error,
        Error::InvalidArgument
    );
}

#[test]
fn empty_source_stays_distinct_from_unsupported_nonempty_source() {
    let context = TimelineContext::default();
    assert!(decode(b"", &context).unwrap().is_empty());
    assert_eq!(
        decode(b"{}", &context).unwrap_err().error,
        Error::UnsupportedVersion
    );
    assert_eq!(
        TimelineEdit::new(b"", &context, EditLimits::default())
            .err()
            .unwrap()
            .error,
        Error::Limit
    );
    assert_eq!(
        TimelineEdit::new(b"{}", &context, EditLimits::default())
            .err()
            .unwrap()
            .error,
        Error::UnsupportedVersion
    );
}
