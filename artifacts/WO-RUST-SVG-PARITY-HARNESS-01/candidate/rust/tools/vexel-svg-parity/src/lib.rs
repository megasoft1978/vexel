#![cfg(test)]

use std::sync::Arc;
use vexel_base::{Span, WorkControl};
use vexel_geometry::affine_identity;
use vexel_scene::LexicalDocument;
use vexel_scene::*;
use vexel_svg_css::Property;
use vexel_svg_definitions::DefinitionPlanner;
use vexel_xml::{TokenKind, XmlSpan};

struct Run;
impl WorkControl for Run {
    fn charge(&mut self, _: WorkKind, _: u64) -> std::result::Result<(), Stop> {
        Ok(())
    }
    fn checkpoint(&mut self) -> std::result::Result<(), Stop> {
        Ok(())
    }
}

fn source_fixture(
    bytes: Vec<u8>,
) -> (
    SourceOwner,
    LexicalDocument,
    SceneBuilder,
    SceneId,
    Vec<(ShapeId, ElementId)>,
    ChargeLedger,
) {
    let source = Arc::new(bytes);
    let mut run = Run;
    let doc = LexicalDocument::parse(source.clone(), 2 * 1024 * 1024, &mut run).unwrap();
    let animation = vexel_svg_animation::parse_owned(&source, doc.tokens()).unwrap();
    let viewport = Viewport {
        width: 1,
        height: 1,
        view_x: 0.,
        view_y: 0.,
        view_width: 1.,
        view_height: 1.,
        aspect: vexel_svg_values::Aspect {
            align_x: 0.5,
            align_y: 0.5,
            none: false,
            slice: false,
            set: true,
        },
    };
    let mut limits = Limits::default();
    limits.source_bytes = 2 * 1024 * 1024;
    limits.reserved_bytes = 32 * 1024 * 1024;
    let mut builder = SceneBuilder::new(source.clone(), animation, 1, viewport, limits).unwrap();
    let scene = builder.root_id();
    let tokens = doc.tokens();
    let owner_elements: Vec<usize> = tokens
        .iter()
        .enumerate()
        .filter_map(|(i, t)| {
            let is_owner = matches!(t.kind, TokenKind::StartElement | TokenKind::EmptyElement)
                && t.local_name.slice(&source).unwrap_or_default() == b"path"
                && attribute(&source, tokens, t, b"id").is_some_and(|s| {
                    s.slice(&source)
                        .is_some_and(|id| id == b"owner" || id.starts_with(b"owner-"))
                });
            is_owner.then_some(i)
        })
        .collect();
    assert!(
        !owner_elements.is_empty(),
        "fixture must contain a path owner"
    );
    let mut owners = Vec::with_capacity(owner_elements.len());
    for &owner_element in &owner_elements {
        let d = attribute(&source, tokens, &tokens[owner_element], b"d").unwrap();
        let path = vexel_svg_path::parse_path(
            &source,
            Span {
                start: d.offset,
                end: d.offset + d.length,
            },
            affine_identity(),
            Default::default(),
        )
        .unwrap();
        let shape = Shape::new(SourceMap {
            element: Span {
                start: tokens[owner_element].source.offset,
                end: tokens[owner_element].source.offset + tokens[owner_element].source.length,
            },
            transform: affine_identity(),
            transform_node: None,
        });
        let owner = builder
            .append_shape(
                scene,
                ShapeInput {
                    shape,
                    path,
                    edge_colors: vec![[0, 0, 0, 255]; 3],
                },
                &mut run,
            )
            .unwrap();
        owners.push((owner, ElementId(owner_element)));
    }
    let ledger = ChargeLedger::new(&source, 32 * 1024 * 1024, 4096).unwrap();
    (source, doc, builder, scene, owners, ledger)
}
fn attribute(
    source: &[u8],
    tokens: &[vexel_xml::XmlToken],
    element: &vexel_xml::XmlToken,
    name: &[u8],
) -> Option<XmlSpan> {
    let i = tokens.iter().position(|t| std::ptr::eq(t, element))?;
    tokens
        .get(i + 1..)?
        .iter()
        .take_while(|t| t.kind == TokenKind::Attribute)
        .find(|t| t.local_name.slice(source) == Some(name))
        .map(|t| t.value)
}
fn named(source: &[u8], tokens: &[vexel_xml::XmlToken], tag: &[u8], id: &[u8]) -> ElementId {
    ElementId(
        tokens
            .iter()
            .enumerate()
            .find_map(|(i, t)| {
                (matches!(t.kind, TokenKind::StartElement | TokenKind::EmptyElement)
                    && t.local_name.slice(source) == Some(tag)
                    && attribute(source, tokens, t, b"id")
                        .is_some_and(|v| v.slice(source) == Some(id)))
                .then_some(i)
            })
            .unwrap(),
    )
}

/// Original reference predicates: 4,096 comments, 512 rectangles, two stops.
#[test]
fn cached_gradient_scan_reference_predicate() {
    let mut bytes = br#"<svg width="8" height="8"><defs><linearGradient id="g"><stop stop-color="red"/><stop offset="1" stop-color="blue"/>"#.to_vec();
    for _ in 0..4096 {
        bytes.extend_from_slice(b"<!-- padding -->");
    }
    bytes.extend_from_slice(b"</linearGradient></defs>");
    for _ in 0..512 {
        bytes.extend_from_slice(b"<rect width=\"1\" height=\"1\" fill=\"url(#g)\"/>");
    }
    bytes.extend_from_slice(b"</svg>");
    let original = bytes.clone();
    let source = Arc::new(bytes);
    let mut run = Run;
    let doc = LexicalDocument::parse(source.clone(), 2 * 1024 * 1024, &mut run).unwrap();
    let mut shared = DecodeSource::new(&doc, &mut run).unwrap();
    let mut cache = PaintCache::default();
    let mut ledger = ChargeLedger::new(&source, 32 * 1024 * 1024, 4096).unwrap();
    let mut rects = 0;
    let mut stop_count = 0;
    let comment_count = doc
        .tokens()
        .iter()
        .filter(|t| t.kind == TokenKind::Comment)
        .count();
    assert_eq!(comment_count, 4096);
    for (i, t) in doc.tokens().iter().enumerate() {
        if t.kind == TokenKind::EmptyElement && t.local_name.slice(&source) == Some(b"stop") {
            stop_count += 1;
        }
        if !matches!(t.kind, TokenKind::StartElement | TokenKind::EmptyElement)
            || t.local_name.slice(&source) != Some(b"rect")
        {
            continue;
        }
        rects += 1;
        let style = shared.compute_style(ElementId(i), None, &mut run).unwrap();
        let span = style.value(Property::Fill).unwrap().span;
        let edges = [vexel_geometry::Edge {
            a: vexel_geometry::Point { x: 0., y: 0. },
            b: vexel_geometry::Point { x: 1., y: 1. },
        }];
        let resolved = vexel_svg_paint::resolve_cached_fill(
            &mut shared,
            &mut cache,
            &mut ledger,
            span,
            &edges,
            affine_identity(),
            [8., 8.],
            &mut run,
        )
        .unwrap();
        assert_eq!(resolved.stops.len(), 2);
    }
    assert_eq!(rects, 512);
    assert_eq!(stop_count, 2);
    assert_eq!(cache.entries.len(), 1);
    assert_eq!(cache.entries[0].stops.len(), 2);
    println!(
        "gradient comments={comment_count} rects={rects} stops={stop_count} token_visits={}",
        cache.token_visits
    );
    assert!(
        (4096..=37376).contains(&cache.token_visits),
        "{}",
        cache.token_visits
    );
    assert_eq!(source.as_slice(), original.as_slice());
}

fn clip_bytes(child: usize, declarations: usize) -> Vec<u8> {
    let prefix=b"<svg width=\"1\" height=\"1\"><defs><clipPath id=\"c\" clipPathUnits=\"objectBoundingBox\">";
    let cp = b"<!--";
    let middle = b"--><rect width=\"1\" height=\"1\"/>";
    let suffix = b"</clipPath></defs>";
    let mut out = Vec::new();
    out.extend_from_slice(prefix);
    out.extend_from_slice(cp);
    out.resize(out.len() + child - cp.len() - middle.len(), b'b');
    out.extend_from_slice(middle);
    out.extend_from_slice(suffix);
    for index in 0..declarations {
        out.extend_from_slice(
            format!("<path id=\"owner-{index}\" d=\"M0 0L1 0L1 1Z\" clip-path=\"url(#c)\"/>")
                .as_bytes(),
        )
    }
    out.extend_from_slice(b"</svg>");
    out
}
fn plan_clip(
    bytes: Vec<u8>,
    planner: &mut DefinitionPlanner,
) -> (std::result::Result<(), Diagnostic>, Vec<u8>) {
    let (source, doc, mut builder, scene, owners, mut ledger) = source_fixture(bytes);
    let before = source.to_vec();
    let mut run = Run;
    let mut shared = DecodeSource::new(&doc, &mut run).unwrap();
    let (owner, owner_element) = owners[0];
    let style = shared.compute_style(owner_element, None, &mut run).unwrap();
    let definition = named(&source, doc.tokens(), b"clipPath", b"c");
    let mut active = ActiveDefinitions::default();
    let mut context = DefinitionContext {
        shared: &mut shared,
        builder: &mut builder,
        scene,
        active: &mut active,
        ledger: &mut ledger,
    };
    let result = planner
        .plan(
            &mut context,
            DefinitionKind::Clip,
            definition,
            owner,
            affine_identity(),
            [0., 0., 1., 1.],
            &style,
            &mut run,
        )
        .map(|_| ());
    assert_eq!(source.as_slice(), before);
    (result, before)
}

fn plan_repeated(bytes: Vec<u8>, count: usize) -> std::result::Result<(), Diagnostic> {
    let (source, doc, mut builder, scene, owners, mut ledger) = source_fixture(bytes);
    let before = source.to_vec();
    let mut run = Run;
    let mut shared = DecodeSource::new(&doc, &mut run).unwrap();
    let definition = named(&source, doc.tokens(), b"clipPath", b"c");
    let mut active = ActiveDefinitions::default();
    let mut planner = DefinitionPlanner::new();
    assert_eq!(owners.len(), count, "one source owner per declaration");
    let mut context = DefinitionContext {
        shared: &mut shared,
        builder: &mut builder,
        scene,
        active: &mut active,
        ledger: &mut ledger,
    };
    let mut outcome = Ok(());
    for (owner, owner_element) in owners {
        let style = context
            .shared
            .compute_style(owner_element, None, &mut run)
            .unwrap();
        match planner.plan(
            &mut context,
            DefinitionKind::Clip,
            definition,
            owner,
            affine_identity(),
            [0., 0., 1., 1.],
            &style,
            &mut run,
        ) {
            Ok(_) => {}
            Err(error) => {
                outcome = Err(error);
                break;
            }
        }
    }
    assert_eq!(source.as_slice(), before);
    outcome
}

/// Original C reference clip boundary predicate: 65,536 succeeds; +1 is a typed limit.
#[test]
fn clip_source_and_aggregate_bounds_reference_predicate() {
    let mut p = DefinitionPlanner::new();
    let (ok, src) = plan_clip(clip_bytes(65536, 1), &mut p);
    assert_eq!(ok, Ok(()));
    assert!(!src.is_empty());
    let (over, src) = plan_clip(clip_bytes(65537, 1), &mut p);
    let e = over.unwrap_err();
    assert_eq!(e.kind, ErrorKind::Limit(LimitKind::References));
    assert_eq!(
        e.offset,
        src.windows(b"clipPath".len())
            .position(|w| w == b"clipPath")
            .unwrap()
    );
    println!(
        "clip single exact=65536 success; over=65537 error={:?} offset={}",
        e.kind, e.offset
    );
    let wrapper = b"<svg width=\"1\" height=\"1\" viewBox=\"0 0 1 1\">".len() + b"</svg>".len() + 1;
    let child = 65536 - wrapper;
    assert_eq!((wrapper + child) * 8, 512 * 1024);
    let aggregate = clip_bytes(child, 8);
    assert_eq!(plan_repeated(aggregate.clone(), 8), Ok(()));
    let total = (wrapper + child) * 8;
    println!("clip aggregate wrapper={wrapper} child={child} total={total} eight=success");
    let aggregate = clip_bytes(child, 9);
    let e = plan_repeated(aggregate, 9).unwrap_err();
    assert_eq!(e.kind, ErrorKind::Limit(LimitKind::References));
    assert_eq!(e.offset, 33);
    println!("clip aggregate nine error={:?} offset={}", e.kind, e.offset);
}
