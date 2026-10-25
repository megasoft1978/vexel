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

#[test]
fn use_cycles_are_refused_and_leave_the_source_alone() {
    let mut doc = doc();
    let spec = UseSpec::default();
    doc.add_use("root", Some("a"), "star", &spec).unwrap();
    let before = doc.source().unwrap();
    // Self reference.
    let self_ref = br##"<use id="loop" href="#loop"/>"##;
    assert_eq!(doc.add_fragment("root", self_ref), Err(V1DocumentError::UseCycle));
    // A -> B -> A through two groups.
    doc.add_fragment("root", br##"<g id="gb"/>"##).unwrap();
    doc.add_fragment("root", br##"<g id="ga"><use id="ua" href="#gb"/></g>"##)
        .unwrap();
    let ok = doc.source().unwrap();
    assert_eq!(
        doc.add_fragment("gb", br##"<use id="ub" href="#ga"/>"##),
        Err(V1DocumentError::UseCycle)
    );
    // A symbol holding a use of its own parent group.
    doc.add_fragment("root", br##"<g id="host"><symbol id="sy"/></g>"##)
        .unwrap();
    assert_eq!(
        doc.add_fragment("sy", br##"<use id="us" href="#host"/>"##),
        Err(V1DocumentError::UseCycle)
    );
    assert_ne!(before, ok);
    assert_eq!(doc.source().unwrap().len(), ok.len() + br##"<g id="host"><symbol id="sy"/></g>"##.len());
    assert_eq!(
        doc.add_use("gb", Some("ub2"), "ga", &spec),
        Err(V1DocumentError::UseCycle)
    );
}

#[test]
fn hit_test_on_cyclic_or_deep_sources_returns_instead_of_overflowing() {
    let cyclic = br##"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><g id="g"><use id="u" href="#g"/></g></svg>"##;
    let doc = V1Document::from_source(cyclic).unwrap();
    assert_eq!(doc.hit_test(1.0, 1.0), None);
    let mut deep = String::from(r#"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10">"#);
    for n in 0..1000 {
        deep.push_str(&format!(r#"<g id="g{n}">"#));
    }
    deep.push_str(r#"<rect id="r" width="10" height="10"/>"#);
    for _ in 0..1000 {
        deep.push_str("</g>");
    }
    deep.push_str("</svg>");
    if let Ok(doc) = V1Document::from_source(deep.as_bytes()) {
        assert_eq!(doc.hit_test(1.0, 1.0), None, "past the depth cap");
    }
}

#[test]
fn fragments_outside_the_allow_list_are_refused() {
    let mut doc = doc();
    let before = doc.source().unwrap();
    for (xml, want) in [
        (r#"<script id="s">alert(1)</script>"#, V1DocumentError::DisallowedElement),
        (r#"<foreignObject id="f"/>"#, V1DocumentError::DisallowedElement),
        (r#"<g id="a"><animate attributeName="x"/></g>"#, V1DocumentError::DisallowedElement),
        (r#"<style id="st">a{}</style>"#, V1DocumentError::DisallowedElement),
        (r#"<g id="a" onclick="x()"/>"#, V1DocumentError::DisallowedAttribute),
        (r#"<g id="a"><rect id="b" OnLoad="x()"/></g>"#, V1DocumentError::DisallowedAttribute),
        (r#"<image id="i" href="http://example.com/a.png"/>"#, V1DocumentError::ExternalReference),
        (r#"<image id="i" href="file:///etc/passwd"/>"#, V1DocumentError::ExternalReference),
        (r#"<use id="u" xlink:href="http://x/y.svg#z"/>"#, V1DocumentError::ExternalReference),
        (r#"<rect id="r" fill="url(http://x/y.svg#p)"/>"#, V1DocumentError::ExternalReference),
        (r#"<rect id="r" style="fill:url('//x/p')"/>"#, V1DocumentError::ExternalReference),
    ] {
        assert_eq!(doc.add_fragment("root", xml.as_bytes()), Err(want), "{xml}");
        assert_eq!(doc.source().unwrap(), before, "{xml}");
    }
    doc.add_fragment(
        "root",
        br##"<g id="ok"><image id="im" href="data:image/png;base64,AAAA"/><rect id="r2" fill="url(#star)" style="fill:url('#star')"/></g>"##,
    )
    .unwrap();
}
