use super::native_properties::{NativePropertyError, apply_native_properties};
use vexel_timeline::{TimelineContext, TimelineView, decode};

fn timeline(tracks: &str, scene_ids: &[&[u8]]) -> TimelineView<'static> {
    let bytes = format!(
        r#"{{"schema":"vexel.animation","version":1,"ticks_per_second":24,"duration_ticks":24,"fps":{{"numerator":24,"denominator":1}},"loop":"once","onion":{{"before":0,"after":0,"opacity":0}},"tracks":[{tracks}]}}"#
    )
    .into_bytes();
    let leaked: &'static [u8] = Box::leak(bytes.into_boxed_slice());
    let mut context = TimelineContext {
        scene_ids: scene_ids.iter().map(|id| id.to_vec()).collect(),
        assets: Vec::new(),
    };
    context.scene_ids.sort();
    decode(leaked, &context).expect("valid native timeline fixture")
}

#[test]
fn applies_ordered_rgba_and_scalar_writes_to_an_owned_snapshot() {
    let source = br#"<svg xmlns="http://www.w3.org/2000/svg"><style>.paint{fill:blue}</style><rect id="shape" class="paint" fill="black"/></svg>"#;
    let original = source.to_vec();
    let timeline = timeline(
        r#"{"id":"color","kind":"property","target":"shape","visible":true,"property":"fill","value_type":"rgba","interpolation":"linear","keys":[{"tick":0,"value":[1,0.5,0,0.25]}]},
           {"id":"alpha","kind":"property","target":"shape","visible":true,"property":"fill-opacity","value_type":"scalar","interpolation":"step","keys":[{"tick":0,"value":0.75}]}"#,
        &[b"shape"],
    );

    let snapshot = apply_native_properties(source, &timeline, 0).unwrap();

    assert_eq!(source.as_slice(), original.as_slice());
    assert!(
        snapshot
            .windows(b"fill=\"#ff8000\"".len())
            .any(|window| window == b"fill=\"#ff8000\"")
    );
    assert!(
        snapshot
            .windows(b"fill-opacity=\"0.75\"".len())
            .any(|window| window == b"fill-opacity=\"0.75\"")
    );
    assert!(
        snapshot
            .windows(b".paint{fill:blue}".len())
            .any(|w| w == b".paint{fill:blue}")
    );
}

#[test]
fn samples_smoothstep_and_writes_transform_and_visibility() {
    let source = br#"<svg><rect id="shape"/></svg>"#;
    let timeline = timeline(
        r#"{"id":"move","kind":"property","target":"shape","visible":true,"property":"x","value_type":"scalar","interpolation":"cubic","keys":[{"tick":0,"value":0},{"tick":10,"value":10}]},
           {"id":"matrix","kind":"property","target":"shape","visible":true,"property":"transform","value_type":"transform2d","interpolation":"step","keys":[{"tick":0,"value":[1,0,0,1,4,5]}]},
           {"id":"shown","kind":"property","target":"shape","visible":true,"property":"visibility","value_type":"boolean","interpolation":"step","keys":[{"tick":0,"value":false}]}"#,
        &[b"shape"],
    );

    let snapshot = apply_native_properties(source, &timeline, 5).unwrap();

    assert!(
        snapshot
            .windows(b"x=\"5\"".len())
            .any(|window| window == b"x=\"5\"")
    );
    assert!(
        snapshot
            .windows(b"transform=\"matrix(1 0 0 1 4 5)\"".len())
            .any(|window| window == b"transform=\"matrix(1 0 0 1 4 5)\"")
    );
    assert!(
        snapshot
            .windows(b"visibility=\"hidden\"".len())
            .any(|window| window == b"visibility=\"hidden\"")
    );
}

#[test]
fn rejects_missing_and_duplicate_persistent_ids() {
    let missing = timeline(
        r#"{"id":"opacity","kind":"property","target":"other","visible":true,"property":"opacity","value_type":"scalar","interpolation":"step","keys":[{"tick":0,"value":0.5}]}"#,
        &[b"other"],
    );
    assert_eq!(
        apply_native_properties(br#"<svg><rect id="shape"/></svg>"#, &missing, 0),
        Err(NativePropertyError::MissingTarget)
    );

    let duplicate = timeline(
        r#"{"id":"opacity","kind":"property","target":"shape","visible":true,"property":"opacity","value_type":"scalar","interpolation":"step","keys":[{"tick":0,"value":0.5}]}"#,
        &[b"shape"],
    );
    assert_eq!(
        apply_native_properties(
            br#"<svg><rect id="shape"/><circle id="shape"/></svg>"#,
            &duplicate,
            0,
        ),
        Err(NativePropertyError::DuplicateTarget)
    );
}

#[test]
fn skips_hidden_tracks_and_rejects_out_of_range_ticks() {
    let timeline = timeline(
        r#"{"id":"opacity","kind":"property","target":"shape","visible":false,"property":"opacity","value_type":"scalar","interpolation":"step","keys":[{"tick":0,"value":0.5}]}"#,
        &[b"shape"],
    );
    let source = br#"<svg><rect id="shape"/></svg>"#;
    assert_eq!(
        apply_native_properties(source, &timeline, 0)
            .unwrap()
            .as_slice(),
        source.as_slice()
    );
    assert_eq!(
        apply_native_properties(source, &timeline, timeline.duration_ticks()),
        Err(NativePropertyError::InvalidTick)
    );
}
