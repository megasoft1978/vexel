//! Explicit lower media fixture, using real guarded requests and admissions.
mod support;
use std::sync::Arc;
use support::*;
use vexel_geometry::affine_identity as identity;
use vexel_scene::*;

struct Provider {
    trace: Vec<&'static str>,
}
impl MediaProvider for Provider {
    fn layout_text(
        &mut self,
        request: TextRequest<'_>,
        c: &mut dyn WorkControl,
    ) -> std::result::Result<TextLayout, MediaError> {
        self.trace.push("layout");
        request.validate(c).map_err(|_| MediaError::Invalid)?;
        Ok(TextLayout {
            glyphs: vec![TextGlyph {
                glyph_id: Some(1),
                x: 0.,
                y: 0.,
                advance: 1.,
                commands: TableRange { first: 0, count: 1 },
            }],
            commands: vec![TextCommand { verb: TextVerb::Move, values: [0.; 6] }],
            advance_width: 1.,
            ascent: 1.,
            descent: 0.,
            leading: 0.,
        })
    }
    fn decode_image(&mut self, data: &[u8], _: &mut dyn WorkControl) -> std::result::Result<ImagePixels, MediaError> {
        self.trace.push("image");
        assert_eq!(data, b"fixture");
        Ok(image().pixels)
    }
}
struct Operations<'a> {
    expected: Option<&'a DraftToken>,
    fail: bool,
    trace: Vec<&'static str>,
}
fn error() -> Diagnostic {
    Diagnostic { kind: ErrorKind::Unsupported, offset: 99 }
}
impl ScopedMediaDecodeOperations for Operations<'_> {
    fn decode_text(
        &mut self,
        request: TextDecodeRequest<'_, '_>,
        b: &mut SceneBuilder,
        draft: &DraftToken,
        c: &mut dyn WorkControl,
    ) -> Result<bool> {
        if let Some(expected) = self.expected {
            assert!(std::ptr::eq(draft, expected));
        }
        self.trace.push("text");
        let token = b.begin_request(draft, c)?;
        let result = (|| {
            let layout = request
                .provider
                .layout_text(
                    TextRequest {
                        text: b"x",
                        font_families: &[],
                        size_points: 12.,
                        weight: 400.,
                        style: TextStyle::Normal,
                        letter_spacing_points: 0.,
                    },
                    c,
                )
                .map_err(|_| error())?;
            layout.validate(c).map_err(|_| error())?;
            b.admit_text_commands(draft, layout.commands.len(), request.source_map.element.start, c)?;
            // Fixture source-backed outline publication, not a text-engine port.
            let mut outline = shape(request.element.source());
            outline.shape.source = request.source_map;
            b.append_shape(request.scene, outline, c)?;
            if self.fail {
                return Err(error());
            }
            b.commit_request(draft, token, c)?;
            Ok(true)
        })();
        if result.is_err() {
            b.rollback_request(draft, token)?;
        }
        result
    }
    fn decode_image(
        &mut self,
        request: ImageDecodeRequest<'_, '_>,
        b: &mut SceneBuilder,
        draft: &DraftToken,
        c: &mut dyn WorkControl,
    ) -> Result<ShapeId> {
        if let Some(expected) = self.expected {
            assert!(std::ptr::eq(draft, expected));
        }
        self.trace.push("image");
        let token = b.begin_request(draft, c)?;
        let result = (|| {
            let pixels = request.provider.decode_image(b"fixture", c).map_err(|_| error())?;
            let mut value = image();
            value.pixels = pixels;
            let image = b.append_image(request.scene, value, c)?;
            let mut outline = shape(request.element.source());
            outline.shape.source = request.source_map;
            outline.shape.image = Some(image);
            let id = b.append_shape(request.scene, outline, c)?;
            if self.fail {
                return Err(error());
            }
            b.commit_request(draft, token, c)?;
            Ok(id)
        })();
        if result.is_err() {
            b.rollback_request(draft, token)?;
        }
        result
    }
}
impl CachedPaintResolver for Operations<'_> {
    fn resolve_cached<'cache>(
        &'cache mut self,
        _: &mut DecodeSource<'_>,
        _: &mut ChargeLedger,
        _: vexel_xml::XmlSpan,
        _: &[Edge],
        _: Affine,
        _: [f64; 2],
        c: &mut dyn WorkControl,
    ) -> Result<CachedPaint<'cache>> {
        c.checkpoint().map_err(|_| error())?;
        self.trace.push("cache");
        Ok(CachedPaint::Plain(paint()))
    }
}
impl DefinitionPlanner for Operations<'_> {
    fn plan_owned_definition<'src>(
        &mut self,
        ctx: &mut DefinitionContext<'src, '_>,
        kind: DefinitionKind,
        _: ElementId,
        owner: DefinitionOwner,
        _: Affine,
        _: [f64; 4],
        _: &CssPaint<'src>,
        _: &mut dyn WorkControl,
    ) -> Result<ProductionDefinitionPlan<'src>> {
        owner.validate(ctx)?;
        assert_eq!(kind, DefinitionKind::Filter);
        self.trace.push("filter");
        let DefinitionTarget::Group(group) = owner.target() else { panic!() };
        Ok(ProductionDefinitionPlan::Definition(DefinitionPlan {
            descriptor: DefinitionDescriptor::Filter { filter: filter(), primitives: vec![primitive()] },
            expansions: Vec::new(),
            post: PostOperation::FinishGroup(group),
        }))
    }
}
impl PaintChannelDefinitionPlanner for Operations<'_> {
    fn plan_owned_pattern_definition<'src>(
        &mut self,
        ctx: &mut DefinitionContext<'src, '_>,
        element: ElementId,
        owner: DefinitionOwner,
        channel: PaintChannel,
        _: Affine,
        bounds: [f64; 4],
        _: &CssPaint<'src>,
        _: &mut dyn WorkControl,
    ) -> Result<ProductionDefinitionPlan<'src>> {
        owner.validate(ctx)?;
        self.trace.push(match channel {
            PaintChannel::Fill => "fill",
            PaintChannel::Stroke => "stroke",
        });
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
            mapping: PatternMapping {
                tile_origin: [0.; 2],
                user_to_base: identity(),
                pattern_transform: identity(),
                user_inverse: identity(),
                viewport: [100.; 2],
                bounds,
                source_map: ctx.builder.traversal_scene(ctx.scene)?.shape(shape).unwrap().source,
                opacity: 1.,
            },
        })
    }
}
impl ScopedProductionDecodeOperations for Operations<'_> {}
fn element(d: &LexicalDocument, name: &[u8]) -> ElementId {
    ElementId(d.tokens().iter().position(|t| t.local_name.slice(d.source()) == Some(name)).unwrap())
}
fn doc() -> LexicalDocument {
    LexicalDocument::parse(Arc::new(br#"<svg width="100" height="100"><path d="M0 0 L1 1 2 3 Z"/><text>x</text><image/><pattern/><filter/><g/></svg>"#.to_vec()),1<<20,&mut Run).unwrap()
}
fn builder(d: &LexicalDocument) -> SceneBuilder {
    let mut b = SceneBuilder::provisional(d.source_owner().clone(), 7, Limits::default()).unwrap();
    b.initialize_root(&mut DecodeSource::new(d, &mut Run).unwrap(), element(d, b"svg"), &mut Run).unwrap();
    b
}
fn text(
    ops: &mut dyn ScopedProductionDecodeOperations,
    b: &mut SceneBuilder,
    draft: &DraftToken,
    shared: &DecodeSource<'_>,
    d: &LexicalDocument,
    provider: &mut Provider,
    ledger: &mut ChargeLedger,
    control: &mut dyn WorkControl,
) -> Result<bool> {
    let e = shared.element(element(d, b"text"), d.tokens().len(), &mut Run).unwrap();
    let mut state = TextState::default();
    let mut cache = TextCache::default();
    let map = SourceMap {
        element: Span {
            start: e.tokens()[e.element.0].source.offset,
            end: e.tokens()[e.element.0].source.end().unwrap(),
        },
        transform: identity(),
        transform_node: None,
    };
    decode_scoped_text(
        ops,
        TextDecodeRequest {
            element: e,
            end: e.end,
            source_map: map,
            style: &CssPaint::initial(),
            transform: identity(),
            state: &mut state,
            preserve_space: false,
            provider,
            cache: &mut cache,
            scene: b.root_id(),
            ledger,
        },
        b,
        draft,
        control,
    )
}
#[test]
fn actual_token_handoff_cumulative_budget_and_late_failure_preserve_prefix_and_authority() {
    let d = doc();
    let mut b = builder(&d);
    let root = b.root_id();
    let prefix = b.append_shape(root, shape(d.source()), &mut Run).unwrap();
    let draft = b.begin_draft(root).unwrap();
    let shared = DecodeSource::new(&d, &mut Run).unwrap();
    let seed = b.begin_request(&draft, &mut Run).unwrap();
    b.admit_text_commands(&draft, DOCUMENT_TEXT_COMMAND_LIMIT - 2, 0, &mut Run).unwrap();
    b.commit_request(&draft, seed, &mut Run).unwrap();
    let mut provider = Provider { trace: Vec::new() };
    let mut ledger = ChargeLedger::new(d.source_owner(), 8 << 20, 65536).unwrap();
    let mut operations = Operations { expected: Some(&draft), fail: false, trace: Vec::new() };
    text(&mut operations, &mut b, &draft, &shared, &d, &mut provider, &mut ledger, &mut Run).unwrap();
    assert_eq!(b.document_text_commands(), DOCUMENT_TEXT_COMMAND_LIMIT - 1);
    let before = b.traversal_scene(root).unwrap().shapes.len();
    operations.fail = true;
    assert_eq!(
        text(&mut operations, &mut b, &draft, &shared, &d, &mut provider, &mut ledger, &mut Run).unwrap_err(),
        error()
    );
    assert_eq!(b.document_text_commands(), DOCUMENT_TEXT_COMMAND_LIMIT - 1);
    assert_eq!(b.traversal_scene(root).unwrap().shapes.len(), before);
    assert!(b.traversal_scene(root).unwrap().shape(prefix).is_some());
    operations.fail = false;
    text(&mut operations, &mut b, &draft, &shared, &d, &mut provider, &mut ledger, &mut Run).unwrap();
    assert_eq!(b.document_text_commands(), DOCUMENT_TEXT_COMMAND_LIMIT);
    let error = text(&mut operations, &mut b, &draft, &shared, &d, &mut provider, &mut ledger, &mut Run).unwrap_err();
    assert_eq!(error.kind, ErrorKind::Limit(LimitKind::TextCommands));
    assert_eq!(b.traversal_scene(root).unwrap().shapes.len(), before + 1);
    assert_eq!(provider.trace, ["layout"; 4]);
    assert_eq!(operations.trace, ["text"; 4]);
    b.commit_draft(&draft, &mut Run).unwrap();
}
#[test]
fn image_real_token_rollback_and_scoped_preflight_reject_wrong_draft_and_cancel_before_provider() {
    let d = doc();
    let mut b = builder(&d);
    let root = b.root_id();
    let prefix = b.append_shape(root, shape(d.source()), &mut Run).unwrap();
    let draft = b.begin_draft(root).unwrap();
    let shared = DecodeSource::new(&d, &mut Run).unwrap();
    let mut other = builder(&d);
    let wrong = other.begin_draft(other.root_id()).unwrap();
    let mut ops = Operations { expected: Some(&draft), fail: true, trace: Vec::new() };
    let mut provider = Provider { trace: Vec::new() };
    let mut ledger = ChargeLedger::new(d.source_owner(), 8 << 20, 65536).unwrap();
    let e = shared.element(element(&d, b"image"), d.tokens().len(), &mut Run).unwrap();
    let map = SourceMap {
        element: Span {
            start: e.tokens()[e.element.0].source.offset,
            end: e.tokens()[e.element.0].source.end().unwrap(),
        },
        transform: identity(),
        transform_node: None,
    };
    macro_rules! image_request {
        ($draft:expr,$c:expr) => {
            decode_scoped_image(
                &mut ops,
                ImageDecodeRequest {
                    element: e,
                    source_map: map,
                    style: &CssPaint::initial(),
                    transform: identity(),
                    provider: &mut provider,
                    scene: root,
                    ledger: &mut ledger,
                },
                &mut b,
                $draft,
                $c,
            )
        };
    }
    assert!(image_request!(&wrong, &mut Run).is_err());
    assert!(image_request!(&draft, &mut StopAfter(0)).is_err());
    assert!(provider.trace.is_empty());
    assert!(ops.trace.is_empty());
    assert_eq!(image_request!(&draft, &mut Run).unwrap_err(), error());
    assert_eq!(b.traversal_scene(root).unwrap().images.len(), 0);
    assert_eq!(b.traversal_scene(root).unwrap().shapes.len(), 1);
    ops.fail = false;
    let id = image_request!(&draft, &mut Run).unwrap();
    assert!(b.traversal_scene(root).unwrap().shape(id).unwrap().image.is_some());
    assert!(b.traversal_scene(root).unwrap().shape(prefix).is_some());
    assert_eq!(provider.trace, ["image", "image"]);
    b.commit_draft(&draft, &mut Run).unwrap();
    other.rollback_draft(wrong).unwrap();
}
#[test]
fn combined_scoped_operations_retain_real_cache_channel_and_nonpaint_planning_routes() {
    let d = doc();
    let mut b = builder(&d);
    let root = b.root_id();
    let draft = b.begin_draft(root).unwrap();
    let shape = b.append_shape(root, shape(d.source()), &mut Run).unwrap();
    let group = b.append_group(root, group(), &mut Run).unwrap();
    let mut shared = DecodeSource::new(&d, &mut Run).unwrap();
    let shape_owner =
        b.definition_owner(&draft, &shared, element(&d, b"path"), DefinitionTarget::Shape(shape), &mut Run).unwrap();
    let group_owner =
        b.definition_owner(&draft, &shared, element(&d, b"g"), DefinitionTarget::Group(group), &mut Run).unwrap();
    let mut ledger = ChargeLedger::new(d.source_owner(), 8 << 20, 65536).unwrap();
    let mut concrete = Operations { expected: Some(&draft), fail: false, trace: Vec::new() };
    let ops: &mut dyn ScopedProductionDecodeOperations = &mut concrete;
    let value = d.tokens().iter().find(|t| t.value.slice(d.source()) == Some(b"M0 0 L1 1 2 3 Z")).unwrap().value;
    let index = resolve_publish_cached(
        ops,
        &mut shared,
        &mut ledger,
        &mut b,
        root,
        value,
        &[],
        identity(),
        [100.; 2],
        PaintChannel::Fill,
        &mut Run,
    )
    .unwrap();
    assert_eq!(index, 0);
    let mut active = ActiveDefinitions::default();
    let style = CssPaint::initial();
    let mut ctx = DefinitionContext {
        shared: &mut shared,
        builder: &mut b,
        scene: root,
        active: &mut active,
        ledger: &mut ledger,
    };
    for channel in [PaintChannel::Fill, PaintChannel::Stroke] {
        let plan = plan_production_pattern(
            ops,
            &mut ctx,
            element(&d, b"pattern"),
            shape_owner,
            channel,
            identity(),
            [-4., -3., -2., -1.],
            &style,
            &mut Run,
        )
        .unwrap();
        assert!(matches!(plan, ProductionDefinitionPlan::Pattern { .. }));
    }
    let plan = plan_production_definition(
        ops,
        &mut ctx,
        DefinitionKind::Filter,
        element(&d, b"filter"),
        group_owner,
        identity(),
        [0.; 4],
        &style,
        &mut Run,
    )
    .unwrap();
    drop(ctx);
    let ProductionDefinitionPlan::Definition(plan) = plan else { panic!() };
    let DefinitionDescriptor::Filter { filter, primitives } = plan.descriptor else { panic!() };
    b.publish_group_filter(&draft, group_owner, filter, &primitives, &mut Run).unwrap();
    assert_eq!(concrete.trace, ["cache", "fill", "stroke", "filter"]);
    b.commit_draft(&draft, &mut Run).unwrap();
}

/// Decoder owns its actual token, builder and providers in disjoint fields.
/// Mandatory suspension prevents the late parser from running before resume.
struct Session<'a> {
    document: &'a LexicalDocument,
    builder: SceneBuilder,
    draft: Option<DraftToken>,
    tickets: PatternTickets,
    state: u8,
    provider: Provider,
    ledger: ChargeLedger,
    parser_calls: usize,
    request: Option<RequestToken>,
}
impl ScopedProductionDecodeSession for Session<'_> {
    fn step<'s>(
        &'s mut self,
        _: u64,
        job: &RasterJob,
        operations: &mut dyn ScopedProductionDecodeOperations,
        c: &mut dyn WorkControl,
    ) -> Result<ScopedDecodeEvent<'s>> {
        if self.state == 1 {
            return Err(error());
        } // outstanding barrier
        if self.state == 0 {
            let shared = DecodeSource::new(self.document, c)?;
            let draft = self.draft.as_ref().unwrap();
            // This is the actual token owned by the decoder, not reconstructed.
            text(
                operations,
                &mut self.builder,
                draft,
                &shared,
                self.document,
                &mut self.provider,
                &mut self.ledger,
                c,
            )?;
            let root = self.builder.root_id();
            self.request = Some(self.builder.begin_request(draft, c)?);
            let owner = self.builder.append_shape(root, shape_input(self.document), c)?;
            let map = self.builder.traversal_scene(root)?.shape(owner).unwrap().source;
            let mut viewport = viewport();
            viewport.width = 1;
            viewport.height = 1;
            viewport.view_width = 1.;
            viewport.view_height = 1.;
            viewport.aspect.none = true;
            let child2 = self.builder.new_scene(viewport)?;
            let cd = self.builder.begin_draft(child2)?;
            self.builder.append_shape(child2, shape_input(self.document), c)?;
            self.builder.commit_draft(&cd, c)?;
            self.builder.seal(child2, c)?;
            let desc = PatternDescriptor {
                definition: element(self.document, b"pattern"),
                width: 1,
                height: 1,
                tile_width: 1.,
                tile_height: 1.,
                content_transform: identity(),
                error_offset: 0,
            };
            let mapping = PatternMapping {
                tile_origin: [0.; 2],
                user_to_base: identity(),
                pattern_transform: identity(),
                user_inverse: identity(),
                viewport: [100.; 2],
                bounds: [-4., -3., -2., -1.],
                source_map: map,
                opacity: 1.,
            };
            self.state = 1;
            return Ok(ScopedDecodeEvent::Pattern(self.tickets.request_mapped_static(
                &self.builder,
                draft,
                &shared,
                child2,
                desc,
                mapping,
                owner,
                PaintChannel::Fill,
                job,
                c,
            )?));
        }
        if self.state == 2 {
            self.parser_calls += 1;
            let actual =
                vexel_svg_animation::parse_owned_controlled(self.document.source_owner(), self.document.tokens(), c)
                    .map_err(|_| error())?;
            self.builder.finalize_decode(actual, 7, c).map_err(|e| e.diagnostic)?;
            self.state = 3;
            return Ok(ScopedDecodeEvent::Progress);
        }
        Ok(ScopedDecodeEvent::Complete)
    }
    fn resume_pattern(
        &mut self,
        ticket: PatternTicket,
        job: &mut RasterJob,
        result: Result<JobRenderOutput>,
        c: &mut dyn WorkControl,
    ) -> Result<()> {
        if self.state != 1 {
            return Err(error());
        }
        let draft = self.draft.as_ref().unwrap();
        self.tickets.complete_mapped(ticket, &mut self.builder, draft, job, result, c)?;
        self.builder.commit_request(draft, self.request.unwrap(), c)?;
        self.request = None;
        self.builder.commit_draft(draft, c)?;
        self.draft = None;
        self.state = 2;
        Ok(())
    }
    fn finish(self, _: &mut dyn WorkControl) -> Result<SceneGraph> {
        if self.state != 3 {
            return Err(error());
        }
        self.builder.freeze()
    }
}
fn shape_input(d: &LexicalDocument) -> ShapeInput {
    shape(d.source())
}
#[test]
fn scoped_session_requires_same_job_static_resume_before_late_parent_finalization() {
    let d = doc();
    let mut b = builder(&d);
    let draft = b.begin_draft(b.root_id()).unwrap();
    let mut session = Session {
        document: &d,
        builder: b,
        draft: Some(draft),
        tickets: PatternTickets::new().unwrap(),
        state: 0,
        provider: Provider { trace: Vec::new() },
        ledger: ChargeLedger::new(d.source_owner(), 8 << 20, 65536).unwrap(),
        parser_calls: 0,
        request: None,
    };
    let mut ops = Operations { expected: None, fail: false, trace: Vec::new() };
    let mut job = RasterJob::new(ChargeLedger::new(d.source_owner(), 8 << 20, 65536).unwrap()).unwrap();
    let event = session.step(100, &job, &mut ops, &mut Run).unwrap();
    let ScopedDecodeEvent::Pattern(request) = event else { panic!() };
    let ticket = request.ticket();
    let input = request.child().input();
    input.validate_job(&job, &mut Run).unwrap();
    assert_eq!(input.static_generation(), Some(7));
    let pixels = job.buffer(RasterPool::Output, 4, &mut Run).unwrap();
    job.extend(pixels, &[1, 2, 3, 255], &mut Run).unwrap();
    drop(request);
    assert_eq!(session.parser_calls, 0);
    assert!(session.step(100, &job, &mut ops, &mut Run).is_err());
    session
        .resume_pattern(
            ticket,
            &mut job,
            Ok(JobRenderOutput { generation: 7, pixels, width: 1, height: 1, stride: 4 }),
            &mut Run,
        )
        .unwrap();
    assert!(matches!(session.step(100, &job, &mut ops, &mut Run).unwrap(), ScopedDecodeEvent::Progress));
    assert_eq!(session.parser_calls, 1);
    assert!(matches!(session.step(100, &job, &mut ops, &mut Run).unwrap(), ScopedDecodeEvent::Complete));
    let graph = session.finish(&mut Run).unwrap();
    assert!(!graph.root().shapes.is_empty());
    assert_eq!(graph.root().patterns.len(), 1);
    assert_eq!(ops.trace, ["text"]);
}
