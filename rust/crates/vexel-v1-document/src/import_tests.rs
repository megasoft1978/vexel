use super::{MAX_IMPORT_BYTES, prepare_import, valid_prefix};
use crate::{V1Document, V1DocumentError};

const SAMPLE: &[u8] = br##"<?xml version="1.0"?><svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="100" height="100" viewBox="0 0 100 100"><defs><linearGradient id="g1"><stop offset="0" stop-color="#ff0000"/></linearGradient><clipPath id="c1"><rect id="cr" x="0" y="0" width="50" height="50"/></clipPath></defs><g id="body" clip-path="url(#c1)"><rect id="torso" x="10" y="10" width="30" height="30" fill="url(#g1)"/><use id="u1" href="#torso" x="5"/><path id="p" d="M0 0L5 5" style="fill:url('#g1')"/></g><desc id="vexel-rig">{"v":1,"bones":[{"id":"body","pivot":[0,0]},{"id":"torso","pivot":[1,2],"parent":"body"}],"roles":{"torso":"body"}}</desc></svg>"##;

#[test]
fn prefixes_ids_and_references_and_strips_rig() {
    let prepared = prepare_import(SAMPLE, "kira_", None).unwrap();
    let text = String::from_utf8(prepared.xml.clone()).unwrap();
    assert!(text.starts_with("<g id=\"kira\">"), "{text}");
    for needle in [
        r##"id="kira_g1""##,
        r##"id="kira_body""##,
        r##"clip-path="url(#kira_c1)""##,
        r##"fill="url(#kira_g1)""##,
        r##"href="#kira_torso""##,
        r##"style="fill:url('#kira_g1')""##,
    ] {
        assert!(text.contains(needle), "{needle} in {text}");
    }
    assert!(!text.contains("vexel-rig"), "{text}");
    assert_eq!(prepared.source_ids.len(), 8);
    let rig = prepared.rig.unwrap();
    assert!(
        rig.contains(r#""id":"kira_body""#) && rig.contains(r#""parent":"kira_body""#),
        "{rig}"
    );
    assert!(rig.contains(r#""roles":{"torso":"kira_body"}"#), "{rig}");
}

#[test]
fn two_imports_with_different_prefixes_do_not_collide() {
    let mut doc = V1Document::new(200, 200).unwrap();
    let a = prepare_import(SAMPLE, "a_", Some([10.0, 20.0, 0.0, 2.0, 2.0])).unwrap();
    let b = prepare_import(SAMPLE, "b_", None).unwrap();
    doc.add_import("root", &a).unwrap();
    doc.add_import("root", &b).unwrap();
    let text = String::from_utf8(doc.source().unwrap()).unwrap();
    assert!(
        text.contains(r#"<g id="a" transform="matrix(2 0 0 2 10 20)">"#),
        "{text}"
    );
    assert!(doc.element_get("a_torso").is_ok() && doc.element_get("b_torso").is_ok());
    // The same prefix again is refused: its ids already exist.
    assert_eq!(
        doc.add_import("root", &a),
        Err(V1DocumentError::InvalidStableId)
    );
    doc.undo().unwrap();
    assert!(doc.element_get("b_torso").is_err());
}

#[test]
fn hostile_files_are_refused() {
    let wrap = |inner: &str| {
        format!(r#"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10">{inner}</svg>"#)
    };
    let mut doc = V1Document::new(100, 100).unwrap();
    let before = doc.source().unwrap();
    for (inner, error) in [
        (
            "<script>alert(1)</script>",
            V1DocumentError::DisallowedElement,
        ),
        (
            r#"<rect id="r" onclick="x()" width="1" height="1"/>"#,
            V1DocumentError::DisallowedAttribute,
        ),
        (
            r#"<image id="i" href="https://evil/x.png"/>"#,
            V1DocumentError::ExternalReference,
        ),
        (
            r#"<rect id="r" fill="url(https://evil/x)" width="1" height="1"/>"#,
            V1DocumentError::ExternalReference,
        ),
        (
            r#"<foreignObject id="f"/>"#,
            V1DocumentError::DisallowedElement,
        ),
        (
            r#"<animate id="a" attributeName="x"/>"#,
            V1DocumentError::DisallowedElement,
        ),
    ] {
        let prepared = prepare_import(wrap(inner).as_bytes(), "h_", None).unwrap();
        assert_eq!(doc.add_import("root", &prepared), Err(error), "{inner}");
    }
    assert_eq!(doc.source().unwrap(), before);
    let entity = r#"<!DOCTYPE svg [<!ENTITY x "y">]><svg xmlns="http://www.w3.org/2000/svg"><rect id="r"/></svg>"#;
    assert!(prepare_import(entity.as_bytes(), "h_", None).is_err());
    assert!(prepare_import(b"<svg/>", "h_", None).is_err());
    assert!(prepare_import(b"not svg", "h_", None).is_err());
    let big = vec![b' '; MAX_IMPORT_BYTES + 1];
    assert!(prepare_import(&big, "h_", None).is_err());
}

#[test]
fn prefix_rules() {
    assert!(valid_prefix("kira_") && valid_prefix("a.b-c") && valid_prefix("_x"));
    assert!(
        !valid_prefix("") && !valid_prefix("1a") && !valid_prefix("a b") && !valid_prefix("a\"b")
    );
    assert!(!valid_prefix(&"a".repeat(65)));
    assert!(prepare_import(SAMPLE, "a\"b", None).is_err());
    assert!(prepare_import(SAMPLE, "a_", Some([f64::NAN, 0.0, 0.0, 1.0, 1.0])).is_err());
}
