use vexel_svg_css::{CssPaint, CssSheet, Property};
use vexel_xml::{TokenKind, XmlError, tokenize_large};

#[test]
fn copied_marker_context_clears_exactly_four_properties_and_retains_spans() {
    let source = br#"<svg><marker style="fill:red;stroke:blue;color:purple;stroke-width:2;marker:url(#a);marker-start:url(#b);marker-mid:url(#c);marker-end:url(#d)"/></svg>"#;
    let (tokens, result) = tokenize_large(source);
    assert_eq!(result.diagnostic.category, XmlError::None);
    let marker = tokens
        .iter()
        .position(|t| {
            matches!(t.kind, TokenKind::StartElement | TokenKind::EmptyElement)
                && t.local_name.slice(source) == Some(b"marker".as_slice())
        })
        .unwrap();
    let mut sheet = CssSheet::create(source, &tokens).unwrap();
    let original = sheet.compute(marker, None).unwrap();
    let cleaned = original.without_inherited_markers();
    for property in Property::ALL {
        if [
            Property::Marker,
            Property::MarkerStart,
            Property::MarkerMid,
            Property::MarkerEnd,
        ]
        .contains(&property)
        {
            assert!(original.value(property).is_some());
            assert!(cleaned.value(property).is_none());
        } else {
            assert_eq!(cleaned.value(property), original.value(property));
            if let Some(value) = cleaned.value(property) {
                assert_eq!(value.span.slice(source), Some(value.bytes));
            }
        }
    }
    assert!(original.value(Property::MarkerEnd).is_some());
    assert_eq!(cleaned.without_inherited_markers(), cleaned);
}

#[test]
fn shorthand_does_not_leak_but_explicit_child_markers_cascade_normally() {
    let source = br#"<svg><marker marker="url(#host)" fill="red"><path marker-end="url(#child)"/></marker></svg>"#;
    let (tokens, result) = tokenize_large(source);
    assert_eq!(result.diagnostic.category, XmlError::None);
    let find = |name: &[u8]| {
        tokens
            .iter()
            .position(|t| {
                matches!(t.kind, TokenKind::StartElement | TokenKind::EmptyElement)
                    && t.local_name.slice(source) == Some(name)
            })
            .unwrap()
    };
    let mut sheet = CssSheet::create(source, &tokens).unwrap();
    let parent = sheet.compute(find(b"marker"), None).unwrap();
    assert_eq!(parent.value(Property::Marker).unwrap().bytes, b"url(#host)");
    assert!(parent.value(Property::MarkerStart).is_none());
    let cleaned = parent.without_inherited_markers();
    let child = sheet.compute(find(b"path"), Some(&cleaned)).unwrap();
    assert!(child.value(Property::Marker).is_none());
    assert!(child.value(Property::MarkerStart).is_none());
    assert!(child.value(Property::MarkerMid).is_none());
    assert_eq!(
        child.value(Property::MarkerEnd).unwrap().bytes,
        b"url(#child)"
    );
    assert_eq!(child.value(Property::Fill).unwrap().bytes, b"red");
    assert_eq!(parent.value(Property::Marker).unwrap().bytes, b"url(#host)");
}

#[test]
fn initial_marker_context_is_allocation_free_and_idempotent() {
    let initial = CssPaint::initial();
    assert_eq!(initial.without_inherited_markers(), initial);
    for property in Property::ALL {
        assert!(
            initial
                .without_inherited_markers()
                .value(property)
                .is_none()
        );
    }
}
