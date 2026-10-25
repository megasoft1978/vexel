use vexel_timeline::{
    AssetRef, BulkKey, BulkTrack, EditLimits, Error, TimelineContext, TimelineEdit, MAX_TRACKS,
};

fn context() -> TimelineContext {
    TimelineContext {
        scene_ids: vec![b"layer".to_vec(), b"shape".to_vec()],
        assets: vec![AssetRef { name: b"cel0".to_vec(), media_type: b"image/svg+xml".to_vec() }],
    }
}
fn initial() -> String {
    r#"{"schema":"vexel.animation","version":1,"ticks_per_second":24,"duration_ticks":24,"fps":{"numerator":24,"denominator":1},"loop":"once","onion":{"before":0,"after":0,"opacity":1},"tracks":[]}"#.to_owned()
}
fn fresh() -> TimelineEdit {
    TimelineEdit::new(initial().as_bytes(), &context(), EditLimits::default()).unwrap()
}
fn track<'a>(id: &'a [u8], keys: &'a [BulkKey<'a>]) -> BulkTrack<'a> {
    BulkTrack { id, target: b"shape", property: b"opacity", value_type: b"scalar", interpolation: b"linear", value: b"1", name: b"", keys }
}
fn key(tick: u32, value: &'static [u8]) -> BulkKey<'static> {
    BulkKey { tick, value, ease: None }
}

#[test]
fn bulk_matches_sequential_edits_byte_for_byte() {
    let ctx = context();
    // Out of order keys, a repeated tick (last wins) and a tick-zero override.
    let keys = [key(9, b"0.5"), key(3, b"0.25"), key(9, b"0.75"), key(0, b"0"),
        BulkKey { tick: 5, value: b"1", ease: Some(b"\"step\"") }];
    let mut bulk = fresh();
    let mut named = track(b"b", &keys[..2]);
    named.name = b"Blink \"x\"";
    bulk.add_property_tracks(&ctx, &[track(b"a", &keys), named]).unwrap();

    let mut seq = fresh();
    seq.add_property_track(&ctx, b"a", b"shape", b"opacity", b"scalar", b"linear", b"1", b"").unwrap();
    for k in &keys {
        seq.set_property_key_with_ease(&ctx, b"a", k.tick, k.value, k.ease).unwrap();
    }
    seq.add_property_track(&ctx, b"b", b"shape", b"opacity", b"scalar", b"linear", b"1", b"Blink \"x\"").unwrap();
    for k in &keys[..2] {
        seq.set_property_key_with_ease(&ctx, b"b", k.tick, k.value, k.ease).unwrap();
    }
    assert_eq!(bulk.source(), seq.source());
    assert_eq!(bulk.undo_count(), 1);
}

#[test]
fn bulk_appends_after_existing_tracks() {
    let ctx = context();
    let mut edit = fresh();
    edit.add_property_track(&ctx, b"first", b"shape", b"opacity", b"scalar", b"linear", b"1", b"").unwrap();
    let keys = [key(4, b"0")];
    edit.add_property_tracks(&ctx, &[track(b"second", &keys)]).unwrap();
    let view = vexel_timeline::decode(edit.source(), &ctx).unwrap();
    assert_eq!(view.track_count(), 2);
}

#[test]
fn rejection_names_the_track_and_leaves_the_source_unchanged() {
    let ctx = context();
    let mut edit = fresh();
    let before = edit.source().to_vec();
    let late = [key(24, b"0")];
    let cases: [(Vec<BulkTrack>, Option<usize>, Error); 5] = [
        (vec![track(b"ok", &[]), track(b"bad id!", &[])], Some(1), Error::InvalidArgument),
        (vec![track(b"ok", &[]), BulkTrack { target: b"missing", ..track(b"x", &[]) }], Some(1), Error::InvalidReference),
        (vec![track(b"ok", &[]), track(b"dup", &[]), track(b"dup", &[])], Some(2), Error::Duplicate),
        (vec![track(b"ok", &[]), track(b"late", &late)], Some(1), Error::InvalidValue),
        (vec![track(b"ok", &[]), BulkTrack { value: b"\"text\"", ..track(b"str", &[]) }], Some(1), Error::InvalidValue),
    ];
    for (tracks, index, error) in cases {
        let failure = edit.add_property_tracks(&ctx, &tracks).unwrap_err();
        assert_eq!((failure.index, failure.failure.error), (index, error));
        assert_eq!(edit.source(), before.as_slice());
        assert_eq!(edit.undo_count(), 0);
    }
}

#[test]
fn rejects_empty_oversize_and_existing_duplicates() {
    let ctx = context();
    let mut edit = fresh();
    assert!(edit.add_property_tracks(&ctx, &[]).is_err());
    let ids: Vec<String> = (0..=MAX_TRACKS).map(|i| format!("t{i}")).collect();
    let many: Vec<BulkTrack> = ids.iter().map(|id| track(id.as_bytes(), &[])).collect();
    assert_eq!(edit.add_property_tracks(&ctx, &many).unwrap_err().failure.error, Error::InvalidArgument);
    edit.add_property_tracks(&ctx, &many[..MAX_TRACKS]).unwrap();
    assert_eq!(edit.add_property_tracks(&ctx, &many[..1]).unwrap_err().failure.error, Error::Limit);
    let mut other = fresh();
    other.add_property_tracks(&ctx, &[track(b"a", &[])]).unwrap();
    let failure = other.add_property_tracks(&ctx, &[track(b"b", &[]), track(b"a", &[])]).unwrap_err();
    assert_eq!((failure.index, failure.failure.error), (Some(1), Error::Duplicate));
}
