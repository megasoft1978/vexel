use vexel_scene::{ErrorKind, LimitKind};
use vexel_svg_definitions::parse_marker_attributes;
use vexel_xml::{TokenKind, XmlLimits, XmlSpan, tokenize};

fn parsed(xml: &[u8]) -> (Vec<vexel_xml::XmlToken>, usize) {
    let (tokens, output) = tokenize(xml, XmlLimits::default());
    assert_eq!(output.diagnostic.category, vexel_xml::XmlError::None);
    let element = tokens
        .iter()
        .position(|token| {
            matches!(
                token.kind,
                TokenKind::StartElement | TokenKind::EmptyElement
            ) && token.local_name.slice(xml) == Some(b"marker")
        })
        .unwrap();
    (tokens, element)
}

fn attribute_span(
    xml: &[u8],
    tokens: &[vexel_xml::XmlToken],
    element: usize,
    name: &[u8],
) -> XmlSpan {
    tokens[element + 1..]
        .iter()
        .take_while(|token| token.kind == TokenKind::Attribute)
        .find(|token| token.local_name.slice(xml) == Some(name))
        .unwrap()
        .value
}

#[test]
fn nonpositive_marker_dimensions_are_limits_at_the_marker_name() {
    for (attribute, value) in [
        (b"markerWidth".as_slice(), b"0".as_slice()),
        (b"markerWidth", b"-2"),
        (b"markerHeight", b"0"),
        (b"markerHeight", b"-2"),
    ] {
        let xml = format!(
            "<svg><marker {name}=\"{value}\"/></svg>",
            name = std::str::from_utf8(attribute).unwrap(),
            value = std::str::from_utf8(value).unwrap()
        );
        let (tokens, element) = parsed(xml.as_bytes());
        let error = parse_marker_attributes(xml.as_bytes(), &tokens, element).unwrap_err();
        assert_eq!(error.kind, ErrorKind::Limit(LimitKind::Shapes));
        assert_eq!(error.offset, tokens[element].name.offset);
    }
}

#[test]
fn overlimit_marker_dimensions_keep_the_same_limit_diagnostic() {
    for attribute in [b"markerWidth".as_slice(), b"markerHeight"] {
        let xml = format!(
            "<svg><marker {}=\"4097\"/></svg>",
            std::str::from_utf8(attribute).unwrap()
        );
        let (tokens, element) = parsed(xml.as_bytes());
        let error = parse_marker_attributes(xml.as_bytes(), &tokens, element).unwrap_err();
        assert_eq!(error.kind, ErrorKind::Limit(LimitKind::Shapes));
        assert_eq!(error.offset, tokens[element].name.offset);
    }
}

#[test]
fn malformed_dimensions_remain_number_errors_at_the_value() {
    for attribute in [b"markerWidth".as_slice(), b"markerHeight"] {
        let xml = format!(
            "<svg><marker {}=\"bad\"/></svg>",
            std::str::from_utf8(attribute).unwrap()
        );
        let (tokens, element) = parsed(xml.as_bytes());
        let value = attribute_span(xml.as_bytes(), &tokens, element, attribute);
        let error = parse_marker_attributes(xml.as_bytes(), &tokens, element).unwrap_err();
        assert_eq!(error.kind, ErrorKind::Number);
        assert_eq!(error.offset, value.offset);
    }
}

#[test]
fn positive_and_default_marker_dimensions_are_unchanged() {
    let xml = b"<svg><marker markerWidth='2' markerHeight='4'/><marker/></svg>";
    let (tokens, _) = parsed(xml);
    let elements: Vec<_> = tokens
        .iter()
        .enumerate()
        .filter_map(|(index, token)| {
            (matches!(
                token.kind,
                TokenKind::StartElement | TokenKind::EmptyElement
            ) && token.local_name.slice(xml) == Some(b"marker"))
            .then_some(index)
        })
        .collect();
    let explicit = parse_marker_attributes(xml, &tokens, elements[0]).unwrap();
    assert_eq!((explicit.width, explicit.height), (2.0, 4.0));
    let defaults = parse_marker_attributes(xml, &tokens, elements[1]).unwrap();
    assert_eq!((defaults.width, defaults.height), (3.0, 3.0));
}
