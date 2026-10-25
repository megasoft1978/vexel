use vexel_timeline::noise::Noise;
use vexel_timeline::sample::{sample_properties, PropertyValue};
use vexel_timeline::{decode, Error, TimelineContext};

fn context() -> TimelineContext {
    TimelineContext { scene_ids: vec![b"node".to_vec()], assets: Vec::new() }
}
fn doc(track: &str) -> String {
    format!(
        r#"{{"schema":"vexel.animation","version":1,"ticks_per_second":24,"duration_ticks":200,"fps":{{"numerator":24,"denominator":1}},"loop":"once","onion":{{"before":0,"after":0,"opacity":0}},"tracks":[{track}]}}"#
    )
}
fn track(extra: &str, vt: &str, prop: &str, v0: &str, v1: &str) -> String {
    doc(&format!(
        r#"{{"id":"t","kind":"property","target":"node","visible":true,"property":"{prop}","value_type":"{vt}","interpolation":"linear",{extra}"keys":[{{"tick":0,"value":{v0}}},{{"tick":199,"value":{v1}}}]}}"#
    ))
}
fn scalar(src: &str, tick: u32) -> f64 {
    let view = decode(src.as_bytes(), &context()).unwrap();
    match sample_properties(&view, tick).unwrap()[0].value {
        PropertyValue::Scalar(v) => v,
        other => panic!("{other:?}"),
    }
}

const NOISE: &str = r#""noise":{"amplitude":5,"frequency":3,"seed":42,"octaves":2},"#;

#[test]
fn deterministic_bounded_and_seeded() {
    let plain = track("", "scalar", "x", "0", "0");
    let noisy = track(NOISE, "scalar", "x", "0", "0");
    let first: Vec<f64> = (0..200).map(|t| scalar(&noisy, t)).collect();
    let second: Vec<f64> = (0..200).map(|t| scalar(&noisy, t)).collect();
    assert_eq!(first, second, "same bits on every run");
    assert!(first.iter().all(|v| v.abs() <= 5.0 + 1e-9));
    assert!(first.iter().any(|v| v.abs() > 1.0), "actually moves");
    assert!((0..200).all(|t| scalar(&plain, t) == 0.0), "no noise member, no change");
    let other = track(&NOISE.replace("42", "43"), "scalar", "x", "0", "0");
    assert_ne!(first, (0..200).map(|t| scalar(&other, t)).collect::<Vec<_>>());
    // Smooth: adjacent ticks of a 3 Hz noise differ by far less than the amplitude range.
    let max_step = first.windows(2).map(|w| (w[1] - w[0]).abs()).fold(0.0, f64::max);
    assert!(max_step < 3.0, "{max_step}");
}

#[test]
fn golden_values_pin_the_algorithm() {
    let n = Noise { amplitude: 1.0, frequency: 2.0, seed: 7, octaves: 1 };
    let got: Vec<f64> = [0u32, 1, 5, 17, 100].iter().map(|t| n.unit(*t, 24, 0)).collect();
    let again: Vec<f64> = [0u32, 1, 5, 17, 100].iter().map(|t| n.unit(*t, 24, 0)).collect();
    assert_eq!(got, again);
    assert!(got.iter().all(|v| (-1.0..=1.0).contains(v)));
    // Lattice points: integer cell positions equal the hashed lattice value exactly.
    assert_eq!(n.unit(0, 24, 0), n.unit(0, 24, 0));
    assert_ne!(n.unit(0, 24, 0), n.unit(0, 24, 1), "channels decorrelate");
    let golden = [
        0.29119620844873007,
        0.2964422983981182,
        0.3914891045399737,
        0.12149424882165899,
        -0.5019777753040623,
    ];
    for (g, e) in got.iter().zip(golden) {
        assert!((g - e).abs() < 1e-12, "{g} vs {e}");
    }
}

#[test]
fn applies_to_trs_and_adds_to_keyed_value() {
    let src = track(
        NOISE,
        "trs",
        "transform",
        r#"{"tx":10,"ty":0,"rotation_deg":0,"sx":1,"sy":1,"pivot_x":0,"pivot_y":0}"#,
        r#"{"tx":10,"ty":0,"rotation_deg":0,"sx":1,"sy":1,"pivot_x":0,"pivot_y":0}"#,
    );
    let view = decode(src.as_bytes(), &context()).unwrap();
    let mut moved = false;
    for tick in 0..200 {
        match sample_properties(&view, tick).unwrap()[0].value {
            PropertyValue::Trs(t) => {
                assert!((t.tx - 10.0).abs() <= 5.0 + 1e-9 && t.ty.abs() <= 5.0 + 1e-9);
                assert_eq!((t.sx, t.sy), (1.0, 1.0));
                moved |= t.rotation_deg != 0.0;
            }
            other => panic!("{other:?}"),
        }
    }
    assert!(moved);
}

#[test]
fn rejects_bad_noise() {
    for bad in [
        r#""noise":{},"#,
        r#""noise":{"frequency":1},"#,
        r#""noise":{"amplitude":-1},"#,
        r#""noise":{"amplitude":1e9},"#,
        r#""noise":{"amplitude":1,"frequency":0},"#,
        r#""noise":{"amplitude":1,"frequency":1000},"#,
        r#""noise":{"amplitude":1,"seed":-1},"#,
        r#""noise":{"amplitude":1,"seed":1.5},"#,
        r#""noise":{"amplitude":1,"octaves":0},"#,
        r#""noise":{"amplitude":1,"octaves":5},"#,
        r#""noise":{"amplitude":1,"amplitude":2},"#,
        r#""noise":{"amplitude":1,"extra":1},"#,
        r#""noise":{"amplitude":"1"},"#,
        r#""noise":[1],"#,
        r#""noise":5,"#,
    ] {
        let src = track(bad, "scalar", "x", "0", "0");
        assert!(decode(src.as_bytes(), &context()).is_err(), "{bad}");
    }
    for (vt, prop, v) in [("boolean", "visibility", "true"), ("rgba", "fill", "[0,0,0,1]")] {
        let src = track(NOISE, vt, prop, v, v);
        let err = decode(src.as_bytes(), &context()).unwrap_err();
        assert_eq!(err.error, Error::InvalidValue, "{vt}");
    }
}

#[test]
fn camera_target_needs_no_scene_element_and_only_camera_properties() {
    let ok = |prop: &str, v: &str| {
        doc(&format!(
            r#"{{"id":"c","kind":"property","target":"@camera","visible":true,"property":"{prop}","value_type":"scalar","interpolation":"linear","keys":[{{"tick":0,"value":{v}}}]}}"#
        ))
    };
    for (p, v) in [("x", "10"), ("y", "-4"), ("zoom", "2"), ("rotate", "15")] {
        decode(ok(p, v).as_bytes(), &context()).unwrap();
    }
    for (p, v) in [("opacity", "1"), ("zoom", "0"), ("zoom", "1000"), ("rotate", "99999")] {
        assert!(decode(ok(p, v).as_bytes(), &context()).is_err(), "{p} {v}");
    }
    let plain_zoom = track("", "scalar", "zoom", "1", "1");
    assert!(decode(plain_zoom.as_bytes(), &context()).is_err(), "zoom is camera-only");
}
