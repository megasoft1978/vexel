use vexel_svg_decode::TraversalDecodeEvent as DecodeEvent;
mod support;
use std::sync::Arc;
use support::*;
use vexel_scene::*;
use vexel_svg_decode::{ContractBlocker, Decoder, DecoderState};
use vexel_xml::{TokenKind, XmlSpan};
/// Explicit fixture planner builds actual source-only expansion plans. It does
/// not stand in for the downstream production definition algorithm.
struct FixturePlans;
impl DecodeOperations for FixturePlans {
    fn resolve_paint(
        &mut self,
        c: &mut ResolveContext<'_, '_>,
        v: XmlSpan,
        e: &[Edge],
        t: Affine,
        p: [f64; 2],
        w: &mut dyn WorkControl,
    ) -> Result<ResolvedPaint> {
        RejectOperations.resolve_paint(c, v, e, t, p, w)
    }
    fn plan_definition<'s, 'c>(
        &mut self,
        c: &mut DefinitionContext<'s, 'c>,
        kind: DefinitionKind,
        id: ElementId,
        owner: ShapeId,
        transform: Affine,
        _: [f64; 4],
        _: &CssPaint<'s>,
        w: &mut dyn WorkControl,
    ) -> Result<DefinitionPlan<'s>> {
        c.validate()?;
        assert!(c.builder.traversal_scene(c.scene)?.shape(owner).is_some());
        let tokens = c.shared.tokens();
        let token = tokens[id.0];
        let end = tokens
            .iter()
            .enumerate()
            .skip(id.0 + 1)
            .find(|(_, v)| v.kind == TokenKind::EndElement && v.depth == token.depth)
            .unwrap()
            .0;
        let first = tokens
            .iter()
            .enumerate()
            .take(end)
            .skip(id.0 + 1)
            .find(|(_, v)| matches!(v.kind, TokenKind::StartElement | TokenKind::EmptyElement))
            .unwrap()
            .0;
        let style = c.shared.compute_style(id, None, w)?;
        let descriptor = match kind {
            DefinitionKind::Clip => DefinitionDescriptor::Clip(Clip {
                geometry: ClipGeometry {
                    edges: vec![],
                    shapes: vec![],
                },
                parent: None,
                owner_node: None,
                owner_before_suffix: false,
                object_bounding_box: false,
                bounds: [0.0, 0.0, 8.0, 8.0],
                user_inverse: vexel_svg_decode::affine_inverse(transform).unwrap(),
            }),
            DefinitionKind::Pattern => DefinitionDescriptor::Pattern(PatternDescriptor {
                definition: id,
                width: 2,
                height: 2,
                tile_width: 2.0,
                tile_height: 2.0,
                content_transform: transform,
                error_offset: token.source.offset,
            }),
            _ => panic!("unexpected fixture definition"),
        };
        Ok(DefinitionPlan {
            descriptor,
            expansions: vec![Expansion {
                range: TableRange {
                    first,
                    count: end - first,
                },
                transform,
                viewport: [8.0, 8.0],
                style,
                flavor: if kind == DefinitionKind::Clip {
                    DecodeFlavor::Clip { clip_rule: false }
                } else {
                    DecodeFlavor::Pattern
                },
                post: PostOperation::None,
            }],
            post: PostOperation::None,
        })
    }
    fn decode_text(
        &mut self,
        r: TextDecodeRequest<'_, '_>,
        b: &mut SceneBuilder,
        w: &mut dyn WorkControl,
    ) -> Result<bool> {
        RejectOperations.decode_text(r, b, w)
    }
    fn decode_image(
        &mut self,
        r: ImageDecodeRequest<'_, '_>,
        b: &mut SceneBuilder,
        w: &mut dyn WorkControl,
    ) -> Result<ShapeId> {
        RejectOperations.decode_image(r, b, w)
    }
}
const PATTERN: &str = "<svg width=\"8\" height=\"8\"><defs><pattern id=\"p\"><rect width=\"2\" height=\"2\" fill=\"red\"/></pattern></defs><rect width=\"8\" height=\"8\" fill=\"url(#p)\"/></svg>";
#[test]
fn actual_clip_child_traversal_sealing_and_parent_link() {
    let graph = decode_with("<svg width=\"8\" height=\"8\"><defs><clipPath id=\"c\"><rect width=\"2\" height=\"2\"/></clipPath></defs><rect width=\"8\" height=\"8\" clip-path=\"url(#c)\"/></svg>",b"en",&mut FixturePlans).unwrap();
    let view = graph.root();
    assert_eq!(view.shapes.len(), 1);
    let clip = view.clip(view.shapes[0].clip.unwrap()).unwrap();
    assert_eq!(clip.geometry.edges.len(), 4);
    assert_eq!(clip.geometry.shapes.len(), 1);
    assert_eq!(clip.geometry.shapes[0].edges.count, 4);
}
fn document() -> LexicalDocument {
    LexicalDocument::parse(
        Arc::new(PATTERN.as_bytes().to_vec()),
        256 * 1024 * 1024,
        &mut Run::default(),
    )
    .unwrap()
}
#[test]
fn actual_pattern_child_ticket_wrong_dimensions_abort() {
    let doc = document();
    let mut run = Run::default();
    let mut shared = DecodeSource::new(&doc, &mut run).unwrap();
    let mut d = Decoder::from_source(
        &doc,
        &mut shared,
        1,
        DecodeOptions {
            limits: Limits::default(),
            system_language: b"en",
            media: None,
        },
        &mut run,
    )
    .unwrap();
    let ticket = loop {
        match d.step_traversal(256, &mut FixturePlans, &mut run).unwrap() {
            DecodeEvent::NeedPattern(r) => {
                assert_eq!((r.width, r.height), (2, 2));
                assert_eq!(r.child.shapes.len(), 1);
                assert_eq!(r.child.edges.len(), 4);
                assert_eq!(r.child.edge_colors[0], [255, 0, 0, 255]);
                break r.ticket;
            }
            DecodeEvent::Progress { charged_units } => assert!(charged_units > 0),
            _ => panic!("pattern not requested"),
        }
    };
    assert_eq!(
        d.resume_pattern(ticket, Ok(vexel_raster::Tile::new(1, 2).unwrap()), &mut run)
            .unwrap_err()
            .kind,
        ErrorKind::InvalidArgument
    );
    assert_eq!(d.state(), DecoderState::Failed);
    assert!(d.finish(&mut run).is_err());
}
#[test]
fn exact_pattern_tile_reaches_named_handoff_blocker_and_rolls_back() {
    let doc = document();
    let mut run = Run::default();
    let mut shared = DecodeSource::new(&doc, &mut run).unwrap();
    let mut d = Decoder::from_source(
        &doc,
        &mut shared,
        1,
        DecodeOptions {
            limits: Limits::default(),
            system_language: b"en",
            media: None,
        },
        &mut run,
    )
    .unwrap();
    let ticket = loop {
        if let DecodeEvent::NeedPattern(r) =
            d.step_traversal(256, &mut FixturePlans, &mut run).unwrap()
        {
            break r.ticket;
        }
    };
    let e = d
        .resume_pattern(ticket, Ok(vexel_raster::Tile::new(2, 2).unwrap()), &mut run)
        .unwrap_err();
    assert_eq!(e.kind, ErrorKind::InvalidArgument);
    assert_eq!(
        d.blocked_contract(),
        Some(ContractBlocker::PatternPaintHandoff)
    );
    assert_eq!(d.state(), DecoderState::Failed);
    assert!(
        d.resume_pattern(ticket, Ok(vexel_raster::Tile::new(2, 2).unwrap()), &mut run)
            .is_err()
    );
}
