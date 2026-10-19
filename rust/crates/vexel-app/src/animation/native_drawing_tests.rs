use super::native_drawing::*;

const GEOMETRY: SvgGeometry = SvgGeometry {
    width: 1.0,
    height: 1.0,
    view_box: [0.0, 0.0, 1.0, 1.0],
};
const RED: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" width="1" height="1" viewBox="0 0 1 1"><rect width="1" height="1" fill="#ff0000"/></svg>"##;
const BLUE: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" width="1" height="1" viewBox="0 0 1 1"><rect width="1" height="1" fill="#0000ff"/></svg>"##;

#[test]
fn active_cel_holds_until_next_boundary() {
    let cels = [
        DrawingCel {
            tick: 2,
            asset: b"a",
        },
        DrawingCel {
            tick: 5,
            asset: b"b",
        },
    ];
    assert_eq!(active_cel(&cels, 1).unwrap(), None);
    assert_eq!(active_cel(&cels, 2).unwrap().unwrap().asset, b"a");
    assert_eq!(active_cel(&cels, 4).unwrap().unwrap().asset, b"a");
    assert_eq!(active_cel(&cels, 5).unwrap().unwrap().asset, b"b");
}

#[test]
fn rejects_non_increasing_cels_and_hidden_tracks_are_skipped() {
    let duplicate = [
        DrawingCel {
            tick: 1,
            asset: b"a",
        },
        DrawingCel {
            tick: 1,
            asset: b"b",
        },
    ];
    assert_eq!(
        active_cel(&duplicate, 1),
        Err(DrawingError::InvalidTimeline)
    );
    let base = crate::compose::compose_svg(RED, 1, 1).unwrap();
    let track = DrawingTrack {
        visible: false,
        cels: &[DrawingCel {
            tick: 0,
            asset: b"missing",
        }],
    };
    let output = compose_drawing_tracks(&base, &[track], &[], 0, GEOMETRY).unwrap();
    assert_eq!(output, base);
}

#[test]
fn asset_must_match_scene_geometry_and_have_safe_local_references() {
    let wrong_geometry =
        br#"<svg xmlns="http://www.w3.org/2000/svg" width="2" height="1" viewBox="0 0 1 1"/>"#;
    assert_eq!(
        validate_drawing_svg(wrong_geometry, GEOMETRY),
        Err(DrawingError::GeometryMismatch)
    );
    let external = br#"<svg xmlns="http://www.w3.org/2000/svg" width="1" height="1" viewBox="0 0 1 1"><image href="https://example.invalid/a.svg"/></svg>"#;
    assert_eq!(
        validate_drawing_svg(external, GEOMETRY),
        Err(DrawingError::UnsafeSvg)
    );
    let dangling = br##"<svg xmlns="http://www.w3.org/2000/svg" width="1" height="1" viewBox="0 0 1 1"><use href="#missing"/></svg>"##;
    assert_eq!(
        validate_drawing_svg(dangling, GEOMETRY),
        Err(DrawingError::UnsafeSvg)
    );
    let script = br#"<svg xmlns="http://www.w3.org/2000/svg" width="1" height="1" viewBox="0 0 1 1"><script/></svg>"#;
    assert_eq!(
        validate_drawing_svg(script, GEOMETRY),
        Err(DrawingError::UnsafeSvg)
    );
}

#[test]
fn composes_layers_in_timeline_order() {
    let base = crate::compose::compose_svg(
        br#"<svg xmlns="http://www.w3.org/2000/svg" width="1" height="1" viewBox="0 0 1 1"/>"#,
        1,
        1,
    )
    .unwrap();
    let tracks = [
        DrawingTrack {
            visible: true,
            cels: &[DrawingCel {
                tick: 0,
                asset: b"red",
            }],
        },
        DrawingTrack {
            visible: true,
            cels: &[DrawingCel {
                tick: 0,
                asset: b"blue",
            }],
        },
    ];
    let assets = [
        DrawingAsset {
            name: b"red",
            media_type: SVG_MEDIA_TYPE,
            source: RED,
        },
        DrawingAsset {
            name: b"blue",
            media_type: SVG_MEDIA_TYPE,
            source: BLUE,
        },
    ];
    let frame = compose_drawing_tracks(&base, &tracks, &assets, 0, GEOMETRY).unwrap();
    assert_eq!(&frame.rgba[..4], &[0, 0, 255, 255]);
}

#[test]
fn enforces_checked_cumulative_pixel_cap() {
    assert_eq!(
        admit_frame_bytes(MAX_FRAME_PIXEL_BYTES - 4, 4),
        Ok(MAX_FRAME_PIXEL_BYTES)
    );
    assert_eq!(
        admit_frame_bytes(MAX_FRAME_PIXEL_BYTES, 1),
        Err(DrawingError::Limit)
    );
    assert_eq!(admit_frame_bytes(usize::MAX, 1), Err(DrawingError::Limit));
}
