use crate::{
    TimelineContext, decode,
    sample::{PropertyValue, sample_properties},
};

fn context() -> TimelineContext {
    TimelineContext {
        scene_ids: vec![b"node".to_vec()],
        assets: Vec::new(),
    }
}

#[test]
fn samples_endpoints_and_interpolation_modes_in_serialized_track_order() {
    let source = br#"{"schema":"vexel.animation","version":1,"ticks_per_second":24,"duration_ticks":10,"fps":{"numerator":24,"denominator":1},"loop":"once","onion":{"before":0,"after":0,"opacity":0},"tracks":[{"id":"linear","kind":"property","target":"node","visible":true,"property":"x","value_type":"scalar","interpolation":"linear","keys":[{"tick":0,"value":2},{"tick":8,"value":10}]},{"id":"smooth","kind":"property","target":"node","visible":true,"property":"y","value_type":"scalar","interpolation":"cubic","keys":[{"tick":0,"value":0},{"tick":8,"value":8}]},{"id":"step","kind":"property","target":"node","visible":true,"property":"opacity","value_type":"scalar","interpolation":"step","keys":[{"tick":0,"value":1},{"tick":8,"value":0}]},{"id":"hidden","kind":"property","target":"node","visible":false,"property":"width","value_type":"scalar","interpolation":"linear","keys":[{"tick":0,"value":0},{"tick":8,"value":8}]}]}"#;
    let view = decode(source, &context()).unwrap();

    let before = sample_properties(&view, 0).unwrap();
    assert_eq!(before.len(), 3);
    assert_eq!(
        before
            .iter()
            .map(|sample| sample.property)
            .collect::<Vec<_>>(),
        vec![b"x".as_slice(), b"y", b"opacity"]
    );
    assert_eq!(before[0].value, PropertyValue::Scalar(2.0));
    assert_eq!(
        sample_properties(&view, 4).unwrap()[0].value,
        PropertyValue::Scalar(6.0)
    );
    assert_eq!(
        sample_properties(&view, 4).unwrap()[1].value,
        PropertyValue::Scalar(4.0)
    );
    assert_eq!(
        sample_properties(&view, 4).unwrap()[2].value,
        PropertyValue::Scalar(1.0)
    );
    let after = sample_properties(&view, 9).unwrap();
    assert_eq!(after[0].value, PropertyValue::Scalar(10.0));
    assert_eq!(after[1].value, PropertyValue::Scalar(8.0));
    assert_eq!(after[2].value, PropertyValue::Scalar(0.0));
}

#[test]
fn retains_typed_values_and_rejects_ticks_outside_the_timeline() {
    let source = br#"{"schema":"vexel.animation","version":1,"ticks_per_second":24,"duration_ticks":2,"fps":{"numerator":24,"denominator":1},"loop":"once","onion":{"before":0,"after":0,"opacity":0},"tracks":[{"id":"matrix","kind":"property","target":"node","visible":true,"property":"transform","value_type":"transform2d","interpolation":"linear","keys":[{"tick":0,"value":[1,2,3,4,5,6]}]},{"id":"color","kind":"property","target":"node","visible":true,"property":"fill","value_type":"rgba","interpolation":"linear","keys":[{"tick":0,"value":[0.1,0.2,0.3,0.4]}]},{"id":"flag","kind":"property","target":"node","visible":true,"property":"visibility","value_type":"boolean","interpolation":"step","keys":[{"tick":0,"value":true}]}]}"#;
    let view = decode(source, &context()).unwrap();
    let values = sample_properties(&view, 1).unwrap();
    assert_eq!(
        values[0].value,
        PropertyValue::Transform2d([1.0, 2.0, 3.0, 4.0, 5.0, 6.0])
    );
    assert_eq!(values[1].value, PropertyValue::Rgba([0.1, 0.2, 0.3, 0.4]));
    assert_eq!(values[2].value, PropertyValue::Boolean(true));
    let error = sample_properties(&view, 2).unwrap_err();
    assert_eq!(error.error, crate::Error::InvalidArgument);
}
