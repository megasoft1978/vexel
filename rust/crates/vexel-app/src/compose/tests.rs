//! Pure compose tests: frame bytes, pointer mapping, hit testing, errors.
use super::*;
use vexel_render::{PreserveAspect, ViewBox};

pub(super) fn fixture_path(name: &str) -> String {
    format!(
        "{}/../../fixtures/v1/compose/{name}",
        env!("CARGO_MANIFEST_DIR")
    )
}

pub(super) fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(fixture_path(name)).unwrap_or_else(|e| panic!("fixture {name}: {e}"))
}

/// C `write_tile_ppm` conversion: straight RGBA composited over white.
pub(super) fn ppm_over_white(frame: &Frame) -> Vec<u8> {
    let mut ppm = format!("P6\n{} {}\n255\n", frame.width, frame.height).into_bytes();
    for p in frame.rgba.chunks_exact(4) {
        let a = u32::from(p[3]);
        ppm.extend((0..3).map(|c| (u32::from(p[c]) + 255 - a).min(255) as u8));
    }
    ppm
}

fn px(frame: &Frame, x: u32, y: u32) -> [u8; 4] {
    let o = (y as usize * frame.width as usize + x as usize) * 4;
    frame.rgba[o..o + 4].try_into().unwrap()
}

// ---- AC1: frame bytes -------------------------------------------------------

#[test]
fn compose_fixture_matches_frozen_ppm_and_requested_size() {
    let frame = compose_svg(&fixture("compose-v1.svg"), 160, 120).unwrap();
    assert_eq!((frame.width, frame.height), (160, 120));
    assert_eq!(frame.rgba.len(), 160 * 120 * 4);
    assert!(frame.disclosures.is_empty());
    assert_eq!(ppm_over_white(&frame), fixture("compose-v1.ppm"));
}

#[test]
fn compose_is_byte_identical_across_runs() {
    let svg = fixture("compose-v1.svg");
    let a = compose_svg(&svg, 160, 120).unwrap();
    let b = compose_svg(&svg, 160, 120).unwrap();
    assert_eq!(a, b);
}

#[test]
fn compose_fixture_pixels_match_independent_expectations() {
    let frame = compose_svg(&fixture("compose-v1.svg"), 160, 120).unwrap();
    // viewBox 0 0 80 60 into 160x120 is scale 2.
    assert_eq!(px(&frame, 28, 20), [0xe5, 0x39, 0x35, 255], "rect fill");
    assert_eq!(px(&frame, 80, 24), [0x43, 0xa0, 0x47, 255], "circle fill");
    assert_eq!(px(&frame, 128, 24), [0xfd, 0xd8, 0x35, 255], "ellipse fill");
    assert_eq!(
        px(&frame, 32, 72),
        [0, 0, 0, 0],
        "evenodd hole is transparent"
    );
    assert_eq!(
        px(&frame, 22, 62),
        [0x8e, 0x24, 0xaa, 255],
        "path fill ring"
    );
    assert_eq!(px(&frame, 136, 80), [0xfb, 0x8c, 0x00, 255], "polygon");
    assert_eq!(
        px(&frame, 80, 50),
        [0, 0, 0, 0],
        "background stays transparent"
    );
    assert_eq!(px(&frame, 8, 8), [0x1a, 0x23, 0x7e, 255], "rect stroke");
    let nested = (0..frame.height)
        .flat_map(|y| (0..frame.width).map(move |x| (x, y)))
        .filter(|&(x, y)| px(&frame, x, y)[..3] == [0x39, 0x49, 0xab] && y > 90)
        .count();
    assert!(nested > 20, "nested rotated group is drawn ({nested} px)");
}

#[test]
fn frame_size_equals_request_for_other_sizes_and_letterboxes() {
    let svg = fixture("compose-v1.svg");
    for (w, h) in [(1, 1), (100, 100), (160, 120), (333, 77), (400, 400)] {
        let frame = compose_svg(&svg, w, h).unwrap();
        assert_eq!((frame.width, frame.height), (w, h));
        assert_eq!(frame.rgba.len(), (w * h * 4) as usize);
    }
    // 100x100 meet: content 100x75 centred, so the top band is letterbox.
    let frame = compose_svg(&svg, 100, 100).unwrap();
    assert_eq!(px(&frame, 50, 5), [0, 0, 0, 0]);
}

// ---- AC3: pointer mapping ---------------------------------------------------

const VIEW: ViewBox = ViewBox {
    x: 10.0,
    y: 20.0,
    width: 200.0,
    height: 100.0,
};

fn aspects() -> [(&'static str, Option<PreserveAspect>); 3] {
    let base = PreserveAspect::MID_MEET;
    [
        ("meet", None),
        (
            "slice",
            Some(PreserveAspect {
                slice: true,
                ..base
            }),
        ),
        ("none", Some(PreserveAspect { none: true, ..base })),
    ]
}

#[test]
fn canvas_doc_canvas_round_trip_within_1e9_for_meet_slice_none() {
    for (name, aspect) in aspects() {
        for (w, h) in [(300, 300), (120, 400), (640, 360), (97, 31)] {
            let map = PointMap::new(VIEW, w, h, aspect).unwrap();
            let mut inside = 0;
            for iy in 0..=20 {
                for ix in 0..=20 {
                    let (cx, cy) = (
                        f64::from(w) * f64::from(ix) / 20.5,
                        f64::from(h) * f64::from(iy) / 20.5,
                    );
                    let Some((dx, dy)) = map.canvas_to_doc(cx, cy) else {
                        continue;
                    };
                    inside += 1;
                    let (bx, by) = map.doc_to_canvas(dx, dy);
                    assert!(
                        (bx - cx).abs() < 1e-9 && (by - cy).abs() < 1e-9,
                        "{name} {w}x{h}"
                    );
                }
            }
            assert!(inside > 0, "{name} {w}x{h} has mapped points");
        }
    }
}

#[test]
fn points_outside_the_viewport_report_none() {
    let meet = PointMap::new(VIEW, 300, 300, None).unwrap();
    // 200x100 into 300x300 meet: scale 1.5, content 300x150, band 75..225.
    assert!(meet.canvas_to_doc(150.0, 74.9).is_none());
    assert!(meet.canvas_to_doc(150.0, 225.0).is_none());
    assert!(meet.canvas_to_doc(150.0, 75.0).is_some());
    assert!(meet.canvas_to_doc(150.0, 224.9).is_some());
    for map in aspects().map(|(_, a)| PointMap::new(VIEW, 300, 300, a).unwrap()) {
        assert!(map.canvas_to_doc(-0.1, 150.0).is_none());
        assert!(map.canvas_to_doc(150.0, -0.1).is_none());
        assert!(map.canvas_to_doc(300.0, 150.0).is_none());
        assert!(map.canvas_to_doc(150.0, 300.0).is_none());
        assert!(map.canvas_to_doc(f64::NAN, 1.0).is_none());
        assert!(map.canvas_to_doc(1.0, f64::INFINITY).is_none());
    }
    let (_, none) = aspects()[2];
    let none = PointMap::new(VIEW, 300, 300, none).unwrap();
    assert!(none.canvas_to_doc(0.0, 0.0).is_some());
    assert!(PointMap::new(ViewBox { width: 0.0, ..VIEW }, 10, 10, None).is_none());
    assert!(PointMap::new(VIEW, 0, 10, None).is_none());
}

#[test]
fn render_agrees_with_pointer_map() {
    // A 4x3 rect at doc (3,2) in viewBox 0 0 20 10, composed through the real
    // ViewMap; the pointer map must place it on the same pixels.
    for aspect in [
        "xMidYMid meet",
        "xMidYMid slice",
        "none",
        "xMinYMax meet",
        "xMaxYMin slice",
    ] {
        let svg = format!(
            "<svg xmlns='http://www.w3.org/2000/svg' width='20' height='10' viewBox='0 0 20 10' \
             preserveAspectRatio='{aspect}'><rect x='3' y='2' width='4' height='3' fill='#ff0000'/></svg>"
        );
        for (w, h) in [(100, 100), (60, 30), (80, 200)] {
            let composed = compose_document(svg.as_bytes(), w, h, &|| false).unwrap();
            let lit = |x: f64, y: f64| {
                let (xi, yi) = (x.floor(), y.floor());
                if xi < 0.0 || yi < 0.0 || xi >= f64::from(w) || yi >= f64::from(h) {
                    return None;
                }
                Some(px(&composed.frame, xi as u32, yi as u32)[3] == 255)
            };
            let map = composed.map;
            let (cx, cy) = map.doc_to_canvas(5.0, 3.5);
            if let Some(on) = lit(cx, cy) {
                assert!(on, "{aspect} {w}x{h}: centre of rect is painted");
            }
            let (x0, y0) = map.doc_to_canvas(3.0, 2.0);
            let (x1, y1) = map.doc_to_canvas(7.0, 5.0);
            assert_eq!(
                lit(x0 - 1.5, y0 - 1.5).filter(|on| *on),
                None,
                "{aspect} just outside"
            );
            assert_eq!(
                lit(x1 + 1.5, y1 + 1.5).filter(|on| *on),
                None,
                "{aspect} just outside"
            );
            if let Some(on) = lit(x0 + 1.5, y0 + 1.5) {
                assert!(on, "{aspect} {w}x{h}: just inside the corner is painted");
            }
        }
    }
}

#[test]
fn hit_test_picks_topmost_shape_at_known_points() {
    let svg = fixture("hit-overlap.svg");
    let composed = compose_document(&svg, 100, 50, &|| false).unwrap();
    let hit = |x: f64, y: f64| composed.hit_test(x, y);
    assert_eq!(hit(20.0, 15.0), Some(0), "red only");
    assert_eq!(hit(30.0, 30.0), Some(1), "overlap: later blue is on top");
    assert_eq!(hit(50.0, 30.0), Some(1), "blue only");
    assert_eq!(hit(85.0, 25.0), Some(2), "circle stroke ring");
    assert_eq!(hit(75.0, 25.0), None, "circle fill is none");
    assert_eq!(hit(6.0, 7.0), Some(3), "evenodd ring");
    assert_eq!(hit(10.0, 7.0), None, "evenodd hole");
    assert_eq!(hit(1.0, 1.0), None);
    assert_eq!(hit(-5.0, 10.0), None);
    assert_eq!(hit(100.0, 10.0), None);
    // Same shapes at 2x: canvas (60,60) is doc (30,30).
    let big = compose_document(&svg, 200, 100, &|| false).unwrap();
    assert_eq!(big.hit_test(60.0, 60.0), Some(1));
    assert_eq!(big.hit_test(40.0, 30.0), Some(0));
}

// ---- AC4: errors and disclosures -------------------------------------------

#[test]
fn invalid_utf8_and_malformed_xml_are_typed() {
    assert_eq!(
        compose_svg(b"<svg \xff/>", 10, 10),
        Err(ComposeError::InvalidUtf8 { valid_up_to: 5 })
    );
    let broken = b"<svg xmlns='http://www.w3.org/2000/svg' width='10' height='10'><rect></svg>";
    assert!(matches!(
        compose_svg(broken, 10, 10),
        Err(ComposeError::Malformed { .. })
    ));
    assert!(compose_svg(b"", 10, 10).is_err());
    assert!(compose_svg(b"not xml at all", 10, 10).is_err());
}

#[test]
fn size_limits_are_typed() {
    let svg = fixture("compose-v1.svg");
    assert_eq!(compose_svg(&svg, 0, 10), Err(ComposeError::InvalidSize));
    assert_eq!(compose_svg(&svg, 10, 0), Err(ComposeError::InvalidSize));
    assert_eq!(compose_svg(&svg, 5000, 10), Err(ComposeError::InvalidSize));
    let huge = vec![b' '; MAX_SVG_BYTES + 1];
    assert_eq!(compose_svg(&huge, 10, 10), Err(ComposeError::LimitExceeded));
}

// WO-RUST-SVG-WIRE-01 (deliberate behaviour change): the unsupported-{mask,filter,
// text}.svg fixtures used to be rejected; masks, filters and text now render, so
// these assertions describe the rendered result instead of the typed failure.
#[test]
fn filter_mask_and_text_fixtures_now_compose() {
    let mask = compose_svg(&fixture("unsupported-mask.svg"), 40, 40).unwrap();
    assert!(mask.disclosures.is_empty(), "{:?}", mask.disclosures);
    assert_eq!(
        px(&mask, 10, 10),
        [255, 0, 0, 255],
        "white mask keeps the rect"
    );
    assert_eq!(px(&mask, 35, 10), [0, 0, 0, 0]);

    let filter = compose_svg(&fixture("unsupported-filter.svg"), 40, 40).unwrap();
    assert!(filter.disclosures.is_empty(), "{:?}", filter.disclosures);
    assert_eq!(
        px(&filter, 15, 15),
        [255, 0, 0, 255],
        "blur keeps the interior"
    );
    let edge = px(&filter, 30, 15);
    assert!(
        edge[3] > 0 && edge[3] < 255,
        "blurred edge is partially transparent: {edge:?}"
    );

    let text = compose_svg(&fixture("unsupported-text.svg"), 60, 40).unwrap();
    assert!(text.disclosures.is_empty(), "{:?}", text.disclosures);
    assert_eq!(
        px(&text, 5, 5),
        [0, 255, 0, 255],
        "the plain rect is still drawn"
    );
    let glyph_pixels = (18..32)
        .flat_map(|y| (4..24).map(move |x| (x, y)))
        .filter(|&(x, y)| px(&text, x, y)[3] > 128)
        .count();
    assert!(glyph_pixels > 8, "text glyphs are drawn ({glyph_pixels})");
}

#[test]
fn unsupported_error_text_names_feature_and_location() {
    // `textPath` is still deferred; the typed error keeps feature and element.
    let svg = b"<svg xmlns='http://www.w3.org/2000/svg' width='40' height='40'>\
        <defs><path id='p' d='M0 20 L40 20'/></defs>\
        <text font-size='10'><textPath href='#p'>Hi</textPath></text></svg>";
    let error = compose_svg(svg, 40, 40).unwrap_err();
    let text = error.to_string();
    assert!(
        text.contains("unsupported in V1") && text.contains("text") && text.contains("textPath"),
        "{text}"
    );
}

#[test]
fn deferred_shapes_are_skipped_and_disclosed_not_dropped_silently() {
    let svg = fixture("deferred-paint.svg");
    let composed = compose_document(&svg, 60, 40, &|| false).unwrap();
    // The dashed line now renders (WO-RUST-V1-RENDER-DASH-01 is integrated); only
    // the gradient rect is still deferred.
    assert_eq!(composed.shapes.len(), 2, "solid rect and dashed line drawn");
    let features: Vec<_> = composed
        .frame
        .disclosures
        .iter()
        .map(|d| (d.feature, d.element.as_deref()))
        .collect();
    assert_eq!(features, [("gradient paint", Some("rect"))]);
    assert!(
        composed
            .frame
            .disclosures
            .iter()
            .all(|d| d.offset.is_some())
    );
    assert_eq!(px(&composed.frame, 8, 8), [0, 0xaa, 0, 255]);
    assert_eq!(
        px(&composed.frame, 26, 8),
        [0, 0, 0, 0],
        "gradient rect not drawn"
    );
    // dasharray "4 2" from x=2: on 2..6, off 6..8.
    assert_eq!(px(&composed.frame, 3, 29), [0, 0, 0, 255], "dash drawn");
    assert_eq!(px(&composed.frame, 6, 29), [0, 0, 0, 0], "gap left open");
    let text = status_text("deferred-paint.svg", &composed);
    assert!(
        text.contains("1 deferred")
            && text.contains("gradient paint")
            && !text.contains("dashed stroke"),
        "{text}"
    );
}

#[test]
fn cancellation_returns_cancelled_and_never_a_frame() {
    let svg = fixture("compose-v1.svg");
    assert_eq!(
        compose_document(&svg, 160, 120, &|| true).map(|_| ()),
        Err(ComposeError::Cancelled)
    );
    let calls = std::cell::Cell::new(0u32);
    let cancel_late = || {
        calls.set(calls.get() + 1);
        calls.get() > 200
    };
    let result = compose_document(&svg, 160, 120, &cancel_late);
    assert!(matches!(result, Err(ComposeError::Cancelled)) || result.is_ok());
}

#[test]
fn every_truncated_prefix_is_handled_without_panic() {
    let svg = fixture("compose-v1.svg");
    for end in 0..svg.len() {
        let _ = compose_svg(&svg[..end], 16, 12);
    }
    let junk: Vec<u8> = (0..4096u32)
        .map(|i| (i.wrapping_mul(2654435761) >> 24) as u8)
        .collect();
    let _ = compose_svg(&junk, 16, 12);
}
