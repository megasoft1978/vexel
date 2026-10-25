use crate::pathmorph::PathKind;
use crate::sample::{PropertyValue, resolve_path, sample_properties};
use crate::{Error, TimelineContext, decode};

fn context() -> TimelineContext {
    TimelineContext {
        scene_ids: vec![b"blur".to_vec(), b"node".to_vec()],
        assets: Vec::new(),
    }
}

fn doc(property: &str, kind: &str, interpolation: &str, keys: &str) -> Vec<u8> {
    format!(
        r#"{{"schema":"vexel.animation","version":1,"ticks_per_second":24,"duration_ticks":20,"fps":{{"numerator":24,"denominator":1}},"loop":"once","onion":{{"before":0,"after":0,"opacity":0}},"tracks":[{{"id":"t","kind":"property","target":"node","visible":true,"property":"{property}","value_type":"{kind}","interpolation":"{interpolation}","keys":{keys}}}]}}"#
    )
    .into_bytes()
}

fn text(source: &[u8], tick: u32) -> String {
    let view = decode(source, &context()).unwrap();
    let sample = &sample_properties(&view, tick).unwrap()[0];
    let PropertyValue::Path(blend) = sample.value else {
        panic!("not a path sample");
    };
    let kind = PathKind::of_property(sample.property).unwrap();
    String::from_utf8(resolve_path(source, kind, blend).unwrap()).unwrap()
}

#[test]
fn path_samples_are_verbatim_at_keys_and_blend_between() {
    let keys = r#"[{"tick":0,"value":"M0 0 L10 0 Z"},{"tick":10,"value":"M10 10 L30 0 Z"}]"#;
    let source = doc("d", "path", "linear", keys);
    assert_eq!(text(&source, 0), "M0 0 L10 0 Z");
    assert_eq!(text(&source, 10), "M10 10 L30 0 Z");
    assert_eq!(text(&source, 5), "M 5 5 L 20 0 Z");
    assert_eq!(text(&doc("d", "path", "step", keys), 5), "M0 0 L10 0 Z");
    let eased = r#"[{"tick":0,"value":"M0 0 L10 0 Z"},{"tick":10,"value":"M10 10 L30 0 Z","ease":"step"}]"#;
    assert_eq!(text(&doc("d", "path", "linear", eased), 5), "M 5 5 L 20 0 Z");
}

#[test]
fn structure_mismatch_and_wrong_kinds_are_invalid_values() {
    let bad = |property: &str, kind: &str, keys: &str| {
        decode(&doc(property, kind, "linear", keys), &context())
            .unwrap_err()
            .error
    };
    let base = r#"{"tick":0,"value":"M0 0 L10 0 Z"}"#;
    assert_eq!(bad("d", "path", &format!(r#"[{base},{{"tick":5,"value":"M0 0 L1 1 L2 2 Z"}}]"#)), Error::InvalidValue);
    assert_eq!(bad("d", "path", &format!(r#"[{base},{{"tick":5,"value":"M0 0 Q1 1 2 2 Z"}}]"#)), Error::InvalidValue);
    assert_eq!(bad("d", "path", &format!(r#"[{base},{{"tick":5,"value":"M0 0 A1 1 0 0 0 2 2 Z"}}]"#)), Error::InvalidValue);
    assert_eq!(bad("d", "path", &format!(r#"[{base},{{"tick":5,"value":7}}]"#)), Error::InvalidValue);
    assert_eq!(bad("d", "scalar", "[{\"tick\":0,\"value\":1}]"), Error::InvalidValue);
    assert_eq!(bad("opacity", "path", &format!("[{base}]")), Error::InvalidValue);
    assert!(decode(&doc("points", "path", "linear", r#"[{"tick":0,"value":"1,1 2,2"},{"tick":4,"value":"3,3 4,4"}]"#), &context()).is_ok());
    assert_eq!(bad("points", "path", r#"[{"tick":0,"value":"1,1 2,2"},{"tick":4,"value":"3,3"}]"#), Error::InvalidValue);
}

#[test]
fn filter_scalars_respect_renderer_limits() {
    let ok = |property: &str, value: &str| {
        let keys = format!(r#"[{{"tick":0,"value":{value}}}]"#);
        decode(&doc(property, "scalar", "linear", &keys), &context()).is_ok()
    };
    assert!(ok("stdDeviation", "32") && !ok("stdDeviation", "32.5") && !ok("stdDeviation", "-1"));
    assert!(ok("dx", "-4096") && !ok("dy", "4097"));
    assert!(ok("flood-opacity", "0.5") && !ok("flood-opacity", "1.5"));
    assert!(ok("values", "90"));
}
