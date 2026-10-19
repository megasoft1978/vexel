use super::AppDocument;
use vexel_timeline::{EditLimits, EmptyTimelineSettings, LoopMode};

fn settings() -> EmptyTimelineSettings {
    EmptyTimelineSettings {
        ticks_per_second: 24,
        duration_ticks: 240,
        fps_numerator: 24,
        fps_denominator: 1,
        loop_mode: LoopMode::Once,
        onion_before: 0,
        onion_after: 0,
        onion_opacity: 0.5,
    }
}

#[test]
fn payload_only_native_edits_advance_state_revision_not_source_revision() {
    let mut document = AppDocument::new(32, 24).unwrap();
    let original_source_revision = document.revision();
    let original_state_revision = document.state_revision();
    let candidate = document
        .native_timeline_candidate(settings(), EditLimits::default())
        .unwrap();
    assert!(candidate.is_editable());
    assert!(!document.is_dirty());
    assert_eq!(document.revision(), original_source_revision);
    assert_eq!(document.state_revision(), original_state_revision);

    assert!(
        document
            .edit_timeline(settings(), EditLimits::default(), |candidate| {
                candidate.set_loop(LoopMode::Repeat)
            })
            .unwrap()
    );
    assert!(document.is_dirty());
    assert_eq!(document.revision(), original_source_revision);
    assert_eq!(document.state_revision(), original_state_revision + 1);

    assert!(document.undo().unwrap());
    assert!(!document.is_dirty());
    assert_eq!(document.revision(), original_source_revision);
    assert_eq!(document.state_revision(), original_state_revision + 2);
}

#[test]
fn no_op_native_edit_does_not_advance_state_revision() {
    let mut document = AppDocument::new(32, 24).unwrap();
    let revision = document.state_revision();
    assert!(
        !document
            .edit_timeline(settings(), EditLimits::default(), |candidate| {
                candidate.set_loop(LoopMode::Once)
            })
            .unwrap()
    );
    assert_eq!(document.state_revision(), revision);
    assert!(!document.is_dirty());
}
