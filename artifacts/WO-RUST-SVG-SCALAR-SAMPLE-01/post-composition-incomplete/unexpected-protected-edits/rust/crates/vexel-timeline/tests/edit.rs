use std::fmt::Write as _;
use std::sync::Arc;
use vexel_timeline::{
    AssetRef, DEFAULT_HISTORY_BYTES, DEFAULT_HISTORY_COMMANDS, EditFailure, EditLimits, Error,
    LoopMode, MAX_HISTORY_BYTES, TimelineContext, TimelineEdit,
};

fn context() -> TimelineContext {
    TimelineContext {
        scene_ids: vec![b"layer".to_vec(), b"shape".to_vec()],
        assets: vec![
            AssetRef {
                name: b"cel0".to_vec(),
                media_type: b"image/svg+xml".to_vec(),
            },
            AssetRef {
                name: b"cel1".to_vec(),
                media_type: b"image/svg+xml".to_vec(),
            },
        ],
    }
}
fn initial() -> String {
    r#"{ "schema":"vexel.animation","version":1,"ticks_per_second":24,"duration_ticks":24,"fps":{"numerator":24,"denominator":1},"loop":"once","onion":{"before":0,"after":0,"opacity":1},"tracks" : [ ] }"#.to_owned()
}
fn fresh() -> TimelineEdit {
    let src = initial();
    TimelineEdit::new(src.as_bytes(), &context(), EditLimits::default()).unwrap()
}
fn has(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}
fn rejected_without_history_change(
    edit: &mut TimelineEdit,
    call: impl FnOnce(&mut TimelineEdit) -> Result<(), EditFailure>,
) {
    let source = edit.source().to_vec();
    let undo = edit.undo_count();
    let redo = edit.redo_count();
    assert_eq!(
        call(edit).unwrap_err(),
        EditFailure {
            error: Error::InvalidArgument,
            offset: 0
        }
    );
    assert_eq!(edit.source(), source);
    assert_eq!(edit.undo_count(), undo);
    assert_eq!(edit.redo_count(), redo);
}

#[test]
fn snapshots_copy_source_and_leases_survive_editor_drop_and_are_send_sync() {
    fn send_sync<T: Send + Sync>() {}
    send_sync::<vexel_timeline::TimelineSourceLease>();
    let mut input = initial().into_bytes();
    let ctx = context();
    let mut edit = TimelineEdit::new(&input, &ctx, EditLimits::default()).unwrap();
    input.fill(b'x');
    let lease = edit.source_lease();
    let expected = lease.bytes().to_vec();
    edit.set_loop(&ctx, LoopMode::Repeat).unwrap();
    drop(edit);
    assert_eq!(lease.bytes(), expected.as_slice());
    let moved = Arc::new(lease);
    let worker = std::thread::spawn(move || moved.bytes().to_vec());
    assert_eq!(worker.join().unwrap(), expected);
}

#[test]
fn leases_survive_history_eviction_without_copying_their_snapshot() {
    let ctx = context();
    let src = initial();
    let mut edit = TimelineEdit::new(
        src.as_bytes(),
        &ctx,
        EditLimits {
            commands: 2,
            bytes: 1_000_000,
        },
    )
    .unwrap();
    let lease = edit.source_lease();
    let pinned = lease.bytes().as_ptr();
    for mode in [
        LoopMode::PingPong,
        LoopMode::Repeat,
        LoopMode::Once,
        LoopMode::PingPong,
        LoopMode::Repeat,
    ] {
        edit.set_loop(&ctx, mode).unwrap();
    }
    assert_eq!(edit.undo_count(), 2);
    assert_eq!(lease.bytes().as_ptr(), pinned);
    assert_eq!(lease.bytes(), src.as_bytes());
    drop(edit);
    assert_eq!(lease.bytes(), src.as_bytes());
}

#[test]
fn drawing_track_cel_edits_keep_json_strings_and_baseline_rules() {
    let ctx = context();
    let mut e = fresh();
    e.add_drawing_track(&ctx, b"draw", b"layer", b"cel0", b"Sketch \"A\"")
        .unwrap();
    assert!(has(e.source(), br#""name":"Sketch \"A\""#));
    assert!(has(e.source(), b"\"tracks\" : ["));
    e.set_cel(&ctx, b"draw", 12, b"cel1", b"pose \"B\"\nline")
        .unwrap();
    assert!(has(e.source(), br#""label":"pose \"B\"\u000aline"#));
    e.set_cel(&ctx, b"draw", 12, b"cel0", b"").unwrap();
    assert!(!has(e.source(), b"label"));
    e.set_cel(&ctx, b"draw", 6, b"cel1", b"").unwrap();
    let view = vexel_timeline::decode(e.source(), &ctx).unwrap();
    assert_eq!(view.item_at(0, 1).unwrap().tick, 6);
    e.remove_cel(&ctx, b"draw", 6).unwrap();
    assert_eq!(
        vexel_timeline::decode(e.source(), &ctx)
            .unwrap()
            .track_at(0)
            .unwrap()
            .item_count,
        2
    );
    let before = e.source().to_vec();
    let error = e.remove_cel(&ctx, b"draw", 0).unwrap_err();
    assert_eq!(error.error, Error::Duplicate);
    assert_eq!(e.source(), before);
}

#[test]
fn splices_preserve_other_tracks_and_surrounding_source_bytes() {
    let source = r#"{"schema":"vexel.animation","version":1,"ticks_per_second":24,"duration_ticks":24,"fps":{"numerator":24,"denominator":1},"loop":"once","onion":{"before":0,"after":0,"opacity":1},"tracks" : [ { "id":"draw", "kind":"drawing", "target":"layer", "visible":true, "cels" : [ { "tick" : 0 , "asset" : "cel0" } ,  {"tick":12,"asset":"cel1","label":"old"}  ] }, { "id":"other", "kind":"drawing", "target":"layer", "visible":true, "cels":[{"tick":0,"asset":"cel0"}], "name":"keep" } ] }"#;
    let ctx = context();
    let mut e = TimelineEdit::new(source.as_bytes(), &ctx, EditLimits::default()).unwrap();
    e.set_cel(&ctx, b"draw", 12, b"cel0", b"replacement")
        .unwrap();
    assert!(has(
        e.source(),
        b"\"cels\" : [ { \"tick\" : 0 , \"asset\" : \"cel0\" } ,  "
    ));
    assert!(has(e.source(),b"  ] }, { \"id\":\"other\", \"kind\":\"drawing\", \"target\":\"layer\", \"visible\":true, \"cels\":[{\"tick\":0,\"asset\":\"cel0\"}], \"name\":\"keep\" } ] }"));
    e.set_cel(&ctx, b"draw", 6, b"cel1", b"").unwrap();
    e.remove_cel(&ctx, b"draw", 6).unwrap();
    assert!(has(e.source(), b"{ \"tick\" : 0 , \"asset\" : \"cel0\" }"));
    assert!(has(e.source(), b"\"label\":\"replacement\""));
    let invalid = [0xc0, 0x80];
    let before = e.source().to_vec();
    assert_eq!(
        e.set_cel(&ctx, b"draw", 6, b"cel1", &invalid)
            .unwrap_err()
            .error,
        Error::InvalidArgument
    );
    assert_eq!(e.source(), before);
}

#[test]
fn rejected_edits_preserve_source_cursor_and_redo() {
    let ctx = context();
    let mut e = fresh();
    e.add_drawing_track(&ctx, b"draw", b"layer", b"cel0", b"")
        .unwrap();
    e.set_cel(&ctx, b"draw", 12, b"cel1", b"").unwrap();
    e.undo(&ctx).unwrap();
    let before = e.source().to_vec();
    let redo = e.redo_count();
    let undo = e.undo_count();
    assert_eq!(
        e.add_drawing_track(&ctx, b"draw", b"layer", b"cel0", b"")
            .unwrap_err()
            .error,
        Error::Duplicate
    );
    assert_eq!(
        e.set_cel(&ctx, b"draw", 24, b"cel0", b"")
            .unwrap_err()
            .error,
        Error::InvalidValue
    );
    assert_eq!(
        e.remove_cel(&ctx, b"draw", 12).unwrap_err().error,
        Error::InvalidReference
    );
    assert_eq!(e.source(), before);
    assert_eq!(e.undo_count(), undo);
    assert_eq!(e.redo_count(), redo);
}

#[test]
fn undo_redo_and_branching_restore_exact_sources() {
    let ctx = context();
    let mut e = fresh();
    let initial = e.source().to_vec();
    e.add_drawing_track(&ctx, b"draw", b"layer", b"cel0", b"")
        .unwrap();
    let track = e.source().to_vec();
    e.set_onion(&ctx, 2, 4, 0.25).unwrap();
    let onion = e.source().to_vec();
    assert!(has(&onion, b"\"before\":2,\"after\":4,\"opacity\":0.25"));
    e.undo(&ctx).unwrap();
    assert_eq!(e.source(), track);
    e.redo(&ctx).unwrap();
    assert_eq!(e.source(), onion);
    e.undo(&ctx).unwrap();
    e.set_loop(&ctx, LoopMode::PingPong).unwrap();
    assert_eq!(e.redo_count(), 0);
    assert_ne!(e.source(), track);
    e.undo(&ctx).unwrap();
    assert_eq!(e.source(), track);
    e.undo(&ctx).unwrap();
    assert_eq!(e.source(), initial);
}

#[test]
fn history_limits_trim_and_reject_without_partial_commit() {
    assert_eq!(
        EditLimits::default(),
        EditLimits {
            commands: DEFAULT_HISTORY_COMMANDS,
            bytes: DEFAULT_HISTORY_BYTES
        }
    );
    assert!(
        TimelineEdit::new(
            initial().as_bytes(),
            &context(),
            EditLimits {
                commands: 0,
                bytes: 1000
            }
        )
        .is_err()
    );
    assert!(
        TimelineEdit::new(
            initial().as_bytes(),
            &context(),
            EditLimits {
                commands: 1,
                bytes: MAX_HISTORY_BYTES + 1
            }
        )
        .is_err()
    );
    let ctx = context();
    let src = initial();
    let mut e = TimelineEdit::new(
        src.as_bytes(),
        &ctx,
        EditLimits {
            commands: 1,
            bytes: 1_000_000,
        },
    )
    .unwrap();
    e.add_drawing_track(&ctx, b"draw", b"layer", b"cel0", b"")
        .unwrap();
    e.set_cel(&ctx, b"draw", 12, b"cel1", b"").unwrap();
    assert_eq!(e.undo_count(), 1);
    e.undo(&ctx).unwrap();
    assert_eq!(e.redo_count(), 1);
    e.redo(&ctx).unwrap();
    assert_eq!(e.undo_count(), 1);
    let mut tight = TimelineEdit::new(
        src.as_bytes(),
        &ctx,
        EditLimits {
            commands: 4,
            bytes: src.len(),
        },
    )
    .unwrap();
    let before = tight.source().to_vec();
    assert_eq!(
        tight
            .add_drawing_track(&ctx, b"draw", b"layer", b"cel0", b"")
            .unwrap_err()
            .error,
        Error::Limit
    );
    assert_eq!(tight.source(), before);
}

#[test]
fn original_byte_limited_history_fixture_trims_and_preserves_redo_on_rejection() {
    let ctx = context();
    let src = initial();
    let mut generous = TimelineEdit::new(src.as_bytes(), &ctx, EditLimits::default()).unwrap();
    generous
        .add_drawing_track(&ctx, b"draw", b"layer", b"cel0", b"")
        .unwrap();
    let track = generous.source().to_vec();
    generous.set_cel(&ctx, b"draw", 12, b"cel1", b"").unwrap();
    let cel = generous.source().to_vec();
    let limits = EditLimits {
        commands: DEFAULT_HISTORY_COMMANDS,
        bytes: track.len() + cel.len(),
    };
    let mut edit = TimelineEdit::new(src.as_bytes(), &ctx, limits).unwrap();
    edit.add_drawing_track(&ctx, b"draw", b"layer", b"cel0", b"")
        .unwrap();
    assert_eq!(edit.source(), track);
    edit.set_cel(&ctx, b"draw", 12, b"cel1", b"").unwrap();
    assert_eq!(edit.source(), cel);
    assert_eq!(edit.undo_count(), 1);
    edit.undo(&ctx).unwrap();
    assert_eq!(edit.source(), track);
    assert_eq!(edit.redo_count(), 1);
    let before = edit.source().to_vec();
    let label = vec![b'x'; 4096];
    assert_eq!(
        edit.set_cel(&ctx, b"draw", 6, b"cel1", &label)
            .unwrap_err()
            .error,
        Error::Limit
    );
    assert_eq!(edit.source(), before);
    assert_eq!(edit.undo_count(), 0);
    assert_eq!(edit.redo_count(), 1);
}

#[test]
fn editor_track_limit_is_exact_and_rejection_is_atomic() {
    let ctx = context();
    let mut tracks = String::new();
    for i in 0..256 {
        if i > 0 {
            tracks.push(',');
        }
        write!(&mut tracks,r#"{{"id":"t{i}","kind":"property","target":"shape","visible":true,"property":"opacity","value_type":"scalar","interpolation":"linear","keys":[{{"tick":0,"value":1}}]}}"#).unwrap();
    }
    let source = format!(
        r#"{{"schema":"vexel.animation","version":1,"ticks_per_second":24,"duration_ticks":24,"fps":{{"numerator":24,"denominator":1}},"loop":"once","onion":{{"before":0,"after":0,"opacity":1}},"tracks":[{tracks}]}}"#
    );
    let mut edit = TimelineEdit::new(source.as_bytes(), &ctx, EditLimits::default()).unwrap();
    let before = edit.source().to_vec();
    assert_eq!(
        edit.add_drawing_track(&ctx, b"extra", b"layer", b"cel0", b"")
            .unwrap_err()
            .error,
        Error::Limit
    );
    assert_eq!(edit.source(), before);
}

#[test]
fn property_track_keys_and_combined_authoring_preserve_raw_values() {
    let ctx = context();
    let mut e = fresh();
    e.add_property_track(
        &ctx, b"fade", b"shape", b"opacity", b"scalar", b"linear", b"1.00", b"Fade",
    )
    .unwrap();
    assert!(has(e.source(), b"\"value\":1.00"));
    e.set_property_key(&ctx, b"fade", 12, b"0.250").unwrap();
    assert!(has(e.source(), b"\"value\":0.250"));
    e.set_property_key(&ctx, b"fade", 12, b"0.5").unwrap();
    let view = vexel_timeline::decode(e.source(), &ctx).unwrap();
    assert_eq!(view.track_at(0).unwrap().item_count, 2);
    let mut one = fresh();
    one.add_property_track_key(
        &ctx, b"fade", b"shape", b"opacity", b"scalar", b"linear", b"1", 12, b"0.25", b"",
    )
    .unwrap();
    assert_eq!(one.undo_count(), 1);
    one.undo(&ctx).unwrap();
    assert!(one.source().ends_with(b"[ ] }"));
    one.redo(&ctx).unwrap();
}

#[test]
fn empty_required_property_arguments_match_c_errors_without_changing_history() {
    let ctx = context();
    let mut edit = fresh();
    edit.add_drawing_track(&ctx, b"draw", b"layer", b"cel0", b"")
        .unwrap();
    edit.set_cel(&ctx, b"draw", 12, b"cel1", b"").unwrap();
    edit.undo(&ctx).unwrap();
    // Empty property, value_type, and interpolation are invalid arguments for
    // both APIs, with the C API's offset zero and atomic-history behavior.
    rejected_without_history_change(&mut edit, |e| {
        e.add_property_track(
            &ctx, b"", b"shape", b"opacity", b"scalar", b"linear", b"1", b"",
        )
    });
    rejected_without_history_change(&mut edit, |e| {
        e.add_property_track(&ctx, b"p", b"", b"opacity", b"scalar", b"linear", b"1", b"")
    });
    rejected_without_history_change(&mut edit, |e| {
        e.add_property_track(&ctx, b"p", b"shape", b"", b"scalar", b"linear", b"1", b"")
    });
    rejected_without_history_change(&mut edit, |e| {
        e.add_property_track(&ctx, b"p", b"shape", b"opacity", b"", b"linear", b"1", b"")
    });
    rejected_without_history_change(&mut edit, |e| {
        e.add_property_track(&ctx, b"p", b"shape", b"opacity", b"scalar", b"", b"1", b"")
    });
    rejected_without_history_change(&mut edit, |e| {
        e.add_property_track(
            &ctx, b"p", b"shape", b"opacity", b"scalar", b"linear", b"", b"",
        )
    });
    rejected_without_history_change(&mut edit, |e| {
        e.add_property_track_key(
            &ctx, b"", b"shape", b"opacity", b"scalar", b"linear", b"1", 12, b"0", b"",
        )
    });
    rejected_without_history_change(&mut edit, |e| {
        e.add_property_track_key(
            &ctx, b"p", b"", b"opacity", b"scalar", b"linear", b"1", 12, b"0", b"",
        )
    });
    rejected_without_history_change(&mut edit, |e| {
        e.add_property_track_key(
            &ctx, b"p", b"shape", b"", b"scalar", b"linear", b"1", 12, b"0", b"",
        )
    });
    rejected_without_history_change(&mut edit, |e| {
        e.add_property_track_key(
            &ctx, b"p", b"shape", b"opacity", b"", b"linear", b"1", 12, b"0", b"",
        )
    });
    rejected_without_history_change(&mut edit, |e| {
        e.add_property_track_key(
            &ctx, b"p", b"shape", b"opacity", b"scalar", b"", b"1", 12, b"0", b"",
        )
    });
    rejected_without_history_change(&mut edit, |e| {
        e.add_property_track_key(
            &ctx, b"p", b"shape", b"opacity", b"scalar", b"linear", b"", 12, b"0", b"",
        )
    });
    rejected_without_history_change(&mut edit, |e| {
        e.add_property_track_key(
            &ctx, b"p", b"shape", b"opacity", b"scalar", b"linear", b"1", 12, b"", b"",
        )
    });
}

#[test]
fn item_moves_keep_payload_and_reject_baseline_or_collision() {
    let ctx = context();
    let mut e = fresh();
    e.add_drawing_track(&ctx, b"draw", b"layer", b"cel0", b"")
        .unwrap();
    e.set_cel(&ctx, b"draw", 6, b"cel1", b"special").unwrap();
    e.set_cel(&ctx, b"draw", 12, b"cel0", b"").unwrap();
    e.move_item(&ctx, b"draw", 6, 18).unwrap();
    let v = vexel_timeline::decode(e.source(), &ctx).unwrap();
    assert_eq!(v.item_at(0, 1).unwrap().tick, 12);
    assert_eq!(v.item_at(0, 2).unwrap().tick, 18);
    assert!(has(e.source(), b"special"));
    let before = e.source().to_vec();
    assert_eq!(
        e.move_item(&ctx, b"draw", 18, 12).unwrap_err().error,
        Error::Duplicate
    );
    assert_eq!(e.source(), before);
    assert_eq!(
        e.move_item(&ctx, b"draw", 0, 1).unwrap_err().error,
        Error::InvalidValue
    );
    assert_eq!(e.source(), before);
    e.move_item(&ctx, b"draw", 18, 18).unwrap();
    assert_eq!(e.source(), before);
}

#[test]
fn loop_onion_controls_are_source_preserving_locale_independent_and_undoable() {
    let ctx = context();
    let mut e = fresh();
    let before = e.source().to_vec();
    e.set_loop(&ctx, LoopMode::Once).unwrap();
    assert_eq!(e.source(), before);
    e.set_loop(&ctx, LoopMode::PingPong).unwrap();
    assert!(has(e.source(), b"\"loop\":\"ping_pong\""));
    assert!(has(e.source(), b"\"tracks\" : [ ]"));
    let loop_source = e.source().to_vec();
    e.set_onion(&ctx, 1, 3, 0.25).unwrap();
    assert!(has(
        e.source(),
        b"\"before\":1,\"after\":3,\"opacity\":0.25"
    ));
    e.undo(&ctx).unwrap();
    assert_eq!(e.source(), loop_source);
    e.redo(&ctx).unwrap();
    let current = e.source().to_vec();
    assert_eq!(
        e.set_onion(&ctx, 9, 0, 0.2).unwrap_err().error,
        Error::InvalidArgument
    );
    assert_eq!(
        e.set_onion(&ctx, 1, 1, f64::NAN).unwrap_err().error,
        Error::InvalidArgument
    );
    assert_eq!(e.source(), current);
}

#[test]
fn context_is_per_operation_and_failed_redo_keeps_cursor() {
    let ctx = context();
    let mut e = fresh();
    e.add_drawing_track(&ctx, b"draw", b"layer", b"cel0", b"")
        .unwrap();
    e.set_cel(&ctx, b"draw", 12, b"cel1", b"").unwrap();
    e.undo(&ctx).unwrap();
    let before = e.source().to_vec();
    let empty = TimelineContext::default();
    assert!(e.redo(&empty).is_err());
    assert_eq!(e.source(), before);
    assert_eq!(e.redo_count(), 1);
    e.redo(&ctx).unwrap();
}

#[test]
fn edit_errors_keep_failure_offsets_and_source() {
    let ctx = context();
    let mut e = fresh();
    let before = e.source().to_vec();
    let error: EditFailure = e.set_cel(&ctx, b"missing", 2, b"cel0", b"").unwrap_err();
    assert_eq!(error.error, Error::InvalidReference);
    assert_eq!(error.offset, 0);
    assert_eq!(e.source(), before);
    assert_eq!(
        e.add_drawing_track(&ctx, b"bad/id", b"layer", b"cel0", b"")
            .unwrap_err(),
        EditFailure {
            error: Error::InvalidValue,
            offset: 3
        }
    );
}
