use vexel_svg_animation::{
    Attribute, DiagnosticKind, MAX_DIAGNOSTICS, MAX_TRACKS, MAX_VALUES, ParseError, ValueKind,
    evaluate, evaluate_view, parse, parse_owned,
};
use vexel_xml::{XmlError, XmlLimits, tokenize};

fn decode(src: &[u8]) -> vexel_svg_animation::Document<'_> {
    let (tokens, r) = tokenize(src, XmlLimits::default());
    assert_eq!(r.diagnostic.category, XmlError::None);
    parse(src, &tokens[..r.token_count]).unwrap()
}

#[test]
fn owned_source_and_view_retain_tables_without_copying() {
    use std::sync::Arc;
    let bytes = fixture("spline-opacity.svg").into_bytes();
    let mut allocation = Vec::with_capacity(bytes.len() + 73);
    allocation.extend_from_slice(&bytes);
    let owner = Arc::new(allocation);
    let pointer = owner.as_ptr();
    let spare = owner.capacity() - owner.len();
    let (tokens, report) = tokenize(owner.as_slice(), XmlLimits::default());
    assert_eq!(report.diagnostic.category, XmlError::None);
    let owned = parse_owned(&owner, &tokens[..report.token_count]).unwrap();
    assert_eq!(Arc::strong_count(&owner), 2);
    drop(owner);
    assert_eq!(owned.source().as_ptr(), pointer);
    assert_eq!(owned.source(), bytes.as_slice());
    assert_eq!(
        owned.source().len() + spare,
        owned.source_owner().capacity()
    );
    let view = owned.view();
    assert_eq!(view.tracks.as_ptr(), owned.tracks().as_ptr());
    assert_eq!(view.tracks.len(), owned.tracks().len());
    assert_eq!(view.diagnostics.as_ptr(), owned.diagnostics().as_ptr());
    assert_eq!(view.source.as_ptr(), pointer);
    assert_eq!(evaluate_view(view, 0.5), evaluate_view(owned.view(), 0.5));
    let expected = owned.source_owner().capacity()
        + owned.track_capacity() * std::mem::size_of::<vexel_svg_animation::Track>()
        + owned.diagnostic_capacity() * std::mem::size_of::<vexel_svg_animation::Diagnostic>()
        + owned
            .tracks()
            .iter()
            .map(|track| {
                track.keys.capacity() * std::mem::size_of::<vexel_svg_animation::Keyframe>()
                    + track.motion_points.capacity()
                        * std::mem::size_of::<vexel_svg_animation::MotionPoint>()
                    + track.splines.capacity() * std::mem::size_of::<[f64; 4]>()
            })
            .sum::<usize>();
    assert_eq!(owned.storage_bytes(), Some(expected));
}

#[test]
fn owned_parse_failure_does_not_clone_source_owner_and_worker_move_is_coherent() {
    use std::sync::Arc;
    let source = Arc::new(fixture("motion-line.svg").into_bytes());
    let (tokens, report) = tokenize(source.as_slice(), XmlLimits::default());
    assert_eq!(report.diagnostic.category, XmlError::None);
    let owned = parse_owned(&source, &tokens[..report.token_count]).unwrap();
    assert_eq!(Arc::strong_count(&source), 2);
    let moved = std::thread::spawn(move || {
        let view = owned.view();
        let span = view.tracks[0].element;
        (
            span.slice(view.source).unwrap().to_vec(),
            evaluate_view(view, 1.0).unwrap(),
        )
    })
    .join()
    .unwrap();
    assert!(moved.0.starts_with(b"<animateMotion"));
    assert_eq!(moved.1.items[0].value.kind, ValueKind::Motion);
}

#[test]
fn owned_track_limit_error_keeps_source_count_and_evaluation_parity() {
    use std::sync::Arc;
    let mut content = String::from("<svg><g>");
    for _ in 0..=MAX_TRACKS {
        content.push_str("<animate attributeName='x' from='0' to='1' dur='1s'/>");
    }
    content.push_str("</g></svg>");
    let owner = Arc::new(content.into_bytes());
    let (tokens, report) = tokenize(owner.as_slice(), XmlLimits::default());
    let count = Arc::strong_count(&owner);
    assert!(matches!(
        parse_owned(&owner, &tokens[..report.token_count]),
        Err(ParseError::TrackLimit { .. })
    ));
    assert_eq!(Arc::strong_count(&owner), count);

    for fixture_name in [
        "loader-opacity.svg",
            "morph-compatible.svg",
        "spline-opacity.svg",
        "motion-line.svg",
        "repeat-wave.svg",
        "slide-x.svg",
    ] {
        let owner = Arc::new(fixture(fixture_name).into_bytes());
        let (tokens, report) = tokenize(owner.as_slice(), XmlLimits::default());
        let borrowed = parse(owner.as_slice(), &tokens[..report.token_count]).unwrap();
        let owned = parse_owned(&owner, &tokens[..report.token_count]).unwrap();
        for time in [0.0, 0.5, 1.0, 2.0, 3600.0, -1.0, f64::NAN] {
            assert_eq!(
                evaluate(&borrowed, time),
                evaluate_view(owned.view(), time),
                "{fixture_name} at {time}"
            );
        }
    }
}
fn fixture(name: &str) -> String {
    match name {
        "color-cycle.svg" => include_str!("../../../../tests/anim/fixtures/color-cycle.svg").into(),
        "discrete-opacity.svg" => {
            include_str!("../../../../tests/anim/fixtures/discrete-opacity.svg").into()
        }
        "group-translate.svg" => {
            include_str!("../../../../tests/anim/fixtures/group-translate.svg").into()
        }
        "loader-opacity.svg" => {
            include_str!("../../../../tests/anim/fixtures/loader-opacity.svg").into()
        }
        "morph-compatible.svg" => {
            include_str!("../../../../tests/anim/fixtures/morph-compatible.svg").into()
        }
        "motion-line.svg" => include_str!("../../../../tests/anim/fixtures/motion-line.svg").into(),
        "motion-mpath.svg" => {
            include_str!("../../../../tests/anim/fixtures/motion-mpath.svg").into()
        }
        "pulse-radius.svg" => {
            include_str!("../../../../tests/anim/fixtures/pulse-radius.svg").into()
        }
        "repeat-wave.svg" => include_str!("../../../../tests/anim/fixtures/repeat-wave.svg").into(),
        "set-fill.svg" => include_str!("../../../../tests/anim/fixtures/set-fill.svg").into(),
        "slide-x.svg" => include_str!("../../../../tests/anim/fixtures/slide-x.svg").into(),
        "spinner-rotation.svg" => {
            include_str!("../../../../tests/anim/fixtures/spinner-rotation.svg").into()
        }
        "spline-opacity.svg" => {
            include_str!("../../../../tests/anim/fixtures/spline-opacity.svg").into()
        }
        "stroke-dash.svg" => include_str!("../../../../tests/anim/fixtures/stroke-dash.svg").into(),
        "unsupported-click.svg" => {
            include_str!("../../../../tests/anim/fixtures/unsupported-click.svg").into()
        }
        _ => panic!("fixture"),
    }
}
fn cubic(t: f64, first: f64, second: f64) -> f64 {
    let inverse = 1.0 - t;
    3.0 * inverse * inverse * t * first + 3.0 * inverse * t * t * second + t * t * t
}
fn spline(progress: f64) -> f64 {
    let (mut low, mut high) = (0.0, 1.0);
    for _ in 0..60 {
        let parameter = (low + high) / 2.0;
        if cubic(parameter, 0.42, 0.58) < progress {
            low = parameter
        } else {
            high = parameter
        }
    }
    cubic((low + high) / 2.0, 0.0, 1.0)
}

#[test]
fn authored_fixtures_parse_and_both_motion_sources_evaluate() {
    for name in [
        "color-cycle.svg",
        "discrete-opacity.svg",
        "group-translate.svg",
        "loader-opacity.svg",
        "morph-compatible.svg",
        "motion-line.svg",
        "motion-mpath.svg",
        "pulse-radius.svg",
        "repeat-wave.svg",
        "set-fill.svg",
        "slide-x.svg",
        "spinner-rotation.svg",
        "spline-opacity.svg",
        "stroke-dash.svg",
        "unsupported-click.svg",
    ] {
        let s = fixture(name);
        let d = decode(s.as_bytes());
        if name == "unsupported-click.svg" {
            assert!(d.tracks.is_empty());
            assert!(
                d.diagnostics
                    .iter()
                    .any(|x| x.kind == DiagnosticKind::UnsupportedTiming)
            );
            continue;
        }
        assert_eq!(
            d.tracks.len(),
            if name == "repeat-wave.svg" { 2 } else { 1 },
            "{name}"
        );
        let at = if name.starts_with("motion-") {
            1.0
        } else if name == "set-fill.svg" {
            1.25
        } else {
            0.25
        };
        let o = evaluate(&d, at).unwrap();
        assert_eq!(o.items.len(), if name == "repeat-wave.svg" { 2 } else { 1 });
        assert!(o.items[0].active, "{name}");
        if name.starts_with("motion-") {
            assert_eq!(d.tracks[0].motion_points.len(), 2);
            assert_eq!(o.items[0].value.kind, ValueKind::Motion)
        }
        if name == "morph-compatible.svg" {
            assert_eq!(d.tracks[0].attribute, Attribute::D);
            assert_eq!(o.items[0].value.kind, ValueKind::Path);
        }
    }
}

#[test]
fn interpolation_repeat_fill_sync_and_transform_samples_match_contract() {
    let s = fixture("loader-opacity.svg");
    let d = decode(s.as_bytes());
    for (t, v) in [(0., 1.), (0.25, 0.6), (0.5, 0.2), (0.75, 0.6)] {
        let o = evaluate(&d, t).unwrap();
        assert!((o.items[0].value.number[0] - v).abs() < 1e-12)
    }
    for sample in 0..100 {
        let t = sample as f64 * 0.02;
        let progress = t % 1.0;
        let expected = if progress <= 0.5 {
            1.0 - 1.6 * progress
        } else {
            0.2 + 1.6 * (progress - 0.5)
        };
        let actual = evaluate(&d, t).unwrap().items[0].value.number[0];
        assert!((actual - expected).abs() < 1e-10, "t={t}");
    }
    let s = fixture("repeat-wave.svg");
    let d = decode(s.as_bytes());
    assert_eq!(d.tracks.len(), 2);
    let o = evaluate(&d, 4.0).unwrap();
    assert!(o.items[0].active);
    assert!(!o.items[1].active);
    let o = evaluate(&d, 5.).unwrap();
    assert!(o.items[0].active);
    assert!(o.items[1].active);
    assert!((o.items[1].value.number[0] - 0.4).abs() < 1e-12);
    assert_eq!(d.tracks[0].attribute, Attribute::X);
    assert_eq!(d.tracks[1].attribute, Attribute::Opacity);
    let s = fixture("group-translate.svg");
    let d = decode(s.as_bytes());
    let o = evaluate(&d, 1.).unwrap();
    assert_eq!(d.tracks[0].attribute, Attribute::Transform);
    assert_eq!(o.items[0].value.kind, ValueKind::Transform);
    assert!((o.items[0].value.number[0] - 16.).abs() < 1e-12);
    let s = fixture("spline-opacity.svg");
    let d = decode(s.as_bytes());
    let o = evaluate(&d, 0.5).unwrap();
    assert!((o.items[0].value.number[0] - 0.5).abs() < 1e-8);
    for sample in 1..50 {
        let t = sample as f64 / 50.0;
        let actual = evaluate(&d, t).unwrap().items[0].value.number[0];
        assert!((actual - spline(t)).abs() < 1e-8, "spline t={t}");
    }
    let discrete_source = fixture("discrete-opacity.svg");
    let discrete = decode(discrete_source.as_bytes());
    assert_eq!(
        evaluate(&discrete, 0.75).unwrap().items[0].value.number[0],
        0.2
    );
    let motion_source = fixture("motion-line.svg");
    let motion_doc = decode(motion_source.as_bytes());
    let motion = evaluate(&motion_doc, 1.0).unwrap().items[0].value;
    assert_eq!(motion.kind, ValueKind::Motion);
    assert!((motion.number[0] - 20.0).abs() < 1e-12);
    assert_eq!(motion.number[1], 0.0);
    assert_eq!(motion.number[2], 0.0);

    let color_source = fixture("color-cycle.svg");
    let color_doc = decode(color_source.as_bytes());
    let color = evaluate(&color_doc, 1.0).unwrap().items[0].value;
    assert_eq!(color.kind, ValueKind::Color);
    assert_eq!(color.rgba, [69, 123, 157, 255]);
}

#[test]
fn spans_borrow_original_source_and_invalid_eval_is_rejected() {
    let s = fixture("slide-x.svg");
    let d = decode(s.as_bytes());
    let track = &d.tracks[0];
    assert_eq!(track.id.length, 0);
    assert!(track.target.slice(d.source).unwrap().starts_with(b"<rect"));
    assert!(
        track
            .element
            .slice(d.source)
            .unwrap()
            .starts_with(b"<animate")
    );
    assert!(evaluate(&d, -1.).is_none());
    assert!(evaluate(&d, f64::NAN).is_none());
    assert!(evaluate(&d, 3601.).is_none())
}

#[test]
fn track_bound_and_sync_cycles_are_bounded_without_partial_results() {
    let mut s = String::from("<svg><g>");
    for _ in 0..=MAX_TRACKS {
        s.push_str("<animate attributeName='x' from='0' to='1' dur='1s'/>")
    }
    s.push_str("</g></svg>");
    let (tokens, r) = tokenize(s.as_bytes(), XmlLimits::default());
    assert_eq!(r.diagnostic.category, XmlError::None);
    assert!(matches!(
        parse(s.as_bytes(), &tokens[..r.token_count]),
        Err(ParseError::TrackLimit { .. })
    ));
    let s=b"<svg><g><animate id='a' attributeName='x' from='0' to='1' dur='1s' begin='b.end'/><animate id='b' attributeName='y' from='0' to='1' dur='1s' begin='a.end'/></g></svg>";
    let d = decode(s);
    let o = evaluate(&d, 2.).unwrap();
    assert_eq!(o.items.len(), 2);
    assert!(o.items.iter().all(|i| !i.active));
}

#[test]
fn value_and_diagnostic_limits_and_offsets_are_preserved() {
    let mut values = String::from("<svg><rect><animate attributeName='x' values='");
    for i in 0..MAX_VALUES + 1 {
        if i != 0 {
            values.push(';');
        }
        values.push('1');
    }
    values.push_str("' dur='1s'/></rect></svg>");
    let d = decode(values.as_bytes());
    assert!(d.tracks.is_empty());
    assert_eq!(d.diagnostics[0].kind, DiagnosticKind::InvalidTrack);

    let unsupported = fixture("unsupported-click.svg");
    let click = unsupported.find("click").unwrap();
    let d = decode(unsupported.as_bytes());
    assert_eq!(d.diagnostics[0].kind, DiagnosticKind::UnsupportedTiming);
    assert_eq!(d.diagnostics[0].offset, click);

    let mut css = String::from("<svg>");
    for _ in 0..MAX_DIAGNOSTICS + 4 {
        css.push_str("<style>animation: spin 1s</style>");
    }
    css.push_str("</svg>");
    let d = decode(css.as_bytes());
    assert_eq!(d.diagnostics.len(), MAX_DIAGNOSTICS);
    assert!(
        d.diagnostics
            .iter()
            .all(|item| item.kind == DiagnosticKind::CssAnimation)
    );
}

#[test]
fn path_and_motion_internal_bounds_are_kept() {
    let mut exactly_max_points = String::from("M0 0");
    for _ in 0..4095 {
        exactly_max_points.push_str(" L1 0");
    }
    let source = format!("<svg><g><animateMotion path='{exactly_max_points}' dur='1s'/></g></svg>");
    let document = decode(source.as_bytes());
    assert_eq!(document.tracks[0].motion_points.len(), 4096);

    exactly_max_points.push_str(" L1 0");
    let source = format!("<svg><g><animateMotion path='{exactly_max_points}' dur='1s'/></g></svg>");
    let document = decode(source.as_bytes());
    assert!(document.tracks.is_empty());
    assert_eq!(
        document.diagnostics[0].kind,
        DiagnosticKind::UnsupportedPath
    );

    let long_path = format!("M0 0{}", " ".repeat(65_536));
    let source = format!(
        "<svg><path d='{long_path}'><animate attributeName='d' values='{long_path};{long_path}' dur='1s'/></path></svg>"
    );
    let document = decode(source.as_bytes());
    assert!(document.tracks.is_empty());
    assert_eq!(
        document.diagnostics[0].kind,
        DiagnosticKind::UnsupportedPath
    );

    let motion_path = format!("M0 0{}", " ".repeat(65_536));
    let source = format!("<svg><g><animateMotion path='{motion_path}' dur='1s'/></g></svg>");
    let document = decode(source.as_bytes());
    assert!(document.tracks.is_empty());
    assert_eq!(
        document.diagnostics[0].kind,
        DiagnosticKind::UnsupportedPath
    );

    let too_many_commands = format!("M0 0{}", " L1 0".repeat(1024));
    let source = format!(
        "<svg><path d='{too_many_commands}'><animate attributeName='d' values='{too_many_commands};{too_many_commands}' dur='1s'/></path></svg>"
    );
    let document = decode(source.as_bytes());
    assert!(document.tracks.is_empty());
    assert_eq!(
        document.diagnostics[0].kind,
        DiagnosticKind::UnsupportedPath
    );

    let too_many_numbers = format!("M0 0{}", " 1 0".repeat(2048));
    let source = format!(
        "<svg><path d='{too_many_numbers}'><animate attributeName='d' values='{too_many_numbers};{too_many_numbers}' dur='1s'/></path></svg>"
    );
    let document = decode(source.as_bytes());
    assert!(document.tracks.is_empty());
    assert_eq!(
        document.diagnostics[0].kind,
        DiagnosticKind::UnsupportedPath
    );

    let source = b"<svg><g><animateMotion path='M0 0 L1000001 0' dur='1s'/></g></svg>";
    let document = decode(source);
    assert!(document.tracks.is_empty());
    assert_eq!(
        document.diagnostics[0].kind,
        DiagnosticKind::UnsupportedPath
    );
}

#[test]
fn invalid_base_value_reports_invalid_track_at_animation_name() {
    let source =
        b"<svg><rect x='bad'><animate attributeName='x' from='0' to='1' dur='1s'/></rect></svg>";
    let expected_offset = source
        .windows(b"animate".len())
        .position(|window| window == b"animate")
        .unwrap();
    let document = decode(source);
    assert!(document.tracks.is_empty());
    assert_eq!(document.diagnostics.len(), 1);
    assert_eq!(document.diagnostics[0].kind, DiagnosticKind::InvalidTrack);
    assert_eq!(document.diagnostics[0].offset, expected_offset);
}

#[test]
fn controlled_parse_evaluate_preserve_results_and_stop_during_real_work() {
    use std::sync::Arc;
    use vexel_base::{Stop, WorkControl, WorkKind};
    use vexel_svg_animation::{
        EvaluationError, evaluate_view_controlled, parse_controlled, parse_owned_controlled,
    };

    struct StopOn(WorkKind);
    impl WorkControl for StopOn {
        fn charge(&mut self, kind: WorkKind, _units: u64) -> Result<(), Stop> {
            if kind == self.0 {
                Err(Stop::Limit)
            } else {
                Ok(())
            }
        }
        fn checkpoint(&mut self) -> Result<(), Stop> {
            Ok(())
        }
    }

    struct StopAfter {
        kind: WorkKind,
        seen: u64,
        target: u64,
    }
    impl WorkControl for StopAfter {
        fn charge(&mut self, kind: WorkKind, _units: u64) -> Result<(), Stop> {
            if kind == self.kind {
                self.seen += 1;
                if self.seen == self.target {
                    return Err(Stop::Canceled);
                }
            }
            Ok(())
        }
        fn checkpoint(&mut self) -> Result<(), Stop> {
            Ok(())
        }
    }

    #[derive(Default)]
    struct Counts([u64; WorkKind::ALL.len()]);
    impl WorkControl for Counts {
        fn charge(&mut self, kind: WorkKind, units: u64) -> Result<(), Stop> {
            self.0[kind as usize] += units;
            Ok(())
        }
        fn checkpoint(&mut self) -> Result<(), Stop> {
            Ok(())
        }
    }

    let owner = Arc::new(fixture("spline-opacity.svg").into_bytes());
    let (tokens, report) = tokenize(owner.as_slice(), XmlLimits::default());
    assert_eq!(report.diagnostic.category, XmlError::None);
    let tokens = &tokens[..report.token_count];
    let normal = parse(owner.as_slice(), tokens).unwrap();
    let mut unlimited = StopOn(WorkKind::Media);
    let controlled = parse_controlled(owner.as_slice(), tokens, &mut unlimited).unwrap();
    assert_eq!(normal.tracks, controlled.tracks);
    assert_eq!(normal.diagnostics, controlled.diagnostics);
    let mut counts = Counts::default();
    let _ = parse_controlled(owner.as_slice(), tokens, &mut counts).unwrap();
    assert!(counts.0[WorkKind::SourceRead as usize] > 0);
    assert!(counts.0[WorkKind::TokenVisit as usize] > 0);

    let mut stop_source = StopOn(WorkKind::SourceRead);
    assert!(matches!(
        parse_controlled(owner.as_slice(), tokens, &mut stop_source),
        Err(vexel_base::ControlledError::Limit)
    ));
    let count = Arc::strong_count(&owner);
    let mut stop_owned = StopOn(WorkKind::TokenVisit);
    assert!(matches!(
        parse_owned_controlled(&owner, tokens, &mut stop_owned),
        Err(vexel_base::ControlledError::Limit)
    ));
    assert_eq!(Arc::strong_count(&owner), count);

    let expected = evaluate_view(normal.view(), 0.5).unwrap();
    let mut unlimited = StopOn(WorkKind::Media);
    assert_eq!(
        evaluate_view_controlled(normal.view(), 0.5, &mut unlimited),
        Ok(expected)
    );
    let mut stop_interpolation = StopOn(WorkKind::AnimationTransform);
    assert_eq!(
        evaluate_view_controlled(normal.view(), 0.5, &mut stop_interpolation),
        Err(EvaluationError::Limit)
    );
    let mut stop_dependency = StopAfter {
        kind: WorkKind::AnimationTransform,
        seen: 0,
        target: 1,
    };
    assert_eq!(
        evaluate_view_controlled(normal.view(), 0.5, &mut stop_dependency),
        Err(EvaluationError::Canceled)
    );
    let mut counts = Counts::default();
    let _ = evaluate_view_controlled(normal.view(), 0.5, &mut counts).unwrap();
    assert!(counts.0[WorkKind::AnimationTransform as usize] > 0);
    let mut stop_spline = StopAfter {
        kind: WorkKind::AnimationTransform,
        seen: 0,
        target: 5,
    };
    assert_eq!(
        evaluate_view_controlled(normal.view(), 0.5, &mut stop_spline),
        Err(EvaluationError::Canceled)
    );
    let mut unlimited = StopOn(WorkKind::Media);
    assert_eq!(
        evaluate_view_controlled(normal.view(), f64::NAN, &mut unlimited),
        Err(EvaluationError::InvalidTime)
    );
}

#[test]
fn controlled_motion_reference_stop_and_invalid_public_track_are_typed() {
    use vexel_base::{Stop, WorkControl, WorkKind};
    use vexel_svg_animation::{EvaluationError, evaluate_view_controlled, parse_controlled};
    struct StopReference;
    impl WorkControl for StopReference {
        fn charge(&mut self, kind: WorkKind, _units: u64) -> Result<(), Stop> {
            if kind == WorkKind::ReferenceProbe {
                Err(Stop::Overflow)
            } else {
                Ok(())
            }
        }
        fn checkpoint(&mut self) -> Result<(), Stop> {
            Ok(())
        }
    }
    let source = fixture("motion-mpath.svg");
    let (tokens, report) = tokenize(source.as_bytes(), XmlLimits::default());
    assert_eq!(report.diagnostic.category, XmlError::None);
    let mut stop = StopReference;
    assert!(matches!(
        parse_controlled(source.as_bytes(), &tokens[..report.token_count], &mut stop),
        Err(vexel_base::ControlledError::Overflow)
    ));

    let fixture_source = fixture("slide-x.svg");
    let mut document = decode(fixture_source.as_bytes());
    document.tracks[0].keys.clear();
    let mut unlimited = StopReference;
    assert_eq!(
        evaluate_view_controlled(document.view(), 0.5, &mut unlimited),
        Err(EvaluationError::InvalidTrack)
    );
    assert!(evaluate_view(document.view(), 0.5).is_none());
}

#[test]
fn controlled_engines_match_the_original_animation_fixture_matrix() {
    use vexel_base::{Stop, WorkControl, WorkKind};
    use vexel_svg_animation::{evaluate_view_controlled, parse_controlled};
    struct Unlimited;
    impl WorkControl for Unlimited {
        fn charge(&mut self, _kind: WorkKind, _units: u64) -> Result<(), Stop> {
            Ok(())
        }
        fn checkpoint(&mut self) -> Result<(), Stop> {
            Ok(())
        }
    }
    for name in [
        "color-cycle.svg",
        "discrete-opacity.svg",
        "group-translate.svg",
        "loader-opacity.svg",
        "morph-compatible.svg",
        "motion-line.svg",
        "motion-mpath.svg",
        "pulse-radius.svg",
        "repeat-wave.svg",
        "set-fill.svg",
        "slide-x.svg",
        "spinner-rotation.svg",
        "spline-opacity.svg",
        "stroke-dash.svg",
        "unsupported-click.svg",
    ] {
        let source = fixture(name);
        let (tokens, report) = tokenize(source.as_bytes(), XmlLimits::default());
        assert_eq!(report.diagnostic.category, XmlError::None, "{name}");
        let tokens = &tokens[..report.token_count];
        let original = parse(source.as_bytes(), tokens).unwrap();
        let mut control = Unlimited;
        let controlled = parse_controlled(source.as_bytes(), tokens, &mut control).unwrap();
        assert_eq!(controlled.tracks, original.tracks, "{name}");
        assert_eq!(controlled.diagnostics, original.diagnostics, "{name}");
        for time in [0.0, 0.25, 1.0, 2.0, 3600.0] {
            let mut control = Unlimited;
            assert_eq!(
                evaluate_view_controlled(controlled.view(), time, &mut control),
                evaluate_view(original.view(), time)
                    .ok_or(vexel_svg_animation::EvaluationError::InvalidTrack),
                "{name} at {time}"
            );
        }
    }
}

#[test]
fn duplicate_mpath_ids_reject_even_when_first_path_has_no_data() {
    let source = b"<svg><defs><path id='p'/><path id='p' d='M0 0 L10 0'/></defs><rect><animateMotion dur='1s'><mpath href='#p'/></animateMotion></rect></svg>";
    let expected_offset = source
        .windows(b"animateMotion".len())
        .position(|window| window == b"animateMotion")
        .unwrap();
    let document = decode(source);
    assert!(document.tracks.is_empty());
    assert_eq!(document.diagnostics.len(), 1);
    assert_eq!(
        document.diagnostics[0].kind,
        DiagnosticKind::UnsupportedPath
    );
    assert_eq!(document.diagnostics[0].offset, expected_offset);
    assert!(document.tracks.iter().all(|track| !track.supported));
}

#[test]
fn direct_end_and_additive_accumulation_keep_c_evaluation_semantics() {
    let source = b"<svg><rect x='10' y='20'><animate attributeName='x' values='2;6' dur='1s' repeatCount='3' end='2.5s' fill='freeze' additive='sum' accumulate='sum'/><animate attributeName='y' values='3;7' dur='1s' repeatCount='3' accumulate='sum'/><animate attributeName='opacity' from='0' to='1' dur='2s' end='1s'/></rect></svg>";
    let document = decode(source);
    assert_eq!(document.tracks.len(), 3);

    let additive = &document.tracks[0];
    assert!(additive.has_end);
    assert_eq!(additive.end_seconds, 2.5);
    assert!(additive.additive_sum);
    assert!(additive.accumulate);
    assert_eq!(additive.base_value.number[0], 10.0);

    let accumulated = &document.tracks[1];
    assert!(!accumulated.additive_sum);
    assert!(accumulated.accumulate);
    assert_eq!(accumulated.base_value.number[0], 20.0);

    let early = evaluate(&document, 0.5).unwrap();
    assert!(early.items[2].active);
    assert_eq!(early.items[2].value.number[0], 0.25);

    let sample = evaluate(&document, 1.5).unwrap();
    assert_eq!(sample.items[0].iteration, 1);
    assert_eq!(sample.items[0].value.number[0], 18.0);
    assert_eq!(sample.items[1].iteration, 1);
    assert_eq!(sample.items[1].value.number[0], 9.0);
    assert!(!sample.items[2].active);
}
