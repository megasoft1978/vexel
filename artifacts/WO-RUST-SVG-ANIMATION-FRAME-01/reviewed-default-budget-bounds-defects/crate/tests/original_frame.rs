#[path = "support/mod.rs"]
mod common;
use common::{Control, edges, fixture, fixture_with_options};
use std::sync::Arc;
use vexel_scene::LexicalDocument;
use vexel_svg_animation::{
    DiagnosticKind as AnimationDiagnosticKind, evaluate_view_controlled, parse_owned,
};

#[test]
fn original_numeric_overlay_samples_repeat_without_mutating_owned_source() {
    let fixture = fixture(
        "<animate attributeName='x' to='5' dur='4s' fill='freeze'/>",
        "M1 1 L3 1 L3 5 L1 5 Z",
        false,
    );
    let original = fixture.source.as_slice().to_vec();
    let base_edges = fixture.graph.root().edges.to_vec();
    for (time, expected_x) in [(0.0, 1.0), (2.0, 3.0), (4.0, 5.0), (2.0, 3.0)] {
        let mut control = Control::new(u64::MAX);
        let frame =
            vexel_svg_animation_frame::evaluate_frame(fixture.graph.root(), time, &mut control)
                .unwrap();
        let actual = edges(&frame, fixture.shape);
        assert_eq!(actual[0].a.x, expected_x);
        assert_eq!(actual[1].a.x, expected_x + 2.0);
    }
    assert_eq!(fixture.graph.root().edges, base_edges);
    assert_eq!(fixture.graph.source(), original);
}

#[test]
fn original_color_opacity_and_discrete_assertions_reach_effective_shape_colors() {
    let color_fixture = fixture(
        "<animate attributeName='fill' values='#ff0000;#0000ff' dur='2s'/><animate attributeName='opacity' values='1;0' dur='2s'/>",
        "M1 1 L9 1 L9 9 L1 9 Z",
        false,
    );
    let mut control = Control::new(u64::MAX);
    let frame =
        vexel_svg_animation_frame::evaluate_frame(color_fixture.graph.root(), 1.0, &mut control)
            .unwrap();
    let rgba = frame.view().edge_colors[0];
    assert!((127..=128).contains(&rgba[0]));
    assert!((127..=128).contains(&rgba[2]));
    assert!((127..=128).contains(&rgba[3]));

    let discrete = fixture(
        "<animate attributeName='opacity' values='0.25;1' keyTimes='0;1' calcMode='discrete' dur='2s'/>",
        "M1 1 L9 1 L9 9 L1 9 Z",
        false,
    );
    let mut control = Control::new(u64::MAX);
    let frame = vexel_svg_animation_frame::evaluate_frame(discrete.graph.root(), 1.5, &mut control)
        .unwrap();
    assert_eq!(frame.view().edge_colors[0][3], 64);
}

#[test]
fn original_python_linear_reference_samples_are_preserved_by_animation_overlay() {
    let fixture = fixture(
        "<animate attributeName='opacity' values='1;0.2;1' keyTimes='0;0.5;1' dur='1s' repeatCount='indefinite'/>",
        "M1 1 L9 1 L9 9 L1 9 Z",
        false,
    );
    for sample in 0..100 {
        let seconds = sample as f64 * 0.02;
        let phase = seconds.rem_euclid(1.0);
        let expected = if phase <= 0.5 {
            1.0 + (0.2 - 1.0) * (phase / 0.5)
        } else {
            0.2 + (1.0 - 0.2) * ((phase - 0.5) / 0.5)
        };
        let mut control = Control::new(u64::MAX);
        let overlay =
            evaluate_view_controlled(fixture.graph.animation(), seconds, &mut control).unwrap();
        assert_eq!(overlay.items.len(), 1);
        assert!(overlay.items[0].active);
        assert!((overlay.items[0].value.number[0] - expected).abs() <= 1e-10);
    }
}

fn cubic(time: f64, first: f64, second: f64) -> f64 {
    let inverse = 1.0 - time;
    3.0 * inverse * inverse * time * first
        + 3.0 * inverse * time * time * second
        + time * time * time
}

fn spline(progress: f64, points: [f64; 4]) -> f64 {
    let (mut low, mut high) = (0.0, 1.0);
    for _ in 0..60 {
        let parameter = (low + high) * 0.5;
        if cubic(parameter, points[0], points[2]) < progress {
            low = parameter;
        } else {
            high = parameter;
        }
    }
    cubic((low + high) * 0.5, points[1], points[3])
}

#[test]
fn original_python_spline_discrete_and_syncbase_assertions_are_preserved() {
    let spline_fixture = fixture(
        "<animate attributeName='opacity' values='0;1' keyTimes='0;1' keySplines='0.42 0 0.58 1' calcMode='spline' dur='1s'/>",
        "M1 1 L9 1 L9 9 L1 9 Z",
        false,
    );
    for sample in 1..50 {
        let seconds = sample as f64 / 50.0;
        let expected = spline(seconds, [0.42, 0.0, 0.58, 1.0]);
        let mut control = Control::new(u64::MAX);
        let overlay =
            evaluate_view_controlled(spline_fixture.graph.animation(), seconds, &mut control)
                .unwrap();
        assert!((overlay.items[0].value.number[0] - expected).abs() <= 1e-8);
    }
    let discrete_fixture = fixture(
        "<animate attributeName='opacity' values='0.2;1' calcMode='discrete' dur='1s' repeatCount='indefinite'/>",
        "M1 1 L9 1 L9 9 L1 9 Z",
        false,
    );
    let mut control = Control::new(u64::MAX);
    let overlay =
        evaluate_view_controlled(discrete_fixture.graph.animation(), 0.75, &mut control).unwrap();
    assert_eq!(overlay.items[0].value.number[0], 0.2);

    let sync_fixture = fixture(
        "<animate id='move' attributeName='x' from='1' to='5' dur='2s' repeatCount='2' fill='freeze'/><animate attributeName='y' from='1' to='5' dur='1s' begin='move.end+1s' fill='freeze'/>",
        "M1 1 L3 1 L3 5 L1 5 Z",
        false,
    );
    let mut control = Control::new(u64::MAX);
    let overlay =
        evaluate_view_controlled(sync_fixture.graph.animation(), 5.0, &mut control).unwrap();
    assert_eq!(overlay.items.len(), 2);
    assert!(overlay.items.iter().all(|item| item.active));
    let mut control = Control::new(u64::MAX);
    let frame =
        vexel_svg_animation_frame::evaluate_frame(sync_fixture.graph.root(), 5.5, &mut control)
            .unwrap();
    let updated = edges(&frame, sync_fixture.shape);
    assert_eq!(updated[0].a.x, 5.0);
    assert_eq!(updated[0].a.y, 3.0);
}

#[test]
fn active_transform_overlay_updates_real_edges_and_keeps_repeated_frames_independent() {
    let fixture = fixture(
        "<animateTransform attributeName='transform' type='translate' from='0 0' to='4 0' dur='2s' fill='freeze'/>",
        "M1 1 L3 1 L3 5 L1 5 Z",
        false,
    );
    let base = fixture.graph.root();
    let before = base.edges.to_vec();
    for (time, expected) in [(1.0, 3.0), (0.0, 1.0), (1.0, 3.0)] {
        let mut control = Control::new(u64::MAX);
        let frame = vexel_svg_animation_frame::evaluate_frame(base, time, &mut control).unwrap();
        assert_eq!(edges(&frame, fixture.shape)[0].a.x, expected);
    }
    assert_eq!(base.edges, before);
}

#[test]
fn object_paint_bounds_stroke_inverse_and_motion_follow_effective_geometry() {
    let resized = fixture_with_options(
        "<animate attributeName='width' from='8' to='4' dur='2s' fill='freeze'/>",
        "M1 1 L9 1 L9 9 L1 9 Z",
        false,
        true,
    );
    let mut control = Control::new(u64::MAX);
    let frame =
        vexel_svg_animation_frame::evaluate_frame(resized.graph.root(), 1.0, &mut control).unwrap();
    assert_eq!(frame.view().fill_paints[0].bounds, [1.0, 1.0, 7.0, 9.0]);

    let moved = fixture_with_options(
        "<animateTransform attributeName='transform' type='translate' from='0 0' to='4 0' dur='2s' fill='freeze'/>",
        "M1 1 L3 1 L3 5 L1 5 Z",
        false,
        true,
    );
    let mut control = Control::new(u64::MAX);
    let frame =
        vexel_svg_animation_frame::evaluate_frame(moved.graph.root(), 1.0, &mut control).unwrap();
    assert_eq!(edges(&frame, moved.shape)[0].a.x, 3.0);
    assert_eq!(
        frame.view().shape(moved.shape).unwrap().stroke_inverse.e,
        -2.0
    );
    assert_eq!(frame.view().fill_paints[0].user_inverse.e, -2.0);

    let motion = fixture(
        "<animateMotion path='M0 0 L4 0' dur='2s' rotate='auto' fill='freeze'/>",
        "M1 1 L3 1 L3 5 L1 5 Z",
        false,
    );
    let mut control = Control::new(u64::MAX);
    let frame =
        vexel_svg_animation_frame::evaluate_frame(motion.graph.root(), 1.0, &mut control).unwrap();
    assert_eq!(edges(&frame, motion.shape)[0].a.x, 3.0);
}

#[test]
fn unsupported_negative_stroke_animation_fails_without_a_frame() {
    let stroke = fixture(
        "<animate attributeName='stroke-width' from='2' to='-2' dur='2s'/>",
        "M1 1 L3 1 L3 5 L1 5 Z",
        false,
    );
    let mut control = Control::new(u64::MAX);
    assert_eq!(
        vexel_svg_animation_frame::evaluate_frame(stroke.graph.root(), 1.5, &mut control)
            .unwrap_err()
            .kind,
        vexel_scene::ErrorKind::Unsupported
    );
}

#[test]
fn direct_motion_and_mpath_use_owned_motion_points_on_real_scene_geometry() {
    let mpath = fixture(
        "<defs><path id='route' d='M0 0 H4'/></defs><animateMotion dur='2s' fill='freeze'><mpath href='#route'/></animateMotion>",
        "M1 1 L3 1 L3 5 L1 5 Z",
        false,
    );
    assert_eq!(mpath.graph.animation().tracks[0].motion_points.len(), 2);
    let mut control = Control::new(u64::MAX);
    let frame =
        vexel_svg_animation_frame::evaluate_frame(mpath.graph.root(), 1.0, &mut control).unwrap();
    assert_eq!(edges(&frame, mpath.shape)[0].a.x, 3.0);

    let direct = fixture(
        "<animateMotion path='M0 0 L4 0' dur='2s' rotate='auto' fill='freeze'/>",
        "M1 1 L3 1 L3 5 L1 5 Z",
        false,
    );
    assert_eq!(direct.graph.animation().tracks[0].motion_points.len(), 2);
}

#[test]
fn original_unsupported_timing_css_and_timeline_limit_keep_diagnostics() {
    let source = Arc::new(
        b"<svg width='4' height='4'><rect width='4' height='4'><animate attributeName='opacity' from='0' to='1' dur='1s' begin='click'/></rect><style>@keyframes pulse { from { opacity: 0; } }</style></svg>".to_vec(),
    );
    let mut control = Control::new(u64::MAX);
    let lexical = LexicalDocument::parse(source.clone(), 1 << 20, &mut control).unwrap();
    let document = parse_owned(&source, lexical.tokens()).unwrap();
    assert!(document.tracks().is_empty());
    assert!(
        document
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.kind == AnimationDiagnosticKind::UnsupportedTiming)
    );
    assert!(
        document
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.kind == AnimationDiagnosticKind::CssAnimation)
    );

    let source = Arc::new(
        b"<svg><rect><animate attributeName='opacity' from='0' to='1' dur='3601s'/></rect></svg>"
            .to_vec(),
    );
    let mut control = Control::new(u64::MAX);
    let lexical = LexicalDocument::parse(source.clone(), 1 << 20, &mut control).unwrap();
    let document = parse_owned(&source, lexical.tokens()).unwrap();
    assert!(document.tracks().is_empty());
    assert!(!document.diagnostics().is_empty());
}

#[test]
fn original_fifteen_authored_fixtures_keep_their_source_token_animation_inputs() {
    let fixtures =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../tests/anim/fixtures");
    let mut paths: Vec<_> = std::fs::read_dir(fixtures)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "svg"))
        .collect();
    paths.sort();
    assert_eq!(paths.len(), 15);
    for path in paths {
        let bytes = std::fs::read(&path).unwrap();
        assert!(bytes.starts_with(b"<svg"));
        let source = Arc::new(bytes);
        let mut control = Control::new(u64::MAX);
        let lexical = LexicalDocument::parse(source.clone(), 1 << 20, &mut control).unwrap();
        let animation = parse_owned(&source, lexical.tokens()).unwrap();
        assert!(
            animation
                .diagnostics()
                .iter()
                .all(|diagnostic| diagnostic.offset <= source.len())
        );
        let overlay = evaluate_view_controlled(animation.view(), 1.0, &mut control).unwrap();
        assert_eq!(overlay.items.len(), animation.tracks().len());
        match path.file_name().and_then(|name| name.to_str()).unwrap() {
            "loader-opacity.svg" => {
                for sample in 0..100 {
                    let seconds = sample as f64 * 0.02;
                    let phase = seconds.rem_euclid(1.0);
                    let expected = if phase <= 0.5 {
                        1.0 + (0.2 - 1.0) * (phase / 0.5)
                    } else {
                        0.2 + (1.0 - 0.2) * ((phase - 0.5) / 0.5)
                    };
                    let mut control = Control::new(u64::MAX);
                    let overlay =
                        evaluate_view_controlled(animation.view(), seconds, &mut control).unwrap();
                    assert!(overlay.items[0].active);
                    assert!((overlay.items[0].value.number[0] - expected).abs() <= 1e-10);
                }
            }
            "spline-opacity.svg" => {
                for sample in 1..50 {
                    let seconds = sample as f64 / 50.0;
                    let expected = spline(seconds, [0.42, 0.0, 0.58, 1.0]);
                    let mut control = Control::new(u64::MAX);
                    let overlay =
                        evaluate_view_controlled(animation.view(), seconds, &mut control).unwrap();
                    assert!((overlay.items[0].value.number[0] - expected).abs() <= 1e-8);
                }
            }
            "discrete-opacity.svg" => {
                let mut control = Control::new(u64::MAX);
                let overlay =
                    evaluate_view_controlled(animation.view(), 0.75, &mut control).unwrap();
                assert_eq!(overlay.items[0].value.number[0], 0.2);
            }
            "repeat-wave.svg" => {
                let mut control = Control::new(u64::MAX);
                let overlay =
                    evaluate_view_controlled(animation.view(), 5.0, &mut control).unwrap();
                assert_eq!(
                    animation
                        .tracks()
                        .iter()
                        .map(|track| track.attribute.name())
                        .collect::<Vec<_>>(),
                    ["x", "opacity"]
                );
                assert!(overlay.items.iter().all(|item| item.active));
            }
            _ => {}
        }
    }
}
