mod support;
use std::sync::Arc;
use support::Run;
use vexel_scene::{ErrorKind, LexicalDocument, LimitKind};

fn parse_kind(source: Vec<u8>) -> ErrorKind {
    LexicalDocument::parse(Arc::new(source), 256 * 1024 * 1024, &mut Run)
        .unwrap_err()
        .kind
}

#[test]
fn deliberate_depth_limit_reads_as_limit_not_xml() {
    let mut source = Vec::new();
    for _ in 0..1000 {
        source.extend_from_slice(b"<g>");
    }
    for _ in 0..1000 {
        source.extend_from_slice(b"</g>");
    }
    assert_eq!(parse_kind(source), ErrorKind::Limit(LimitKind::Depth));
}

#[test]
fn deliberate_node_limit_reads_as_limit_not_xml() {
    let mut source = b"<r>".to_vec();
    for _ in 0..1_000_010 {
        source.extend_from_slice(b"<a/>");
    }
    source.extend_from_slice(b"</r>");
    assert_eq!(parse_kind(source), ErrorKind::Limit(LimitKind::Nodes));
}

#[test]
fn deliberate_attribute_limit_reads_as_limit_not_xml() {
    let mut source = b"<a".to_vec();
    for index in 0..5000 {
        source.extend_from_slice(format!(" a{index}=\"1\"").as_bytes());
    }
    source.extend_from_slice(b"/>");
    assert_eq!(parse_kind(source), ErrorKind::Limit(LimitKind::Attributes));
}

#[test]
fn deliberate_text_limit_reads_as_limit_not_xml() {
    let mut source = b"<a>".to_vec();
    source.extend(std::iter::repeat(b'x').take(4 * 1024 * 1024 + 1));
    source.extend_from_slice(b"</a>");
    assert_eq!(parse_kind(source), ErrorKind::Limit(LimitKind::TextBytes));
}

#[test]
fn malformed_xml_still_reads_as_xml() {
    assert!(matches!(parse_kind(b"<svg>".to_vec()), ErrorKind::Xml(_)));
}
