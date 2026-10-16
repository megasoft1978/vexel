use std::sync::Arc;
use vexel_geometry::{Affine, Edge, Point, affine_identity};
use vexel_scene::{
    ChargeLedger, DecodeSource, ErrorKind, LexicalDocument, PaintCache, PaintKind, ResolveContext,
    SourceOwner, Spread, Stop, WorkControl, WorkKind,
};
use vexel_svg_paint::{PaintReference, classify_fill, resolve_fill, sample};
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

struct Cancel;
impl WorkControl for Cancel {
    fn charge(&mut self, _: WorkKind, _: u64) -> Result<(), Stop> {
        Err(Stop::Canceled)
    }
    fn checkpoint(&mut self) -> Result<(), Stop> {
        Err(Stop::Canceled)
    }
}

struct StopAfter(usize);
impl WorkControl for StopAfter {
    fn charge(&mut self, _: WorkKind, _: u64) -> Result<(), Stop> {
        if self.0 == 0 {
            Err(Stop::Canceled)
        } else {
            self.0 -= 1;
            Ok(())
        }
    }
    fn checkpoint(&mut self) -> Result<(), Stop> {
        if self.0 == 0 {
            Err(Stop::Canceled)
        } else {
            Ok(())
        }
    }
}

fn owner(text: &[u8]) -> SourceOwner {
    Arc::new(text.to_vec())
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

#[test]
fn resolves_href_inheritance_current_color_transform_and_scene_stop_sharing() {
    let bytes = b"<svg color='#204060'><defs><linearGradient id='base' x2='100%'><stop offset='0%' stop-color='currentColor' stop-opacity='50%'/><stop offset='1' stop-color='white'/></linearGradient><linearGradient id='child' href='#base' spreadMethod='reflect' gradientTransform='translate(1 2)'/></defs><rect fill='url(#child)'/></svg>";
    let source = owner(bytes);
    let mut work = Continue;
    let document = LexicalDocument::parse(source.clone(), 1 << 20, &mut work).unwrap();
    let mut shared = DecodeSource::new(&document, &mut work).unwrap();
    let mut cache = PaintCache::default();
    let mut ledger = ChargeLedger::new(&source, 1 << 24, 128).unwrap();
    let mut ctx = ResolveContext {
        shared: &mut shared,
        cache: &mut cache,
        ledger: &mut ledger,
    };
    let fill = span(bytes, b"url(#child)");
    let edges = [
        Edge {
            a: Point { x: 0., y: 0. },
            b: Point { x: 10., y: 0. },
        },
        Edge {
            a: Point { x: 10., y: 0. },
            b: Point { x: 10., y: 10. },
        },
        Edge {
            a: Point { x: 10., y: 10. },
            b: Point { x: 0., y: 10. },
        },
        Edge {
            a: Point { x: 0., y: 10. },
            b: Point { x: 0., y: 0. },
        },
    ];
    let first = resolve_fill(
        &mut ctx,
        fill,
        &edges,
        affine_identity(),
        [100., 100.],
        &mut work,
    )
    .unwrap();
    assert_eq!(first.paint.kind, PaintKind::Linear);
    assert_eq!(first.paint.spread, Spread::Reflect);
    assert!(first.paint.object_bounding_box);
    assert_eq!(first.paint.coordinates[2], 1.0);
    assert_eq!(first.paint.gradient_inverse.e, -1.0);
    assert_eq!(first.paint.gradient_inverse.f, -2.0);
    assert_eq!(first.stops[0].rgba, [32, 64, 96, 128]);
    assert_eq!(first.stops[1].rgba, [255, 255, 255, 255]);
    assert_eq!(ctx.cache.entries.len(), 1);
    let moved = Affine {
        a: 0.5,
        d: 0.5,
        ..affine_identity()
    };
    let second = resolve_fill(&mut ctx, fill, &edges, moved, [100., 100.], &mut work).unwrap();
    assert_eq!(ctx.cache.entries.len(), 1);
    assert_eq!(ctx.cache.entries[0].stops.len(), 2);
    assert_eq!(second.stops, first.stops);
    assert!(
        first
            .template
            .as_ref()
            .unwrap()
            .same_template(second.template.as_ref().unwrap())
    );
    assert_eq!(second.paint.user_inverse, moved);
    assert_eq!(second.paint.bounds, [0., 0., 5., 5.]);
    let third = resolve_fill(&mut ctx, fill, &edges, moved, [200., 100.], &mut work).unwrap();
    assert_eq!(ctx.cache.entries.len(), 2);
    assert!(
        !second
            .template
            .as_ref()
            .unwrap()
            .same_template(third.template.as_ref().unwrap())
    );
}

#[test]
fn radial_defaults_and_pattern_classification_are_explicit() {
    let bytes = b"<svg><defs><radialGradient id='r'><stop stop-color='red'/><stop offset='1' stop-color='blue'/></radialGradient><pattern id='p'/></defs><rect fill='url(#p)'/><rect fill='url(#r)'/></svg>";
    let source = owner(bytes);
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
    assert!(matches!(
        classify_fill(&mut ctx, span(bytes, b"url(#p)"), &mut work).unwrap(),
        PaintReference::Pattern(_)
    ));
    let edge = [Edge {
        a: Point { x: 0., y: 0. },
        b: Point { x: 10., y: 10. },
    }];
    let paint = resolve_fill(
        &mut ctx,
        span(bytes, b"url(#r)"),
        &edge,
        affine_identity(),
        [100., 50.],
        &mut work,
    )
    .unwrap();
    assert_eq!(paint.paint.kind, PaintKind::Radial);
    assert_eq!(paint.paint.coordinates[..5], [0.5, 0.5, 0.5, 0.5, 0.5]);
    let mut rgba = [0; 4];
    sample(&paint.paint, &paint.stops, 5., 5., &mut rgba);
    assert_eq!(rgba, [255, 0, 0, 255]);
}

#[test]
fn href_cycles_bad_stop_order_and_empty_edges_fail_without_fake_success() {
    let bytes = b"<svg><defs><linearGradient id='a' href='#b'/><linearGradient id='b' href='#a'><stop offset='.8'/><stop offset='.2'/></linearGradient></defs><rect fill='url(#a)'/></svg>";
    let source = owner(bytes);
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
    let cycle = resolve_fill(
        &mut ctx,
        span(bytes, b"url(#a)"),
        &edge,
        affine_identity(),
        [10., 10.],
        &mut work,
    )
    .unwrap_err();
    assert_eq!(cycle.kind, ErrorKind::Unsupported);
    let malformed = b"<svg><linearGradient id='g'><stop offset='.8'/><stop offset='.2'/></linearGradient><rect fill='url(#g)'/></svg>";
    let source = owner(malformed);
    let doc = LexicalDocument::parse(source.clone(), 1 << 20, &mut work).unwrap();
    let mut shared = DecodeSource::new(&doc, &mut work).unwrap();
    let mut cache = PaintCache::default();
    let mut ledger = ChargeLedger::new(&source, 1 << 24, 64).unwrap();
    let mut ctx = ResolveContext {
        shared: &mut shared,
        cache: &mut cache,
        ledger: &mut ledger,
    };
    assert!(
        resolve_fill(
            &mut ctx,
            span(malformed, b"url(#g)"),
            &edge,
            affine_identity(),
            [10., 10.],
            &mut work
        )
        .is_err()
    );
    let empty_edges: [Edge; 0] = [];
    assert!(
        resolve_fill(
            &mut ctx,
            span(malformed, b"url(#g)"),
            &empty_edges,
            affine_identity(),
            [10., 10.],
            &mut work
        )
        .is_err()
    );
}

#[test]
fn resolver_preserves_fill_offsets_and_cancellation_kind() {
    let bytes = b"<svg><linearGradient id='g'><stop/><stop offset='1'/></linearGradient><rect fill='url(#missing)'/><rect fill='url(#g)'/></svg>";
    let source = owner(bytes);
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
    let fill = span(bytes, b"url(#missing)");
    let edge = [Edge {
        a: Point { x: 0., y: 0. },
        b: Point { x: 1., y: 1. },
    }];
    let error = resolve_fill(
        &mut ctx,
        fill,
        &edge,
        affine_identity(),
        [10., 10.],
        &mut work,
    )
    .unwrap_err();
    assert_eq!(error.kind, ErrorKind::Unsupported);
    assert_eq!(error.offset, fill.offset);

    let mut cancel = Cancel;
    let error = resolve_fill(
        &mut ctx,
        fill,
        &edge,
        affine_identity(),
        [10., 10.],
        &mut cancel,
    )
    .unwrap_err();
    assert_eq!(error.kind, ErrorKind::Canceled);
    assert_eq!(error.offset, fill.offset);

    let mut mid_operation = StopAfter(5);
    let error = resolve_fill(
        &mut ctx,
        span(bytes, b"url(#g)"),
        &edge,
        affine_identity(),
        [10., 10.],
        &mut mid_operation,
    )
    .unwrap_err();
    assert_eq!(error.kind, ErrorKind::Canceled);
    assert_eq!(error.offset, span(bytes, b"url(#g)").offset);
}

#[test]
fn bound_xlink_and_child_stop_override_follow_the_nearest_gradient() {
    let bytes = b"<svg xmlns:xlink='http://www.w3.org/1999/xlink'><defs><linearGradient id='base'><stop stop-color='red'/><stop offset='1' stop-color='blue'/></linearGradient><linearGradient id='g' xlink:href='#base'><stop stop-color='white'/><stop offset='1' stop-color='black'/></linearGradient></defs><rect fill='url(#g)'/></svg>";
    let source = owner(bytes);
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
    let resolved = resolve_fill(
        &mut ctx,
        span(bytes, b"url(#g)"),
        &edge,
        affine_identity(),
        [10., 10.],
        &mut work,
    )
    .unwrap();
    assert_eq!(resolved.stops[0].rgba, [255, 255, 255, 255]);
    assert_eq!(resolved.stops[1].rgba, [0, 0, 0, 255]);

    let unbound = b"<svg><linearGradient id='base'><stop/><stop offset='1'/></linearGradient><linearGradient id='g' xlink:href='#base'/><rect fill='url(#g)'/></svg>";
    let source = owner(unbound);
    let document = LexicalDocument::parse(source.clone(), 1 << 20, &mut work).unwrap();
    let mut shared = DecodeSource::new(&document, &mut work).unwrap();
    let mut cache = PaintCache::default();
    let mut ledger = ChargeLedger::new(&source, 1 << 24, 64).unwrap();
    let mut ctx = ResolveContext {
        shared: &mut shared,
        cache: &mut cache,
        ledger: &mut ledger,
    };
    let error = resolve_fill(
        &mut ctx,
        span(unbound, b"url(#g)"),
        &edge,
        affine_identity(),
        [10., 10.],
        &mut work,
    )
    .unwrap_err();
    assert_eq!(error.kind, ErrorKind::Unsupported);
}

#[test]
fn cache_admits_exact_global_stop_limit_and_rejects_the_next_template() {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"<svg><linearGradient id='g'>");
    for _ in 0..256 {
        bytes.extend_from_slice(b"<stop stop-color='red'/>");
    }
    bytes.extend_from_slice(b"</linearGradient><rect fill='url(#g)'/></svg>");
    let source = owner(&bytes);
    let mut work = Continue;
    let document = LexicalDocument::parse(source.clone(), 1 << 20, &mut work).unwrap();
    let mut shared = DecodeSource::new(&document, &mut work).unwrap();
    let mut cache = PaintCache::default();
    let mut ledger = ChargeLedger::new(&source, 1 << 26, 512).unwrap();
    let mut ctx = ResolveContext {
        shared: &mut shared,
        cache: &mut cache,
        ledger: &mut ledger,
    };
    let edge = [Edge {
        a: Point { x: 0., y: 0. },
        b: Point { x: 1., y: 1. },
    }];
    let fill = span(&bytes, b"url(#g)");
    for index in 0..256 {
        let resolved = resolve_fill(
            &mut ctx,
            fill,
            &edge,
            affine_identity(),
            [10. + index as f64, 10.],
            &mut work,
        )
        .unwrap();
        assert_eq!(resolved.stops.len(), 256);
    }
    assert_eq!(ctx.cache.entries.len(), 256);
    assert!(
        ctx.ledger.stats().live_bytes >= source.capacity() + ctx.cache.storage_bytes().unwrap()
    );
    let error = resolve_fill(
        &mut ctx,
        fill,
        &edge,
        affine_identity(),
        [999., 10.],
        &mut work,
    )
    .unwrap_err();
    assert_eq!(
        error.kind,
        ErrorKind::Limit(vexel_scene::LimitKind::PaintStops)
    );
    assert_eq!(ctx.cache.entries.len(), 256);
}

#[test]
fn one_gradient_cannot_exceed_256_stops() {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"<svg><linearGradient id='g'>");
    for _ in 0..257 {
        bytes.extend_from_slice(b"<stop stop-color='red'/>");
    }
    bytes.extend_from_slice(b"</linearGradient><rect fill='url(#g)'/></svg>");
    let source = owner(&bytes);
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
    let error = resolve_fill(
        &mut ctx,
        span(&bytes, b"url(#g)"),
        &edge,
        affine_identity(),
        [10., 10.],
        &mut work,
    )
    .unwrap_err();
    assert_eq!(
        error.kind,
        ErrorKind::Limit(vexel_scene::LimitKind::PaintStops)
    );
    assert!(ctx.cache.entries.is_empty());
}

#[test]
fn external_references_and_singular_gradient_transforms_are_rejected_at_fill() {
    let external = b"<svg><rect fill='url(https://example.invalid/g)'/></svg>";
    let source = owner(external);
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
    let fill = span(external, b"url(https://example.invalid/g)");
    let error = resolve_fill(
        &mut ctx,
        fill,
        &edge,
        affine_identity(),
        [10., 10.],
        &mut work,
    )
    .unwrap_err();
    assert_eq!(error.kind, ErrorKind::Unsupported);
    assert_eq!(error.offset, fill.offset);

    let singular = b"<svg><linearGradient id='g' gradientTransform='scale(0)'><stop/><stop offset='1'/></linearGradient><rect fill='url(#g)'/></svg>";
    let source = owner(singular);
    let document = LexicalDocument::parse(source.clone(), 1 << 20, &mut work).unwrap();
    let mut shared = DecodeSource::new(&document, &mut work).unwrap();
    let mut cache = PaintCache::default();
    let mut ledger = ChargeLedger::new(&source, 1 << 24, 64).unwrap();
    let mut ctx = ResolveContext {
        shared: &mut shared,
        cache: &mut cache,
        ledger: &mut ledger,
    };
    let error = resolve_fill(
        &mut ctx,
        span(singular, b"url(#g)"),
        &edge,
        affine_identity(),
        [10., 10.],
        &mut work,
    )
    .unwrap_err();
    assert_eq!(error.kind, ErrorKind::Unsupported);
}

#[test]
fn current_color_without_a_computed_color_uses_opaque_black() {
    let bytes = b"<svg><linearGradient id='g'><stop stop-color='currentColor'/><stop offset='1' stop-color='white'/></linearGradient><rect fill='url(#g)'/></svg>";
    let source = owner(bytes);
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
    let resolved = resolve_fill(
        &mut ctx,
        span(bytes, b"url(#g)"),
        &edge,
        affine_identity(),
        [10., 10.],
        &mut work,
    )
    .unwrap();
    assert_eq!(resolved.stops[0].rgba, [0, 0, 0, 255]);
}

#[test]
fn cached_gradient_scan_original_scale_and_css_stop_cascade() {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(
        b"<svg><defs><linearGradient id='g'><stop style='stop-color:#f00;stop-opacity:1'/>",
    );
    bytes.extend_from_slice(b"<stop class='end' offset='1'/>");
    for _ in 0..4096 {
        bytes.extend_from_slice(b"<!-- padding -->");
    }
    bytes.extend_from_slice(b"</linearGradient></defs><style>.end{stop-color:#00f;stop-opacity:1} stop{color:#f00}</style>");
    bytes.extend_from_slice(b"<rect width='1' height='1' fill='url(#g)'/>");
    for _ in 1..512 {
        bytes.extend_from_slice(b"<rect width='1' height='1' fill='url(#g)'/>");
    }
    bytes.extend_from_slice(b"</svg>");
    let owner = owner(&bytes);
    let mut work = Continue;
    let document = LexicalDocument::parse(owner.clone(), 1 << 20, &mut work).unwrap();
    let mut shared = DecodeSource::new(&document, &mut work).unwrap();
    let mut cache = PaintCache::default();
    let mut ledger = ChargeLedger::new(&owner, 1 << 26, 64).unwrap();
    let mut ctx = ResolveContext {
        shared: &mut shared,
        cache: &mut cache,
        ledger: &mut ledger,
    };
    let fill = span(&bytes, b"url(#g)");
    let edges = [Edge {
        a: Point { x: 0., y: 0. },
        b: Point { x: 1., y: 1. },
    }];
    let first = resolve_fill(
        &mut ctx,
        fill,
        &edges,
        affine_identity(),
        [8., 8.],
        &mut work,
    )
    .unwrap();
    assert_eq!(first.stops[0].rgba, [255, 0, 0, 255]);
    assert_eq!(first.stops[1].rgba, [0, 0, 255, 255]);
    for _ in 1..512 {
        let result = resolve_fill(
            &mut ctx,
            fill,
            &edges,
            affine_identity(),
            [8., 8.],
            &mut work,
        )
        .unwrap();
        assert!(
            first
                .template
                .as_ref()
                .unwrap()
                .same_template(result.template.as_ref().unwrap())
        );
    }
    assert_eq!(ctx.cache.entries.len(), 1);
    assert_eq!(ctx.cache.entries[0].stops.len(), 2);
    assert!(ctx.cache.token_visits >= 4096);
    assert!(ctx.cache.token_visits <= 8 * (4096 + 512 + 64));
}
