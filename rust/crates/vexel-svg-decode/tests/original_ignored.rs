mod support;
use support::*;
use vexel_scene::{ErrorKind, IgnoredKind};
const RECT: &str = r##"<rect x="1" y="1" width="4" height="3" fill="red""##;
#[test]
fn original_ignored_inputs_counts_offsets_and_geometry() {
    let cases = [
        (
            format!(
                r##"<svg width="8" height="8" data-doc="1">{RECT} data-a="1" foo="bar"/></svg>"##
            ),
            format!(r##"<svg width="8" height="8">{RECT}/></svg>"##),
            IgnoredKind::UnknownAttribute,
            3,
            "data-doc",
        ),
        (
            format!(r##"<svg width="8" height="8"><g custom="x">{RECT}/></g></svg>"##),
            format!(r##"<svg width="8" height="8"><g>{RECT}/></g></svg>"##),
            IgnoredKind::UnknownAttribute,
            1,
            "custom",
        ),
        (
            format!(r##"<svg width="8" height="8"><meta charset="utf-8"/>{RECT}/></svg>"##),
            format!(r##"<svg width="8" height="8">{RECT}/></svg>"##),
            IgnoredKind::Meta,
            1,
            "<meta",
        ),
        (
            format!(
                "<?xml version=\"1.0\"?>\n<!DOCTYPE svg PUBLIC \"-//W3C//DTD SVG 1.1//EN\" \"x.dtd\">\n<svg width=\"8\" height=\"8\">{RECT}/></svg>"
            ),
            format!(r##"<svg width="8" height="8">{RECT}/></svg>"##),
            IgnoredKind::Doctype,
            1,
            "<!DOCTYPE",
        ),
        (
            format!(
                r##"<!DOCTYPE svg [<!-- ]> --><!ENTITY e "x>y]"><?p ]>?>]><svg width="8" height="8">{RECT}/></svg>"##
            ),
            format!(r##"<svg width="8" height="8">{RECT}/></svg>"##),
            IgnoredKind::Doctype,
            1,
            "<!DOCTYPE",
        ),
        (
            format!(
                r##"<svg width="8" height="8"><style>.a{{writing-mode:tb}}</style>{RECT} writing-mode="tb"/><g style="fill:red; writing-mode : tb-rl"/></svg>"##
            ),
            format!(r##"<svg width="8" height="8"><style>.a{{}}</style>{RECT}/><g/></svg>"##),
            IgnoredKind::WritingMode,
            3,
            "writing-mode:tb",
        ),
    ];
    for (with, without, kind, count, needle) in cases {
        let a = decode(&with).unwrap();
        let b = decode(&without).unwrap();
        let av = a.root();
        let bv = b.root();
        assert_eq!(av.shapes.len(), bv.shapes.len());
        assert!(!av.edges.is_empty());
        assert_eq!(av.edges, bv.edges);
        assert_eq!(av.edge_colors, bv.edge_colors);
        assert_eq!(av.ignored(kind).count, count);
        assert_eq!(av.ignored(kind).first_offset, Some(offset(&with, needle)));
        for other in [
            IgnoredKind::UnknownAttribute,
            IgnoredKind::Meta,
            IgnoredKind::Doctype,
            IgnoredKind::WritingMode,
        ] {
            if other != kind {
                assert_eq!(av.ignored(other).count, 0)
            }
            assert_eq!(bv.ignored(other).count, 0);
        }
    }
}
#[test]
fn original_ignored_foreign_attributes_and_rendering_rejection() {
    let source = format!(r##"<svg width="8" height="8" xmlns:x="urn:x">{RECT} x:foo="1"/></svg>"##);
    assert_eq!(
        decode(&source)
            .unwrap()
            .root()
            .ignored(IgnoredKind::UnknownAttribute)
            .count,
        0
    );
    let source =
        format!(r##"<svg width="8" height="8">{RECT} text-decoration="underline"/></svg>"##);
    assert_eq!(decode(&source).unwrap_err().kind, ErrorKind::Unsupported);
}
#[test]
fn original_doctype_entity_passes_xml_and_trailing_doctype_fails_as_xml() {
    // Declared internal entities now expand; a DOCTYPE after the root is still an XML error.
    let source = format!(
        r##"<!DOCTYPE svg [<!ENTITY e "1">]><svg width="8" height="8">{RECT} rx="&e;"/></svg>"##
    );
    // The XML layer accepts the declared entity; attribute values in the decoder are not
    // expanded yet, so this fails later as a number error, not as XML.
    assert!(!matches!(
        decode(&source).unwrap_err().kind,
        ErrorKind::Xml(_)
    ));
    let source = "<svg width=\"8\" height=\"8\"/><!DOCTYPE svg>";
    assert!(matches!(
        decode(source).unwrap_err().kind,
        ErrorKind::Xml(_)
    ));
}
#[test]
fn expanded_unknown_attribute_notes_count_once_and_choose_minimum_source_offset() {
    let s = r##"<svg width="8" height="8"><defs><g id="a" data-a="x"><rect width="1" height="1"/></g><g id="b" data-b="x"><rect width="1" height="1"/></g></defs><use href="#b"/><use href="#a"/><use href="#a"/></svg>"##;
    let g = decode(s).unwrap();
    let r = g.root().ignored(IgnoredKind::UnknownAttribute);
    assert_eq!(r.count, 2);
    assert_eq!(r.first_offset, Some(offset(s, "data-a")));
}
