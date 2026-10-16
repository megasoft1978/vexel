//! Explicit production paint-channel handoff through real pattern publication.
mod support;
use support::*;
use std::sync::Arc;
use vexel_geometry::affine_identity as identity;
use vexel_scene::*;
use vexel_svg_css::Property;

fn element(doc: &LexicalDocument, name: &[u8]) -> ElementId {
    ElementId(doc.tokens().iter().position(|t| t.local_name.slice(doc.source()) == Some(name)).unwrap())
}

struct ChannelPlanner;
impl PaintChannelDefinitionPlanner for ChannelPlanner {
    fn plan_owned_pattern_definition<'src>(
        &mut self,
        ctx: &mut DefinitionContext<'src, '_>,
        element: ElementId,
        owner: DefinitionOwner,
        channel: PaintChannel,
        _: Affine,
        bounds: [f64; 4],
        style: &vexel_svg_css::CssPaint<'src>,
        _: &mut dyn WorkControl,
    ) -> Result<ProductionDefinitionPlan<'src>> {
        owner.validate(ctx)?;
        let property = match channel {
            PaintChannel::Fill => Property::FillOpacity,
            PaintChannel::Stroke => Property::StrokeOpacity,
        };
        let opacity = style.value(property).map_or(Ok(1.0), |value| {
            let bytes = value.span.slice(ctx.shared.source()).ok_or(Diagnostic {
                kind: ErrorKind::InvalidArgument,
                offset: value.span.offset,
            })?;
            std::str::from_utf8(bytes)
                .ok()
                .and_then(|value| value.parse::<f64>().ok())
                .ok_or(Diagnostic { kind: ErrorKind::InvalidArgument, offset: value.span.offset })
        })?;
        let DefinitionTarget::Shape(shape) = owner.target() else { unreachable!() };
        let source_map = ctx.builder.scene(ctx.scene)?.shape(shape).unwrap().source;
        let mapping = PatternMapping {
            tile_origin: [0., 0.], user_to_base: identity(), pattern_transform: identity(),
            user_inverse: identity(), viewport: [100., 100.], bounds, source_map, opacity,
        };
        let descriptor = PatternDescriptor {
            definition: element, width: 1, height: 1, tile_width: 1., tile_height: 1.,
            content_transform: identity(), error_offset: 0,
        };
        Ok(ProductionDefinitionPlan::Pattern {
            plan: DefinitionPlan {
                descriptor: DefinitionDescriptor::Pattern(descriptor),
                expansions: Vec::new(), post: PostOperation::None,
            },
            mapping,
        })
    }
}

fn publish_channel(style_attr: &str, channel: PaintChannel, expected: f64) {
    let source = format!(
        "<svg><pattern id=\"p\"/><path style=\"{style_attr}\" d=\"M0 0 L1 1 2 3 Z\"/></svg>"
    );
    let doc = LexicalDocument::parse(Arc::new(source.into_bytes()), 1 << 20, &mut Run).unwrap();
    let mut shared = DecodeSource::new(&doc, &mut Run).unwrap();
    let mut builder = SceneBuilder::new(
        doc.source_owner().clone(), animation(doc.source_owner()), 7, viewport(), Limits::default(),
    ).unwrap();
    let root = builder.root_id();
    let draft = builder.begin_draft(root).unwrap();
    let path_start = doc.source().windows(6).position(|w| w == b"<path ").unwrap();
    let path_end = path_start + doc.source()[path_start..].windows(2).position(|w| w == b"/>").unwrap() + 2;
    let mut input = shape(doc.source());
    input.shape.source.element = Span { start: path_start, end: path_end };
    let shape_id = builder.append_shape(root, input, &mut Run).unwrap();
    let pattern_element = element(&doc, b"pattern");
    let owner = builder.definition_owner(
        &draft, &shared, pattern_element, DefinitionTarget::Shape(shape_id), &mut Run,
    ).unwrap();
    let style = shared.compute_style(element(&doc, b"path"), None, &mut Run).unwrap();
    let bounds = [0., 0., 10., 10.];
    let mut active = ActiveDefinitions::default();
    let mut ledger = ChargeLedger::new(doc.source_owner(), 1 << 20, 65536).unwrap();
    let mut ctx = DefinitionContext { shared: &mut shared, builder: &mut builder, scene: root, active: &mut active, ledger: &mut ledger };
    let plan = plan_production_pattern(
        &mut ChannelPlanner, &mut ctx, pattern_element, owner, channel, identity(), bounds, &style, &mut Run,
    ).unwrap();
    let ProductionDefinitionPlan::Pattern { plan, mapping } = plan else { panic!("pattern mapping required") };
    assert_eq!(mapping.opacity, expected);
    let DefinitionDescriptor::Pattern(descriptor) = plan.descriptor else { panic!("pattern descriptor required") };
    drop(plan);
    drop(ctx);

    let child = builder.new_scene(viewport()).unwrap();
    builder.seal(child, &mut Run).unwrap();
    let mut job = RasterJob::new(ChargeLedger::new(doc.source_owner(), 1 << 20, 65536).unwrap()).unwrap();
    let mut tickets = PatternTickets::new().unwrap();
    let request = tickets.request_mapped(
        &builder, &draft, &DecodeSource::new(&doc, &mut Run).unwrap(), child,
        descriptor, mapping, shape_id, channel, &job, &mut Run,
    ).unwrap();
    let ticket = request.ticket;
    let pixels = job.buffer(RasterPool::Output, 16, &mut Run).unwrap();
    job.reserve(pixels, 4, &mut Run).unwrap();
    job.extend(pixels, &[1, 2, 3, 255], &mut Run).unwrap();
    let output = JobRenderOutput { generation: 7, pixels, width: 1, height: 1, stride: 4 };
    let (pattern_id, paint_index) = tickets.complete_mapped(
        ticket, &mut builder, &draft, &mut job, Ok(output), &mut Run,
    ).unwrap();
    let view = builder.scene(root).unwrap();
    let paint = match channel {
        PaintChannel::Fill => &view.fill_paints[paint_index],
        PaintChannel::Stroke => &view.stroke_paints[paint_index],
    };
    assert_eq!(paint.pattern, Some(pattern_id));
    assert_eq!(paint.pattern_opacity, expected);
    assert_eq!(view.pattern(pattern_id).unwrap().tile.rgba(), &[1, 2, 3, 255]);
    let shape = view.shape(shape_id).unwrap();
    assert_eq!(match channel { PaintChannel::Fill => shape.fill_paint, PaintChannel::Stroke => shape.stroke_paint }, Some(paint_index));
    builder.commit_draft(&draft, &mut Run).unwrap();
    assert!(builder.freeze().is_ok());
}

#[test]
fn explicit_fill_stroke_channels_and_independent_initial_opacity_reach_real_publication() {
    publish_channel("fill-opacity:0.25;stroke-opacity:0.75", PaintChannel::Fill, 0.25);
    publish_channel("fill-opacity:0.25;stroke-opacity:0.75", PaintChannel::Stroke, 0.75);
    publish_channel("", PaintChannel::Fill, 1.0);
    publish_channel("", PaintChannel::Stroke, 1.0);
}
