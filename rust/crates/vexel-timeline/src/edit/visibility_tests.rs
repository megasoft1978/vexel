use super::{EditLimits, TimelineEdit};
use crate::{Error, TimelineContext};

const SOURCE: &[u8] = br#"{"schema":"vexel.animation","version":1,"ticks_per_second":24,"duration_ticks":2,"fps":{"numerator":24,"denominator":1},"loop":"once","onion":{"before":0,"after":0,"opacity":0},"tracks":[ {"id":"track_1","kind":"property","target":"node","visible":true,"property":"opacity","value_type":"scalar","interpolation":"linear","keys":[{"tick":0,"value":0.5}],"name":"named track"} ]}"#;

fn context() -> TimelineContext {
    TimelineContext {
        scene_ids: vec![b"node".to_vec()],
        assets: Vec::new(),
    }
}

fn edit(source: &[u8], limits: EditLimits) -> TimelineEdit {
    TimelineEdit::new(source, &context(), limits).unwrap()
}

#[test]
fn visibility_edit_replaces_only_the_boolean_and_preserves_serialized_order() {
    let mut timeline = edit(SOURCE, EditLimits::default());
    timeline
        .set_track_visibility(&context(), b"track_1", false)
        .unwrap();

    let expected = SOURCE
        .windows(14)
        .position(|window| window == b"\"visible\":true")
        .unwrap();
    let mut bytes = SOURCE.to_vec();
    bytes.splice(expected + 10..expected + 14, b"false".iter().copied());
    assert_eq!(timeline.source(), bytes);
    assert_eq!(timeline.undo_count(), 1);
}

#[test]
fn setting_existing_visibility_is_a_no_op() {
    let mut timeline = edit(SOURCE, EditLimits::default());
    timeline
        .set_track_visibility(&context(), b"track_1", true)
        .unwrap();
    assert_eq!(timeline.source(), SOURCE);
    assert_eq!(timeline.undo_count(), 0);
}

#[test]
fn visibility_edit_rejects_missing_tracks_and_malformed_ids() {
    let mut timeline = edit(SOURCE, EditLimits::default());
    assert_eq!(
        timeline
            .set_track_visibility(&context(), b"missing", false)
            .unwrap_err()
            .error,
        Error::InvalidReference
    );
    assert_eq!(
        timeline
            .set_track_visibility(&context(), b"bad/id", false)
            .unwrap_err()
            .error,
        Error::InvalidArgument
    );
    assert_eq!(timeline.undo_count(), 0);
}

#[test]
fn visibility_edit_obeys_history_byte_limit() {
    let limits = EditLimits {
        commands: 1,
        bytes: SOURCE.len(),
    };
    let mut timeline = edit(SOURCE, limits);
    assert_eq!(
        timeline
            .set_track_visibility(&context(), b"track_1", false)
            .unwrap_err()
            .error,
        Error::Limit
    );
    assert_eq!(timeline.source(), SOURCE);
    assert_eq!(timeline.undo_count(), 0);
}
