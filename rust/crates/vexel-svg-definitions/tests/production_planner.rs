#![allow(dead_code)]

use std::sync::atomic::AtomicBool;
use vexel_base::{BudgetControl, Span};
use vexel_geometry::{Affine, affine_identity};
use vexel_scene::*;
use vexel_svg_css::Property;
use vexel_svg_definitions::DefinitionPlanner as LocalDefinitionPlanner;
use vexel_svg_definitions::resolve_local_definition;

mod common;
use common::{attribute, fixture, named_element};

fn append_source_path(
    fixture: &mut common::Fixture,
    scene: SceneId,
    control: &mut dyn vexel_base::WorkControl,
) -> ShapeId {
    let token = &fixture.document.tokens()[fixture.owner_element.0];
    let data = attribute(&fixture.source, fixture.document.tokens(), token, b"d").unwrap();
    let path = vexel_svg_path::parse_path(
        &fixture.source,
        Span {
            start: data.offset,
            end: data.offset + data.length,
        },
        affine_identity(),
        vexel_svg_path::ParseOptions::default(),
    )
    .unwrap();
    let source = SourceMap {
        element: Span {
            start: token.source.offset,
            end: token.source.offset + token.source.length,
        },
        transform: affine_identity(),
        transform_node: None,
    };
    let mut shape = Shape::new(source);
    shape.edges.count = path.edges.len();
    shape.subpaths.count = path.subpaths.len();
    let colors = vec![[0, 0, 0, 255]; path.edges.len()];
    fixture
        .builder
        .append_shape(
            scene,
            ShapeInput {
                shape,
                path,
                edge_colors: colors,
            },
            control,
        )
        .unwrap()
}

#[test]
fn production_pattern_plan_carries_complete_actual_source_mapping() {
    let xml = br#"<svg width="20" height="20" viewBox="0 0 20 20"><defs><pattern id="p" x=".1" y=".2" width=".5" height=".25" patternUnits="objectBoundingBox" patternTransform="translate(1 2)"><path d="M0 0L1 0L1 1Z"/></pattern></defs><path id="owner" fill="url(#p)" stroke="url(#p)" fill-opacity=".25" stroke-opacity=".75" d="M0 0L10 0L10 10Z"/></svg>"#;
    let mut fixture = fixture(xml);
    let cancel = AtomicBool::new(false);
    let mut control = BudgetControl::new(&cancel, u64::MAX);
    let owner_scene = fixture
        .builder
        .new_scene(fixture.builder.scene(fixture.scene).unwrap().viewport)
        .unwrap();
    let draft = fixture.builder.begin_draft(owner_scene).unwrap();
    let owner_shape = append_source_path(&mut fixture, owner_scene, &mut control);
    let mut shared = DecodeSource::new(&fixture.document, &mut control).unwrap();
    let style = shared
        .compute_style(fixture.owner_element, None, &mut control)
        .unwrap();
    let fill_pattern = resolve_local_definition(
        &mut shared,
        style.value(Property::Fill).unwrap().span,
        DefinitionKind::Pattern,
        &mut control,
    )
    .unwrap();
    let stroke_pattern = resolve_local_definition(
        &mut shared,
        style.value(Property::Stroke).unwrap().span,
        DefinitionKind::Pattern,
        &mut control,
    )
    .unwrap();
    assert_eq!(fill_pattern, stroke_pattern);
    let pattern = fill_pattern;
    let owner = fixture
        .builder
        .definition_owner(
            &draft,
            &shared,
            fixture.owner_element,
            DefinitionTarget::Shape(owner_shape),
            &mut control,
        )
        .unwrap();
    let actual_source_map = fixture
        .builder
        .scene(owner_scene)
        .unwrap()
        .shape(owner_shape)
        .unwrap()
        .source;
    let mut active = ActiveDefinitions::default();
    let mut planner = LocalDefinitionPlanner::new();
    let mut context = DefinitionContext {
        shared: &mut shared,
        builder: &mut fixture.builder,
        scene: owner_scene,
        active: &mut active,
        ledger: &mut fixture.ledger,
    };
    let bounds = [2.0, 3.0, 12.0, 13.0];
    let mut channel_mappings = Vec::new();
    let mut pattern_descriptor = None;
    for (channel, expected_opacity) in [(PaintChannel::Fill, 0.25), (PaintChannel::Stroke, 0.75)] {
        let planned = plan_production_pattern(
            &mut planner,
            &mut context,
            pattern,
            owner,
            channel,
            affine_identity(),
            bounds,
            &style,
            &mut control,
        )
        .unwrap();
        let ProductionDefinitionPlan::Pattern { plan, mapping } = planned else {
            panic!("pattern mapping must be retained")
        };
        let DefinitionDescriptor::Pattern(descriptor) = plan.descriptor else {
            panic!("pattern descriptor")
        };
        assert!((mapping.opacity - expected_opacity).abs() < 1e-12);
        channel_mappings.push((channel, mapping));
        pattern_descriptor = Some(descriptor);
    }
    let descriptor = pattern_descriptor.unwrap();
    let mapping = channel_mappings[0].1;
    assert_eq!(descriptor.definition, pattern);
    assert_eq!(mapping.tile_origin, [3.0, 5.0]);
    assert_eq!(
        mapping.pattern_transform,
        Affine {
            e: 1.0,
            f: 2.0,
            ..affine_identity()
        }
    );
    assert_eq!(mapping.bounds, bounds);
    assert_eq!(mapping.source_map, actual_source_map);
    assert_eq!(mapping.opacity, 0.25);
    assert_eq!(
        mapping.pixel_inverse(descriptor).unwrap(),
        Affine {
            a: 4.0,
            d: 4.0,
            e: -16.0,
            f: -28.0,
            ..affine_identity()
        }
    );
    drop(context);
    assert!(fixture.ledger.stats().reference_index_bytes > 0);

    let child = fixture
        .builder
        .new_scene(fixture.builder.scene(owner_scene).unwrap().viewport)
        .unwrap();
    fixture.builder.seal(child, &mut control).unwrap();
    let mut job =
        RasterJob::new(ChargeLedger::new(&fixture.source, 4 * 1024 * 1024, 128).unwrap()).unwrap();
    let mut tickets = PatternTickets::new().unwrap();
    assert!(
        mapping
            .source_map
            .element
            .slice(fixture.builder.source_owner())
            .is_some()
    );
    assert!(mapping.pixel_inverse(descriptor).is_ok());
    assert!(job.validate_source(fixture.builder.source_owner()).is_ok());
    for (channel, mapping) in channel_mappings {
        let request = tickets
            .request_mapped(
                &fixture.builder,
                &draft,
                &shared,
                child,
                descriptor,
                mapping,
                owner_shape,
                channel,
                &job,
                &mut control,
            )
            .unwrap();
        let output_pixels = job.buffer(RasterPool::Output, 1024, &mut control).unwrap();
        let pixel_bytes = (descriptor.width as usize) * (descriptor.height as usize) * 4;
        job.reserve(output_pixels, pixel_bytes, &mut control)
            .unwrap();
        let pixels = vec![255; pixel_bytes];
        job.extend(output_pixels, &pixels, &mut control).unwrap();
        let output = JobRenderOutput {
            generation: 1,
            pixels: output_pixels,
            width: descriptor.width,
            height: descriptor.height,
            stride: descriptor.width as usize * 4,
        };
        let (pattern_id, paint_index) = tickets
            .complete_mapped(
                request.ticket,
                &mut fixture.builder,
                &draft,
                &mut job,
                Ok(output),
                &mut control,
            )
            .unwrap();
        let view = fixture.builder.scene(owner_scene).unwrap();
        let paint = match channel {
            PaintChannel::Fill => &view.fill_paints[paint_index],
            PaintChannel::Stroke => &view.stroke_paints[paint_index],
        };
        assert_eq!(paint.pattern, Some(pattern_id));
        assert_eq!(
            paint.pattern_inverse,
            mapping.pixel_inverse(descriptor).unwrap()
        );
        assert_eq!(paint.user_inverse, mapping.user_inverse);
        assert_eq!(paint.bounds, mapping.bounds);
        assert_eq!(paint.pattern_opacity, mapping.opacity);
    }
    fixture.builder.commit_draft(&draft, &mut control).unwrap();
}

#[test]
fn production_filter_plan_uses_actual_empty_group_owner() {
    let xml = br#"<svg width="20" height="20"><defs><filter id="f"><feGaussianBlur stdDeviation="1"/></filter></defs><g id="group-owner"></g><path id="owner" d="M0 0L10 0L10 10Z"/></svg>"#;
    let mut fixture = fixture(xml);
    let cancel = AtomicBool::new(false);
    let mut control = BudgetControl::new(&cancel, u64::MAX);
    let mut shared = DecodeSource::new(&fixture.document, &mut control).unwrap();
    let filter = named_element(&fixture.source, fixture.document.tokens(), b"filter", b"f");
    let group_element = named_element(
        &fixture.source,
        fixture.document.tokens(),
        b"g",
        b"group-owner",
    );
    let style = shared
        .compute_style(group_element, None, &mut control)
        .unwrap();
    let draft = fixture.builder.begin_draft(fixture.scene).unwrap();
    let group = fixture
        .builder
        .append_group(
            fixture.scene,
            OpacityGroup {
                shapes: TableRange::EMPTY,
                parent: None,
                opacity: 1.0,
                filter: None,
                mask: None,
                blend_mode: 0,
                isolation: false,
            },
            &mut control,
        )
        .unwrap();
    assert_eq!(
        fixture
            .builder
            .scene(fixture.scene)
            .unwrap()
            .group(group)
            .unwrap()
            .shapes,
        TableRange::EMPTY
    );
    let owner = fixture
        .builder
        .definition_owner(
            &draft,
            &shared,
            group_element,
            DefinitionTarget::Group(group),
            &mut control,
        )
        .unwrap();
    let mut active = ActiveDefinitions::default();
    let mut planner = LocalDefinitionPlanner::new();
    let mut context = DefinitionContext {
        shared: &mut shared,
        builder: &mut fixture.builder,
        scene: fixture.scene,
        active: &mut active,
        ledger: &mut fixture.ledger,
    };
    let planned = plan_production_definition(
        &mut planner,
        &mut context,
        DefinitionKind::Filter,
        filter,
        owner,
        affine_identity(),
        [0.0, 0.0, 10.0, 10.0],
        &style,
        &mut control,
    )
    .unwrap();
    let ProductionDefinitionPlan::Definition(plan) = planned else {
        panic!("filter plan")
    };
    let DefinitionDescriptor::Filter { filter, primitives } = plan.descriptor else {
        panic!("typed filter")
    };
    assert_eq!(primitives.len(), 1);
    assert_eq!(plan.post, PostOperation::FinishGroup(group));
    assert!(plan.expansions.is_empty());
    fixture
        .builder
        .publish_group_filter(&draft, owner, filter, &primitives, &mut control)
        .unwrap();
    assert!(
        fixture
            .builder
            .scene(fixture.scene)
            .unwrap()
            .group(group)
            .unwrap()
            .filter
            .is_some()
    );
    fixture.builder.rollback_draft(draft).unwrap();
}

#[test]
fn omitted_selected_channel_opacity_defaults_to_one_through_publication() {
    let xml = br#"<svg width="20" height="20"><defs><pattern id="p" width="2" height="2"><path d="M0 0L1 0L1 1Z"/></pattern></defs><path id="owner" fill="url(#p)" d="M0 0L10 0L10 10Z"/></svg>"#;
    let mut fixture = fixture(xml);
    let cancel = AtomicBool::new(false);
    let mut control = BudgetControl::new(&cancel, u64::MAX);
    let owner_scene = fixture
        .builder
        .new_scene(fixture.builder.scene(fixture.scene).unwrap().viewport)
        .unwrap();
    let draft = fixture.builder.begin_draft(owner_scene).unwrap();
    let owner_shape = append_source_path(&mut fixture, owner_scene, &mut control);
    let mut shared = DecodeSource::new(&fixture.document, &mut control).unwrap();
    let style = shared
        .compute_style(fixture.owner_element, None, &mut control)
        .unwrap();
    let definition = named_element(&fixture.source, fixture.document.tokens(), b"pattern", b"p");
    let owner = fixture
        .builder
        .definition_owner(
            &draft,
            &shared,
            fixture.owner_element,
            DefinitionTarget::Shape(owner_shape),
            &mut control,
        )
        .unwrap();
    let mut active = ActiveDefinitions::default();
    let mut planner = LocalDefinitionPlanner::new();
    let mut context = DefinitionContext {
        shared: &mut shared,
        builder: &mut fixture.builder,
        scene: owner_scene,
        active: &mut active,
        ledger: &mut fixture.ledger,
    };
    let mut channel_plans = Vec::new();
    for channel in [PaintChannel::Fill, PaintChannel::Stroke] {
        let planned = plan_production_pattern(
            &mut planner,
            &mut context,
            definition,
            owner,
            channel,
            affine_identity(),
            [0.0, 0.0, 10.0, 10.0],
            &style,
            &mut control,
        )
        .unwrap();
        let ProductionDefinitionPlan::Pattern { plan, mapping } = planned else {
            panic!("pattern mapping")
        };
        let DefinitionDescriptor::Pattern(descriptor) = plan.descriptor else {
            panic!("pattern descriptor")
        };
        assert_eq!(mapping.opacity, 1.0);
        channel_plans.push((channel, descriptor, mapping));
    }
    drop(context);
    let child = fixture
        .builder
        .new_scene(fixture.builder.scene(fixture.scene).unwrap().viewport)
        .unwrap();
    fixture.builder.seal(child, &mut control).unwrap();
    let mut job =
        RasterJob::new(ChargeLedger::new(&fixture.source, 4 * 1024 * 1024, 128).unwrap()).unwrap();
    let mut tickets = PatternTickets::new().unwrap();
    for (channel, descriptor, mapping) in channel_plans {
        assert!(job.validate_source(fixture.builder.source_owner()).is_ok());
        assert_eq!(
            fixture
                .builder
                .scene(owner_scene)
                .unwrap()
                .shape(owner_shape)
                .unwrap()
                .source,
            mapping.source_map
        );
        assert!(
            mapping
                .source_map
                .element
                .slice(fixture.builder.source_owner())
                .is_some()
        );
        assert!(mapping.pixel_inverse(descriptor).is_ok());
        assert!(std::sync::Arc::ptr_eq(
            fixture.builder.source_owner(),
            shared.source_owner()
        ));
        assert!(fixture.builder.scene(child).unwrap().scene(child).is_some());
        assert_eq!(
            fixture.document.tokens()[descriptor.definition.0]
                .local_name
                .slice(&fixture.source),
            Some(b"pattern".as_slice())
        );
        assert!(descriptor.width > 0 && descriptor.width <= 4096);
        assert!(descriptor.height > 0 && descriptor.height <= 4096);
        assert!(descriptor.tile_width.is_finite() && descriptor.tile_width > 0.0);
        assert!(descriptor.tile_height.is_finite() && descriptor.tile_height > 0.0);
        assert!(vexel_geometry::affine_valid(descriptor.content_transform));
        let request = tickets
            .request_mapped(
                &fixture.builder,
                &draft,
                &shared,
                child,
                descriptor,
                mapping,
                owner_shape,
                channel,
                &job,
                &mut control,
            )
            .unwrap();
        let pixel_bytes = (descriptor.width as usize)
            .checked_mul(descriptor.height as usize)
            .and_then(|value| value.checked_mul(4))
            .unwrap();
        let pixels = job
            .buffer(RasterPool::Output, pixel_bytes, &mut control)
            .unwrap();
        job.reserve(pixels, pixel_bytes, &mut control).unwrap();
        job.extend(pixels, &vec![255; pixel_bytes], &mut control)
            .unwrap();
        let (_, paint_index) = tickets
            .complete_mapped(
                request.ticket,
                &mut fixture.builder,
                &draft,
                &mut job,
                Ok(JobRenderOutput {
                    generation: 1,
                    pixels,
                    width: descriptor.width,
                    height: descriptor.height,
                    stride: descriptor.width as usize * 4,
                }),
                &mut control,
            )
            .unwrap();
        let view = fixture.builder.scene(owner_scene).unwrap();
        let paint = match channel {
            PaintChannel::Fill => &view.fill_paints[paint_index],
            PaintChannel::Stroke => &view.stroke_paints[paint_index],
        };
        assert_eq!(paint.pattern_opacity, 1.0);
    }
    fixture.builder.rollback_draft(draft).unwrap();
}

#[test]
fn late_plan_capacity_failure_releases_intermediate_leases_and_planner_counters() {
    let xml = br#"<svg width="20" height="20"><defs><pattern id="p" width="2" height="2"><path d="M0 0L1 0L1 1Z"/></pattern></defs><path id="owner" fill="url(#p)" d="M0 0L10 0L10 10Z"/></svg>"#;
    let mut fixture = fixture(xml);
    let cancel = AtomicBool::new(false);
    let mut control = BudgetControl::new(&cancel, u64::MAX);
    let owner_scene = fixture
        .builder
        .new_scene(fixture.builder.scene(fixture.scene).unwrap().viewport)
        .unwrap();
    let draft = fixture.builder.begin_draft(owner_scene).unwrap();
    let shape = append_source_path(&mut fixture, owner_scene, &mut control);
    let mut shared = DecodeSource::new(&fixture.document, &mut control).unwrap();
    let style = shared
        .compute_style(fixture.owner_element, None, &mut control)
        .unwrap();
    let definition = named_element(&fixture.source, fixture.document.tokens(), b"pattern", b"p");
    let owner = fixture
        .builder
        .definition_owner(
            &draft,
            &shared,
            definition,
            DefinitionTarget::Shape(shape),
            &mut control,
        )
        .unwrap();
    fixture.ledger =
        ChargeLedger::new(&fixture.source, fixture.source.capacity() + 1024, 128).unwrap();
    let mut active = ActiveDefinitions::default();
    let mut planner = LocalDefinitionPlanner::new();
    {
        let mut context = DefinitionContext {
            shared: &mut shared,
            builder: &mut fixture.builder,
            scene: owner_scene,
            active: &mut active,
            ledger: &mut fixture.ledger,
        };
        let error = plan_production_definition(
            &mut planner,
            &mut context,
            DefinitionKind::Pattern,
            definition,
            owner,
            affine_identity(),
            [0.0, 0.0, 10.0, 10.0],
            &style,
            &mut control,
        )
        .unwrap_err();
        assert_eq!(error.kind, ErrorKind::Limit(LimitKind::ReservedBytes));
    }
    assert_eq!(fixture.ledger.stats().reference_index_bytes, 0);
    assert_eq!(fixture.ledger.stats().live_bytes, fixture.source.capacity());

    fixture.ledger = ChargeLedger::new(&fixture.source, 32 * 1024 * 1024, 128).unwrap();
    let mut context = DefinitionContext {
        shared: &mut shared,
        builder: &mut fixture.builder,
        scene: owner_scene,
        active: &mut active,
        ledger: &mut fixture.ledger,
    };
    assert!(
        plan_production_definition(
            &mut planner,
            &mut context,
            DefinitionKind::Pattern,
            definition,
            owner,
            affine_identity(),
            [0.0, 0.0, 10.0, 10.0],
            &style,
            &mut control,
        )
        .is_ok()
    );
}
