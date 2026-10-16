//! Public input fixture only; no runtime/native scalar-renderer claim.
mod support;
use std::sync::Arc;
use support::*;
use vexel_geometry::affine_identity as identity;
use vexel_scene::*;

fn doc(invalid_animation: bool) -> LexicalDocument {
    let mut text = "<svg width=\"100\" height=\"100\"><pattern/><path d=\"M0 0 L1 1 2 3 Z\"><animate attributeName=\"opacity\" values=\"0;1\" dur=\"1s\"/></path>".to_owned();
    if invalid_animation {
        for _ in 0..129 {
            text.push_str("<animate attributeName=\"opacity\" values=\"0;1\" dur=\"1s\"/>");
        }
    }
    text.push_str("</svg>");
    LexicalDocument::parse(Arc::new(text.into_bytes()), 1 << 20, &mut Run).unwrap()
}
fn element(d: &LexicalDocument, name: &[u8]) -> ElementId {
    ElementId(d.tokens().iter().position(|t| t.local_name.slice(d.source()) == Some(name)).unwrap())
}
fn descriptor(d: &LexicalDocument) -> PatternDescriptor {
    PatternDescriptor {
        definition: element(d, b"pattern"),
        width: 1,
        height: 1,
        tile_width: 1.,
        tile_height: 1.,
        content_transform: identity(),
        error_offset: 10,
    }
}
fn tile_viewport() -> Viewport {
    let mut v = viewport();
    v.width = 1;
    v.height = 1;
    v.view_width = 1.;
    v.view_height = 1.;
    v.aspect.none = true;
    v
}
fn provisional(d: &LexicalDocument) -> SceneBuilder {
    let mut b = SceneBuilder::provisional(d.source_owner().clone(), 7, Limits::default()).unwrap();
    b.initialize_root(&mut DecodeSource::new(d, &mut Run).unwrap(), element(d, b"svg"), &mut Run).unwrap();
    b
}
fn input(d: &LexicalDocument) -> ShapeInput {
    let mut s = shape(d.source());
    let start = d.source().windows(5).position(|v| v == b"<path").unwrap();
    s.shape.source.element = Span { start, end: d.source().len() - 6 };
    s
}
fn mapping(b: &SceneBuilder, scene: SceneId, shape: ShapeId, bounds: [f64; 4]) -> PatternMapping {
    PatternMapping {
        tile_origin: [0.; 2],
        user_to_base: identity(),
        pattern_transform: identity(),
        user_inverse: identity(),
        viewport: [100.; 2],
        bounds,
        source_map: b.traversal_scene(scene).unwrap().shape(shape).unwrap().source,
        opacity: 0.75,
    }
}
fn job(d: &LexicalDocument) -> RasterJob {
    RasterJob::new(ChargeLedger::new(d.source_owner(), 8 << 20, 65536).unwrap()).unwrap()
}
fn output(job: &mut RasterJob, c: &mut dyn WorkControl) -> Result<JobRenderOutput> {
    let pixels = job.buffer(RasterPool::Output, 4, c)?;
    job.extend(pixels, &[1, 2, 3, 255], c)?;
    Ok(JobRenderOutput { generation: 7, pixels, width: 1, height: 1, stride: 4 })
}

/// Intentional table-consuming fixture; lower production algorithms adopt later.
struct Engine {
    fail: bool,
    calls: usize,
    rich: bool,
}
impl CommonKernelResources for Engine {
    fn paint_sample(&self, _: &Paint, stops: &[PaintStop], _: f64, _: f64, c: &mut dyn WorkControl) -> Result<[u8; 4]> {
        for _ in stops {
            c.charge(WorkKind::PaintVisit, 1).map_err(|_| Diagnostic { kind: ErrorKind::Canceled, offset: 0 })?;
        }
        Ok([1, 2, 3, 255])
    }
    fn clip_contains(&self, v: RasterInput<'_>, id: ClipId, _: f64, _: f64, c: &mut dyn WorkControl) -> Result<bool> {
        self.clip_work_counts(v, id, c).map(|n| n.edges == 0)
    }
    fn clip_boundary_edges(
        &self,
        v: RasterInput<'_>,
        id: ClipId,
        visitor: &mut dyn FnMut(Edge) -> bool,
        c: &mut dyn WorkControl,
    ) -> Result<bool> {
        for e in &v.clip(id).unwrap().geometry.edges {
            c.checkpoint().map_err(|_| Diagnostic { kind: ErrorKind::Canceled, offset: 0 })?;
            if !visitor(*e) {
                return Ok(false);
            }
        }
        Ok(true)
    }
    fn clip_work_counts(&self, v: RasterInput<'_>, id: ClipId, c: &mut dyn WorkControl) -> Result<ClipWorkCounts> {
        c.checkpoint().map_err(|_| Diagnostic { kind: ErrorKind::Canceled, offset: 0 })?;
        let clip = v.clip(id).unwrap();
        Ok(ClipWorkCounts {
            edges: clip.geometry.edges.len(),
            chain_elements: 1,
            child_shapes: clip.geometry.shapes.len(),
        })
    }
    fn stroke_sample_work(
        &self,
        v: RasterInput<'_>,
        id: ShapeId,
        _: ValidatedDashView<'_>,
        _: f64,
        _: f64,
        c: &mut dyn WorkControl,
    ) -> Result<(usize, bool, bool)> {
        c.checkpoint().map_err(|_| Diagnostic { kind: ErrorKind::Canceled, offset: 0 })?;
        Ok((v.shape(id).unwrap().edges.count, false, false))
    }
}
impl CommonIndexKernel for Engine {
    fn build_index(
        &mut self,
        v: RasterInput<'_>,
        j: &mut RasterJob,
        _: ViewMapping,
        c: &mut dyn WorkControl,
    ) -> Result<JobRasterScratch> {
        v.validate_job(j, c)?;
        JobRasterScratch::new(j, c)
    }
}
impl CommonSampleKernel for Engine {
    fn sample(
        &mut self,
        v: RasterInput<'_>,
        id: ShapeId,
        i: &CommonSampleInput<'_>,
        c: &mut dyn WorkControl,
    ) -> Result<[u8; 4]> {
        i.validate(v, c)?;
        i.paint_sample(v.fill_paint(id).unwrap(), v.stops(), c)
    }
}
impl CommonFlatKernel for Engine {
    fn render_flat(
        &mut self,
        v: RasterInput<'_>,
        range: TableRange,
        _: RenderOptions,
        j: &mut RasterJob,
        c: &mut dyn WorkControl,
    ) -> Result<JobRenderOutput> {
        self.calls += 1;
        v.validate_job(j, c)?;
        assert!(range.slice(v.shapes()).is_some());
        if v.static_generation().is_some() {
            assert_eq!(v.viewport(), tile_viewport());
        }
        assert_eq!(v.edges()[0].a.x, 0.); // original base geometry, no clock
        assert_eq!(v.edge_colors().len(), v.edges().len());
        assert_eq!(v.edge_shape_ids().len(), v.edges().len());
        assert!(!v.path_subpaths().is_empty());
        assert!(!v.path_segments().is_empty());
        assert!(!v.path_numbers().is_empty());
        let id = v.shape_id(0).unwrap();
        assert_eq!(v.edge_shape(0), Some(id));
        assert_eq!(v.shape(id).unwrap().source.element.slice(v.source()).unwrap()[0], b'<');
        assert!(Arc::ptr_eq(v.source_owner(), j.source_owner()));
        if v.static_generation().is_some() {
            assert!(v.static_ticket().is_some());
            assert!(v.static_request().is_some());
            assert!(v.static_draft_nonce().is_some());
        }
        let _ = v.ignored(IgnoredKind::Meta);
        if self.rich {
            assert_eq!(v.images().len(), 1);
            assert_eq!(v.transforms().len(), 1);
            assert!(v.transform(v.transform_id(0).unwrap()).is_some());
            assert_eq!(v.groups().len(), 1);
            assert!(v.group(v.group_id(0).unwrap()).unwrap().filter.is_some());
            assert_eq!(v.filters().len(), 1);
            assert!(v.filter(v.filter_id(0).unwrap()).is_some());
            assert_eq!(v.filter_primitives(), &[primitive()]);
            assert_eq!(v.patterns().len(), 1);
            assert_eq!(v.pattern(v.pattern_id(0).unwrap()).unwrap().tile.rgba(), &[1, 2, 3, 255]);
            assert_eq!(v.masks().len(), 1);
            let mask = v.mask_id(0).unwrap();
            assert!(v.mask(mask).is_some());
            let m = v.mask_content(mask).unwrap();
            assert_ne!(v.id(), m.id());
            assert_eq!(m.shapes().len(), 1);
            assert_eq!(m.mask_content(m.mask_id(0).unwrap()).unwrap().shapes().len(), 1);
            assert!(m.mask_content(mask).is_none()); // wrong-scene handle
            assert_eq!(v.clips().len(), 1);
            assert!(self.clip_contains(v, v.clip_id(0).unwrap(), 0., 0., c)?);
            assert!(self.clip_boundary_edges(v, v.clip_id(0).unwrap(), &mut |_| true, c)?);
            assert_eq!(v.dash_values(), &[1., 2.]);
            assert_eq!(v.shape_dashes(id), Some(&[1., 2.][..]));
            assert_eq!(v.fill_paints().len(), 1);
            assert_eq!(v.stroke_paints().len(), 1);
            assert!(v.stroke_paint(id).is_some());
            assert_eq!(v.stops().len(), 1);
            let scratch =
                self.build_index(v, j, ViewMapping { scale_x: 1., scale_y: 1., offset_x: 0., offset_y: 0. }, c)?;
            let a = j.buffer(RasterPool::Scratch, 1, c)?;
            let b = j.buffer(RasterPool::Scratch, 1, c)?;
            j.extend(a, &[0u8], c)?;
            j.extend(b, &[0u32], c)?;
            let mut scope = j.compose(v.source_owner(), scratch.dash, c)?;
            scope.with_buffers(a, b, c, |cache, _, _, c| {
                let resources = Engine { fail: false, calls: 0, rich: false };
                let sample = CommonSampleInput {
                    resources: &resources,
                    active_edges: &[0],
                    local_edges: v.edges(),
                    dash_seams: &[false; 3],
                    dash_cache: cache,
                    px: 0.,
                    py: 0.,
                    paint_x: 0.,
                    paint_y: 0.,
                };
                sample.validate(v, c)?;
                assert!(sample.validate(v, &mut StopAfter(0)).is_err());
                assert!(sample.paint_sample(v.fill_paint(id).unwrap(), v.stops(), &mut StopAfter(0)).is_err());
                assert_eq!(sample.paint_sample(v.fill_paint(id).unwrap(), v.stops(), c)?, [1, 2, 3, 255]);
                assert_eq!(self.sample(v, id, &sample, c)?, [1, 2, 3, 255]);
                assert_eq!(self.stroke_sample_work(v, id, cache, 0., 0., c)?.0, v.shape(id).unwrap().edges.count);
                Ok(())
            })?;
        }
        if self.fail {
            return Err(Diagnostic { kind: ErrorKind::Limit(LimitKind::Patterns), offset: 77 });
        }
        output(j, c)
    }
}
impl CommonRasterKernel for Engine {}

fn child(b: &mut SceneBuilder, d: &LexicalDocument) -> SceneId {
    let child = b.new_scene(tile_viewport()).unwrap();
    let draft = b.begin_draft(child).unwrap();
    b.append_shape(child, input(d), &mut Run).unwrap();
    b.commit_draft(&draft, &mut Run).unwrap();
    b.seal(child, &mut Run).unwrap();
    child
}
#[test]
fn real_child_failure_wins_and_success_publishes_before_actual_late_animation_parse() {
    for fail in [true, false] {
        let d = doc(true);
        let mut b = provisional(&d);
        let root = b.root_id();
        let shared = DecodeSource::new(&d, &mut Run).unwrap();
        let draft = b.begin_draft(root).unwrap();
        let scope = b.begin_request(&draft, &mut Run).unwrap();
        let shape = b.append_shape(root, input(&d), &mut Run).unwrap();
        let child = child(&mut b, &d);
        let map = mapping(&b, root, shape, [-9., -8., -1., -2.]);
        let mut j = job(&d);
        let mut tickets = PatternTickets::new().unwrap();
        let request = tickets
            .request_mapped_static(
                &b,
                &draft,
                &shared,
                child,
                descriptor(&d),
                map,
                shape,
                PaintChannel::Stroke,
                &j,
                &mut Run,
            )
            .unwrap();
        assert_eq!(request.descriptor(), descriptor(&d));
        let ticket = request.ticket();
        let mut engine = Engine { fail, calls: 0, rich: false };
        let rendered = render_common_pattern(&mut engine, &request, RenderOptions::default(), &mut j, &mut Run);
        assert_eq!(engine.calls, 1);
        drop(request);
        let published = tickets.complete_mapped(ticket, &mut b, &draft, &mut j, rendered, &mut Run);
        let mut parser_calls = 0;
        let error_offset = match published {
            Err(e) => {
                assert!(fail);
                e.offset
            }
            Ok((id, index)) => {
                assert!(!fail);
                let view = b.traversal_scene(root).unwrap();
                assert_eq!(view.pattern(id).unwrap().tile.rgba(), &[1, 2, 3, 255]);
                assert_eq!(view.stroke_paints[index].bounds, [-9., -8., -1., -2.]);
                assert_eq!(view.shape(shape).unwrap().source, map.source_map);
                assert!(view.parsed_animation().is_none());
                parser_calls += 1;
                let error =
                    vexel_svg_animation::parse_owned_controlled(d.source_owner(), d.tokens(), &mut Run).unwrap_err();
                assert!(matches!(
                    error,
                    vexel_base::ControlledError::Original(vexel_svg_animation::ParseError::TrackLimit { .. })
                ));
                0
            }
        };
        assert_eq!(parser_calls, usize::from(!fail));
        if fail {
            assert_eq!(error_offset, 77);
            assert!(b.traversal_scene(root).unwrap().patterns.is_empty());
        }
        assert!(!tickets.outstanding());
        assert!(b.scene(root).is_err());
        assert!(b.scene(child).is_err());
        b.rollback_request(&draft, scope).unwrap();
        b.rollback_draft(draft).unwrap();
        assert!(b.freeze().is_err());
        assert_eq!(j.finish().unwrap().live_bytes, 0);
    }
}

#[test]
fn nested_depth_first_tiles_mask_closure_and_complete_static_effect_metadata() {
    let d = doc(false);
    let mut b = provisional(&d);
    let root = b.root_id();
    let shared = DecodeSource::new(&d, &mut Run).unwrap();
    let draft = b.begin_draft(root).unwrap();
    let transaction = b.begin_request(&draft, &mut Run).unwrap();
    let host = b.append_shape(root, input(&d), &mut Run).unwrap();
    let outer = b.new_scene(tile_viewport()).unwrap();
    let outer_draft = b.begin_draft(outer).unwrap();
    let transform = b.append_transform(outer, transform(None), &mut Run).unwrap();
    let mut shape_input = input(&d);
    shape_input.shape.source.transform_node = Some(transform);
    let shape = b.append_shape(outer, shape_input, &mut Run).unwrap();
    let inner = child(&mut b, &d);
    let mut j = job(&d);
    let mut tickets = PatternTickets::new().unwrap();
    let inner_map = mapping(&b, outer, shape, [-4., -3., -2., -1.]);
    let request = tickets
        .request_mapped_static(
            &b,
            &outer_draft,
            &shared,
            inner,
            descriptor(&d),
            inner_map,
            shape,
            PaintChannel::Fill,
            &j,
            &mut Run,
        )
        .unwrap();
    let ticket = request.ticket();
    let mut engine = Engine { fail: false, calls: 0, rich: false };
    let out = render_common_pattern(&mut engine, &request, Default::default(), &mut j, &mut Run).unwrap();
    drop(request);
    tickets.complete_mapped(ticket, &mut b, &outer_draft, &mut j, Ok(out), &mut Run).unwrap();
    // Inner tile has already published, before containing child seals.
    assert_eq!(b.traversal_scene(outer).unwrap().patterns.len(), 1);
    let mask_child = b.new_scene(tile_viewport()).unwrap();
    let mask_draft = b.begin_draft(mask_child).unwrap();
    b.append_shape(mask_child, input(&d), &mut Run).unwrap();
    let deeper = child(&mut b, &d);
    b.append_mask(mask_child, mask(deeper), &mut Run).unwrap();
    b.commit_draft(&mask_draft, &mut Run).unwrap();
    b.seal(mask_child, &mut Run).unwrap();
    let mask = b.append_mask(outer, mask(mask_child), &mut Run).unwrap();
    let filter = b.append_filter(outer, filter(), &[primitive()], &mut Run).unwrap();
    let mut group_value = group();
    group_value.filter = Some(filter);
    group_value.mask = Some(mask);
    group_value.shapes = TableRange { first: 0, count: 1 };
    group_value.opacity = 0.5;
    b.append_group(outer, group_value, &mut Run).unwrap();
    let clip = b.append_clip(outer, clip(), &mut Run).unwrap();
    let image = b.append_image(outer, image(), &mut Run).unwrap();
    let dashes = b.append_dashes(outer, &[1., 2.], &mut Run).unwrap();
    b.append_stops(outer, &[PaintStop { offset: 0., rgba: [5, 6, 7, 255] }], &mut Run).unwrap();
    let stroke = b.append_stroke_paint(outer, paint(), &mut Run).unwrap();
    let mut value = *b.traversal_scene(outer).unwrap().shape(shape).unwrap();
    value.stroke.dash_values = dashes;
    value.stroke_paint = Some(stroke);
    value.image = Some(image);
    value.clip = Some(clip);
    b.finalize_shape(&outer_draft, shape, value, &mut Run).unwrap();
    b.commit_draft(&outer_draft, &mut Run).unwrap();
    b.seal(outer, &mut Run).unwrap();
    let outer_map = mapping(&b, root, host, [-8., -7., -2., -1.]);
    let request = tickets
        .request_mapped_static(
            &b,
            &draft,
            &shared,
            outer,
            descriptor(&d),
            outer_map,
            host,
            PaintChannel::Fill,
            &j,
            &mut Run,
        )
        .unwrap();
    let ticket = request.ticket();
    engine.rich = true;
    let out = render_common_pattern(&mut engine, &request, Default::default(), &mut j, &mut Run).unwrap();
    drop(request);
    tickets.complete_mapped(ticket, &mut b, &draft, &mut j, Ok(out), &mut Run).unwrap();
    assert_eq!(engine.calls, 2);
    b.commit_request(&draft, transaction, &mut Run).unwrap();
    b.commit_draft(&draft, &mut Run).unwrap();
    let actual = vexel_svg_animation::parse_owned_controlled(d.source_owner(), d.tokens(), &mut Run).unwrap();
    assert!(!actual.view().tracks.is_empty());
    b.finalize_decode(actual, 7, &mut Run).unwrap();
    let graph = b.freeze().unwrap();
    assert_eq!(graph.root().patterns.len(), 1);
    let ready = RasterInput::ready(graph.root());
    assert!(ready.static_generation().is_none());
    assert_eq!(ready.patterns().len(), 1);
    engine.rich = false;
    let output = engine
        .render_flat(ready, TableRange { first: 0, count: ready.shapes().len() }, Default::default(), &mut j, &mut Run)
        .unwrap();
    j.release(output.pixels).unwrap();
    assert_eq!(j.finish().unwrap().live_bytes, 0);
}

struct Planner {
    calls: usize,
}
impl PaintChannelDefinitionPlanner for Planner {
    fn plan_owned_pattern_definition<'src>(
        &mut self,
        ctx: &mut DefinitionContext<'src, '_>,
        element: ElementId,
        owner: DefinitionOwner,
        _: PaintChannel,
        _: Affine,
        bounds: [f64; 4],
        _: &CssPaint<'src>,
        _: &mut dyn WorkControl,
    ) -> Result<ProductionDefinitionPlan<'src>> {
        self.calls += 1;
        let DefinitionTarget::Shape(shape) = owner.target() else { panic!() };
        Ok(ProductionDefinitionPlan::Pattern {
            plan: DefinitionPlan {
                descriptor: DefinitionDescriptor::Pattern(PatternDescriptor {
                    definition: element,
                    width: 1,
                    height: 1,
                    tile_width: 1.,
                    tile_height: 1.,
                    content_transform: identity(),
                    error_offset: 0,
                }),
                expansions: Vec::new(),
                post: PostOperation::None,
            },
            mapping: mapping(ctx.builder, ctx.scene, shape, bounds),
        })
    }
}
#[test]
fn ordered_minmax_guard_precedes_planner_and_negative_mapping_publishes() {
    assert_eq!(ordered_bounds_extent([-9., -8., -1., -2.]), Some([8., 6.]));
    assert_eq!(ordered_bounds_extent([0.; 4]), Some([0.; 2]));
    for bounds in [
        [2., 2., 1., 3.],
        [0., 2., 3., 1.],
        [0., 0., f64::NAN, 1.],
        [f64::NEG_INFINITY, 0., 0., 1.],
        [-f64::MAX, 0., f64::MAX, 1.],
    ] {
        assert_eq!(ordered_bounds_extent(bounds), None);
    }
    let d = doc(false);
    let mut b = provisional(&d);
    let root = b.root_id();
    let mut source = DecodeSource::new(&d, &mut Run).unwrap();
    let draft = b.begin_draft(root).unwrap();
    let request = b.begin_request(&draft, &mut Run).unwrap();
    let shape = b.append_shape(root, input(&d), &mut Run).unwrap();
    let owner =
        b.definition_owner(&draft, &source, element(&d, b"path"), DefinitionTarget::Shape(shape), &mut Run).unwrap();
    let style = source.compute_style(element(&d, b"path"), None, &mut Run).unwrap();
    let mut ledger = ChargeLedger::new(d.source_owner(), 1 << 20, 1024).unwrap();
    let mut active = ActiveDefinitions::default();
    let mut planner = Planner { calls: 0 };
    let mut ctx = DefinitionContext {
        shared: &mut source,
        builder: &mut b,
        scene: root,
        active: &mut active,
        ledger: &mut ledger,
    };
    for bounds in [[2., 2., 1., 3.], [0., 2., 3., 1.], [0., 0., f64::NAN, 1.]] {
        assert!(plan_production_pattern(
            &mut planner,
            &mut ctx,
            element(&d, b"pattern"),
            owner,
            PaintChannel::Fill,
            identity(),
            bounds,
            &style,
            &mut Run
        )
        .is_err());
    }
    assert_eq!(planner.calls, 0);
    let plan = plan_production_pattern(
        &mut planner,
        &mut ctx,
        element(&d, b"pattern"),
        owner,
        PaintChannel::Fill,
        identity(),
        [-9., -8., -1., -2.],
        &style,
        &mut Run,
    )
    .unwrap();
    assert_eq!(planner.calls, 1);
    drop(ctx);
    let ProductionDefinitionPlan::Pattern { plan, mapping } = plan else { panic!() };
    let DefinitionDescriptor::Pattern(desc) = plan.descriptor else { panic!() };
    let child = child(&mut b, &d);
    let mut j = job(&d);
    let mut tickets = PatternTickets::new().unwrap();
    let req = tickets
        .request_mapped_static(&b, &draft, &source, child, desc, mapping, shape, PaintChannel::Fill, &j, &mut Run)
        .unwrap();
    let ticket = req.ticket();
    drop(req);
    let out = output(&mut j, &mut Run).unwrap();
    let (_, index) = tickets.complete_mapped(ticket, &mut b, &draft, &mut j, Ok(out), &mut Run).unwrap();
    assert_eq!(b.traversal_scene(root).unwrap().fill_paints[index].bounds, [-9., -8., -1., -2.]);
    b.rollback_request(&draft, request).unwrap();
    b.rollback_draft(draft).unwrap();
}

#[test]
fn mint_render_and_completion_enforce_scope_cancel_source_job_replay_and_rollback() {
    let d = doc(false);
    let mut b = provisional(&d);
    let root = b.root_id();
    let shared = DecodeSource::new(&d, &mut Run).unwrap();
    let draft = b.begin_draft(root).unwrap();
    let old = child(&mut b, &d);
    let scope = b.begin_request(&draft, &mut Run).unwrap();
    let shape = b.append_shape(root, input(&d), &mut Run).unwrap();
    let child = child(&mut b, &d);
    let map = mapping(&b, root, shape, [0.; 4]);
    let mut j = job(&d);
    let mut tickets = PatternTickets::new().unwrap();
    macro_rules! mint {
        ($source:expr,$child:expr,$desc:expr,$map:expr,$c:expr) => {
            tickets.request_mapped_static(&b, &draft, $source, $child, $desc, $map, shape, PaintChannel::Fill, &j, $c)
        };
    }
    assert!(mint!(&shared, old, descriptor(&d), map, &mut Run).is_err());
    assert!(mint!(&shared, root, descriptor(&d), map, &mut Run).is_err());
    assert!(mint!(&shared, child, descriptor(&d), map, &mut StopAfter(0)).is_err());
    let mut wrong = descriptor(&d);
    wrong.width = 2;
    assert!(mint!(&shared, child, wrong, map, &mut Run).is_err());
    let mut wrongmap = map;
    wrongmap.bounds = [4., 0., 2., 1.];
    assert!(mint!(&shared, child, descriptor(&d), wrongmap, &mut Run).is_err());
    assert!(!tickets.outstanding());
    let other = doc(false);
    let foreign = DecodeSource::new(&other, &mut Run).unwrap();
    assert!(mint!(&foreign, child, descriptor(&d), map, &mut Run).is_err());
    let req = mint!(&shared, child, descriptor(&d), map, &mut Run).unwrap();
    let ticket = req.ticket();
    let mut foreignjob = job(&d);
    let mut engine = Engine { fail: false, calls: 0, rich: false };
    assert!(render_common_pattern(&mut engine, &req, Default::default(), &mut foreignjob, &mut Run).is_err());
    assert_eq!(engine.calls, 0);
    assert_eq!(
        render_common_pattern(&mut engine, &req, Default::default(), &mut j, &mut StopAfter(0)).unwrap_err().kind,
        ErrorKind::Canceled
    );
    drop(req);
    let mut othertickets = PatternTickets::new().unwrap();
    let wrong = othertickets
        .request_mapped_static(&b, &draft, &shared, child, descriptor(&d), map, shape, PaintChannel::Fill, &j, &mut Run)
        .unwrap()
        .ticket();
    assert!(tickets
        .complete_mapped(
            wrong,
            &mut b,
            &draft,
            &mut j,
            Err(Diagnostic { kind: ErrorKind::Unsupported, offset: 1 }),
            &mut Run
        )
        .is_err());
    assert!(tickets.outstanding());
    let out = output(&mut j, &mut Run).unwrap();
    b.rollback_request(&draft, scope).unwrap();
    assert!(tickets.complete_mapped(ticket, &mut b, &draft, &mut j, Ok(out), &mut Run).is_err());
    assert!(j.slice(out.pixels).is_err());
    assert!(!tickets.outstanding());
    assert!(tickets.complete_mapped(ticket, &mut b, &draft, &mut j, Ok(out), &mut Run).is_err());
    let replacement = b.append_shape(root, input(&d), &mut Run).unwrap();
    assert_ne!(replacement, shape);
    assert!(b.traversal_scene(root).unwrap().patterns.is_empty());
    b.rollback_draft(draft).unwrap();
    assert_eq!(j.finish().unwrap().live_bytes, 0);
}

struct LimitWork;
impl WorkControl for LimitWork {
    fn charge(&mut self, _: WorkKind, _: u64) -> std::result::Result<(), Stop> {
        Err(Stop::Limit)
    }
    fn checkpoint(&mut self) -> std::result::Result<(), Stop> {
        Err(Stop::Limit)
    }
}
#[test]
fn actual_common_render_limit_keeps_parent_pending_and_never_calls_engine() {
    let d = doc(false);
    let mut b = provisional(&d);
    let root = b.root_id();
    let shared = DecodeSource::new(&d, &mut Run).unwrap();
    let draft = b.begin_draft(root).unwrap();
    let transaction = b.begin_request(&draft, &mut Run).unwrap();
    let shape = b.append_shape(root, input(&d), &mut Run).unwrap();
    let child = child(&mut b, &d);
    let map = mapping(&b, root, shape, [0.; 4]);
    let mut job = job(&d);
    let mut tickets = PatternTickets::new().unwrap();
    let request = tickets
        .request_mapped_static(
            &b,
            &draft,
            &shared,
            child,
            descriptor(&d),
            map,
            shape,
            PaintChannel::Fill,
            &job,
            &mut Run,
        )
        .unwrap();
    let ticket = request.ticket();
    let mut engine = Engine { fail: false, calls: 0, rich: false };
    let error = render_common_pattern(&mut engine, &request, Default::default(), &mut job, &mut LimitWork).unwrap_err();
    assert_eq!(error.kind, ErrorKind::Limit(LimitKind::Work));
    assert_eq!(engine.calls, 0);
    drop(request);
    assert_eq!(tickets.complete_mapped(ticket, &mut b, &draft, &mut job, Err(error), &mut Run).unwrap_err(), error);
    assert!(b.traversal_scene(root).unwrap().patterns.is_empty());
    assert!(b.scene(child).is_err());
    b.rollback_request(&draft, transaction).unwrap();
    b.rollback_draft(draft).unwrap();
}
