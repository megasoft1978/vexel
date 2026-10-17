//! WO-RUST-SVG-COMPAT-01: caller-supplied namespace resolution for prefixed attributes.
use vexel_svg_reference::{attributes_supported, attributes_supported_with};
use vexel_xml::{TokenKind, XmlSpan, XmlToken, tokenize_large};

fn parsed(source: &[u8]) -> Vec<XmlToken> {
    let (tokens, result) = tokenize_large(source);
    assert_eq!(
        result.diagnostic.category,
        vexel_xml::XmlError::None,
        "{result:?}"
    );
    tokens
}

fn element(tokens: &[XmlToken]) -> (usize, XmlSpan) {
    let index = tokens
        .iter()
        .position(|t| t.kind == TokenKind::EmptyElement)
        .unwrap();
    (index, tokens[index].local_name)
}

#[test]
fn foreign_prefix_is_ignored_only_when_caller_reports_it_foreign() {
    let source = b"<circle sodipodi:cx='1' cx='2' r='1'/>";
    let tokens = parsed(source);
    let (index, kind) = element(&tokens);
    let prefix = tokens[index + 1].prefix;
    assert!(!attributes_supported(source, &tokens, index, false, kind));
    assert!(attributes_supported_with(
        source,
        &tokens,
        index,
        false,
        kind,
        &|p| p == prefix
    ));
    assert!(!attributes_supported_with(
        source,
        &tokens,
        index,
        false,
        kind,
        &|_| false
    ));
}

#[test]
fn xlink_href_is_never_ignored_as_foreign() {
    let image = b"<image xlink:href='a.png' width='1' height='1'/>";
    let tokens = parsed(image);
    let (index, kind) = element(&tokens);
    assert!(attributes_supported_with(
        image,
        &tokens,
        index,
        false,
        kind,
        &|_| true
    ));
    let rect = b"<rect xlink:href='#a'/>";
    let tokens = parsed(rect);
    let (index, kind) = element(&tokens);
    assert!(!attributes_supported_with(
        rect,
        &tokens,
        index,
        false,
        kind,
        &|_| true
    ));
}

#[test]
fn foreign_callback_applies_to_prefixed_shape_names_only_when_reported() {
    let source = b"<rect foo:width='1' width='3'/>";
    let tokens = parsed(source);
    let (index, kind) = element(&tokens);
    assert!(attributes_supported_with(
        source,
        &tokens,
        index,
        false,
        kind,
        &|_| true
    ));
    assert!(!attributes_supported_with(
        source,
        &tokens,
        index,
        false,
        kind,
        &|_| false
    ));
}
