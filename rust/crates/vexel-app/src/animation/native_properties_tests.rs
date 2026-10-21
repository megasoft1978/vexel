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

#[test]
fn writes_gradient_stop_rgba_and_scalar_properties() {
    let timeline = timeline(
        r##"{"id":"color","kind":"property","target":"stop","visible":true,"property":"stop-color","value_type":"rgba","interpolation":"linear","keys":[{"tick":0,"value":[1,0,0,0.25]}]},
            {"id":"offset","kind":"property","target":"stop","visible":true,"property":"offset","value_type":"scalar","interpolation":"linear","keys":[{"tick":0,"value":0.4}]}"##,
        &[b"stop"],
    );
    let result = apply_native_properties(br#"<svg><stop id="stop"/></svg>"#, &timeline, 0).unwrap();
    assert!(
        result
            .windows(b"stop-color=\"#ff0000\"".len())
            .any(|w| w == b"stop-color=\"#ff0000\"")
    );
    assert!(
        result
            .windows(b"stop-opacity=\"0.25\"".len())
            .any(|w| w == b"stop-opacity=\"0.25\"")
    );
    assert!(
        result
            .windows(b"offset=\"0.4\"".len())
            .any(|w| w == b"offset=\"0.4\"")
    );
}

#[test]
fn per_key_bezier_easing_is_used_by_native_render_sampling() {
    let source = br##"<svg xmlns="http://www.w3.org/2000/svg" width="1" height="1"><rect id="shape" width="1" height="1" fill="#0000ff"/></svg>"##;
    let ease = timeline(
        r##"{"id":"paint","kind":"property","target":"shape","visible":true,"property":"fill","value_type":"rgba","interpolation":"step","keys":[{"tick":0,"value":[0,0,0,1],"ease":{"bezier":[0,0,1,1]}},{"tick":12,"value":[1,0,0,1]}]}"##,
        &[b"shape"],
    );
    let linear = timeline(
        r##"{"id":"paint","kind":"property","target":"shape","visible":true,"property":"fill","value_type":"rgba","interpolation":"linear","keys":[{"tick":0,"value":[0,0,0,1]},{"tick":12,"value":[1,0,0,1]}]}"##,
        &[b"shape"],
    );
    let geometry = crate::animation::native_drawing::SvgGeometry {
        width: 1.0,
        height: 1.0,
        view_box: [0.0, 0.0, 1.0, 1.0],
    };
    let eased_frame =
        crate::animation::native_frame::render_native_frame(source, &ease, &[], 6, 1, 1, geometry);
    let linear_frame = crate::animation::native_frame::render_native_frame(
        source,
        &linear,
        &[],
        6,
        1,
        1,
        geometry,
    );
    assert_eq!(eased_frame.unwrap().rgba, linear_frame.unwrap().rgba);
}
