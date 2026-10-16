use std::sync::Arc;
use vexel_base::{Stop, WorkControl, WorkKind};
use vexel_scene::*;
use vexel_svg_engine::SvgEngineSession;

#[derive(Default)]
struct Control;
impl WorkControl for Control {
    fn charge(&mut self, _: WorkKind, _: u64) -> std::result::Result<(), Stop> {
        Ok(())
    }
    fn checkpoint(&mut self) -> std::result::Result<(), Stop> {
        Ok(())
    }
}

fn decode(text: &str) -> SceneGraph {
    let owner = Arc::new(text.as_bytes().to_vec());
    let mut control = Control;
    let document = LexicalDocument::parse(owner.clone(), 1 << 20, &mut control).unwrap();
    let mut source = DecodeSource::new(&document, &mut control).unwrap();
    let mut job = RasterJob::new(ChargeLedger::new(&owner, 1 << 24, 65536).unwrap()).unwrap();
    let mut provider = UnsupportedProvider;
    let mut session = SvgEngineSession::from_source(
        &document,
        &mut source,
        8,
        DecodeOptions {
            limits: Limits::default(),
            system_language: b"en",
            media: Some(&mut provider),
        },
        &mut control,
    )
    .unwrap();
    loop {
        match session.step(1, &job, &mut control).unwrap() {
            vexel_svg_decode::TraversalDecodeEvent::Progress { charged_units } => {
                assert!(charged_units > 0)
            }
            vexel_svg_decode::TraversalDecodeEvent::Complete => break,
            vexel_svg_decode::TraversalDecodeEvent::NeedPattern(_) => panic!("unexpected pattern"),
        }
    }
    let graph = match session.finish(&mut control).unwrap() {
        DecodedScene::Drawable(graph) => graph,
        DecodedScene::Hidden(_) => panic!("expected drawable"),
    };
    assert!(Arc::ptr_eq(graph.source_owner(), &owner));
    assert_eq!(graph.source(), text.as_bytes());
    let _ = &mut job;
    graph
}

#[test]
fn real_decoder_publishes_vector_shapes_transform_solid_channels_and_source() {
    let graph = decode(
        "<svg width='16' height='16'><g transform='translate(2 3)'><rect x='1' y='2' width='4' height='5' fill='#ff0000' stroke='#0000ff' stroke-width='2'/><circle cx='8' cy='8' r='2' fill='green'/></g></svg>",
    );
    let scene = graph.root();
    assert_eq!(scene.shapes.len(), 2);
    assert!(!scene.transforms.is_empty());
    assert!(
        scene
            .shapes
            .iter()
            .any(|s| s.filled && s.stroke.width == 2.0)
    );
    assert!(!scene.edge_colors.is_empty());
}

#[test]
fn real_decoder_resolves_local_gradient_through_cached_paint_path() {
    let graph = decode(
        "<svg width='16' height='16'><defs><linearGradient id='g'><stop offset='0' stop-color='#f00'/><stop offset='1' stop-color='#00f'/></linearGradient></defs><rect width='8' height='8' fill='url(#g)'/></svg>",
    );
    let scene = graph.root();
    assert_eq!(scene.shapes.len(), 1);
    let fill = scene.shapes[0].fill_paint.expect("gradient fill published");
    assert_eq!(scene.fill_paints[fill].kind, PaintKind::Linear);
    assert_eq!(scene.fill_paints[fill].stops.count, 2);
}

#[test]
fn actual_clip_planner_completes_supported_clip_at_zero() {
    let text = "<svg width='16' height='16'><defs><clipPath id='c'><path d='M0 0L8 0L8 8Z'/></clipPath></defs><rect id='owner' width='12' height='12' clip-path='url(#c)'/></svg>";
    let owner = Arc::new(text.as_bytes().to_vec());
    let mut control = Control;
    let doc = LexicalDocument::parse(owner.clone(), 1 << 20, &mut control).unwrap();
    let mut source = DecodeSource::new(&doc, &mut control).unwrap();
    let job = RasterJob::new(ChargeLedger::new(&owner, 1 << 24, 65536).unwrap()).unwrap();
    let mut session = SvgEngineSession::from_source(
        &doc,
        &mut source,
        8,
        DecodeOptions {
            limits: Limits::default(),
            system_language: b"en",
            media: None,
        },
        &mut control,
    )
    .unwrap();
    loop {
        match session.step(64, &job, &mut control) {
            Ok(vexel_svg_decode::TraversalDecodeEvent::Progress { .. }) => {}
            Ok(vexel_svg_decode::TraversalDecodeEvent::Complete) => break,
            Ok(vexel_svg_decode::TraversalDecodeEvent::NeedPattern(_)) => {
                panic!("unexpected pattern")
            }
            Err(error) => {
                panic!("supported clip failed: {error:?}");
            }
        }
    }
    assert!(session.operations().clip_plan_was_attempted());
    assert!(session.operations().has_successful_clip_plan());
    let DecodedScene::Drawable(graph) = session.finish(&mut control).unwrap() else {
        panic!("expected clip scene")
    };
    assert_eq!(graph.root().shapes.len(), 1);
}

#[test]
fn engine_operations_plan_real_clip_through_definition_planner() {
    use vexel_geometry::affine_identity;
    use vexel_scene::DefinitionPlanner as _;
    use vexel_svg_css::CssPaint;

    let text = "<svg width='16' height='16'><defs><clipPath id='c'><path d='M0 0L8 0L8 8Z'/></clipPath></defs><rect id='owner' width='12' height='12' clip-path='url(#c)'/></svg>";
    let owner = Arc::new(text.as_bytes().to_vec());
    let mut control = Control;
    let document = LexicalDocument::parse(owner.clone(), 1 << 20, &mut control).unwrap();
    let mut source = DecodeSource::new(&document, &mut control).unwrap();
    let animation = vexel_svg_animation::parse_owned(&owner, document.tokens()).unwrap();
    let mut builder = SceneBuilder::provisional(owner.clone(), 8, Limits::default()).unwrap();
    let tokens = document.tokens();
    let root = tokens
        .iter()
        .position(|t| t.local_name.slice(text.as_bytes()) == Some(b"svg"))
        .unwrap();
    builder
        .initialize_root(&mut source, ElementId(root), &mut control)
        .unwrap();
    builder.finalize_decode(animation, 8, &mut control).unwrap();
    let host_offset = text.find("<rect id='owner'").unwrap();
    let host = tokens
        .iter()
        .position(|t| t.source.offset == host_offset)
        .unwrap();
    let definition_offset = text.find("<clipPath").unwrap();
    let definition = tokens
        .iter()
        .position(|t| t.source.offset == definition_offset)
        .unwrap();
    let geometry = vexel_svg_decode::element_geometry(
        text.as_bytes(),
        tokens,
        host,
        affine_identity(),
        [16.0, 16.0],
    )
    .unwrap();
    let mut shape = Shape::new(SourceMap {
        element: vexel_base::Span {
            start: tokens[host].source.offset,
            end: tokens[host].source.end().unwrap(),
        },
        transform: affine_identity(),
        transform_node: None,
    });
    shape.edges.count = geometry.path.edges.len();
    shape.subpaths.count = geometry.path.subpaths.len();
    shape.filled = geometry.filled;
    shape.stroke_closed = geometry.stroke_closed;
    let edge_colors = vec![[0, 0, 0, 255]; geometry.path.edges.len()];
    let scene = builder.root_id();
    let target = builder
        .append_shape(
            scene,
            ShapeInput {
                shape,
                path: geometry.path,
                edge_colors,
            },
            &mut control,
        )
        .unwrap();
    let draft = builder.begin_draft(scene).unwrap();
    let definition_owner = builder
        .definition_owner(
            &draft,
            &source,
            ElementId(host),
            DefinitionTarget::Shape(target),
            &mut control,
        )
        .unwrap();
    let mut active = ActiveDefinitions::default();
    let mut ledger = ChargeLedger::new(&owner, 1 << 24, 4096).unwrap();
    let mut operations = vexel_svg_engine::SvgEngineOperations::new();
    let planned = {
        let mut context = DefinitionContext {
            shared: &mut source,
            builder: &mut builder,
            scene,
            active: &mut active,
            ledger: &mut ledger,
        };
        operations
            .plan_owned_definition(
                &mut context,
                DefinitionKind::Clip,
                ElementId(definition),
                definition_owner,
                affine_identity(),
                [0.0, 0.0, 12.0, 12.0],
                &CssPaint::initial(),
                &mut control,
            )
            .unwrap()
    };
    let ProductionDefinitionPlan::Definition(plan) = planned else {
        panic!("clip planner must return a definition plan")
    };
    assert!(operations.has_successful_clip_plan());
    assert!(matches!(plan.descriptor, DefinitionDescriptor::Clip(_)));
    assert_eq!(plan.expansions.len(), 1);
    assert_eq!(
        plan.expansions[0].flavor,
        DecodeFlavor::Clip { clip_rule: true }
    );
}

fn source_element_offset(text: &str, local_name: &[u8]) -> usize {
    let mut control = Control;
    let document =
        LexicalDocument::parse(Arc::new(text.as_bytes().to_vec()), 1 << 20, &mut control).unwrap();
    document
        .tokens()
        .iter()
        .find(|token| token.local_name.slice(text.as_bytes()) == Some(local_name))
        .unwrap()
        .source
        .offset
}

struct UnsupportedProvider;
impl MediaProvider for UnsupportedProvider {
    fn layout_text(
        &mut self,
        _: TextRequest<'_>,
        _: &mut dyn WorkControl,
    ) -> std::result::Result<TextLayout, MediaError> {
        Err(MediaError::Unsupported)
    }
    fn decode_image(
        &mut self,
        _: &[u8],
        _: &mut dyn WorkControl,
    ) -> std::result::Result<ImagePixels, MediaError> {
        Err(MediaError::Unsupported)
    }
}

fn decode_error(text: &str) -> Diagnostic {
    let owner = Arc::new(text.as_bytes().to_vec());
    let mut control = Control;
    let document = LexicalDocument::parse(owner.clone(), 1 << 20, &mut control).unwrap();
    let mut source = DecodeSource::new(&document, &mut control).unwrap();
    let job = RasterJob::new(ChargeLedger::new(&owner, 1 << 24, 65536).unwrap()).unwrap();
    let mut provider = UnsupportedProvider;
    let mut session = SvgEngineSession::from_source(
        &document,
        &mut source,
        8,
        DecodeOptions {
            limits: Limits::default(),
            system_language: b"en",
            media: Some(&mut provider),
        },
        &mut control,
    )
    .unwrap();
    loop {
        match session.step(64, &job, &mut control) {
            Err(error) => return error,
            Ok(vexel_svg_decode::TraversalDecodeEvent::Progress { .. }) => {}
            Ok(vexel_svg_decode::TraversalDecodeEvent::Complete) => {
                panic!("unsupported media unexpectedly completed")
            }
            Ok(vexel_svg_decode::TraversalDecodeEvent::NeedPattern(_)) => {
                panic!("unexpected pattern")
            }
        }
    }
}

#[test]
fn actual_text_request_unsupported_error_points_to_text_element() {
    let input = "<svg width='16' height='16'><g/><text x='1' y='2'>hi</text></svg>";
    let expected = source_element_offset(input, b"text");
    let error = decode_error(input);
    assert_eq!(error.kind, ErrorKind::Unsupported);
    assert!(expected > 0);
    assert_eq!(error.offset, expected);
}

#[test]
fn actual_image_request_unsupported_error_points_to_image_element() {
    let input = "<svg width='16' height='16'><g/><image href='data:image/png;base64,AA==' x='1' y='2' width='3' height='4'/></svg>";
    let expected = source_element_offset(input, b"image");
    let error = decode_error(input);
    assert_eq!(error.kind, ErrorKind::Unsupported);
    assert!(expected > 0);
    assert_eq!(error.offset, expected);
}
