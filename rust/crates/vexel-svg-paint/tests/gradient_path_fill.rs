use std::sync::Arc;
use vexel_base::{WorkControl, WorkKind};
use vexel_geometry::{Edge, Point, affine_identity};
use vexel_scene::*;
use vexel_svg_paint::resolve_cached_fill;
use vexel_xml::XmlSpan;

struct Run;
impl WorkControl for Run {
    fn charge(&mut self, _: WorkKind, _: u64) -> std::result::Result<(), Stop> {
        Ok(())
    }
    fn checkpoint(&mut self) -> std::result::Result<(), Stop> {
        Ok(())
    }
}

fn resolve(source: &str, fill: &str) -> Result<PaintKind> {
    let owner = Arc::new(source.as_bytes().to_vec());
    let mut work = Run;
    let document = LexicalDocument::parse(owner.clone(), 256 * 1024 * 1024, &mut work)?;
    let mut shared = DecodeSource::new(&document, &mut work)?;
    let mut ledger = ChargeLedger::new(&owner, 64 * 1024 * 1024, 4096)?;
    let mut cache = PaintCache::default();
    let offset = source.find(fill).unwrap();
    let value = XmlSpan {
        offset,
        length: fill.len(),
    };
    let edges = [
        Edge {
            a: Point { x: 1.0, y: 1.0 },
            b: Point { x: 7.0, y: 1.0 },
        },
        Edge {
            a: Point { x: 7.0, y: 1.0 },
            b: Point { x: 7.0, y: 7.0 },
        },
        Edge {
            a: Point { x: 7.0, y: 7.0 },
            b: Point { x: 1.0, y: 1.0 },
        },
    ];
    let resolved = resolve_cached_fill(
        &mut shared,
        &mut cache,
        &mut ledger,
        value,
        &edges,
        affine_identity(),
        [8.0, 8.0],
        &mut work,
    )?;
    Ok(resolved.paint.kind)
}

const STOPS: &str = "<stop stop-color=\"red\"/><stop offset=\"1\" stop-color=\"blue\"/>";

#[test]
fn gradient_path_fill_attribute_resolves() {
    let source = format!(
        "<svg xmlns:xlink=\"http://www.w3.org/1999/xlink\"><defs><linearGradient id=\"g\">{STOPS}</linearGradient></defs><path d=\"M1 1L7 1L7 7Z\" fill=\"url(#g)\"/></svg>"
    );
    assert_eq!(resolve(&source, "url(#g)").unwrap(), PaintKind::Linear);
}

#[test]
fn gradient_path_fill_style_attribute_resolves() {
    let source = format!(
        "<svg><defs><linearGradient id=\"g\">{STOPS}</linearGradient></defs><path d=\"M1 1L7 1L7 7Z\" style=\"fill:url(#g)\"/></svg>"
    );
    assert_eq!(resolve(&source, "url(#g)").unwrap(), PaintKind::Linear);
}

#[test]
fn gradient_path_fill_xlink_inheritance_with_transform_resolves() {
    let source = format!(
        "<svg xmlns:xlink=\"http://www.w3.org/1999/xlink\"><defs><linearGradient id=\"base\" gradientTransform=\"translate(.1 .2)\">{STOPS}</linearGradient><linearGradient id=\"g\" xlink:href=\"#base\"/></defs><path d=\"M1 1L7 1L7 7Z\" fill=\"url(#g)\"/></svg>"
    );
    assert_eq!(resolve(&source, "url(#g)").unwrap(), PaintKind::Linear);
}

#[test]
fn gradient_path_fill_inkscape_chain_resolves() {
    let source = format!(
        "<svg xmlns:xlink=\"http://www.w3.org/1999/xlink\"><defs><linearGradient id=\"linear0\">{STOPS}</linearGradient><linearGradient id=\"linear1\" xlink:href=\"#linear0\" gradientTransform=\"matrix(1 0 0 1 2 3)\"/><linearGradient id=\"linear2\" xlink:href=\"#linear1\"/></defs><path d=\"M1 1L7 1L7 7Z\" style=\"fill:url(#linear2)\"/></svg>"
    );
    assert_eq!(
        resolve(&source, "url(#linear2)").unwrap(),
        PaintKind::Linear
    );
}

/// Corpus regression (ideogram): gradient attributes located in a document
/// larger than the 16 MiB preview cap must still parse (the value limits used
/// the whole-source length and rejected `gradientTransform` and coordinates).
#[test]
fn corpus_regression_ideogram() {
    let padding = "x".repeat(17 * 1024 * 1024);
    let source = format!(
        "<svg><defs><!--{padding}--><linearGradient id=\"linear0\" gradientUnits=\"userSpaceOnUse\" x1=\"0\" y1=\"0\" x2=\"1\" y2=\"0\" gradientTransform=\"matrix(10,0,0,63.428,20,20.426)\">{STOPS}</linearGradient></defs><path d=\"M1 1L7 1L7 7Z\" style=\"fill:url(#linear0);fill-rule:nonzero\"/></svg>"
    );
    assert!(source.len() > 16 * 1024 * 1024);
    assert_eq!(
        resolve(&source, "url(#linear0)").unwrap(),
        PaintKind::Linear
    );
}
