use super::*;

#[test]
fn set_and_remove_attributes_round_trip_undo_redo() {
    let mut d=V1Document::from_source(b"<svg width=\"20\" height=\"20\"><rect id=\"r\" x=\"0\" y=\"0\" width=\"2\" height=\"2\"/><linearGradient id=\"grad\"/></svg>").unwrap();
    let before = d.source().unwrap();
    let count = d.history_count();
    d.set_attribute("r", "opacity", "0.5").unwrap();
    assert_eq!(d.history_count(), count + 1);
    let after = d.source().unwrap();
    d.undo().unwrap();
    assert_eq!(d.source().unwrap(), before);
    d.redo().unwrap();
    assert_eq!(d.source().unwrap(), after);
    d.set_attribute("r", "fill", "url(#grad)").unwrap();
    let filled = d.source().unwrap();
    d.remove_attribute("r", "fill").unwrap();
    let removed = d.source().unwrap();
    d.undo().unwrap();
    assert_eq!(d.source().unwrap(), filled);
    d.redo().unwrap();
    assert_eq!(d.source().unwrap(), removed);
}

#[test]
fn invalid_attribute_is_atomic_and_unknown_name_is_unsupported() {
    let mut d =
        V1Document::from_source(b"<svg width=\"20\" height=\"20\"><rect id=\"r\"/></svg>").unwrap();
    let before = d.source().unwrap();
    let count = d.history_count();
    assert_eq!(
        d.set_attribute("r", "opacity", "2"),
        Err(V1DocumentError::Unsupported)
    );
    assert_eq!(
        d.set_attribute("r", "xlink:href", "https://example.com"),
        Err(V1DocumentError::Unsupported)
    );
    assert_eq!(
        d.set_attribute("r", "stroke-width", "999"),
        Err(V1DocumentError::InvalidStrokeWidth)
    );
    assert_eq!(
        d.set_attribute("r", "fill", "url(#missing)"),
        Err(V1DocumentError::MissingStableId)
    );
    assert_eq!(d.source().unwrap(), before);
    assert_eq!(d.history_count(), count);
    assert_eq!(
        d.remove_attribute("r", "onclick"),
        Err(V1DocumentError::Unsupported)
    );
}
