use crate::{V1Document, V1DocumentError};

fn doc() -> V1Document {
    let mut doc = V1Document::new(100, 100).unwrap();
    doc.add_fragment(
        "root",
        br##"<defs><g id="star"><rect id="star-body" x="0" y="0" width="10" height="10" fill="#ff0000"/></g></defs>"##,
    )
    .unwrap();
    doc.add_fragment("root", br##"<use id="u1" href="#star" x="5"/>"##)
        .unwrap();
    doc.add_fragment(
        "root",
        br##"<rect id="r2" x="1" y="1" width="5" height="5" clip-path="url(#star)" fill="url('#star')"/>"##,
    )
    .unwrap();
    doc
}

fn text(doc: &V1Document) -> String {
    String::from_utf8(doc.source().unwrap()).unwrap()
}

#[test]
fn rename_rewrites_the_id_and_local_references_in_one_undo() {
    let mut doc = doc();
    let before = text(&doc);
    doc.rename_element("star", "hero").unwrap();
    let after = text(&doc);
    assert!(after.contains(r##"<g id="hero">"##), "{after}");
    assert!(after.contains(r##"href="#hero""##), "{after}");
    assert!(after.contains(r##"clip-path="url(#hero)""##), "{after}");
    assert!(after.contains(r##"fill="url('#hero')""##), "{after}");
    assert!(after.contains(r#"id="star-body""#), "{after}");
    assert!(!after.contains("#star\""), "{after}");
    doc.undo().unwrap();
    assert_eq!(text(&doc), before, "one undo restores the source");
}

#[test]
fn rename_refuses_used_invalid_and_unknown_ids() {
    let mut doc = doc();
    let before = text(&doc);
    assert_eq!(
        doc.rename_element("star", "u1"),
        Err(V1DocumentError::InvalidStableId)
    );
    assert_eq!(
        doc.rename_element("star", "bad id"),
        Err(V1DocumentError::InvalidStableId)
    );
    assert_eq!(
        doc.rename_element("nope", "fresh"),
        Err(V1DocumentError::MissingStableId)
    );
    assert_eq!(doc.rename_element("star", "star"), Ok(()));
    assert_eq!(text(&doc), before);
}
