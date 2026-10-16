//! Explicit ordered source fixtures, not a production SVG decoder/backend.
mod support;
use support::*;
use std::sync::Arc;
use vexel_geometry::affine_identity as identity;
use vexel_scene::*;
use vexel_svg_animation::{parse_owned_controlled, ParseError};
use vexel_base::ControlledError;

fn doc(bytes: &[u8]) -> LexicalDocument {
    let mut source = Vec::with_capacity(bytes.len() + 31);
    source.extend_from_slice(bytes);
    LexicalDocument::parse(Arc::new(source), 1 << 22, &mut Run).unwrap()
}
fn element(d: &LexicalDocument, name: &[u8]) -> ElementId {
    ElementId(d.tokens().iter().position(|t| t.local_name.slice(d.source()) == Some(name)).unwrap())
}
fn provisional(d: &LexicalDocument) -> SceneBuilder {
    SceneBuilder::provisional(d.source_owner().clone(), 7, Limits::default()).unwrap()
}
fn visible(d: &LexicalDocument) -> SceneBuilder {
    let mut b = provisional(d);
    let mut source = DecodeSource::new(d, &mut Run).unwrap();
    b.initialize_root(&mut source, element(d, b"svg"), &mut Run).unwrap();
    b
}
fn parsed(d: &LexicalDocument) -> ShapeInput {
    let source = d.source();
    let start = source.windows(4).position(|b| b == b" d=\"").unwrap() + 4;
    let end = start + source[start..].iter().position(|b| *b == b'"').unwrap();
    let element_start = source.windows(5).position(|b| b == b"<path").unwrap();
    let element_end = end + 3;
    let path = vexel_svg_path::parse_path(source, Span { start, end }, identity(), Default::default()).unwrap();
    ShapeInput { edge_colors: vec![[1, 2, 3, 255]; path.edges.len()], path,
        shape: Shape::new(SourceMap { element: Span { start: element_start, end: element_end },
            transform: identity(), transform_node: None }) }
}
fn finish(b: &mut SceneBuilder, d: &LexicalDocument) {
    let actual = parse_owned_controlled(d.source_owner(), d.tokens(), &mut Run).unwrap();
    b.finalize_decode(actual, 7, &mut Run).unwrap();
}
fn overflow(hidden: bool) -> LexicalDocument {
    let mut source = if hidden { "<svg display=\"none\" width=\"malformed\">" } else { "<svg width=\"100\" height=\"100\">" }.to_owned();
    source.push_str("<path d=\"M0 0 L1 1 Z\"/>");
    for _ in 0..129 {
        source.push_str("<animate attributeName=\"opacity\" values=\"0;1\" dur=\"1s\"/>");
    }
    source.push_str("</svg>");
    doc(source.as_bytes())
}
#[test]
fn unresolved_views_source_identity_and_ready_publication_guards() {
    let d = doc(b"<svg width=\"100\" height=\"100\"><path d=\"M0 0 L1 1 Z\"/></svg>");
    let mut b = provisional(&d);
    let root = b.root_id();
    let read = b.traversal_scene(root).unwrap();
    assert!(read.viewport.is_none());
    assert!(read.parsed_animation().is_none());
    assert!(Arc::ptr_eq(read.source_owner(), d.source_owner()));
    assert_eq!(b.initialization(), DecodeInitialization::Unresolved);
    assert!(b.scene(root).is_err());
    assert!(b.begin_draft(root).is_err());
    assert!(b.append_shape(root, parsed(&d), &mut Run).is_err());
    assert!(b.new_scene(viewport()).is_err());
    assert!(b.seal(root, &mut Run).is_err());
    let actual = parse_owned_controlled(d.source_owner(), d.tokens(), &mut Run).unwrap();
    let failure = b.finalize_decode(actual, 7, &mut Run).unwrap_err();
    assert_eq!(failure.diagnostic.kind, ErrorKind::InvalidArgument);
    assert!(Arc::ptr_eq(failure.animation.source_owner(), d.source_owner()));
    assert!(b.freeze().is_err());
    assert!(provisional(&d).freeze_decode(&mut Run).is_err());
}
#[test]
fn real_geometry_failure_precedes_actual_animation_failure() {
    let d = overflow(false);
    let mut b = visible(&d);
    let root = b.root_id();
    let draft = b.begin_draft(root).unwrap();
    let request = b.begin_request(&draft, &mut Run).unwrap();
    let mut input = parsed(&d);
    input.edge_colors.pop();
    // Actual append validates geometry before any caller installs animation.
    let geometry = b.append_shape(root, input, &mut Run).unwrap_err();
    assert_eq!(geometry.kind, ErrorKind::InvalidArgument);
    assert!(b.traversal_scene(root).unwrap().parsed_animation().is_none());
    b.rollback_request(&draft, request).unwrap();
    b.rollback_draft(draft).unwrap();
    let late = parse_owned_controlled(d.source_owner(), d.tokens(), &mut Run).unwrap_err();
    assert!(matches!(late, ControlledError::Original(ParseError::TrackLimit { offset }) if offset > 0));
    assert!(b.freeze().is_err()); // late parser failure cannot publish a graph
}
#[test]
fn successful_traversal_then_actual_late_parse_error_is_preserved() {
    let d = overflow(false);
    let mut b = visible(&d);
    let root = b.root_id();
    b.append_shape(root, parsed(&d), &mut Run).unwrap();
    assert!(matches!(parse_owned_controlled(d.source_owner(), d.tokens(), &mut Run),
        Err(ControlledError::Original(ParseError::TrackLimit { offset })) if offset > 0));
    assert_eq!(b.initialization(), DecodeInitialization::Traversing);
    assert!(b.freeze_decode(&mut Run).is_err());
}
#[test]
fn hidden_missing_or_malformed_viewport_has_actual_animation_and_no_drawable_graph() {
    for source in [b"<svg display=\"none\"><path d=\"bad\"/><animate/></svg>".as_slice(),
        b"<svg display=\"none\" width=\"bad\" transform=\"bad\"><animate/></svg>"] {
        let d = doc(source);
        let mut shared = DecodeSource::new(&d, &mut Run).unwrap();
        let root_element = element(&d, b"svg");
        // Actual style computation precedes the fixture's explicit hidden branch.
        let style = shared.compute_style(root_element, None, &mut Run).unwrap();
        assert_eq!(style.value(vexel_svg_css::Property::Display).unwrap().bytes, b"none");
        let mut b = provisional(&d);
        b.initialize_hidden_root(&mut shared, root_element, &mut Run).unwrap();
        assert!(b.new_scene(viewport()).is_err());
        assert!(b.begin_draft(b.root_id()).is_err());
        assert!(b.append_shape(b.root_id(), shape(owner().as_slice()), &mut Run).is_err());
        assert!(b.scene(b.root_id()).is_err());
        finish(&mut b, &d);
        assert_eq!(b.initialization(), DecodeInitialization::HiddenFinalized);
        assert!(b.new_scene(viewport()).is_err());
        let DecodedScene::Hidden(hidden) = b.freeze_decode(&mut Run).unwrap() else { panic!() };
        assert_eq!(hidden.source(), source);
        assert!(Arc::ptr_eq(hidden.source_owner(), d.source_owner()));
        assert_eq!(hidden.generation(), 7);
        assert!(hidden.view().viewport.is_none());
        assert!(hidden.view().shapes.is_empty());
        assert!(hidden.frame().view().edges.is_empty());
        assert!(hidden.view().shape_id(0).is_none());
        assert!(!hidden.animation().diagnostics.is_empty()); // real InvalidTrack
        assert_eq!(hidden.animation().source.as_ptr(), d.source().as_ptr());
        assert!(hidden.view().parsed_animation().is_some());
    }
}
#[test]
fn hidden_still_obeys_late_animation_error_and_normal_viewport_stays_strict() {
    let d = overflow(true);
    let mut shared = DecodeSource::new(&d, &mut Run).unwrap();
    let root_element = element(&d, b"svg");
    let mut b = provisional(&d);
    b.initialize_hidden_root(&mut shared, root_element, &mut Run).unwrap();
    assert!(matches!(parse_owned_controlled(d.source_owner(), d.tokens(), &mut Run),
        Err(ControlledError::Original(ParseError::TrackLimit { .. }))));
    assert!(b.freeze_decode(&mut Run).is_err());
    for bad in [0, 1] {
        let d = doc(b"<svg width=\"malformed\"/>");
        let mut shared = DecodeSource::new(&d, &mut Run).unwrap();
        let mut b = provisional(&d);
        let mut v = viewport();
        if bad == 0 { v.width = 0; } else { v.view_width = f64::NAN; }
        assert_eq!(b.initialize_root(&mut shared, element(&d, b"svg"), &mut Run).unwrap_err().kind,
            ErrorKind::Viewport);
        assert_eq!(b.initialization(), DecodeInitialization::Unresolved);
        assert!(b.freeze().is_err());
        assert!(SceneBuilder::new(d.source_owner().clone(),
            parse_owned_controlled(d.source_owner(), d.tokens(), &mut Run).unwrap(), 7, v, Limits::default()).is_err());
    }
}
#[test]
fn root_witness_wrong_source_nonroot_double_and_cancellation_guards() {
    let d = doc(b"<svg width=\"100\" height=\"100\"><g/><svg/></svg>");
    let other = doc(d.source());
    let mut shared = DecodeSource::new(&d, &mut Run).unwrap();
    let mut foreign = DecodeSource::new(&other, &mut Run).unwrap();
    let mut b = provisional(&d);
    assert!(b.initialize_root(&mut foreign, element(&other, b"svg"), &mut Run).is_err());
    assert!(b.initialize_hidden_root(&mut shared, element(&d, b"g"), &mut Run).is_err());
    let nested = ElementId(d.tokens().iter().position(|t| t.depth == 1 && t.local_name.slice(d.source()) == Some(b"svg")).unwrap());
    assert!(b.initialize_root(&mut shared, nested, &mut Run).is_err());
    assert_eq!(b.initialize_root(&mut shared, element(&d, b"svg"), &mut StopAfter(0)).unwrap_err().kind, ErrorKind::Canceled);
    assert_eq!(b.initialization(), DecodeInitialization::Unresolved);
    b.initialize_root(&mut shared, element(&d, b"svg"), &mut Run).unwrap();
    assert!(b.initialize_root(&mut shared, element(&d, b"svg"), &mut Run).is_err());
    assert!(b.initialize_hidden_root(&mut shared, element(&d, b"svg"), &mut Run).is_err());
    assert!(b.freeze().is_err());
}
#[test]
fn late_owner_generation_duplicate_and_cancel_return_input_without_mutation() {
    let d = doc(b"<svg width=\"100\" height=\"100\"><animate attributeName=\"opacity\" values=\"0;1\" dur=\"1s\"/></svg>");
    let other = doc(d.source());
    let mut b = visible(&d);
    let before = b.memory_stats();
    let wrong = parse_owned_controlled(other.source_owner(), other.tokens(), &mut Run).unwrap();
    let wrong = b.finalize_decode(wrong, 7, &mut Run).unwrap_err();
    assert!(Arc::ptr_eq(wrong.animation.source_owner(), other.source_owner()));
    assert_eq!(b.memory_stats(), before);
    let actual = parse_owned_controlled(d.source_owner(), d.tokens(), &mut Run).unwrap();
    let failure = b.finalize_decode(actual, 8, &mut Run).unwrap_err();
    let failure = b.finalize_decode(failure.animation, 7, &mut StopAfter(0)).unwrap_err();
    assert_eq!(failure.diagnostic.kind, ErrorKind::Canceled);
    assert_eq!(b.memory_stats(), before);
    // Stop inside actual track measurement rather than only the entry checkpoint.
    let failure = b.finalize_decode(failure.animation, 7, &mut StopAfter(1)).unwrap_err();
    assert_eq!(failure.diagnostic.kind, ErrorKind::Canceled);
    assert_eq!(b.memory_stats(), before);
    let extra = failure.animation.storage_bytes().unwrap() - d.source_owner().capacity();
    b.finalize_decode(failure.animation, 7, &mut Run).unwrap();
    assert_eq!(b.memory_stats().live_bytes, before.live_bytes + extra);
    assert_eq!(b.memory_stats().total_allocated_bytes, before.total_allocated_bytes + extra);
    assert_eq!(b.memory_stats().source_bytes, d.source_owner().capacity());
    assert_eq!(b.memory_stats().peak_live_bytes, before.peak_live_bytes.max(before.live_bytes + extra));
    let finalized = b.memory_stats();
    let duplicate = parse_owned_controlled(d.source_owner(), d.tokens(), &mut Run).unwrap();
    assert!(b.finalize_decode(duplicate, 7, &mut Run).is_err());
    assert_eq!(b.memory_stats(), finalized);
    let graph = b.freeze().unwrap();
    assert_eq!(graph.memory_stats(), finalized);
    assert_eq!(graph.animation().tracks.len(), 1);
    let frame = FrameBuilder::new(graph.root(), 1 << 20).unwrap().finish(&mut Run).unwrap();
    assert_eq!(frame.view().animation().tracks.len(), 1);
}
#[test]
fn actual_capacity_budget_failure_is_abortable_and_not_double_charged() {
    let d = doc(b"<svg width=\"100\" height=\"100\"><path d=\"M0 0 L1 1 Z\"/><animate attributeName=\"opacity\" values=\"0;1\" dur=\"1s\"/></svg>");
    let probe = visible(&d);
    let base = probe.memory_stats().live_bytes;
    let actual = parse_owned_controlled(d.source_owner(), d.tokens(), &mut Run).unwrap();
    let extra = actual.storage_bytes().unwrap() - d.source_owner().capacity();
    let mut limits = Limits::default();
    limits.reserved_bytes = base + extra - 1;
    let mut b = SceneBuilder::provisional(d.source_owner().clone(), 7, limits).unwrap();
    b.initialize_root(&mut DecodeSource::new(&d, &mut Run).unwrap(), element(&d, b"svg"), &mut Run).unwrap();
    let before = b.memory_stats();
    let failure = b.finalize_decode(actual, 7, &mut Run).unwrap_err();
    assert_eq!(failure.diagnostic.kind, ErrorKind::Limit(LimitKind::ReservedBytes));
    assert_eq!(b.memory_stats(), before);
    assert_eq!(b.initialization(), DecodeInitialization::Traversing);
    assert!(Arc::ptr_eq(failure.animation.source_owner(), d.source_owner()));
    assert!(b.freeze().is_err()); // explicit abort by Drop, no false completed graph
    drop(failure);
    // Exact actual-capacity envelope admits once, without source double charge.
    limits.reserved_bytes = base + extra;
    let mut b = SceneBuilder::provisional(d.source_owner().clone(), 7, limits).unwrap();
    b.initialize_root(&mut DecodeSource::new(&d, &mut Run).unwrap(), element(&d, b"svg"), &mut Run).unwrap();
    finish(&mut b, &d);
    assert_eq!(b.freeze().unwrap().memory_stats().live_bytes, base + extra);
}

struct FixtureProvider;
impl MediaProvider for FixtureProvider {
    fn layout_text(&mut self, request: TextRequest<'_>, c: &mut dyn WorkControl)
        -> std::result::Result<TextLayout, MediaError> {
        request.validate(c).map_err(|_| MediaError::Invalid)?;
        Ok(TextLayout { glyphs: vec![TextGlyph { glyph_id: Some(1), x: 0., y: 0., advance: 1.,
            commands: TableRange { first: 0, count: 1 } }],
            commands: vec![TextCommand { verb: TextVerb::Move, values: [0.; 6] }],
            advance_width: 1., ascent: 1., descent: 0., leading: 0. })
    }
    fn decode_image(&mut self, bytes: &[u8], c: &mut dyn WorkControl)
        -> std::result::Result<ImagePixels, MediaError> {
        c.checkpoint().map_err(|_| MediaError::Invalid)?;
        assert_eq!(bytes, b"fixture");
        Ok(image().pixels)
    }
}
#[test]
fn pending_real_media_requests_owner_maps_and_owned_append_are_usable() {
    let d = doc(b"<svg width=\"100\" height=\"100\"><path d=\"M0 0 L1 1 Z\"/><text>x</text><image/></svg>");
    let mut b = visible(&d);
    let mut shared = DecodeSource::new(&d, &mut Run).unwrap();
    let root = b.root_id();
    let draft = b.begin_draft(root).unwrap();
    let transaction = b.begin_request(&draft, &mut Run).unwrap();
    let actual = parse_owned_controlled(d.source_owner(), d.tokens(), &mut Run).unwrap();
    let mut provider = FixtureProvider;
    let mut state = TextState::default();
    let mut cache = TextCache::default();
    let mut ledger = ChargeLedger::new(d.source_owner(), 1 << 20, 1024).unwrap();
    let text_id = element(&d, b"text");
    let text_element = shared.element(text_id, text_id.0 + 1, &mut Run).unwrap();
    let style = shared.compute_style(text_id, None, &mut Run).unwrap();
    let map = parsed(&d).shape.source;
    let request = TextDecodeRequest { element: text_element, end: text_id.0 + 1, source_map: map,
        style: &style, transform: identity(), state: &mut state, preserve_space: false,
        provider: &mut provider, cache: &mut cache, scene: root, ledger: &mut ledger };
    request.validate_source_map(&b, &mut Run).unwrap();
    let layout = request.provider.layout_text(TextRequest { text: b"x", font_families: &[],
        size_points: 12., weight: 400., style: TextStyle::Normal, letter_spacing_points: 0. }, &mut Run).unwrap();
    layout.validate(&mut Run).unwrap();
    b.admit_text_commands(&draft, layout.commands.len(), 0, &mut Run).unwrap();
    let image_id = element(&d, b"image");
    let image_element = shared.element(image_id, image_id.0 + 1, &mut Run).unwrap();
    let request = ImageDecodeRequest { element: image_element, source_map: map, style: &style,
        transform: identity(), provider: &mut provider, scene: root, ledger: &mut ledger };
    request.validate_source_map(&b, &mut Run).unwrap();
    let mut value = image();
    value.pixels = request.provider.decode_image(b"fixture", &mut Run).unwrap();
    let index = b.append_image(root, value, &mut Run).unwrap();
    let mut input = parsed(&d);
    input.shape.image = Some(index);
    let shape = b.append_shape(root, input, &mut Run).unwrap();
    let owner = b.definition_owner(&draft, &shared, element(&d, b"path"), DefinitionTarget::Shape(shape), &mut Run).unwrap();
    let mut active = ActiveDefinitions::default();
    let ctx = DefinitionContext { shared: &mut shared, builder: &mut b, scene: root,
        active: &mut active, ledger: &mut ledger };
    owner.validate(&ctx).unwrap();
    assert!(ctx.builder.traversal_scene(root).unwrap().shape(shape).is_some());
    drop(ctx);
    b.commit_request(&draft, transaction, &mut Run).unwrap();
    b.commit_draft(&draft, &mut Run).unwrap();
    b.finalize_decode(actual, 7, &mut Run).unwrap();
    let graph = b.freeze().unwrap();
    assert_eq!(graph.root().images.len(), 1);
    assert!(graph.root().shape(shape).is_some());
}
struct FixturePlanner;
impl PaintChannelDefinitionPlanner for FixturePlanner {
    fn plan_owned_pattern_definition<'src>(&mut self, ctx: &mut DefinitionContext<'src, '_>,
        definition: ElementId, owner: DefinitionOwner, _: PaintChannel, _: Affine,
        bounds: [f64; 4], _: &CssPaint<'src>, _: &mut dyn WorkControl)
        -> Result<ProductionDefinitionPlan<'src>> {
        owner.validate(ctx)?;
        let DefinitionTarget::Shape(shape) = owner.target() else { panic!() };
        let map = ctx.builder.traversal_scene(ctx.scene)?.shape(shape).unwrap().source;
        Ok(ProductionDefinitionPlan::Pattern { plan: DefinitionPlan {
            descriptor: DefinitionDescriptor::Pattern(PatternDescriptor { definition, width: 1, height: 1,
                tile_width: 1., tile_height: 1., content_transform: identity(), error_offset: 0 }),
            expansions: Vec::new(), post: PostOperation::None },
            mapping: PatternMapping { tile_origin: [0., 0.], user_to_base: identity(), pattern_transform: identity(),
                user_inverse: identity(), viewport: [100., 100.], bounds, source_map: map, opacity: 1. } })
    }
}
#[test]
fn pending_actual_planner_child_request_and_stale_ownership_are_usable() {
    let d = doc(b"<svg width=\"100\" height=\"100\"><pattern/><path d=\"M0 0 L1 1 Z\"/></svg>");
    let mut b = visible(&d);
    let root = b.root_id();
    let mut shared = DecodeSource::new(&d, &mut Run).unwrap();
    let mut ledger = ChargeLedger::new(d.source_owner(), 1 << 20, 1024).unwrap();
    let mut active = ActiveDefinitions::default();
    let mut tickets = PatternTickets::new().unwrap();
    let job = RasterJob::new(ChargeLedger::new(d.source_owner(), 1 << 20, 1024).unwrap()).unwrap();
    let draft = b.begin_draft(root).unwrap();
    let request_scope = b.begin_request(&draft, &mut Run).unwrap();
    let shape = b.append_shape(root, parsed(&d), &mut Run).unwrap();
    let owner = b.definition_owner(&draft, &shared, element(&d, b"path"), DefinitionTarget::Shape(shape), &mut Run).unwrap();
    let style = shared.compute_style(element(&d, b"path"), None, &mut Run).unwrap();
    let mut ctx = DefinitionContext { shared: &mut shared, builder: &mut b, scene: root,
        active: &mut active, ledger: &mut ledger };
    let plan = plan_production_pattern(&mut FixturePlanner, &mut ctx, element(&d, b"pattern"),
        owner, PaintChannel::Fill, identity(), [0., 0., 1., 1.], &style, &mut Run).unwrap();
    let ProductionDefinitionPlan::Pattern { plan, mapping } = plan else { panic!() };
    let DefinitionDescriptor::Pattern(descriptor) = plan.descriptor else { panic!() };
    drop(ctx);
    let child = b.new_scene(viewport()).unwrap();
    let child_draft = b.begin_draft(child).unwrap();
    b.append_shape(child, parsed(&d), &mut Run).unwrap();
    b.commit_draft(&child_draft, &mut Run).unwrap();
    b.seal(child, &mut Run).unwrap();
    let request = tickets.request_mapped_traversal(&b, &draft, &shared, child, descriptor,
        mapping, shape, PaintChannel::Fill, &job, &mut Run).unwrap();
    assert_eq!(request.child.shapes.len(), 1);
    assert!(request.child.parsed_animation().is_none());
    assert!(request.child.viewport.is_some());
    assert!(request.child.scene(child).is_some());
    assert!(tickets.outstanding());
    drop(request);
    // Production graph/frame access remains unavailable even for a sealed child.
    assert!(b.scene(child).is_err());
    b.rollback_request(&draft, request_scope).unwrap();
    assert!(b.traversal_scene(child).is_err());
    let stale = b.traversal_scene(root).unwrap();
    assert!(stale.shape(shape).is_none());
    tickets.cancel();
    let replacement = b.append_shape(root, parsed(&d), &mut Run).unwrap();
    assert_ne!(shape, replacement);
    assert!(b.traversal_scene(root).unwrap().shape(shape).is_none());
    let ctx = DefinitionContext { shared: &mut shared, builder: &mut b, scene: root,
        active: &mut active, ledger: &mut ledger };
    assert!(owner.validate(&ctx).is_err());
    drop(ctx);
    b.commit_draft(&draft, &mut Run).unwrap();
    finish(&mut b, &d);
    let graph = b.freeze().unwrap();
    assert!(graph.root().shape(replacement).is_some());
    assert_eq!(graph.scene_count(), 1);
}
#[test]
fn canceled_append_and_late_work_failure_leave_explicit_retryable_storage() {
    let d = doc(b"<svg width=\"100\" height=\"100\"><path d=\"M0 0 L1 1 Z\"/></svg>");
    let mut b = visible(&d);
    assert_eq!(b.append_shape(b.root_id(), parsed(&d), &mut StopAfter(1)).unwrap_err().kind, ErrorKind::Canceled);
    assert!(b.traversal_scene(b.root_id()).unwrap().shapes.is_empty());
    b.append_shape(b.root_id(), parsed(&d), &mut Run).unwrap();
    struct WorkLimit;
    impl WorkControl for WorkLimit {
        fn charge(&mut self, _: WorkKind, _: u64) -> std::result::Result<(), Stop> { Err(Stop::Limit) }
        fn checkpoint(&mut self) -> std::result::Result<(), Stop> { Err(Stop::Limit) }
    }
    let actual = parse_owned_controlled(d.source_owner(), d.tokens(), &mut Run).unwrap();
    let before = b.memory_stats();
    let failure = b.finalize_decode(actual, 7, &mut WorkLimit).unwrap_err();
    assert_eq!(failure.diagnostic.kind, ErrorKind::Limit(LimitKind::Work));
    assert_eq!(b.memory_stats(), before);
    assert!(b.scene(b.root_id()).is_err());
    b.finalize_decode(failure.animation, 7, &mut Run).unwrap();
    assert_eq!(b.freeze().unwrap().root().shapes.len(), 1);
}

fn root_attributes(d: &LexicalDocument) -> Vec<vexel_svg_values::Attribute> {
    d.tokens()[element(d, b"svg").0 + 1..].iter()
        .take_while(|t| t.kind == vexel_xml::TokenKind::Attribute)
        .map(|t| vexel_svg_values::Attribute {
            name: Span { start: t.name.offset, end: t.name.end().unwrap() },
            value: Span { start: t.value.offset, end: t.value.end().unwrap() },
            prefix: (t.prefix.length != 0).then_some(Span {
                start: t.prefix.offset, end: t.prefix.end().unwrap() }),
        }).collect()
}
fn attribute_list<'a>(d: &'a LexicalDocument, attrs: &'a [vexel_svg_values::Attribute])
    -> vexel_svg_values::AttributeList<'a> {
    let t = &d.tokens()[element(d, b"svg").0];
    vexel_svg_values::AttributeList::new(d.source(),
        Span { start: t.name.offset, end: t.name.end().unwrap() }, attrs).unwrap()
}
#[test]
fn actual_viewport_and_transform_engines_preserve_order_and_authored_normal_frame() {
    let malformed = doc(b"<svg width=\"bad\" height=\"10\"/>");
    let attrs = root_attributes(&malformed);
    let mut b = provisional(&malformed);
    let original = vexel_svg_values::parse_root_viewport(attribute_list(&malformed, &attrs)).unwrap_err();
    let error = b.initialize_root(&mut DecodeSource::new(&malformed, &mut Run).unwrap(),
        element(&malformed, b"svg"), &mut Run).unwrap_err();
    assert_eq!(error.offset, original.offset);
    assert_eq!(error.kind, ErrorKind::Number);
    assert_eq!(&malformed.source()[error.offset..error.offset + 3], b"bad");
    assert_eq!(b.initialization(), DecodeInitialization::Unresolved);
    let actual = parse_owned_controlled(malformed.source_owner(), malformed.tokens(), &mut Run).unwrap();
    assert!(b.finalize_decode(actual, 7, &mut Run).is_err());
    assert!(b.freeze().is_err());

    let transform = doc(b"<svg display=\"none\" width=\"bad\" height=\"10\" transform=\"bad\"/>");
    let attrs = root_attributes(&transform);
    assert!(vexel_svg_values::parse_transform_node(attribute_list(&transform, &attrs), identity()).is_err());
    assert!(vexel_svg_values::parse_root_viewport(attribute_list(&transform, &attrs)).is_err());
    let mut shared = DecodeSource::new(&transform, &mut Run).unwrap();
    let mut b = provisional(&transform);
    b.initialize_hidden_root(&mut shared, element(&transform, b"svg"), &mut Run).unwrap();
    finish(&mut b, &transform);
    assert!(matches!(b.freeze_decode(&mut Run).unwrap(), DecodedScene::Hidden(_)));

    let normal = doc(b"<svg width=\"23\" height=\"17\"><path d=\"M0 0 L1 1 Z\"/></svg>");
    let attrs = root_attributes(&normal);
    let root = vexel_svg_values::parse_root_viewport(attribute_list(&normal, &attrs)).unwrap();
    let mut b = provisional(&normal);
    let viewport = Viewport { width: root.width, height: root.height, view_x: root.view_x,
        view_y: root.view_y, view_width: root.view_width, view_height: root.view_height,
        aspect: root.preserve_aspect };
    b.initialize_root(&mut DecodeSource::new(&normal, &mut Run).unwrap(), element(&normal, b"svg"), &mut Run).unwrap();
    let shape = b.append_shape(b.root_id(), parsed(&normal), &mut Run).unwrap();
    finish(&mut b, &normal);
    let DecodedScene::Drawable(graph) = b.freeze_decode(&mut Run).unwrap() else { panic!() };
    let frame = FrameBuilder::new(graph.root(), 1 << 20).unwrap().finish(&mut Run).unwrap();
    assert_eq!(frame.view().viewport, viewport);
    assert_eq!(frame.view().viewport.width, 23);
    assert_eq!(frame.view().viewport.height, 17);
    assert!(frame.view().shape(shape).is_some());
    assert_eq!(frame.view().source(), normal.source());
}
#[test]
fn hidden_bypass_requires_actual_canonical_display_and_invalid_style_wins() {
    for (bytes, expected) in [
        (b"<svg/>".as_slice(), ErrorKind::InvalidArgument),
        (b"<svg display=\"block\"/>", ErrorKind::InvalidArgument),
        (b"<svg display=\"unsupported\" width=\"bad\"/>", ErrorKind::Unsupported),
    ] {
        let d = doc(bytes);
        let mut b = provisional(&d);
        let mut shared = DecodeSource::new(&d, &mut Run).unwrap();
        let root = element(&d, b"svg");
        assert_eq!(b.initialize_hidden_root(&mut shared, root, &mut Run).unwrap_err().kind, expected);
        if expected == ErrorKind::Unsupported {
            assert_eq!(b.initialize_root(&mut shared, root, &mut Run).unwrap_err().kind, expected);
        }
        assert_eq!(b.initialization(), DecodeInitialization::Unresolved);
        assert!(b.freeze_decode(&mut Run).is_err());
    }
    let d = doc(b"<svg class=\"hidden\"/><!---->");
    let mut b = provisional(&d);
    let mut shared = DecodeSource::new(&d, &mut Run).unwrap();
    assert!(b.initialize_hidden_root(&mut shared, element(&d, b"svg"), &mut Run).is_err());
    let d = doc(b"<svg width=\"100\" height=\"100\"><style>svg { display: none }</style></svg>");
    let mut b = provisional(&d);
    let mut shared = DecodeSource::new(&d, &mut Run).unwrap();
    b.initialize_hidden_root(&mut shared, element(&d, b"svg"), &mut Run).unwrap();
    finish(&mut b, &d);
    assert!(matches!(b.freeze_decode(&mut Run).unwrap(), DecodedScene::Hidden(_)));
}
#[test]
fn missing_root_and_actual_visible_transform_error_cannot_finalize() {
    let d = doc(b"<g/>"); // valid XML without an SVG root_seen witness
    let mut b = provisional(&d);
    let mut shared = DecodeSource::new(&d, &mut Run).unwrap();
    assert!(b.initialize_hidden_root(&mut shared, ElementId(0), &mut Run).is_err());
    let actual = parse_owned_controlled(d.source_owner(), d.tokens(), &mut Run).unwrap();
    assert!(b.finalize_decode(actual, 7, &mut Run).is_err());
    assert!(b.freeze().is_err());

    let mut bytes = b"<svg width=\"bad\" transform=\"bad\">".to_vec();
    for _ in 0..129 { bytes.extend_from_slice(b"<animate attributeName=\"opacity\" values=\"0;1\" dur=\"1s\"/>"); }
    bytes.extend_from_slice(b"</svg>");
    let d = doc(&bytes);
    let b = provisional(&d);
    let attrs = root_attributes(&d);
    let error = vexel_svg_values::parse_transform_node(attribute_list(&d, &attrs), identity()).unwrap_err();
    assert_eq!(&d.source()[error.offset..error.offset + 3], b"bad");
    assert_eq!(b.initialization(), DecodeInitialization::Unresolved);
    // These later diagnostics exist, but original traversal stops at transform.
    assert!(vexel_svg_values::parse_root_viewport(attribute_list(&d, &attrs)).is_err());
    assert!(matches!(parse_owned_controlled(d.source_owner(), d.tokens(), &mut Run),
        Err(ControlledError::Original(ParseError::TrackLimit { .. }))));
    assert!(b.freeze().is_err());
}
#[test]
fn deferred_child_render_after_late_install_completes_same_pending_request() {
    let d = doc(b"<svg width=\"100\" height=\"100\"><pattern/><path d=\"M0 0 L1 1 Z\"/></svg>");
    let mut b = visible(&d);
    let root = b.root_id();
    let shared = DecodeSource::new(&d, &mut Run).unwrap();
    let draft = b.begin_draft(root).unwrap();
    let scope = b.begin_request(&draft, &mut Run).unwrap();
    let shape = b.append_shape(root, parsed(&d), &mut Run).unwrap();
    let child = b.new_scene(viewport()).unwrap();
    b.seal(child, &mut Run).unwrap();
    let descriptor = PatternDescriptor { definition: element(&d, b"pattern"), width: 1, height: 1,
        tile_width: 1., tile_height: 1., content_transform: identity(), error_offset: 0 };
    let mapping = PatternMapping { tile_origin: [0., 0.], user_to_base: identity(), pattern_transform: identity(),
        user_inverse: identity(), viewport: [100., 100.], bounds: [0., 0., 1., 1.],
        source_map: b.traversal_scene(root).unwrap().shape(shape).unwrap().source, opacity: 1. };
    let mut job = RasterJob::new(ChargeLedger::new(d.source_owner(), 1 << 20, 1024).unwrap()).unwrap();
    let mut tickets = PatternTickets::new().unwrap();
    let request = tickets.request_mapped_traversal(&b, &draft, &shared, child, descriptor,
        mapping, shape, PaintChannel::Fill, &job, &mut Run).unwrap();
    let ticket = request.ticket;
    assert!(request.child.parsed_animation().is_none());
    assert!(b.scene(child).is_err());
    drop(request);
    finish(&mut b, &d); // exact active request/draft ownership survives late admission
    let frame = FrameBuilder::new(b.scene(child).unwrap(), 1024).unwrap().finish(&mut Run).unwrap();
    assert!(frame.view().shapes.is_empty());
    // Explicit finished child output fixture; no production renderer is invented.
    let pixels = job.buffer(RasterPool::Output, 16, &mut Run).unwrap();
    job.extend(pixels, &[1, 2, 3, 255], &mut Run).unwrap();
    let output = JobRenderOutput { generation: 7, pixels, width: 1, height: 1, stride: 4 };
    let (pattern, _) = tickets.complete_mapped(ticket, &mut b, &draft, &mut job, Ok(output), &mut Run).unwrap();
    assert!(!tickets.outstanding());
    b.commit_request(&draft, scope, &mut Run).unwrap();
    b.commit_draft(&draft, &mut Run).unwrap();
    let graph = b.freeze().unwrap();
    assert!(graph.root().pattern(pattern).is_some());
    assert_eq!(graph.scene_count(), 2);
    assert!(graph.root().shape(shape).is_some());
}
#[test]
fn late_install_cannot_freeze_pending_or_failed_requests() {
    let d = doc(b"<svg width=\"100\" height=\"100\"><path d=\"M0 0 L1 1 Z\"/></svg>");
    for close_request in [false, true] {
        let mut b = visible(&d);
        let draft = b.begin_draft(b.root_id()).unwrap();
        let request = b.begin_request(&draft, &mut Run).unwrap();
        b.append_shape(b.root_id(), parsed(&d), &mut Run).unwrap();
        finish(&mut b, &d);
        if close_request { b.commit_request(&draft, request, &mut Run).unwrap(); }
        assert!(b.freeze().is_err()); // active request or active draft never escapes
    }
    let mut b = visible(&d);
    let draft = b.begin_draft(b.root_id()).unwrap();
    let request = b.begin_request(&draft, &mut Run).unwrap();
    let shape = b.append_shape(b.root_id(), parsed(&d), &mut Run).unwrap();
    finish(&mut b, &d);
    b.rollback_request(&draft, request).unwrap();
    assert!(b.scene(b.root_id()).unwrap().shape(shape).is_none());
    b.rollback_draft(draft).unwrap();
    // Explicit cleanup leaves honestly usable finalized storage, never a suffix.
    assert!(b.freeze().unwrap().root().shapes.is_empty());
}

#[test]
fn canonical_visible_viewport_errors_cannot_be_bypassed_by_valid_values() {
    for bytes in [b"<svg/>".as_slice(), b"<svg width=\"12\"/>",
        b"<svg width=\"bad\" height=\"17\"/>", b"<svg width=\"0\" height=\"17\"/>",
        b"<svg width=\"23\" height=\"17\" viewBox=\"bad\"/>",
        b"<svg xmlns=\"wrong\" width=\"23\" height=\"17\"/>",
        b"<svg width=\"23\" height=\"17\" preserveAspectRatio=\"bad\"/>"] {
        let d = doc(bytes);
        let mut b = provisional(&d);
        let mut source = DecodeSource::new(&d, &mut Run).unwrap();
        let attrs = root_attributes(&d);
        let original = vexel_svg_values::parse_root_viewport(attribute_list(&d, &attrs)).unwrap_err();
        let expected = match original.kind {
            vexel_svg_values::MappingErrorKind::Viewport => ErrorKind::Viewport,
            vexel_svg_values::MappingErrorKind::Number => ErrorKind::Number,
            vexel_svg_values::MappingErrorKind::Unsupported => ErrorKind::Unsupported,
            vexel_svg_values::MappingErrorKind::InvalidArgument => ErrorKind::InvalidArgument,
        };
        let before = b.memory_stats();
        assert_eq!(b.initialize_root(&mut source, element(&d, b"svg"), &mut Run).unwrap_err(),
            Diagnostic { kind: expected, offset: original.offset });
        assert_eq!(b.memory_stats(), before);
        assert_eq!(b.initialization(), DecodeInitialization::Unresolved);
        assert!(b.traversal_scene(b.root_id()).unwrap().viewport.is_none());
        let actual = parse_owned_controlled(d.source_owner(), d.tokens(), &mut Run).unwrap();
        assert!(b.finalize_decode(actual, 7, &mut Run).is_err());
        assert!(b.freeze().is_err());
    }
}
#[test]
fn canonical_spans_all_five_attributes_and_prefixed_child_values_are_bound() {
    let d = doc(b"<svg data-unused=\"ignored\" xmlns:q=\"urn:test\" q:width=\"999\" width=\"23.2\" height=\"17\" viewBox=\"-2 3 46 34\" preserveAspectRatio=\"xMinYMax slice\" xmlns=\"http://www.w3.org/2000/svg\"><svg width=\"88\" height=\"77\"/></svg>");
    let mut b = visible(&d);
    let read = b.traversal_scene(b.root_id()).unwrap().viewport.unwrap();
    assert_eq!((read.width, read.height), (24, 17));
    assert_eq!((read.view_x, read.view_y, read.view_width, read.view_height), (-2., 3., 46., 34.));
    assert_eq!((read.aspect.align_x, read.aspect.align_y, read.aspect.slice), (0., 1., true));
    finish(&mut b, &d);
    assert_eq!(b.freeze().unwrap().root().viewport, read);
    // A valid equal-byte foreign source and valid nested root both fail witness
    // validation before their attributes can authorize publication.
    let other = doc(d.source());
    let mut foreign = DecodeSource::new(&other, &mut Run).unwrap();
    let mut source = DecodeSource::new(&d, &mut Run).unwrap();
    let mut b = provisional(&d);
    assert!(b.initialize_root(&mut foreign, element(&other, b"svg"), &mut Run).is_err());
    let nested = ElementId(d.tokens().iter().position(|t| t.kind == vexel_xml::TokenKind::EmptyElement
        && t.depth == 1 && t.local_name.slice(d.source()) == Some(b"svg")).unwrap());
    assert!(b.initialize_root(&mut source, nested, &mut Run).is_err());
    assert!(b.freeze().is_err());
}
#[test]
fn canonical_viewport_extraction_polls_actual_token_and_value_work_transactionally() {
    struct ExtractionStop { seen: usize, after: usize, reason: Stop }
    impl WorkControl for ExtractionStop {
        fn checkpoint(&mut self) -> std::result::Result<(), Stop> { Ok(()) }
        fn charge(&mut self, kind: WorkKind, _: u64) -> std::result::Result<(), Stop> {
            if kind == WorkKind::SourceRead {
                self.seen += 1;
                if self.seen == self.after { return Err(self.reason); }
            }
            Ok(())
        }
    }
    let d = doc(b"<svg width=\"23\" height=\"17\" viewBox=\"0 0 23 17\"/>");
    for reason in [Stop::Canceled, Stop::Limit, Stop::Overflow] {
        let mut b = provisional(&d);
        let mut source = DecodeSource::new(&d, &mut Run).unwrap();
        let before = b.memory_stats();
        let mut stop = ExtractionStop { seen: 0, after: 2, reason };
        let error = b.initialize_root(&mut source, element(&d, b"svg"), &mut stop).unwrap_err();
        assert_eq!(stop.seen, 2); // actual second canonical value, not entry polling
        assert_eq!(error.kind, match reason { Stop::Canceled => ErrorKind::Canceled,
            Stop::Limit => ErrorKind::Limit(LimitKind::Work), Stop::Overflow => ErrorKind::Limit(LimitKind::Arithmetic) });
        assert_eq!(b.memory_stats(), before);
        assert_eq!(b.initialization(), DecodeInitialization::Unresolved);
        assert!(b.scene(b.root_id()).is_err());
        b.initialize_root(&mut source, element(&d, b"svg"), &mut Run).unwrap();
        finish(&mut b, &d);
        assert_eq!(b.freeze().unwrap().root().viewport.width, 23);
    }
}
