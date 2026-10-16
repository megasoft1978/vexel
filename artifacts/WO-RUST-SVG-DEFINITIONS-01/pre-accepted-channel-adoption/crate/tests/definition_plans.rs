use std::sync::atomic::AtomicBool;
use vexel_base::{BudgetControl, Span};
use vexel_geometry::{Affine, Point, affine_identity};
use vexel_scene::*;
use vexel_svg_css::Property;
use vexel_svg_definitions::{
    DefinitionPlanner, MarkerOrient, MarkerPosition, apply_group, clone_clips, contains,
    create_rect, local_to_document, marker_reference_value, parse_marker_attributes_for_viewport,
    parse_pattern_mapping, resolve_local_definition, update_owner, visit_boundary_edges,
    work_counts, work_units,
};

mod common;
use common::*;

#[test]
fn original_clip_rule_inheritance_and_rejections_are_preserved() {
    let inherited = br#"<svg width="10" height="10"><defs><clipPath id="c"><g clip-rule="evenodd"><path d="M0 0H10V10H0Z M2 2H8V8H2Z"/></g></clipPath></defs><path id="owner" d="M0 0L10 0L10 10Z" clip-path="url(#c)"/></svg>"#;
    assert_eq!(
        clip_plan_rules(inherited).unwrap(),
        vec![Some(b"evenodd".to_vec())]
    );
    let stylesheet = br#"<svg><defs><clipPath id="c"><style>.evenodd{clip-rule:evenodd}</style><path class="evenodd" d="M0 0H1V1Z"/></clipPath></defs><path id="owner" d="M0 0L1 0L1 1Z"/></svg>"#;
    assert_eq!(
        clip_plan_rules(stylesheet).unwrap(),
        vec![Some(b"evenodd".to_vec())]
    );

    let owner_rule_only = br#"<svg width="10" height="10"><defs><clipPath id="c"><path d="M0 0H10V10H0Z M2 2H8V8H2Z"/></clipPath></defs><path id="owner" d="M0 0L10 0L10 10Z" clip-rule="evenodd" clip-path="url(#c)"/></svg>"#;
    assert_eq!(clip_plan_rules(owner_rule_only).unwrap(), vec![None]);

    for (xml, bad_value) in [
        (
            br#"<svg><defs><clipPath id="c"><path clip-rule="invalid" d="M0 0H1V1Z"/></clipPath></defs><path id="owner" d="M0 0L1 0L1 1Z"/></svg>"#.as_slice(),
            b"invalid".as_slice(),
        ),
        (
            br#"<svg><defs><clipPath id="c"><path fill="red" d="M0 0H1V1Z"/></clipPath></defs><path id="owner" d="M0 0L1 0L1 1Z"/></svg>"#.as_slice(),
            b"red".as_slice(),
        ),
    ] {
        let error = clip_plan_result(xml, b"c").unwrap_err();
        assert_eq!(error.kind, ErrorKind::Unsupported);
        assert_eq!(error.offset, xml.windows(bad_value.len()).position(|w| w == bad_value).unwrap());
    }
}

#[test]
fn original_clip_source_and_aggregate_boundaries_are_preserved() {
    let child_tail = b"--><path d=\"M0 0L1 0L1 1Z\"/>";
    let single_limit = 64 * 1024;
    let fixed = b"<!--".len() + child_tail.len();
    let exact = clip_boundary_source(single_limit, 1);
    assert_eq!(clip_plan_result(&exact, b"c0"), Ok(()));
    let over = clip_boundary_source(single_limit + 1, 1);
    assert_eq!(
        clip_plan_result(&over, b"c0").unwrap_err().kind,
        ErrorKind::Limit(LimitKind::References)
    );

    let object_prefix = b"<svg width=\"1\" height=\"1\" viewBox=\"0 0 1 1\">".len();
    let wrapper = object_prefix + b"</svg>".len() + 1;
    let aggregate_limit = 512 * 1024;
    assert_eq!(
        (wrapper + (aggregate_limit / 8 - wrapper)) * 8,
        aggregate_limit
    );
    let aggregate_child = aggregate_limit / 8 - wrapper;
    assert!(aggregate_child <= single_limit);
    assert!(aggregate_child >= fixed);
    let exact_aggregate = clip_boundary_source(aggregate_child, 8);
    let mut fixture = fixture(&exact_aggregate);
    let cancel = AtomicBool::new(false);
    let mut control = BudgetControl::new(&cancel, u64::MAX);
    let mut shared = DecodeSource::new(&fixture.document, &mut control).unwrap();
    let style = shared
        .compute_style(fixture.owner_element, None, &mut control)
        .unwrap();
    let mut active = ActiveDefinitions::default();
    let mut planner = DefinitionPlanner::new();
    for i in 0..8 {
        let id = format!("c{i}");
        let definition = named_element(
            &fixture.source,
            fixture.document.tokens(),
            b"clipPath",
            id.as_bytes(),
        );
        let mut context = DefinitionContext {
            shared: &mut shared,
            builder: &mut fixture.builder,
            scene: fixture.scene,
            active: &mut active,
            ledger: &mut fixture.ledger,
        };
        planner
            .plan(
                &mut context,
                DefinitionKind::Clip,
                definition,
                fixture.owner,
                affine_identity(),
                [0.0, 0.0, 1.0, 1.0],
                &style,
                &mut control,
            )
            .unwrap();
    }
    let definition = named_element(
        &fixture.source,
        fixture.document.tokens(),
        b"clipPath",
        b"c0",
    );
    let mut context = DefinitionContext {
        shared: &mut shared,
        builder: &mut fixture.builder,
        scene: fixture.scene,
        active: &mut active,
        ledger: &mut fixture.ledger,
    };
    assert_eq!(
        planner
            .plan(
                &mut context,
                DefinitionKind::Clip,
                definition,
                fixture.owner,
                affine_identity(),
                [0.0, 0.0, 1.0, 1.0],
                &style,
                &mut control,
            )
            .unwrap_err()
            .kind,
        ErrorKind::Limit(LimitKind::References)
    );
}

#[test]
fn real_source_scene_plans_all_five_definition_kinds() {
    let mut fixture = fixture(SVG);
    let cancel = AtomicBool::new(false);
    let mut control = BudgetControl::new(&cancel, u64::MAX);
    let mut shared = DecodeSource::new(&fixture.document, &mut control).unwrap();
    let owner_style = shared
        .compute_style(fixture.owner_element, None, &mut control)
        .unwrap();
    let clip_value = owner_style.value(Property::ClipPath).unwrap();
    assert_eq!(
        resolve_local_definition(
            &mut shared,
            clip_value.span,
            DefinitionKind::Clip,
            &mut control,
        )
        .unwrap(),
        named_element(
            &fixture.source,
            fixture.document.tokens(),
            b"clipPath",
            b"c"
        )
    );
    for (property, kind, name, id) in [
        (
            Property::Fill,
            DefinitionKind::Pattern,
            b"pattern".as_slice(),
            b"p".as_slice(),
        ),
        (
            Property::Mask,
            DefinitionKind::Mask,
            b"mask".as_slice(),
            b"m".as_slice(),
        ),
        (
            Property::Filter,
            DefinitionKind::Filter,
            b"filter".as_slice(),
            b"f".as_slice(),
        ),
    ] {
        let value = owner_style.value(property).unwrap();
        assert_eq!(
            resolve_local_definition(&mut shared, value.span, kind, &mut control).unwrap(),
            named_element(&fixture.source, fixture.document.tokens(), name, id)
        );
    }
    let mut active = ActiveDefinitions::default();
    let mut planner = DefinitionPlanner::new();
    let bounds = [2.0, 3.0, 12.0, 13.0];
    let identity = affine_identity();

    let plan = {
        let mut context = DefinitionContext {
            shared: &mut shared,
            builder: &mut fixture.builder,
            scene: fixture.scene,
            active: &mut active,
            ledger: &mut fixture.ledger,
        };
        planner
            .plan(
                &mut context,
                DefinitionKind::Clip,
                named_element(
                    &fixture.source,
                    fixture.document.tokens(),
                    b"clipPath",
                    b"c",
                ),
                fixture.owner,
                identity,
                bounds,
                &owner_style,
                &mut control,
            )
            .unwrap()
    };
    let DefinitionDescriptor::Clip(clip) = plan.descriptor else {
        panic!("clip descriptor")
    };
    assert!(clip.object_bounding_box);
    assert_eq!(plan.expansions.len(), 1);
    assert_eq!(
        plan.expansions[0].flavor,
        DecodeFlavor::Clip { clip_rule: true }
    );
    assert_eq!(
        span(
            &fixture.source,
            fixture.document.tokens()[plan.expansions[0].range.first].local_name
        ),
        Some(&b"path"[..])
    );

    let plan = {
        let mut context = DefinitionContext {
            shared: &mut shared,
            builder: &mut fixture.builder,
            scene: fixture.scene,
            active: &mut active,
            ledger: &mut fixture.ledger,
        };
        planner
            .plan(
                &mut context,
                DefinitionKind::Mask,
                named_element(&fixture.source, fixture.document.tokens(), b"mask", b"m"),
                fixture.owner,
                identity,
                bounds,
                &owner_style,
                &mut control,
            )
            .unwrap()
    };
    let DefinitionDescriptor::Mask(mask) = plan.descriptor else {
        panic!("mask descriptor")
    };
    assert!(!mask.luminance);
    assert!(!mask.region_object_bounding_box);
    assert_eq!(mask.region, [2.0, 4.0, 10.0, 12.0]);
    assert_eq!(plan.expansions[0].flavor, DecodeFlavor::Mask);
    assert_eq!(plan.expansions[0].transform.a, 10.0);
    assert_eq!(plan.expansions[0].transform.d, 10.0);

    let plan = {
        let mut context = DefinitionContext {
            shared: &mut shared,
            builder: &mut fixture.builder,
            scene: fixture.scene,
            active: &mut active,
            ledger: &mut fixture.ledger,
        };
        planner
            .plan(
                &mut context,
                DefinitionKind::Pattern,
                named_element(&fixture.source, fixture.document.tokens(), b"pattern", b"p"),
                fixture.owner,
                identity,
                bounds,
                &owner_style,
                &mut control,
            )
            .unwrap()
    };
    let DefinitionDescriptor::Pattern(pattern) = plan.descriptor else {
        panic!("pattern descriptor")
    };
    assert_eq!([pattern.width, pattern.height], [20, 10]);
    assert_eq!([pattern.tile_width, pattern.tile_height], [5.0, 2.5]);
    let pattern_element =
        named_element(&fixture.source, fixture.document.tokens(), b"pattern", b"p");
    let mapping = parse_pattern_mapping(
        &fixture.source,
        fixture.document.tokens(),
        pattern_element.0,
        bounds,
        fixture.builder.scene(fixture.scene).unwrap().viewport,
        &mut control,
    )
    .unwrap();
    assert_eq!(mapping.tile_origin, [3.0, 5.0]);
    assert_eq!([mapping.tile_width, mapping.tile_height], [5.0, 2.5]);
    assert_eq!(
        [mapping.user_to_base.e, mapping.user_to_base.f],
        [-4.0, -7.0]
    );
    assert_eq!(plan.expansions[0].flavor, DecodeFlavor::Pattern);
    let plan = {
        let mut context = DefinitionContext {
            shared: &mut shared,
            builder: &mut fixture.builder,
            scene: fixture.scene,
            active: &mut active,
            ledger: &mut fixture.ledger,
        };
        planner
            .plan(
                &mut context,
                DefinitionKind::Marker,
                named_element(&fixture.source, fixture.document.tokens(), b"marker", b"mk"),
                fixture.owner,
                identity,
                bounds,
                &owner_style,
                &mut control,
            )
            .unwrap()
    };
    let DefinitionDescriptor::Marker(marker) = plan.descriptor else {
        panic!("marker descriptor")
    };
    assert_eq!(marker.viewport, [2.0, 1.0]);
    assert_eq!(plan.expansions[0].flavor, DecodeFlavor::Marker);
    assert!(plan.expansions[0].style.value(Property::Fill).is_some());
    assert_eq!(
        plan.expansions[0]
            .style
            .value(Property::Fill)
            .unwrap()
            .bytes,
        b"#ff0000"
    );
    for p in [Property::Marker, Property::MarkerStart, Property::MarkerEnd] {
        assert!(plan.expansions[0].style.value(p).is_none());
    }
    assert_eq!(
        plan.expansions[0]
            .style
            .value(Property::MarkerMid)
            .unwrap()
            .bytes,
        b"url(#nested)"
    );

    let plan = {
        let mut context = DefinitionContext {
            shared: &mut shared,
            builder: &mut fixture.builder,
            scene: fixture.scene,
            active: &mut active,
            ledger: &mut fixture.ledger,
        };
        planner
            .plan(
                &mut context,
                DefinitionKind::Filter,
                named_element(&fixture.source, fixture.document.tokens(), b"filter", b"f"),
                fixture.owner,
                identity,
                bounds,
                &owner_style,
                &mut control,
            )
            .unwrap()
    };
    let DefinitionDescriptor::Filter { filter, primitives } = plan.descriptor else {
        panic!("filter descriptor")
    };
    assert_eq!(filter.region, [0.1, 0.05, 0.5, 0.6]);
    assert_eq!(primitives.len(), 1);
    assert_eq!(primitives[0].kind, FilterKind::GaussianBlur);
    assert!(plan.expansions.is_empty());
}

#[test]
fn real_scene_clip_query_mutation_and_deduplicated_overlay() {
    let mut fixture =
        fixture(b"<svg width='20' height='20'><path id='owner' d='M0 0L10 0L10 10Z'/></svg>");
    let cancel = AtomicBool::new(false);
    let mut control = BudgetControl::new(&cancel, u64::MAX);
    let token = fixture.builder.begin_draft(fixture.scene).unwrap();
    let owner_transform = fixture
        .builder
        .append_transform(
            fixture.scene,
            TransformFrame {
                source_offset: 0,
                parent: None,
                attribute: affine_identity(),
                suffix: affine_identity(),
                motion: affine_identity(),
            },
            &mut control,
        )
        .unwrap();
    let tokens = fixture.document.tokens();
    let d = attribute(
        &fixture.source,
        tokens,
        &tokens[fixture.owner_element.0],
        b"d",
    )
    .unwrap();
    let path = vexel_svg_path::parse_path(
        &fixture.source,
        Span {
            start: d.offset,
            end: d.offset + d.length,
        },
        affine_identity(),
        vexel_svg_path::ParseOptions::default(),
    )
    .unwrap();
    let mut new_shape = Shape::new(SourceMap {
        element: Span {
            start: tokens[fixture.owner_element.0].source.offset,
            end: tokens[fixture.owner_element.0].source.offset
                + tokens[fixture.owner_element.0].source.length,
        },
        transform: affine_identity(),
        transform_node: None,
    });
    new_shape.edges.count = path.edges.len();
    new_shape.subpaths.count = path.subpaths.len();
    let draft_owner = fixture
        .builder
        .append_shape(
            fixture.scene,
            ShapeInput {
                edge_colors: vec![[0, 0, 0, 255]; path.edges.len()],
                shape: new_shape,
                path,
            },
            &mut control,
        )
        .unwrap();
    let points = [
        Point { x: 0.0, y: 0.0 },
        Point { x: 10.0, y: 0.0 },
        Point { x: 10.0, y: 10.0 },
        Point { x: 0.0, y: 10.0 },
    ];
    let clip = create_rect(
        points,
        affine_identity(),
        Some(owner_transform),
        &mut fixture.builder,
        fixture.scene,
        &mut control,
    )
    .unwrap();
    apply_group(
        &mut fixture.builder,
        &token,
        fixture.scene,
        &[draft_owner],
        clip,
        &mut control,
    )
    .unwrap();
    let view = fixture.builder.scene(fixture.scene).unwrap();
    let attached = fixture
        .builder
        .scene(fixture.scene)
        .unwrap()
        .shape(draft_owner)
        .unwrap()
        .clip
        .unwrap();
    assert!(contains(view, attached, 5.0, 5.0));
    assert!(!contains(view, attached, 15.0, 5.0));
    assert_eq!(work_counts(view, attached).unwrap().edges, 4);
    assert_eq!(work_units(view, attached).unwrap(), 6);
    assert_eq!(
        local_to_document(view, attached, Point { x: 0.5, y: 0.5 }),
        Some(Point { x: 0.5, y: 0.5 })
    );
    let mut visited = 0usize;
    assert_eq!(
        visit_boundary_edges(view, attached, |_| {
            visited += 1;
            true
        })
        .unwrap(),
        (true, true)
    );
    assert_eq!(visited, 4);
    let _ = view;
    let inner = create_rect(
        [
            Point { x: 2.0, y: 2.0 },
            Point { x: 8.0, y: 2.0 },
            Point { x: 8.0, y: 8.0 },
            Point { x: 2.0, y: 8.0 },
        ],
        affine_identity(),
        Some(owner_transform),
        &mut fixture.builder,
        fixture.scene,
        &mut control,
    )
    .unwrap();
    apply_group(
        &mut fixture.builder,
        &token,
        fixture.scene,
        &[draft_owner],
        inner,
        &mut control,
    )
    .unwrap();
    let view = fixture.builder.scene(fixture.scene).unwrap();
    let attached = view.shape(draft_owner).unwrap().clip.unwrap();
    apply_group(
        &mut fixture.builder,
        &token,
        fixture.scene,
        &[draft_owner],
        inner,
        &mut control,
    )
    .unwrap();
    assert_eq!(
        fixture
            .builder
            .scene(fixture.scene)
            .unwrap()
            .shape(draft_owner)
            .unwrap()
            .clip,
        Some(attached)
    );
    let view = fixture.builder.scene(fixture.scene).unwrap();
    assert_eq!(work_counts(view, attached).unwrap().edges, 8);
    assert_eq!(work_units(view, attached).unwrap(), 12);
    assert!(contains(view, attached, 5.0, 5.0));
    assert!(!contains(view, attached, 1.0, 1.0));
    let mut mapped = Vec::new();
    assert_eq!(
        visit_boundary_edges(view, attached, |edge| {
            mapped.push(edge);
            true
        })
        .unwrap(),
        (true, true)
    );
    assert_eq!(mapped[0].a, Point { x: 0.0, y: 0.0 });
    assert_eq!(mapped[4].a, Point { x: 2.0, y: 2.0 });
    let mut overlay = clone_clips(view, &[attached, attached], &mut control).unwrap();
    assert_eq!(overlay.len(), 2);
    let attribute_inverse = Affine {
        e: 3.0,
        ..affine_identity()
    };
    update_owner(
        &mut overlay,
        attached,
        owner_transform,
        affine_identity(),
        attribute_inverse,
        [1.0, 2.0, 11.0, 12.0],
    )
    .unwrap();
    assert_eq!(
        overlay.get(attached).unwrap().user_inverse,
        attribute_inverse
    );
    let _ = view;
    fixture.builder.commit_draft(&token, &mut control).unwrap();
}

#[test]
fn original_marker_shorthand_and_viewport_units_are_preserved() {
    let xml = br#"<svg width="20" height="20"><defs><marker id="m" markerWidth="50%" markerHeight="25%" markerUnits="userSpaceOnUse" orient="auto-start-reverse" refX="1" refY=".5" viewBox="0 0 2 1"><path d="M0 0L2 0"/></marker></defs><path id="owner" marker="url(#m)" d="M0 0L10 0L10 10Z"/></svg>"#;
    let fixture = fixture(xml);
    let cancel = AtomicBool::new(false);
    let mut control = BudgetControl::new(&cancel, u64::MAX);
    let mut shared = DecodeSource::new(&fixture.document, &mut control).unwrap();
    let style = shared
        .compute_style(fixture.owner_element, None, &mut control)
        .unwrap();
    assert_eq!(style.value(Property::MarkerStart), None);
    assert_eq!(style.value(Property::MarkerMid), None);
    assert_eq!(style.value(Property::MarkerEnd), None);
    assert_eq!(style.value(Property::Marker).unwrap().bytes, b"url(#m)");
    for position in [
        MarkerPosition::Start,
        MarkerPosition::Middle,
        MarkerPosition::End,
    ] {
        let value = marker_reference_value(&style, position).unwrap();
        assert_eq!(value.bytes, b"url(#m)");
        assert_eq!(
            resolve_local_definition(
                &mut shared,
                value.span,
                DefinitionKind::Marker,
                &mut control,
            )
            .unwrap(),
            named_element(&fixture.source, fixture.document.tokens(), b"marker", b"m")
        );
    }
    let marker = named_element(&fixture.source, fixture.document.tokens(), b"marker", b"m");
    let attributes = parse_marker_attributes_for_viewport(
        &fixture.source,
        fixture.document.tokens(),
        marker.0,
        [20.0, 20.0],
    )
    .unwrap();
    assert_eq!([attributes.width, attributes.height], [10.0, 5.0]);
    assert_eq!([attributes.ref_x, attributes.ref_y], [5.0, 2.5]);
    assert_eq!(attributes.child_viewport, [2.0, 1.0]);
    assert_eq!(attributes.orient, MarkerOrient::AutoStartReverse);
    assert!(!attributes.stroke_width_units);
}
