use super::super::{EditLimits, TimelineEdit};
use crate::{Error, TimelineContext};

const SOURCE: &[u8] = br#"{"schema":"vexel.animation","version":1,"ticks_per_second":24,"duration_ticks":5,"fps":{"numerator":24,"denominator":1},"loop":"once","onion":{"before":0,"after":0,"opacity":0},"tracks":[ {"id":"p","kind":"property","target":"node","visible":true,"property":"opacity","value_type":"scalar","interpolation":"linear","keys":[{"tick":0,"value":0.5},{"tick":2,"value":0.8}],"name":"prop"}, {"id":"d","kind":"drawing","target":"node","visible":true,"cels":[{"tick":0,"asset":"a"}],"name":"draw"} ]}"#;
fn context() -> TimelineContext {
    TimelineContext {
        scene_ids: vec![b"node".to_vec()],
        assets: vec![crate::AssetRef {
            name: b"a".to_vec(),
            media_type: b"image/svg+xml".to_vec(),
        }],
    }
}
fn editor() -> TimelineEdit {
    TimelineEdit::new(SOURCE, &context(), EditLimits::default()).unwrap()
}
fn roundtrip(mut e: TimelineEdit) {
    let after = e.source().to_vec();
    e.undo(&context()).unwrap();
    assert_eq!(e.source(), SOURCE);
    e.redo(&context()).unwrap();
    assert_eq!(e.source(), after);
}

#[test]
fn remove_and_move_property_keys_are_single_undoable_edits() {
    let mut e = editor();
    e.remove_property_key(&context(), b"p", 2).unwrap();
    assert_eq!(e.undo_count(), 1);
    roundtrip(e);
    let mut e = editor();
    e.move_property_key(&context(), b"p", 2, 3).unwrap();
    roundtrip(e);
}
#[test]
fn property_key_rejects_zero_missing_drawing_and_occupied_tick() {
    let mut e = editor();
    assert_eq!(
        e.remove_property_key(&context(), b"p", 0)
            .unwrap_err()
            .error,
        Error::InvalidValue
    );
    assert_eq!(
        e.move_property_key(&context(), b"p", 0, 3)
            .unwrap_err()
            .error,
        Error::InvalidValue
    );
    assert_eq!(
        e.move_property_key(&context(), b"p", 2, 0)
            .unwrap_err()
            .error,
        Error::Duplicate
    );
    assert_eq!(
        e.remove_property_key(&context(), b"d", 0)
            .unwrap_err()
            .error,
        Error::InvalidArgument
    );
    assert_eq!(
        e.remove_property_key(&context(), b"p", 4)
            .unwrap_err()
            .error,
        Error::InvalidReference
    );
    assert_eq!(e.undo_count(), 0);
}
#[test]
fn track_operations_roundtrip_and_preserve_other_track_bytes() {
    let raw_drawing=b"{\"id\":\"d\",\"kind\":\"drawing\",\"target\":\"node\",\"visible\":true,\"cels\":[{\"tick\":0,\"asset\":\"a\"}],\"name\":\"draw\"}";
    let mut e = editor();
    e.rename_track(&context(), b"p", b"renamed").unwrap();
    assert!(e
        .source()
        .windows(raw_drawing.len())
        .any(|w| w == raw_drawing));
    roundtrip(e);
    let mut e = editor();
    e.set_track_interpolation(&context(), b"p", b"cubic")
        .unwrap();
    roundtrip(e);
    let mut e = editor();
    e.duplicate_track(&context(), b"p", b"p2").unwrap();
    roundtrip(e);
    let mut e = editor();
    e.reorder_track(&context(), b"d", 0).unwrap();
    roundtrip(e);
    let mut e = editor();
    e.remove_track(&context(), b"d").unwrap();
    roundtrip(e);
}
#[test]
fn track_operations_reject_invalid_ids_kinds_indexes_and_limits() {
    let mut e = editor();
    assert_eq!(
        e.duplicate_track(&context(), b"p", b"bad/id")
            .unwrap_err()
            .error,
        Error::InvalidArgument
    );
    assert_eq!(
        e.duplicate_track(&context(), b"p", b"d").unwrap_err().error,
        Error::Duplicate
    );
    assert_eq!(
        e.set_track_interpolation(&context(), b"d", b"step")
            .unwrap_err()
            .error,
        Error::InvalidArgument
    );
    assert_eq!(
        e.set_track_interpolation(&context(), b"p", b"bezier")
            .unwrap_err()
            .error,
        Error::InvalidArgument
    );
    assert_eq!(
        e.reorder_track(&context(), b"p", 2).unwrap_err().error,
        Error::InvalidValue
    );
    assert_eq!(
        e.remove_track(&context(), b"missing").unwrap_err().error,
        Error::InvalidReference
    );
    let limits = EditLimits {
        commands: 1,
        bytes: SOURCE.len(),
    };
    let mut limited = TimelineEdit::new(SOURCE, &context(), limits).unwrap();
    assert_eq!(
        limited.remove_track(&context(), b"d").unwrap_err().error,
        Error::Limit
    );
    assert_eq!(limited.source(), SOURCE);
}
