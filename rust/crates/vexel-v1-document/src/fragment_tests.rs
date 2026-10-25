use super::UseSpec;
use crate::{V1Document, V1DocumentError};

fn doc() -> V1Document {
    let mut doc = V1Document::new(100, 100).unwrap();
    doc.add_fragment(
        "root",
        br##"<defs><g id="star"><rect id="star-body" x="0" y="0" width="10" height="10" fill="#ff0000"/></g></defs>"##,
    )
    .unwrap();
    doc
}

#[test]
fn use_is_one_undoable_element_and_hit_testable() {
    let mut doc = doc();
    let before = doc.source().unwrap();
    let spec = UseSpec {
        x: 40.0,
        y: 30.0,
        fill: Some("#00ff00".into()),
        ..UseSpec::default()
    };
    assert_eq!(doc.add_use("root", Some("s1"), "star", &spec).unwrap(), "s1");
    let text = String::from_utf8(doc.source().unwrap()).unwrap();
    assert!(text.contains(r##"<use id="s1" href="#star" x="40" y="30" fill="#00ff00"/>"##), "{text}");
    assert_eq!(doc.hit_test(45.0, 35.0).as_deref(), Some("s1"));
    assert_eq!(doc.hit_test(5.0, 5.0), None);
    doc.translate("s1", 10.0, 0.0).unwrap();
    assert_eq!(doc.hit_test(55.0, 35.0).as_deref(), Some("s1"));
    doc.undo().unwrap();
    doc.undo().unwrap();
    assert_eq!(doc.source().unwrap(), before);
}

#[test]
fn rejects_duplicate_ids_missing_targets_and_cycles() {
    let mut doc = doc();
    let spec = UseSpec::default();
    assert!(doc.add_fragment("root", br#"<g id="star"/>"#).is_err());
    assert!(doc.add_fragment("root", br#"<g id="a"><g id="a"/></g>"#).is_err());
    assert_eq!(
        doc.add_use("root", Some("u"), "nope", &spec),
        Err(V1DocumentError::MissingStableId)
    );
    assert!(doc.add_use("star", Some("u"), "star", &spec).is_err(), "cycle");
    assert!(doc.add_use("star-body", Some("u"), "star", &spec).is_err(), "cycle");
    doc.add_use("root", Some("u"), "star", &spec).unwrap();
    assert!(doc.add_use("root", Some("u"), "star", &spec).is_err(), "duplicate id");
}

#[test]
fn use_without_id_gets_a_fresh_one() {
    let mut doc = doc();
    let id = doc.add_use("root", None, "star", &UseSpec::default()).unwrap();
    assert_eq!(id, "vexel-object-1");
}
