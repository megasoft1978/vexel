use vexel_timeline::sample::{ease_value, sample_properties, Ease, PropertyValue, Trs};
use vexel_timeline::{decode, Error, TimelineContext};

fn context() -> TimelineContext {
    TimelineContext {
        scene_ids: vec![b"node".to_vec()],
        assets: Vec::new(),
    }
}
fn doc(track: &str) -> String {
    format!(
        r#"{{"schema":"vexel.animation","version":1,"ticks_per_second":24,"duration_ticks":100,"fps":{{"numerator":24,"denominator":1}},"loop":"once","onion":{{"before":0,"after":0,"opacity":0}},"tracks":[{track}]}}"#
    )
}
fn scalar_track(interp: &str, keys: &str) -> String {
    doc(&format!(
        r#"{{"id":"t","kind":"property","target":"node","visible":true,"property":"x","value_type":"scalar","interpolation":"{interp}","keys":[{keys}]}}"#
    ))
}
fn sample_x(source: &str, tick: u32) -> f64 {
    let view = decode(source.as_bytes(), &context()).unwrap();
    match sample_properties(&view, tick).unwrap()[0].value {
        PropertyValue::Scalar(v) => v,
        other => panic!("{other:?}"),
    }
}

const PRESETS: [Ease; 5] = [
    Ease::Linear,
    Ease::EaseIn,
    Ease::EaseOut,
    Ease::EaseInOut,
    Ease::Bezier([0.42, 0.0, 0.58, 1.0]),
];

#[test]
fn presets_are_monotone_with_exact_endpoints() {
    for ease in PRESETS {
        assert_eq!(ease_value(ease, 0.0), 0.0);
        assert_eq!(ease_value(ease, 1.0), 1.0);
        let mut prev = 0.0;
        for i in 1..=1000 {
            let v = ease_value(ease, f64::from(i) / 1000.0);
            assert!(v >= prev - 1e-12, "{ease:?} at {i}: {v} < {prev}");
            prev = v;
        }
    }
    assert_eq!(ease_value(Ease::EaseInOut, 0.5), 0.5);
    assert!((ease_value(Ease::Bezier([0.42, 0.0, 0.58, 1.0]), 0.5) - 0.5).abs() < 1e-9);
    assert_eq!(ease_value(Ease::Step, 0.99), 0.0);
}

#[test]
fn back_eases_overshoot_and_bezier_endpoints_exact() {
    assert!(ease_value(Ease::EaseInBack, 0.2) < 0.0);
    assert!(ease_value(Ease::EaseOutBack, 0.8) > 1.0);
    for p in [
        [0.0, 0.0, 1.0, 1.0],
        [1.0, -4.0, 0.0, 4.0],
        [0.3, 2.0, 0.7, -1.5],
        [0.0, 0.0, 0.0, 0.0],
    ] {
        assert_eq!(ease_value(Ease::Bezier(p), 0.0), 0.0);
        assert_eq!(ease_value(Ease::Bezier(p), 1.0), 1.0);
        for i in 0..=100 {
            assert!(ease_value(Ease::Bezier(p), f64::from(i) / 100.0).is_finite());
        }
    }
    assert!((ease_value(Ease::Bezier([0.0, 0.0, 1.0, 1.0]), 0.37) - 0.37).abs() < 1e-9);
    assert_eq!(ease_value(Ease::Linear, f64::NAN), 0.0);
}

#[test]
fn key_ease_overrides_track_interpolation_only_where_present() {
    let old = scalar_track(
        "linear",
        r#"{"tick":0,"value":0},{"tick":10,"value":10},{"tick":20,"value":0}"#,
    );
    assert_eq!(sample_x(&old, 5), 5.0);
    let eased = scalar_track(
        "linear",
        r#"{"tick":0,"value":0,"ease":"ease_in"},{"tick":10,"value":10},{"tick":20,"value":0}"#,
    );
    assert_eq!(sample_x(&eased, 5), 2.5);
    assert_eq!(sample_x(&eased, 15), 5.0);
    let step = scalar_track(
        "linear",
        r#"{"tick":0,"value":0,"ease":"step"},{"tick":10,"value":10}"#,
    );
    assert_eq!(sample_x(&step, 9), 0.0);
    let bez = scalar_track(
        "step",
        r#"{"tick":0,"value":0,"ease":{"bezier":[0.0,0.0,1.0,1.0]}},{"tick":10,"value":10}"#,
    );
    assert!((sample_x(&bez, 5) - 5.0).abs() < 1e-6);
}

#[test]
fn ease_is_exposed_on_items() {
    let source = scalar_track(
        "linear",
        r#"{"tick":0,"value":0,"ease":{"bezier":[0.1,0.2,0.3,0.4]}},{"tick":10,"value":10}"#,
    );
    let view = decode(source.as_bytes(), &context()).unwrap();
    assert_eq!(
        view.item_at(0, 0).unwrap().ease,
        Some(Ease::Bezier([0.1, 0.2, 0.3, 0.4]))
    );
    assert_eq!(view.item_at(0, 1).unwrap().ease, None);
}

#[test]
fn schema_rejects_bad_ease() {
    for ease in [
        r#""wobble""#,
        "1",
        r#"{"bezier":[0.1,0,0.5]}"#,
        r#"{"bezier":[1.1,0,0.5,1]}"#,
        r#"{"bezier":[0.1,0,-0.5,1]}"#,
        r#"{"bezier":[0.1,4.5,0.5,1]}"#,
        r#"{"bezier":[0.1,0,0.5,-4.5]}"#,
        r#"{"curve":[0,0,1,1]}"#,
        r#"{"bezier":[0,0,1,1],"x":1}"#,
        r#"{"bezier":["a",0,1,1]}"#,
    ] {
        let source = scalar_track(
            "linear",
            &format!(r#"{{"tick":0,"value":0,"ease":{ease}}},{{"tick":10,"value":10}}"#),
        );
        assert_eq!(
            decode(source.as_bytes(), &context()).unwrap_err().error,
            Error::InvalidValue,
            "{ease}"
        );
    }
    let boolean = doc(
        r#"{"id":"t","kind":"property","target":"node","visible":true,"property":"visibility","value_type":"boolean","interpolation":"step","keys":[{"tick":0,"value":true,"ease":"linear"}]}"#,
    );
    assert!(decode(boolean.as_bytes(), &context()).is_err());
    let ok = doc(
        r#"{"id":"t","kind":"property","target":"node","visible":true,"property":"visibility","value_type":"boolean","interpolation":"step","keys":[{"tick":0,"value":true,"ease":"step"}]}"#,
    );
    assert!(decode(ok.as_bytes(), &context()).is_ok());
    let drawing = doc(
        r#"{"id":"d","kind":"drawing","target":"node","visible":true,"cels":[{"tick":0,"asset":"a","ease":"linear"}]}"#,
    );
    assert!(decode(drawing.as_bytes(), &context()).is_err());
}

fn trs_track(keys: &str) -> String {
    doc(&format!(
        r#"{{"id":"t","kind":"property","target":"node","visible":true,"property":"transform","value_type":"trs","interpolation":"linear","keys":[{keys}]}}"#
    ))
}
fn trs_value(rot: f64) -> String {
    format!(r#"{{"tx":0,"ty":0,"rotation_deg":{rot},"sx":2,"sy":3,"pivot_x":5,"pivot_y":5}}"#)
}

#[test]
fn trs_rotation_keeps_scale_constant() {
    let source = trs_track(&format!(
        r#"{{"tick":0,"value":{}}},{{"tick":10,"value":{}}}"#,
        trs_value(0.0),
        trs_value(360.0)
    ));
    let view = decode(source.as_bytes(), &context()).unwrap();
    for tick in 0..10 {
        let PropertyValue::Trs(t) = sample_properties(&view, tick).unwrap()[0].value else {
            panic!()
        };
        assert_eq!((t.sx, t.sy), (2.0, 3.0));
        assert!((t.rotation_deg - 36.0 * f64::from(tick)).abs() < 1e-9);
        let m = t.to_matrix();
        let (sx, sy) = (m[0].hypot(m[1]), m[2].hypot(m[3]));
        assert!(
            (sx - 2.0).abs() < 1e-9 && (sy - 3.0).abs() < 1e-9,
            "shear at {tick}"
        );
    }
}

#[test]
fn trs_matrix_pivot_and_skew() {
    let t = Trs {
        tx: 10.0,
        ty: 20.0,
        rotation_deg: 90.0,
        sx: 2.0,
        sy: 1.0,
        pivot_x: 1.0,
        pivot_y: 1.0,
        skew_x_deg: 0.0,
    };
    let m = t.to_matrix();
    let apply = |x: f64, y: f64| (m[0] * x + m[2] * y + m[4], m[1] * x + m[3] * y + m[5]);
    let (px, py) = apply(1.0, 1.0);
    assert!((px - 11.0).abs() < 1e-9 && (py - 21.0).abs() < 1e-9);
    let (x, y) = apply(2.0, 1.0);
    assert!((x - 11.0).abs() < 1e-9 && (y - 23.0).abs() < 1e-9);
    let skew = Trs {
        tx: 0.0,
        ty: 0.0,
        rotation_deg: 0.0,
        sx: 1.0,
        sy: 1.0,
        pivot_x: 0.0,
        pivot_y: 0.0,
        skew_x_deg: 45.0,
    };
    let m = skew.to_matrix();
    assert!((m[2] - 1.0).abs() < 1e-9 && m[0] == 1.0 && m[1] == 0.0);
}

#[test]
fn trs_skew_optional_and_schema_rejections() {
    let with_skew =
        r#"{"tx":0,"ty":0,"rotation_deg":0,"sx":1,"sy":1,"pivot_x":0,"pivot_y":0,"skew_x_deg":30}"#;
    let source = trs_track(&format!(
        r#"{{"tick":0,"value":{}}},{{"tick":10,"value":{with_skew}}}"#,
        trs_value(0.0)
    ));
    let view = decode(source.as_bytes(), &context()).unwrap();
    let PropertyValue::Trs(t) = sample_properties(&view, 5).unwrap()[0].value else {
        panic!()
    };
    assert!((t.skew_x_deg - 15.0).abs() < 1e-9);
    let zero_scale = r#"{"tx":0,"ty":0,"rotation_deg":0,"sx":0,"sy":0,"pivot_x":0,"pivot_y":0}"#;
    assert!(decode(
        trs_track(&format!(r#"{{"tick":0,"value":{zero_scale}}}"#)).as_bytes(),
        &context()
    )
    .is_ok());
    for bad in [
        r#"[1,0,0,1,0,0]"#,
        r#"{"tx":0,"ty":0,"rotation_deg":0,"sx":1,"sy":1,"pivot_x":0}"#,
        r#"{"tx":0,"ty":0,"rotation_deg":0,"sx":1,"sy":1,"pivot_x":0,"pivot_y":0,"extra":1}"#,
        r#"{"tx":"a","ty":0,"rotation_deg":0,"sx":1,"sy":1,"pivot_x":0,"pivot_y":0}"#,
        r#"{"tx":1e12,"ty":0,"rotation_deg":0,"sx":1,"sy":1,"pivot_x":0,"pivot_y":0}"#,
    ] {
        let s = trs_track(&format!(r#"{{"tick":0,"value":{bad}}}"#));
        assert_eq!(
            decode(s.as_bytes(), &context()).unwrap_err().error,
            Error::InvalidValue,
            "{bad}"
        );
    }
    let wrong = doc(
        r#"{"id":"t","kind":"property","target":"node","visible":true,"property":"x","value_type":"trs","interpolation":"linear","keys":[{"tick":0,"value":{"tx":0,"ty":0,"rotation_deg":0,"sx":1,"sy":1,"pivot_x":0,"pivot_y":0}}]}"#,
    );
    assert!(decode(wrong.as_bytes(), &context()).is_err());
}

#[test]
fn stop_properties_validate_and_sample() {
    let track = |prop: &str, ty: &str, v0: &str, v1: &str| {
        doc(&format!(
            r#"{{"id":"t","kind":"property","target":"node","visible":true,"property":"{prop}","value_type":"{ty}","interpolation":"linear","keys":[{{"tick":0,"value":{v0}}},{{"tick":10,"value":{v1}}}]}}"#
        ))
    };
    let s = track("stop-opacity", "scalar", "0", "1");
    assert_eq!(sample_x_of(&s, 5), PropertyValue::Scalar(0.5));
    let s = track("offset", "scalar", "0", "1");
    assert_eq!(sample_x_of(&s, 10 - 5), PropertyValue::Scalar(0.5));
    let s = track("stop-color", "rgba", "[0,0,0,1]", "[1,1,1,1]");
    assert_eq!(
        sample_x_of(&s, 5),
        PropertyValue::Rgba([0.5, 0.5, 0.5, 1.0])
    );
    let bad = track("stop-color", "scalar", "0", "1");
    assert!(decode(bad.as_bytes(), &context()).is_err());
}
fn sample_x_of(source: &str, tick: u32) -> PropertyValue {
    let view = decode(source.as_bytes(), &context()).unwrap();
    sample_properties(&view, tick).unwrap()[0].value
}
