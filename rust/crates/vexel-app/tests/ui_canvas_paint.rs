//! WO-RUST-UI-CANVAS-01 acceptance 3 (artboard), 4 (checkerboard) and
//! 5 (overlays), measured on rendered pixels. All software-rasterised, so
//! every comparison is exact (tolerance 0) unless it measures coverage.
#[path = "ui_canvas_support.rs"]
mod support;

use support::{COMPOSE_V1, Scene, Shot, composed, hash, render, rgb, tokens};
use vexel_app::canvas::{
    CHECKER, HANDLE, View, artboard_device_rect, artboard_pixels, handle_points, handle_rect, over,
};
use vexel_app::compose::Frame;
use vexel_ui::Rect;

fn lum(p: [u8; 3]) -> u32 {
    u32::from(p[0]) + u32::from(p[1]) + u32::from(p[2])
}

fn scene<'a>(
    scale: u32,
    dark: bool,
    view: View,
    artboard: (f64, f64),
    frame: Option<&'a Frame>,
) -> Scene<'a> {
    Scene {
        window: (420, 320),
        stage: Rect {
            x: 10,
            y: 20,
            width: 400,
            height: 280,
        },
        scale,
        dark,
        view,
        artboard,
        frame,
        selection: None,
        hover: None,
    }
}

#[test]
fn artboard_with_well_shadow_hairline_and_document_pixels() {
    let natural = (160.0, 120.0);
    for dark in [true, false] {
        for scale in [1u32, 2] {
            let view = View::fit((400.0, 280.0), natural);
            let (aw, ah) = artboard_pixels(natural, view.zoom, scale);
            let doc = composed(COMPOSE_V1, (aw, ah));
            let s = scene(scale, dark, view, natural, Some(&doc.frame));
            let shot = render(&s);
            let t = tokens(dark);
            let well = rgb(vexel_app::canvas::well_fill(&t));
            let (ax, ay, w, h) = artboard_device_rect(view, natural, s.stage, scale);
            let sc = i64::from(scale);
            assert_eq!((w, h), (i64::from(aw), i64::from(ah)));

            // Well fills the stage, and only the stage (sentinel outside).
            let stage_x0 = 10 * sc;
            // Pin updated (POLISH-01): the stage has a 1 px `border` edge.
            assert_eq!(shot.at(stage_x0, 20 * sc), rgb(t.border), "stage edge");
            assert_eq!(
                shot.at(stage_x0 + sc, 20 * sc + sc),
                well,
                "inside the edge"
            );
            assert_eq!(
                shot.at(stage_x0 - 1, 20 * sc),
                [0x12, 0x34, 0x56],
                "outside the stage"
            );

            // Shadow ring: darker than the well within 14 px outside, none beyond.
            let mid_y = ay + h / 2;
            let mut last = 0;
            for out in [3i64, 2, 1] {
                let p = shot.at(ax - sc - out * sc, mid_y);
                assert!(
                    lum(p) < lum(well),
                    "{out}px left of the artboard: {p:?} vs {well:?}"
                );
                let depth = lum(well) - lum(p);
                assert!(depth >= last, "the shadow deepens towards the artboard");
                last = depth;
            }
            assert!(
                lum(shot.at(ax + w / 2, ay + h + sc + 4 * sc)) < lum(well),
                "drop below"
            );
            assert_eq!(shot.at(ax - 15 * sc, mid_y), well, "no shadow beyond 14 px");
            // Hairline: one logical pixel, the control edge colour, just outside.
            for k in 1..=sc {
                assert_eq!(
                    shot.at(ax - k, mid_y),
                    rgb(t.control_edge),
                    "hairline column {k}"
                );
            }
            // Document pixels over the checkerboard, exactly.
            let (a, b) = (rgb(t.checker_a), rgb(t.checker_b));
            let cell = CHECKER * sc;
            let mut expected = Vec::new();
            let mut actual = Vec::new();
            for y in 0..h {
                for x in 0..w {
                    let bg = if (x / cell + y / cell) % 2 == 0 { a } else { b };
                    let i = ((y * w + x) * 4) as usize;
                    expected.extend_from_slice(&over(&doc.frame.rgba[i..i + 4], bg));
                    actual.extend_from_slice(&shot.at(ax + x, ay + y));
                }
            }
            assert_eq!(hash(&expected), hash(&actual), "{dark} {scale}x");
            assert!(expected == actual);
            // The rendered document is not blank: it has non-checker colours.
            assert!(actual.chunks_exact(3).any(|p| p != a && p != b));
        }
    }
}

fn solid(width: u32, height: u32, rgba: [u8; 4]) -> Frame {
    Frame {
        width,
        height,
        rgba: rgba.repeat((width * height) as usize),
        disclosures: Vec::new(),
    }
}

#[test]
fn checkerboard_shows_through_transparency_only_anchored_and_clipped() {
    let svg = br##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="48"><rect x="48" y="32" width="8" height="8" fill="#ff0000"/></svg>"##;
    let opaque = br##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="48"><rect width="64" height="48" fill="#336699"/></svg>"##;
    for dark in [true, false] {
        let t = tokens(dark);
        for scale in [1u32, 2] {
            let natural = (64.0, 48.0);
            let view = View {
                zoom: 1.0,
                pan: (40.0, 30.0),
            };
            let (aw, ah) = artboard_pixels(natural, 1.0, scale);
            let doc = composed(svg, (aw, ah));
            let s = scene(scale, dark, view, natural, Some(&doc.frame));
            let shot = render(&s);
            let (ax, ay, w, h) = artboard_device_rect(view, natural, s.stage, scale);
            let cell = CHECKER * i64::from(scale);
            assert_eq!(cell, 8 * i64::from(scale), "8 device px at 1x, 16 at 2x");
            for cy in 0..(h / cell) {
                for cx in 0..(w / cell) {
                    // Skip the cells holding the red rectangle.
                    if cx >= 6 && cy >= 4 {
                        continue;
                    }
                    let want = rgb(if (cx + cy) % 2 == 0 {
                        t.checker_a
                    } else {
                        t.checker_b
                    });
                    for (ox, oy) in [
                        (0, 0),
                        (cell - 1, 0),
                        (0, cell - 1),
                        (cell - 1, cell - 1),
                        (cell / 2, cell / 2),
                    ] {
                        assert_eq!(
                            shot.at(ax + cx * cell + ox, ay + cy * cell + oy),
                            want,
                            "cell {cx},{cy}"
                        );
                    }
                }
            }
            // Clipped to the artboard: the pixel just outside is not a checker colour.
            for p in [
                shot.at(ax - i64::from(scale) - 1, ay + 3),
                shot.at(ax + w + i64::from(scale), ay + 3),
                shot.at(ax + 3, ay + h + i64::from(scale)),
            ] {
                assert_ne!(p, rgb(t.checker_a));
                assert_ne!(p, rgb(t.checker_b));
            }
            // The red square covers its cell: opaque pixels, no checker.
            assert_eq!(
                shot.at(ax + 52 * i64::from(scale), ay + 36 * i64::from(scale)),
                [255, 0, 0]
            );

            // An opaque document shows no checkerboard at all.
            let doc = composed(opaque, (aw, ah));
            let shot = render(&scene(scale, dark, view, natural, Some(&doc.frame)));
            for y in (0..h).step_by(3) {
                for x in (0..w).step_by(3) {
                    assert_eq!(
                        shot.at(ax + x, ay + y),
                        [0x33, 0x66, 0x99],
                        "opaque at {x},{y}"
                    );
                }
            }
        }
    }
}

#[test]
fn checkerboard_is_clipped_to_the_stage_when_the_artboard_overflows_it() {
    let natural = (64.0, 48.0);
    let view = View {
        zoom: 8.0,
        pan: (-100.0, -50.0),
    };
    let doc = solid(8, 6, [0, 0, 0, 0]);
    let s = scene(1, true, view, natural, Some(&doc));
    let shot = render(&s);
    let t = tokens(true);
    // Inside the stage everything is checker; outside it the sentinel survives.
    for (x, y) in [(11, 21), (408, 298), (200, 150)] {
        let p = shot.at(x, y);
        assert!(
            p == rgb(t.checker_a) || p == rgb(t.checker_b),
            "{x},{y}: {p:?}"
        );
    }
    for (x, y) in [(9, 100), (410, 100), (100, 19), (100, 300)] {
        assert_eq!(
            shot.at(x, y),
            [0x12, 0x34, 0x56],
            "{x},{y} outside the stage"
        );
    }
}

/// Opaque backdrop for coverage measurements: one solid-colour artboard.
const BACKDROP: [u8; 4] = [0x33, 0x66, 0x99, 255];

fn overlay_scene<'a>(scale: u32, view: View, frame: &'a Frame) -> Scene<'a> {
    Scene {
        window: (520, 420),
        stage: Rect {
            x: 0,
            y: 0,
            width: 520,
            height: 420,
        },
        scale,
        dark: true,
        view,
        artboard: (400.0, 300.0),
        frame: Some(frame),
        selection: None,
        hover: None,
    }
}

/// Sum of accent coverage along device row `y` between `x0..x1`, in logical
/// points, divided by `opacity`.
fn measured_width(shot: &Shot, y: i64, x0: i64, x1: i64, opacity: f64, accent: [u8; 3]) -> f64 {
    let bg = [BACKDROP[0], BACKDROP[1], BACKDROP[2]];
    let channel = (0..3)
        .max_by_key(|c| (i32::from(accent[*c]) - i32::from(bg[*c])).abs())
        .unwrap();
    let span = f64::from(i32::from(accent[channel]) - i32::from(bg[channel]));
    let sum: f64 = (x0..x1)
        .map(|x| (f64::from(shot.at(x, y)[channel]) - f64::from(bg[channel])) / span)
        .sum();
    sum / opacity / f64::from(shot.scale)
}

#[test]
fn outlines_keep_their_width_and_handles_their_size_at_any_zoom() {
    let frame = solid(4, 3, BACKDROP);
    let accent = rgb(tokens(true).accent);
    for scale in [1u32, 2] {
        for zoom in [0.25, 1.0, 8.0] {
            // Selection rect: document (50,40)-(350,260), left edge placed at x = 100.
            let view = View {
                zoom,
                pan: (100.0 - 50.0 * zoom, 60.0 - 40.0 * zoom),
            };
            let tl = view.doc_to_view((50.0, 40.0));
            let br = view.doc_to_view((350.0, 260.0));
            let rect = (tl.0, tl.1, br.0, br.1);
            let sc = i64::from(scale);

            let mut s = overlay_scene(scale, view, &frame);
            s.selection = Some(rect);
            let shot = render(&s);
            // A row a quarter down the rect, clear of the 8 pt handles.
            let row =
                ((rect.1 + (rect.3 - rect.1) * 0.25).clamp(5.0, 400.0) * f64::from(scale)) as i64;
            let edge = (rect.0 * f64::from(scale)) as i64;
            let w = measured_width(&shot, row, edge - 6 * sc, edge + 6 * sc, 1.0, accent);
            assert!((w - 1.5).abs() < 0.07, "selection {w} at {zoom} {scale}x");

            let mut s = overlay_scene(scale, view, &frame);
            s.hover = Some(rect);
            let shot = render(&s);
            let w = measured_width(&shot, row, edge - 6 * sc, edge + 6 * sc, 0.6, accent);
            assert!((w - 1.0).abs() < 0.07, "hover {w} at {zoom} {scale}x");

            // Handles: 8 logical points, white inside an accent edge, 8 of them.
            let mut s = overlay_scene(scale, view, &frame);
            s.selection = Some(rect);
            let shot = render(&s);
            for centre in handle_points((
                rect.0 * f64::from(scale),
                rect.1 * f64::from(scale),
                rect.2 * f64::from(scale),
                rect.3 * f64::from(scale),
            )) {
                let (x0, y0, x1, y1) = handle_rect(centre, sc);
                assert_eq!(
                    (x1 - x0, y1 - y0),
                    (HANDLE * sc, HANDLE * sc),
                    "handle size"
                );
                if x0 < 0 || y0 < 0 || x1 > 520 * sc || y1 > 420 * sc {
                    continue; // off the stage at this zoom
                }
                assert_eq!(shot.at(x0, y0), accent, "handle edge");
                assert_eq!(shot.at(x0 + sc, y0 + sc), [255, 255, 255], "handle fill");
                assert_eq!(shot.at(x1 - 1 - sc, y1 - 1 - sc), [255, 255, 255]);
                assert_eq!(shot.at(x1 - 1, y1 - 1), accent);
            }
            let white = |shot: &Shot| {
                shot.px
                    .chunks_exact(4)
                    .filter(|p| p[..3] == [255, 255, 255])
                    .count()
            };
            assert!(white(&shot) > 0);
            // No selection: no handles and no outline pixels at all.
            let shot = render(&overlay_scene(scale, view, &frame));
            assert_eq!(white(&shot), 0, "handles hidden when nothing is selected");
            assert_eq!(shot.at(edge, row), [0x33, 0x66, 0x99]);
        }
    }
}

#[test]
fn handles_are_eight_and_all_visible_for_a_mid_stage_selection() {
    let frame = solid(4, 3, BACKDROP);
    let view = View {
        zoom: 1.0,
        pan: (60.0, 40.0),
    };
    let mut s = overlay_scene(1, view, &frame);
    let rect = (100.0, 80.0, 260.0, 200.0);
    s.selection = Some(rect);
    let shot = render(&s);
    let accent = rgb(tokens(true).accent);
    let found = handle_points(rect)
        .iter()
        .filter(|c| {
            let (x0, y0, ..) = handle_rect(**c, 1);
            shot.at(x0 + 1, y0 + 1) == [255, 255, 255] && shot.at(x0, y0) == accent
        })
        .count();
    assert_eq!(found, 8);
}
