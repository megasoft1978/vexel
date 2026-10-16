mod support;
use std::sync::Arc;
use support::*;
use vexel_scene::*;
use vexel_svg_css::Property;
use vexel_xml::XmlSpan;
#[test]
fn capacity_leases_count_source_once_metadata_and_actual_spare_storage() {
    let source = owner();
    let mut ledger = ChargeLedger::new(&source, 1 << 20, 32).unwrap();
    assert!(Arc::ptr_eq(&source, ledger.source_owner()));
    assert_eq!(ledger.stats().live_bytes, source.capacity());
    let mut bytes = Vec::<u8>::with_capacity(137);
    bytes.extend_from_slice(&[1, 2]);
    let a = ledger.admit_vec(AllocationTag::Output, &bytes).unwrap();
    let first = ledger.stats();
    assert_eq!(first.output_bytes, bytes.capacity());
    assert!(first.live_bytes > source.capacity() + bytes.capacity());
    let b = ledger.admit(AllocationTag::ClipCopy, 64).unwrap();
    let peak = ledger.stats().peak_live_bytes;
    ledger.release(a).unwrap();
    assert_eq!(ledger.stats().output_bytes, 0);
    assert_eq!(ledger.stats().clip_copy_bytes, 64);
    ledger.release(b).unwrap();
    assert!(ledger.stats().live_bytes > source.capacity());
    assert_eq!(ledger.stats().peak_live_bytes, peak);
}
#[test]
fn stale_wrong_and_double_released_capacity_leases_fail() {
    let source = owner();
    let mut a = ChargeLedger::new(&source, 1 << 20, 32).unwrap();
    let mut b = ChargeLedger::new(&source, 1 << 20, 32).unwrap();
    let old = a.admit(AllocationTag::Output, 20).unwrap();
    assert!(b.release(old).is_err());
    a.release(old).unwrap();
    assert!(a.release(old).is_err());
    let new = a.admit(AllocationTag::Output, 20).unwrap();
    assert_ne!(old, new);
    assert!(a.release(old).is_err());
    assert_eq!(a.stats().output_bytes, 20);
    a.release(new).unwrap();
}
#[test]
fn ledger_budget_and_work_overflow_do_not_partially_charge() {
    let source = owner();
    let mut ledger = ChargeLedger::new(&source, source.capacity(), 1).unwrap();
    assert_eq!(
        ledger.admit(AllocationTag::Output, 1).unwrap_err().kind,
        ErrorKind::Limit(LimitKind::ReservedBytes)
    );
    assert_eq!(ledger.stats().live_bytes, source.capacity());
    let mut ledger = ChargeLedger::new(&source, 1 << 20, 1).unwrap();
    assert_eq!(
        ledger
            .admit(AllocationTag::None, usize::MAX)
            .unwrap_err()
            .kind,
        ErrorKind::Limit(LimitKind::Arithmetic)
    );
    assert!(ledger.stats().overflow);
    let mut work = RenderWorkStats {
        total_work: u64::MAX,
        ..Default::default()
    };
    let before = work;
    assert!(work.charge(WorkKind::ShapePixel, 1).is_err());
    assert_eq!(work, before);
}
#[test]
fn pattern_tickets_validate_owner_dimensions_sealing_single_use_and_cancel() {
    let mut b = builder();
    let root = b.root_id();
    let child = b.new_scene(viewport()).unwrap();
    let mut tickets = PatternTickets::new().unwrap();
    let d = PatternDescriptor {
        definition: ElementId(1),
        width: 1,
        height: 1,
        tile_width: 1.,
        tile_height: 1.,
        content_transform: vexel_geometry::affine_identity(),
        error_offset: 0,
    };
    assert!(
        tickets
            .request(b.scene(child).unwrap(), d, &mut Run)
            .is_err()
    );
    b.seal(child, &mut Run).unwrap();
    let ticket = tickets
        .request(b.scene(child).unwrap(), d, &mut Run)
        .unwrap()
        .ticket;
    assert!(
        tickets
            .request(b.scene(child).unwrap(), d, &mut Run)
            .is_err()
    );
    let mut other = PatternTickets::new().unwrap();
    let wrong = other
        .request(b.scene(child).unwrap(), d, &mut Run)
        .unwrap()
        .ticket;
    assert!(tickets.complete(wrong, pattern().tile, &mut Run).is_err());
    assert!(tickets.outstanding());
    let (scene, p) = tickets.complete(ticket, pattern().tile, &mut Run).unwrap();
    assert_eq!(scene, child);
    assert_eq!(p.tile.width(), 1);
    assert!(tickets.complete(ticket, pattern().tile, &mut Run).is_err());
    let next = tickets
        .request(b.scene(child).unwrap(), d, &mut Run)
        .unwrap()
        .ticket;
    tickets.cancel();
    assert!(tickets.complete(next, pattern().tile, &mut Run).is_err());
    assert_eq!(b.root_id(), root);
}
#[test]
fn changed_frame_tables_are_visible_and_the_base_remains_immutable() {
    let mut b = builder();
    let root = b.root_id();
    let t = b.append_transform(root, transform(None), &mut Run).unwrap();
    let c = b.append_clip(root, clip(), &mut Run).unwrap();
    let fp = b.append_fill_paint(root, paint(), &mut Run).unwrap();
    let mut input = shape(owner().as_slice());
    input.shape.source.transform_node = Some(t);
    input.shape.clip = Some(c);
    input.shape.fill_paint = Some(fp);
    let id = b.append_shape(root, input, &mut Run).unwrap();
    let graph = b.freeze().unwrap();
    let base = graph.root();
    let mut frame = FrameBuilder::new(base, 1 << 20).unwrap();
    let mut s = *base.shape(id).unwrap();
    s.stroke.width = 3.;
    s.stroke_inverse.e = 2.;
    frame.set_shape(id, s, &mut Run).unwrap();
    let mut tr = *base.transform(t).unwrap();
    tr.motion.e = 4.;
    frame.set_transform(t, tr, &mut Run).unwrap();
    let mut p = paint();
    p.coordinates[0] = 1.;
    frame.set_fill_paint(fp, p, &mut Run).unwrap();
    let mut cl = clip();
    cl.bounds[0] = 2.;
    frame.set_clip(c, cl, &mut Run).unwrap();
    let mut edges = base.edges.to_vec();
    edges[0].a.x += 1.;
    frame.replace_edges(edges, &mut Run).unwrap();
    let colors = vec![[7, 8, 9, 255]; base.edge_colors.len()];
    frame.replace_colors(colors, &mut Run).unwrap();
    let frame = frame.finish(&mut Run).unwrap();
    let view = frame.view();
    assert_eq!(view.shape(id).unwrap().stroke.width, 3.);
    assert_eq!(view.transform(t).unwrap().motion.e, 4.);
    assert_eq!(view.fill_paint(id), Some(&p));
    assert_eq!(view.clip(c).unwrap().bounds[0], 2.);
    assert_eq!(view.edge_colors[0], [7, 8, 9, 255]);
    assert_eq!(
        view.scene(view.id).unwrap().shape(id).unwrap().stroke.width,
        3.
    );
    assert_eq!(base.shape(id).unwrap().stroke.width, 1.);
    assert_eq!(base.clip(c).unwrap().bounds[0], 0.);
    assert_eq!(view.source().as_ptr(), base.source().as_ptr());
    assert_eq!(view.path_numbers.as_ptr(), base.path_numbers.as_ptr());
    assert!(frame.memory_stats().live_bytes > 0);
}
#[test]
fn frame_policy_wrong_handles_and_cancellation_keep_prior_state() {
    let mut b = builder();
    let root = b.root_id();
    let id = b
        .append_shape(root, shape(owner().as_slice()), &mut Run)
        .unwrap();
    let graph = b.freeze().unwrap();
    let base = graph.root();
    let mut f = FrameBuilder::new(base, 0).unwrap();
    let mut s = *base.shape(id).unwrap();
    s.stroke.width = 2.;
    assert_eq!(
        f.set_shape(id, s, &mut Run).unwrap_err().kind,
        ErrorKind::Limit(LimitKind::ReservedBytes)
    );
    assert!(f.finish(&mut Run).unwrap().view().shapes.as_ptr() == base.shapes.as_ptr());
    let mut f = FrameBuilder::new(base, 1 << 20).unwrap();
    assert_eq!(
        f.set_shape(id, s, &mut StopAfter(0)).unwrap_err().kind,
        ErrorKind::Canceled
    );
    let mut other = builder();
    let foreign = other
        .append_shape(other.root_id(), shape(owner().as_slice()), &mut Run)
        .unwrap();
    assert!(f.set_shape(foreign, s, &mut Run).is_err());
    let view = f.finish(&mut Run).unwrap();
    assert_eq!(view.view().shape(id).unwrap().stroke.width, 1.);
}
#[test]
fn scratch_and_cache_storage_include_all_nested_spare_capacities() {
    let mut scratch = RasterScratch::default();
    scratch.bounds.shapes = Vec::with_capacity(3);
    scratch.center_samples = Vec::with_capacity(17);
    scratch.dash.shapes = Vec::with_capacity(2);
    scratch.dash.shapes.push(DashRange {
        segments: TableRange::EMPTY,
        joins: TableRange::EMPTY,
        grid_cells: Vec::with_capacity(7),
        grid_refs: Vec::with_capacity(13),
        grid_min_x: 0.,
        grid_min_y: 0.,
        grid_cell_size: 1.,
        grid_columns: 0,
        grid_rows: 0,
    });
    let expected = scratch.bounds.shapes.capacity() * std::mem::size_of::<ShapeBounds>()
        + scratch.center_samples.capacity()
        + scratch.dash.shapes.capacity() * std::mem::size_of::<DashRange>()
        + scratch.dash.shapes[0].grid_cells.capacity() * std::mem::size_of::<GridCell>()
        + scratch.dash.shapes[0].grid_refs.capacity() * 4;
    assert_eq!(scratch.storage_bytes(&mut Run).unwrap(), expected);
    assert_eq!(
        scratch.storage_bytes(&mut StopAfter(0)).unwrap_err().kind,
        ErrorKind::Canceled
    );
    let cache = PaintCache {
        entries: Vec::with_capacity(2),
        slots: Vec::with_capacity(4),
        token_visits: 0,
    };
    assert_eq!(
        cache.storage_bytes().unwrap(),
        cache.entries.capacity() * std::mem::size_of::<GradientCacheEntry>()
            + cache.slots.capacity() * std::mem::size_of::<Option<usize>>()
    );
}
#[test]
fn checked_pixel_boundary_mapping_and_rect_contracts_reject_bad_input() {
    assert!(
        ViewMapping {
            scale_x: 0.,
            scale_y: 1.,
            offset_x: 0.,
            offset_y: 0.
        }
        .validate()
        .is_err()
    );
    assert!(
        ViewMapping {
            scale_x: 2.,
            scale_y: 2.,
            offset_x: 1.,
            offset_y: 1.
        }
        .point(Point { x: f64::MAX, y: 0. })
        .is_err()
    );
    assert!(
        PixelBounds {
            x_first: 4,
            x_last: 3,
            y_first: 0,
            y_last: 1,
            visible: true
        }
        .validate(10, 10)
        .is_err()
    );
    let mask = BoundaryMask {
        bits: vec![0b10],
        bit_count: 2,
        marked_pixel_count: 1,
        mark_work: 0,
    };
    assert!(mask.validate().is_ok());
    assert_eq!(mask.test(0), Some(false));
    assert_eq!(mask.test(1), Some(true));
    assert_eq!(mask.test(2), None);
    assert!(
        Rect {
            x: 0.,
            y: 0.,
            width: -1.,
            height: 1.
        }
        .validate()
        .is_err()
    );
}
#[test]
fn active_definition_stacks_reject_cycles_depth_and_wrong_unwind() {
    let mut active = ActiveDefinitions::default();
    active
        .push(DecodeFlavor::Mask, ElementId(1), &mut Run)
        .unwrap();
    assert!(
        active
            .push(DecodeFlavor::Mask, ElementId(1), &mut Run)
            .is_err()
    );
    assert!(active.pop(DecodeFlavor::Mask, ElementId(2)).is_err());
    active.pop(DecodeFlavor::Mask, ElementId(1)).unwrap();
    for i in 0..8 {
        active
            .push(DecodeFlavor::Pattern, ElementId(i), &mut Run)
            .unwrap();
    }
    assert_eq!(
        active
            .push(DecodeFlavor::Pattern, ElementId(8), &mut Run)
            .unwrap_err()
            .kind,
        ErrorKind::Limit(LimitKind::Depth)
    );
    assert!(active.storage_bytes().unwrap() > 0);
}
#[test]
fn canonical_source_queries_use_real_css_and_reference_engines() {
    let source = Arc::new(br#"<svg fill="red"><path id="p"/></svg>"#.to_vec());
    let lexical = LexicalDocument::parse(source.clone(), 256 * 1024 * 1024, &mut Run).unwrap();
    let mut shared = DecodeSource::new(&lexical, &mut Run).unwrap();
    let style = shared.compute_style(ElementId(0), None, &mut Run).unwrap();
    assert_eq!(style.value(Property::Fill).unwrap().bytes, b"red");
    let offset = source.windows(4).position(|b| b == b"\"p\"/").unwrap() + 1;
    assert!(
        shared
            .find_reference(XmlSpan { offset, length: 1 }, &mut Run)
            .unwrap()
            .is_some()
    );
    assert!(shared.element(ElementId(usize::MAX), 1, &mut Run).is_err());
    assert!(DecodeSource::new(&lexical, &mut StopAfter(0)).is_err());
    assert!(Arc::ptr_eq(shared.source_owner(), &source));
}
// Trait implementation is compile/lifetime evidence only. Unsupported unused
// methods cannot be mistaken for a production decoder or scalar kernel.
struct ContractAdapter;
impl DecodeOperations for ContractAdapter {
    fn resolve_paint(
        &mut self,
        _: &mut ResolveContext<'_, '_>,
        _: XmlSpan,
        _: &[Edge],
        _: Affine,
        _: [f64; 2],
        _: &mut dyn WorkControl,
    ) -> Result<ResolvedPaint> {
        Err(Diagnostic {
            kind: ErrorKind::Unsupported,
            offset: 0,
        })
    }
    fn plan_definition<'src, 'ctx>(
        &mut self,
        ctx: &mut DefinitionContext<'src, 'ctx>,
        _: DefinitionKind,
        element: ElementId,
        _: ShapeId,
        transform: Affine,
        _: [f64; 4],
        _: &CssPaint<'src>,
        control: &mut dyn WorkControl,
    ) -> Result<DefinitionPlan<'src>> {
        ctx.validate()?;
        let style = ctx.shared.compute_style(element, None, control)?;
        Ok(DefinitionPlan {
            descriptor: DefinitionDescriptor::Marker(MarkerDescriptor {
                definition: element,
                viewport: [1., 1.],
                transform,
                error_offset: 0,
            }),
            expansions: vec![Expansion {
                range: TableRange {
                    first: element.0,
                    count: 1,
                },
                transform,
                viewport: [1., 1.],
                style,
                flavor: DecodeFlavor::Marker,
                post: PostOperation::FinishMarker,
            }],
            post: PostOperation::None,
        })
    }
    fn decode_text(
        &mut self,
        _: TextDecodeRequest<'_, '_>,
        _: &mut SceneBuilder,
        _: &mut dyn WorkControl,
    ) -> Result<bool> {
        Err(Diagnostic {
            kind: ErrorKind::Unsupported,
            offset: 0,
        })
    }
    fn decode_image(
        &mut self,
        _: ImageDecodeRequest<'_, '_>,
        _: &mut SceneBuilder,
        _: &mut dyn WorkControl,
    ) -> Result<ShapeId> {
        Err(Diagnostic {
            kind: ErrorKind::Unsupported,
            offset: 0,
        })
    }
}
#[test]
fn definition_plan_outlives_mutable_context_and_control_without_source_copy() {
    let source = owner();
    let lexical = LexicalDocument::parse(source.clone(), 256 * 1024 * 1024, &mut Run).unwrap();
    let mut shared = DecodeSource::new(&lexical, &mut Run).unwrap();
    let mut b = SceneBuilder::new(
        source.clone(),
        animation(&source),
        0,
        viewport(),
        Default::default(),
    )
    .unwrap();
    let root = b.root_id();
    let id = b.append_shape(root, shape(&source), &mut Run).unwrap();
    let mut active = ActiveDefinitions::default();
    let mut ledger = ChargeLedger::new(&source, 1 << 20, 64).unwrap();
    let mut adapter = ContractAdapter;
    let plan = {
        let mut ctx = DefinitionContext {
            shared: &mut shared,
            builder: &mut b,
            scene: root,
            active: &mut active,
            ledger: &mut ledger,
        };
        let operations: &mut dyn DecodeOperations = &mut adapter;
        operations
            .plan_definition(
                &mut ctx,
                DefinitionKind::Marker,
                ElementId(0),
                id,
                vexel_geometry::affine_identity(),
                [0., 0., 1., 1.],
                &CssPaint::initial(),
                &mut Run,
            )
            .unwrap()
    };
    shared.compute_style(ElementId(0), None, &mut Run).unwrap();
    b.record_ignored(IgnoredKind::Meta, 0).unwrap();
    assert_eq!(plan.expansions.len(), 1);
    assert_eq!(plan.expansions[0].range.first, 0);
}
#[test]
fn temporary_definition_context_rejects_an_equal_bytes_wrong_source_owner() {
    let source = owner();
    let other = Arc::new(source.as_slice().to_vec());
    let lexical = LexicalDocument::parse(source.clone(), 256 * 1024 * 1024, &mut Run).unwrap();
    let mut shared = DecodeSource::new(&lexical, &mut Run).unwrap();
    let mut b = SceneBuilder::new(
        other.clone(),
        animation(&other),
        0,
        viewport(),
        Default::default(),
    )
    .unwrap();
    let root = b.root_id();
    let mut active = ActiveDefinitions::default();
    let mut ledger = ChargeLedger::new(&source, 1 << 20, 64).unwrap();
    let ctx = DefinitionContext {
        shared: &mut shared,
        builder: &mut b,
        scene: root,
        active: &mut active,
        ledger: &mut ledger,
    };
    assert_eq!(ctx.validate().unwrap_err().kind, ErrorKind::InvalidArgument);
}
#[test]
fn all_public_dependency_traits_are_object_safe_without_stub_backends() {
    fn interfaces(
        _: Option<&dyn KernelResources>,
        _: Option<&mut dyn FlatKernel>,
        _: Option<&mut dyn DecodeOperations>,
        _: Option<&mut dyn DecodeSession>,
    ) {
    }
    interfaces(None, None, None, None);
}
#[test]
fn canonical_element_views_are_borrowed_from_one_private_lexical_owner() {
    let source = owner();
    let lexical = LexicalDocument::parse(source.clone(), 1 << 20, &mut Run).unwrap();
    let shared = DecodeSource::new(&lexical, &mut Run).unwrap();
    let view = shared
        .element(ElementId(0), lexical.tokens().len(), &mut Run)
        .unwrap();
    assert_eq!(view.source().as_ptr(), source.as_ptr());
    assert_eq!(view.tokens().as_ptr(), lexical.tokens().as_ptr());
    assert_eq!(
        view.validate(&mut StopAfter(1)).unwrap_err().kind,
        ErrorKind::Canceled
    );
}
#[test]
fn malformed_dash_ranges_cells_and_global_tagged_references_are_rejected() {
    let edge = Edge {
        a: Point { x: 0., y: 0. },
        b: Point { x: 1., y: 1. },
    };
    let join = DashJoin {
        previous: edge.a,
        vertex: edge.b,
        next: edge.a,
    };
    let mut cache = DashCache {
        segments: vec![
            DashSegment {
                edge,
                start_cap: Linecap::Butt,
                end_cap: Linecap::Butt
            };
            2
        ],
        joins: vec![join; 2],
        shapes: vec![DashRange {
            segments: TableRange { first: 1, count: 1 },
            joins: TableRange { first: 1, count: 1 },
            grid_cells: vec![GridCell { first: 0, count: 2 }],
            grid_refs: vec![1, STROKE_GRID_JOIN_BIT | 1],
            grid_min_x: 0.,
            grid_min_y: 0.,
            grid_cell_size: 1.,
            grid_columns: 1,
            grid_rows: 1,
        }],
        ..Default::default()
    };
    assert!(cache.validate(&mut Run).is_ok());
    cache.shapes[0].segments.first = usize::MAX;
    assert!(cache.validate(&mut Run).is_err());
    cache.shapes[0].segments.first = 1;
    cache.shapes[0].grid_refs[1] = STROKE_GRID_JOIN_BIT | 2;
    assert!(cache.validate(&mut Run).is_err());
    cache.shapes[0].grid_refs[1] = STROKE_GRID_JOIN_BIT | 1;
    cache.shapes[0].grid_cells[0].count = 3;
    assert!(cache.validate(&mut Run).is_err());
    assert_eq!(
        cache.validate(&mut StopAfter(0)).unwrap_err().kind,
        ErrorKind::Canceled
    );
}
#[test]
fn lexical_ownership_moves_token_storage_without_copy_and_returns_typed_errors() {
    let source = owner();
    let ptr = source.as_ptr();
    let lexical = LexicalDocument::parse(source.clone(), 1 << 20, &mut Run).unwrap();
    assert_eq!(lexical.source().as_ptr(), ptr);
    assert!(
        lexical.token_storage_bytes().unwrap()
            >= lexical.tokens().len() * std::mem::size_of::<vexel_xml::XmlToken>()
    );
    assert_eq!(
        LexicalDocument::parse(source.clone(), 1, &mut Run)
            .unwrap_err()
            .kind,
        ErrorKind::Limit(LimitKind::TokenBytes)
    );
    assert_eq!(
        LexicalDocument::parse(source.clone(), 1 << 20, &mut StopAfter(4))
            .unwrap_err()
            .kind,
        ErrorKind::Canceled
    );
    let bad = Arc::new(b"<svg>".to_vec());
    assert!(matches!(
        LexicalDocument::parse(bad, 1 << 20, &mut Run)
            .unwrap_err()
            .kind,
        ErrorKind::Xml(_)
    ));
}
