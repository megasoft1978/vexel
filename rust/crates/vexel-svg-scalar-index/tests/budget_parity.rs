mod support;
use std::sync::Arc;
use support::*;
use vexel_scene::*;
use vexel_svg_scalar_index::*;
fn budget_fixture(teeth: usize) -> SceneGraph {
    // Entire original raster_budget_test.c make_source input, including the
    // gradient and the original three-decimal coordinate rounding.
    let mut text = String::with_capacity(512 + teeth * 24);
    text.push_str("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"1056\" height=\"1000\" viewBox=\"0 0 1056 1000\"><defs><linearGradient id=\"g\"><stop offset=\"0\" stop-color=\"#f00\"/><stop offset=\"1\" stop-color=\"#00f\"/></linearGradient></defs><path fill=\"url(#g)\" d=\"M0 0");
    for tooth in 0..teeth {
        text.push_str(&format!(
            " L{:.3} {}",
            (tooth + 1) as f64 * 1056. / teeth as f64,
            if tooth % 2 == 0 { 1000 } else { 0 }
        ));
    }
    text.push_str(" Z\"/></svg>");
    let source = Arc::new(text.into_bytes());
    let lexical = LexicalDocument::parse(source.clone(), 1 << 20, &mut Run).unwrap();
    let animation = vexel_svg_animation::parse_owned(&source, lexical.tokens()).unwrap();
    let viewport = Viewport {
        width: 1056,
        height: 1000,
        view_x: 0.,
        view_y: 0.,
        view_width: 1056.,
        view_height: 1000.,
        aspect: Default::default(),
    };
    let mut builder =
        SceneBuilder::new(source.clone(), animation, 1, viewport, Limits::default()).unwrap();
    let root = builder.root_id();
    let draft = builder.begin_draft(root).unwrap();
    let stops = builder
        .append_stops(
            root,
            &[
                PaintStop {
                    offset: 0.,
                    rgba: [255, 0, 0, 255],
                },
                PaintStop {
                    offset: 1.,
                    rgba: [0, 0, 255, 255],
                },
            ],
            &mut Run,
        )
        .unwrap();
    let identity = vexel_geometry::affine_identity();
    let paint = Paint {
        kind: PaintKind::Linear,
        spread: Spread::Pad,
        object_bounding_box: true,
        coordinates: [0., 0., 1., 0., 0., 0.],
        bounds: [0., 0., 1056., 1000.],
        user_inverse: identity,
        gradient_inverse: identity,
        stops,
        pattern: None,
        pattern_inverse: identity,
        pattern_opacity: 1.,
    };
    let fill = builder.append_fill_paint(root, paint, &mut Run).unwrap();
    let start = source.windows(2).position(|w| w == b"M0").unwrap();
    let end = source[start..].iter().position(|&c| c == b'"').unwrap() + start;
    let span = Span { start, end };
    let path = vexel_svg_path::parse_path(&source, span, identity, Default::default()).unwrap();
    // Original gradient paint keeps uniform fallback colors; geometry intervals remain eligible.
    let colors = (0..path.edges.len()).map(|_| [0, 0, 0, 255]).collect();
    let mut shape = Shape::new(SourceMap {
        element: span,
        transform: identity,
        transform_node: None,
    });
    shape.fill_paint = Some(fill);
    builder
        .append_shape(
            root,
            ShapeInput {
                shape,
                path,
                edge_colors: colors,
            },
            &mut Run,
        )
        .unwrap();
    builder.commit_draft(&draft, &mut Run).unwrap();
    builder.freeze().unwrap()
}
#[test]
fn original_raster_budget_inside_1000_and_over_1100_exact_work_and_error() {
    for teeth in [1000, 1100] {
        let graph = budget_fixture(teeth);
        let view = graph.root();
        let mut job = job(&graph, 64 << 20);
        let resources = Resources::default();
        let mut index = ScalarIndex::build(view, map(), &mut job, &resources, &mut Run).unwrap();
        setup_cost(&index, &mut job, 1);
        assert!(eligible_job(&job, &index.scratch, view.edge_shape(0).unwrap()).unwrap());
        let before = job.stats();
        let mut stats = RenderWorkStats::default();
        let result = preflight_job(
            view,
            &mut index,
            map(),
            &mut job,
            &mut stats,
            &resources,
            &mut Run,
        );
        if teeth == 1000 {
            result.unwrap();
            assert_eq!(stats.total_work, 365_760_771);
            assert!(stats.total_work > 200_000_000);
            assert!(stats.total_work <= work_budget(1056, 1000));
        } else {
            let error = result.unwrap_err();
            assert_eq!(error.kind, ErrorKind::Limit(LimitKind::Work));
            assert_eq!(error.offset, 0);
            assert_eq!(stats.total_work, 0);
        }
        assert_eq!(job.stats(), before);
    }
}
