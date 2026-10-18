use std::sync::Arc;
use vexel_geometry::{Edge, Point, affine_identity};
use vexel_scene::{
    ChargeLedger, DecodeSource, ErrorKind, LexicalDocument, PaintCache, PaintKind, ResolveContext,
    ResolvedPaint, Stop, WorkControl, WorkKind,
};
use vexel_svg_paint::resolve_fill;
use vexel_xml::XmlSpan;

struct Continue;
impl WorkControl for Continue {
    fn charge(&mut self, _: WorkKind, _: u64) -> Result<(), Stop> {
        Ok(())
    }
    fn checkpoint(&mut self) -> Result<(), Stop> {
        Ok(())
    }
}

fn span(source: &[u8], value: &[u8]) -> XmlSpan {
    let offset = source
        .windows(value.len())
        .position(|part| part == value)
        .unwrap();
    XmlSpan {
        offset,
        length: value.len(),
    }
}

fn resolve(bytes: &[u8], fill: &[u8]) -> vexel_scene::Result<ResolvedPaint> {
    let source = Arc::new(bytes.to_vec());
    let mut work = Continue;
    let document = LexicalDocument::parse(source.clone(), 1 << 20, &mut work).unwrap();
    let mut shared = DecodeSource::new(&document, &mut work).unwrap();
    let mut cache = PaintCache::default();
    let mut ledger = ChargeLedger::new(&source, 1 << 24, 64).unwrap();
    let mut ctx = ResolveContext {
        shared: &mut shared,
        cache: &mut cache,
        ledger: &mut ledger,
    };
    let edge = [Edge {
        a: Point { x: 0., y: 0. },
        b: Point { x: 1., y: 1. },
    }];
    resolve_fill(
        &mut ctx,
        span(bytes, fill),
        &edge,
        affine_identity(),
        [10., 10.],
        &mut work,
    )
}

#[test]
fn radial_href_to_linear_inherits_stops_and_keeps_own_geometry() {
    let bytes = b"<svg><defs><linearGradient id='l'><stop stop-color='red'/><stop offset='1' stop-color='blue'/></linearGradient><radialGradient id='r' href='#l' cx='0.25'/></defs><rect fill='url(#r)'/></svg>";
    let resolved = resolve(bytes, b"url(#r)").unwrap();
    assert_eq!(resolved.paint.kind, PaintKind::Radial);
    assert_eq!(resolved.paint.coordinates[0], 0.25);
    assert_eq!(resolved.stops.len(), 2);
    assert_eq!(resolved.stops[0].rgba, [255, 0, 0, 255]);
    assert_eq!(resolved.stops[1].rgba, [0, 0, 255, 255]);
}

#[test]
fn linear_href_to_radial_inherits_stops_but_not_radial_geometry() {
    let bytes = b"<svg><defs><radialGradient id='r'><stop stop-color='red'/><stop offset='1' stop-color='blue'/></radialGradient><linearGradient id='l' href='#r' x2='0.5'/></defs><rect fill='url(#l)'/></svg>";
    let resolved = resolve(bytes, b"url(#l)").unwrap();
    assert_eq!(resolved.paint.kind, PaintKind::Linear);
    assert_eq!(resolved.paint.coordinates[2], 0.5);
    assert_eq!(resolved.stops.len(), 2);
    assert_eq!(resolved.stops[1].rgba, [0, 0, 255, 255]);
}

#[test]
fn cross_kind_geometry_is_ignored_for_the_target_kind() {
    let bytes = b"<svg><defs><radialGradient id='r' cx='0.25'><stop/><stop offset='1'/></radialGradient><linearGradient id='l' href='#r'/></defs><rect fill='url(#l)'/></svg>";
    let resolved = resolve(bytes, b"url(#l)").unwrap();
    assert_eq!(resolved.paint.kind, PaintKind::Linear);
    assert_eq!(resolved.paint.coordinates[0], 0.0);
    assert_eq!(resolved.paint.coordinates[2], 1.0);
}

#[test]
fn geometry_the_element_kind_cannot_carry_is_rejected() {
    let bytes = b"<svg><defs><radialGradient id='r' x1='0.1'><stop/><stop offset='1'/></radialGradient><linearGradient id='l' href='#r'/></defs><rect fill='url(#l)'/></svg>";
    let error = resolve(bytes, b"url(#l)").unwrap_err();
    assert_eq!(error.kind, ErrorKind::Unsupported);
}

#[test]
fn inkscape_collect_is_accepted_when_prefix_is_bound() {
    let bytes = b"<svg xmlns:inkscape='http://www.inkscape.org/namespaces/inkscape'><defs><linearGradient id='g' inkscape:collect='always'><stop/><stop offset='1'/></linearGradient></defs><rect fill='url(#g)'/></svg>";
    let resolved = resolve(bytes, b"url(#g)").unwrap();
    assert_eq!(resolved.paint.kind, PaintKind::Linear);
    assert_eq!(resolved.stops.len(), 2);
}

#[test]
fn unbound_inkscape_prefix_is_rejected() {
    let bytes = b"<svg><defs><linearGradient id='g' inkscape:collect='always'><stop/><stop offset='1'/></linearGradient></defs><rect fill='url(#g)'/></svg>";
    let error = resolve(bytes, b"url(#g)").unwrap_err();
    assert_eq!(error.kind, ErrorKind::Unsupported);
}

#[test]
fn other_bound_inkscape_attributes_stay_rejected() {
    let bytes = b"<svg xmlns:inkscape='http://www.inkscape.org/namespaces/inkscape'><defs><linearGradient id='g' inkscape:label='x'><stop/><stop offset='1'/></linearGradient></defs><rect fill='url(#g)'/></svg>";
    let error = resolve(bytes, b"url(#g)").unwrap_err();
    assert_eq!(error.kind, ErrorKind::Unsupported);
}

#[test]
fn cross_kind_href_cycle_is_rejected() {
    let bytes = b"<svg><defs><radialGradient id='r' href='#l'><stop/><stop offset='1'/></radialGradient><linearGradient id='l' href='#r'/></defs><rect fill='url(#r)'/></svg>";
    let error = resolve(bytes, b"url(#r)").unwrap_err();
    assert_eq!(error.kind, ErrorKind::Unsupported);
}

#[test]
fn dangling_href_is_rejected() {
    let bytes = b"<svg><defs><radialGradient id='r' href='#missing'><stop/><stop offset='1'/></radialGradient></defs><rect fill='url(#r)'/></svg>";
    let error = resolve(bytes, b"url(#r)").unwrap_err();
    assert_eq!(error.kind, ErrorKind::Unsupported);
}
