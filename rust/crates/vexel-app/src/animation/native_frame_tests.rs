use super::*;
use crate::animation::native_drawing::SVG_MEDIA_TYPE;
use vexel_timeline::{TimelineContext, decode};

const GEOMETRY: SvgGeometry = SvgGeometry {
    width: 1.0,
    height: 1.0,
    view_box: [0.0, 0.0, 1.0, 1.0],
};
const SCENE: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" width="1" height="1" viewBox="0 0 1 1"><rect id="shape" width="1" height="1" fill="#0000ff"/></svg>"##;
const RED: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" width="1" height="1" viewBox="0 0 1 1"><rect width="1" height="1" fill="#ff0000"/></svg>"##;
const GREEN: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" width="1" height="1" viewBox="0 0 1 1"><rect width="1" height="1" fill="#00ff00"/></svg>"##;

fn timeline(tracks: &str, scene_ids: &[&[u8]], asset_names: &[&[u8]]) -> TimelineView<'static> {
    let bytes = format!(
        r#"{{"schema":"vexel.animation","version":1,"ticks_per_second":24,"duration_ticks":24,"fps":{{"numerator":24,"denominator":1}},"loop":"once","onion":{{"before":0,"after":0,"opacity":0}},"tracks":[{tracks}]}}"#
    )
    .into_bytes();
    let source: &'static [u8] = Box::leak(bytes.into_boxed_slice());
    let mut context = TimelineContext {
        scene_ids: scene_ids.iter().map(|id| id.to_vec()).collect(),
        assets: asset_names
            .iter()
            .map(|name| vexel_timeline::AssetRef {
                name: name.to_vec(),
                media_type: SVG_MEDIA_TYPE.to_vec(),
            })
            .collect(),
    };
    context.scene_ids.sort();
    context
        .assets
        .sort_by(|left, right| left.name.cmp(&right.name));
    decode(source, &context).expect("valid native timeline fixture")
}

#[test]
fn property_snapshot_is_rendered_first_and_source_stays_unchanged() {
    let source = SCENE.to_vec();
    let view = timeline(
        r#"{"id":"paint","kind":"property","target":"shape","visible":true,"property":"fill","value_type":"rgba","interpolation":"step","keys":[{"tick":0,"value":[1,0,0,1]}]}"#,
        &[b"shape"],
        &[],
    );

    let frame = render_native_frame(&source, &view, &[], 0, 1, 1, GEOMETRY).unwrap();

    assert_eq!(source, SCENE);
    assert_eq!(&frame.rgba[..4], &[255, 0, 0, 255]);
}

#[test]
fn drawing_tracks_composite_in_serialized_order_after_the_base() {
    let view = timeline(
        r#"{"id":"first","kind":"drawing","target":"shape","visible":true,"cels":[{"tick":0,"asset":"red"}]},
           {"id":"second","kind":"drawing","target":"shape","visible":true,"cels":[{"tick":0,"asset":"green"}]}"#,
        &[b"shape"],
        &[b"red", b"green"],
    );
    let assets = [
        DrawingAsset {
            name: b"red",
            media_type: SVG_MEDIA_TYPE,
            source: RED,
        },
        DrawingAsset {
            name: b"green",
            media_type: SVG_MEDIA_TYPE,
            source: GREEN,
        },
    ];

    let frame = render_native_frame(SCENE, &view, &assets, 0, 1, 1, GEOMETRY).unwrap();

    assert_eq!(&frame.rgba[..4], &[0, 255, 0, 255]);
}

#[test]
fn escaped_drawing_kind_and_asset_name_are_decoded_before_composition() {
    let view = timeline(
        r#"{"id":"draw","kind":"draw\u0069ng","target":"shape","visible":true,"cels":[{"tick":0,"asset":"r\u0065d"}]}"#,
        &[b"shape"],
        &[b"red"],
    );
    let assets = [DrawingAsset {
        name: b"red",
        media_type: SVG_MEDIA_TYPE,
        source: RED,
    }];

    let frame = render_native_frame(SCENE, &view, &assets, 0, 1, 1, GEOMETRY).unwrap();

    assert_eq!(&frame.rgba[..4], &[255, 0, 0, 255]);
}

#[test]
fn later_cels_change_at_their_authored_ticks_and_hold_afterward() {
    let view = timeline(
        r#"{"id":"draw","kind":"drawing","target":"shape","visible":true,"cels":[{"tick":0,"asset":"red"},{"tick":8,"asset":"green"},{"tick":16,"asset":"red"}]}"#,
        &[b"shape"],
        &[b"red", b"green"],
    );
    let assets = [
        DrawingAsset {
            name: b"red",
            media_type: SVG_MEDIA_TYPE,
            source: RED,
        },
        DrawingAsset {
            name: b"green",
            media_type: SVG_MEDIA_TYPE,
            source: GREEN,
        },
    ];

    for (tick, expected) in [
        (2, [255, 0, 0, 255]),
        (3, [255, 0, 0, 255]),
        (4, [255, 0, 0, 255]),
        (7, [255, 0, 0, 255]),
        (8, [0, 255, 0, 255]),
        (9, [0, 255, 0, 255]),
        (15, [0, 255, 0, 255]),
        (16, [255, 0, 0, 255]),
        (17, [255, 0, 0, 255]),
    ] {
        let frame = render_native_frame(SCENE, &view, &assets, tick, 1, 1, GEOMETRY).unwrap();
        assert_eq!(&frame.rgba[..4], &expected, "tick {tick}");
    }
}

#[test]
fn hidden_tracks_do_not_resolve_or_render_their_assets() {
    let view = timeline(
        r#"{"id":"hidden","kind":"drawing","target":"shape","visible":false,"cels":[{"tick":0,"asset":"missing"}]}"#,
        &[b"shape"],
        &[b"missing"],
    );

    let frame = render_native_frame(SCENE, &view, &[], 0, 1, 1, GEOMETRY).unwrap();

    assert_eq!(&frame.rgba[..4], &[0, 0, 255, 255]);
}

#[test]
fn reports_invalid_ticks_and_property_errors_with_typed_variants() {
    let empty = timeline("", &[], &[]);
    assert_eq!(
        render_native_frame(SCENE, &empty, &[], 24, 1, 1, GEOMETRY),
        Err(NativeFrameError::InvalidTick)
    );

    let missing = timeline(
        r#"{"id":"paint","kind":"property","target":"other","visible":true,"property":"opacity","value_type":"scalar","interpolation":"step","keys":[{"tick":0,"value":0.5}]}"#,
        &[b"other"],
        &[],
    );
    assert_eq!(
        render_native_frame(SCENE, &missing, &[], 0, 1, 1, GEOMETRY),
        Err(NativeFrameError::Property(
            NativePropertyError::MissingTarget
        ))
    );
}

#[test]
fn maps_drawing_and_base_render_failures_without_returning_a_frame() {
    let missing_asset = timeline(
        r#"{"id":"draw","kind":"drawing","target":"shape","visible":true,"cels":[{"tick":0,"asset":"missing"}]}"#,
        &[b"shape"],
        &[b"missing"],
    );
    assert_eq!(
        render_native_frame(SCENE, &missing_asset, &[], 0, 1, 1, GEOMETRY),
        Err(NativeFrameError::Drawing(DrawingError::MissingAsset))
    );

    let empty = timeline("", &[], &[]);
    assert_eq!(
        render_native_frame(SCENE, &empty, &[], 0, 0, 1, GEOMETRY),
        Err(NativeFrameError::Compose(ComposeError::InvalidSize))
    );
}
