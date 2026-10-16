//! Exact original reference.c inputs. Index probes exercise the accepted actual
//! index directly; decoder-private paint scan counters still require its real
//! lower resolver. Fixture resolution here proves graph publication only.
#[path = "support/production.rs"]
mod production;
mod support;
use std::sync::Arc;
use support::Run;
use vexel_scene::LexicalDocument;
use vexel_svg_reference::ReferenceIndex;
use vexel_xml::{TokenKind, XmlSpan};
#[test]
fn original_512_late_gradient_index_probe_bounds_and_actual_graph_publication() {
    let source = format!(
        "<svg width=\"16\" height=\"16\">{}<defs><linearGradient id=\"late\"><stop stop-color=\"red\"/><stop offset=\"1\" stop-color=\"blue\"/></linearGradient></defs></svg>",
        "<rect width=\"1\" height=\"1\" fill=\"url(#late)\"/>".repeat(512)
    );
    let mut c = Run::default();
    let document =
        LexicalDocument::parse(Arc::new(source.as_bytes().to_vec()), 1 << 24, &mut c).unwrap();
    let mut index =
        ReferenceIndex::create_controlled(document.source_owner(), document.tokens(), &mut c)
            .unwrap();
    let before = index.storage_bytes();
    let mut count = 0;
    for token in document.tokens() {
        if token.kind != TokenKind::Attribute
            || token.name.slice(source.as_bytes()) != Some(b"fill")
        {
            continue;
        }
        let id = XmlSpan {
            offset: token.value.offset + 5,
            length: 4,
        };
        let found = index.find_controlled(id, &mut c).unwrap().unwrap();
        assert_eq!(
            document.tokens()[found].local_name.slice(source.as_bytes()),
            Some(b"linearGradient".as_slice())
        );
        count += 1;
    }
    assert_eq!(count, 512);
    assert!(index.lookup_probes() >= 512 && index.lookup_probes() <= 1024);
    assert_eq!(index.storage_bytes(), before);
    let (g, requests) =
        production::run_production(&source, &mut production::ProductionFixture::default()).unwrap();
    assert_eq!(requests, 0);
    assert_eq!(g.root().shapes.len(), 512);
    assert_eq!(g.root().stops.len(), 2);
    assert_eq!(g.source(), source.as_bytes());
}
#[test]
fn original_full_4096_comment_512_shape_cached_stop_publication_input() {
    let source = format!(
        "<svg width=\"8\" height=\"8\"><defs><linearGradient id=\"g\"><stop stop-color=\"red\"/><stop offset=\"1\" stop-color=\"blue\"/>{}</linearGradient></defs>{}</svg>",
        "<!-- padding -->".repeat(4096),
        "<rect width=\"1\" height=\"1\" fill=\"url(#g)\"/>".repeat(512)
    );
    let mut fixture = production::ProductionFixture::default();
    let (g, requests) = production::run_production(&source, &mut fixture).unwrap();
    assert_eq!(requests, 0);
    assert_eq!(g.root().shapes.len(), 512);
    assert_eq!(g.root().stops.len(), 2);
    assert_eq!(fixture.resolves, 512);
    assert_eq!(g.source(), source.as_bytes());
}
