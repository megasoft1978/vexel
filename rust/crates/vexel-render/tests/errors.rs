//! AC-4: typed errors for deferred features and invalid input; no panics.
mod common;
use common::*;
use std::cell::Cell;
use vexel_geometry::{Affine, affine_identity};
use vexel_raster::Tile;
use vexel_render::{
    FlatEdge, Linecap, Linejoin, RenderError, RenderShape, SceneFeatures,
    UnsupportedKind, ViewBox, ViewMap, from_scene_shape, from_scene_shape_with_gradients,
    render_shapes,
};
use vexel_scene::{
    Affine as SceneAffine, Paint, PaintKind, PaintStop, Shape, SourceMap, Span, Spread, TableRange,
};

const RED: [u8; 4] = [255, 0, 0, 255];

fn unsupported(kind: UnsupportedKind) -> RenderError {
    RenderError::Unsupported(kind)
}

fn scene_shape() -> Shape {
    Shape::new(SourceMap {
        element: Span { start: 0, end: 0 },
        transform: affine_identity(),
        transform_node: None,
    })
}

fn paint(kind: PaintKind) -> Paint {
    Paint {
        kind,
        spread: Spread::Pad,
        object_bounding_box: false,
        coordinates: [0.0; 6],
        bounds: [0.0; 4],
        user_inverse: affine_identity(),
        gradient_inverse: affine_identity(),
        stops: TableRange::EMPTY,
        pattern: None,
        pattern_inverse: affine_identity(),
        pattern_opacity: 1.0,
    }
}

fn convert(shape: &Shape, fill: &[Paint], stroke: &[Paint]) -> Result<(), RenderError> {
    let edges: Vec<FlatEdge> = vec![edge(0.0, 0.0, 1.0, 0.0); shape.edges.count];
    let subpaths = vec![0..0; shape.subpaths.count];
    let closed = vec![false; shape.subpaths.count];
    from_scene_shape(shape, RED, fill, stroke, &edges, &subpaths, &closed).map(|_| ())
}

#[test]
fn adapter_accepts_a_solid_shape_and_inverts_the_stroke_transform() {
    let mut shape = scene_shape();
    shape.stroke_inverse = SceneAffine { a: 0.5, b: 0.0, c: 0.0, d: 0.5, e: -1.0, f: -2.0 };
    let render = from_scene_shape(&shape, RED, &[], &[], &[], &[], &[]).unwrap();
    assert_eq!(render.transform, SceneAffine { a: 2.0, b: 0.0, c: 0.0, d: 2.0, e: 2.0, f: 4.0 });
    shape.fill_paint = Some(0);
    assert_eq!(convert(&shape, &[paint(PaintKind::Solid)], &[]), Ok(()));
}

#[test]
fn adapter_reports_each_unsupported_paint_kind() {
    for (kind, expected) in [
        (PaintKind::Linear, UnsupportedKind::GradientPaint),
        (PaintKind::Radial, UnsupportedKind::GradientPaint),
        (PaintKind::Pattern, UnsupportedKind::PatternPaint),
        (PaintKind::Image, UnsupportedKind::ImagePaint),
    ] {
        let mut fill = scene_shape();
        fill.fill_paint = Some(0);
        assert_eq!(convert(&fill, &[paint(kind)], &[]), Err(unsupported(expected)), "fill {kind:?}");
        let mut stroke = scene_shape();
        stroke.stroke_paint = Some(0);
        assert_eq!(convert(&stroke, &[], &[paint(kind)]), Err(unsupported(expected)), "stroke {kind:?}");
    }
    let mut dangling = scene_shape();
    dangling.fill_paint = Some(3);
    assert_eq!(convert(&dangling, &[], &[]), Err(RenderError::InvalidGeometry));
}

fn gradient_convert<'a>(
    shape: &Shape,
    fill: &'a [Paint],
    stroke: &'a [Paint],
    stops: &'a [PaintStop],
) -> Result<RenderShape<'a>, RenderError> {
    from_scene_shape_with_gradients(shape, RED, fill, stroke, stops, &[], &[], &[])
}

#[test]
fn linear_and_radial_paints_convert_to_render_shapes() {
    let stops = [
        PaintStop {
            offset: 0.0,
            rgba: RED,
        },
        PaintStop {
            offset: 1.0,
            rgba: [0, 0, 255, 255],
        },
    ];
    for kind in [PaintKind::Linear, PaintKind::Radial] {
        let mut gradient = paint(kind);
        gradient.stops = TableRange { first: 0, count: 2 };
        let paints = [gradient];
        let mut fill = scene_shape();
        fill.fill_paint = Some(0);
        let render = gradient_convert(&fill, &paints, &[], &stops).unwrap();
        assert_eq!(render.fill_gradient.map(|g| g.paint.kind), Some(kind));
        assert!(render.stroke_gradient.is_none());
        let mut stroke = scene_shape();
        stroke.stroke_paint = Some(0);
        let render = gradient_convert(&stroke, &[], &paints, &stops).unwrap();
        assert_eq!(render.stroke_gradient.map(|g| g.paint.kind), Some(kind));
        assert!(render.fill_gradient.is_none());
    }
    // A stop span outside the scene stop table is invalid input.
    let mut dangling = scene_shape();
    dangling.fill_paint = Some(0);
    let mut bad = paint(PaintKind::Linear);
    bad.stops = TableRange { first: 1, count: 2 };
    assert_eq!(
        gradient_convert(&dangling, &[bad], &[], &stops).map(|_| ()),
        Err(RenderError::InvalidGeometry)
    );
    let mut gradient = paint(PaintKind::Linear);
    gradient.stops = TableRange { first: 0, count: 2 };
    let paints = [gradient];
    // Solid-only entry points still refuse gradients.
    let mut solid_only = scene_shape();
    solid_only.fill_paint = Some(0);
    assert_eq!(
        from_scene_shape(&solid_only, RED, &paints, &[], &[], &[], &[]).map(|_| ()),
        Err(unsupported(UnsupportedKind::GradientPaint))
    );
}

#[test]
fn adapter_reports_dash_image_paint_order_and_paint_server() {
    let mut dash = scene_shape();
    dash.stroke.dash_values = TableRange { first: 0, count: 2 };
    assert_eq!(convert(&dash, &[], &[]), Err(unsupported(UnsupportedKind::Dash)));
    let mut image = scene_shape();
    image.image = Some(0);
    assert_eq!(convert(&image, &[], &[]), Err(unsupported(UnsupportedKind::ImagePaint)));
    let mut order = scene_shape();
    order.stroke.paint_order = 1;
    assert_eq!(convert(&order, &[], &[]), Err(unsupported(UnsupportedKind::PaintOrder)));
    let mut server = scene_shape();
    server.stroke.paint_server = true;
    assert_eq!(convert(&server, &[], &[]), Err(unsupported(UnsupportedKind::GradientPaint)));
    let mut mismatched = scene_shape();
    mismatched.edges = TableRange { first: 0, count: 3 };
    let result = from_scene_shape(&mismatched, RED, &[], &[], &[], &[], &[]);
    assert_eq!(result.map(|_| ()), Err(RenderError::InvalidGeometry));
}

#[test]
fn scene_level_features_each_have_their_kind() {
    let cases = [
        (SceneFeatures { clips: 1, ..Default::default() }, UnsupportedKind::Clip),
        (SceneFeatures { opacity_groups: 1, ..Default::default() }, UnsupportedKind::OpacityGroup),
        (SceneFeatures { masks: 1, ..Default::default() }, UnsupportedKind::Mask),
        (SceneFeatures { filters: 2, ..Default::default() }, UnsupportedKind::Filter),
        (SceneFeatures { text_runs: 1, ..Default::default() }, UnsupportedKind::Text),
        (SceneFeatures { animation_overlay: true, ..Default::default() }, UnsupportedKind::AnimationOverlay),
        (SceneFeatures { smil: true, ..Default::default() }, UnsupportedKind::Smil),
    ];
    for (features, kind) in cases {
        assert_eq!(features.require_supported(), Err(unsupported(kind)));
    }
    assert_eq!(SceneFeatures::default().require_supported(), Ok(()));
}

#[test]
fn crisp_edges_is_unsupported_at_render_time() {
    let edges = polyline(&[(1.0, 1.0), (8.0, 1.0)]);
    let mut st = style(RED, 2.0, Linecap::Butt, Linejoin::Miter, 4.0);
    st.crisp_edges = true;
    assert_eq!(
        try_render(&[stroke_shape(&edges, false, st)], 8).map(|_| ()),
        Err(unsupported(UnsupportedKind::CrispEdges))
    );
}

#[test]
fn non_empty_tile_is_rejected_without_touching_it() {
    let edges = polygon(&[(0.0, 0.0), (4.0, 0.0), (4.0, 4.0), (0.0, 4.0)]);
    let mut tile = empty_tile(4, 4);
    tile.rgba_mut()[5] = 1;
    let before = tile.clone();
    let result = render_shapes(&[fill_shape(&edges, RED, false)], unit_view(4), &mut tile, NEVER);
    assert_eq!(result, Err(RenderError::TileNotEmpty));
    assert_eq!(tile, before);
    // The opaque test-pattern tile of vexel-raster is not empty either.
    let mut pattern = Tile::new(4, 4).unwrap();
    assert_eq!(render_shapes(&[], unit_view(4), &mut pattern, NEVER), Err(RenderError::TileNotEmpty));
}

fn expect_invalid(shape: RenderShape<'_>) {
    assert_eq!(try_render(&[shape], 8).map(|_| ()), Err(RenderError::InvalidGeometry));
}

#[test]
fn non_finite_and_overflowing_geometry_is_invalid() {
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 1e300, -2.0e6] {
        let edges = vec![edge(0.0, 0.0, bad, 4.0), edge(bad, 4.0, 0.0, 0.0)];
        expect_invalid(fill_shape(&edges, RED, false));
    }
    let edges = polyline(&[(1.0, 1.0), (6.0, 1.0)]);
    let st = style(RED, 2.0, Linecap::Butt, Linejoin::Miter, 4.0);
    for (width, miter, offset) in [(f64::NAN, 4.0, 0.0), (-1.0, 4.0, 0.0), (2.0, 0.5, 0.0), (2.0, f64::NAN, 0.0), (2.0, 4.0, f64::INFINITY)] {
        let mut s = st;
        s.width = width;
        s.miter_limit = miter;
        s.dash_offset = offset;
        expect_invalid(stroke_shape(&edges, false, s));
    }
    // Singular and overflowing element transforms cannot place a stroke.
    for transform in [
        Affine { a: 0.0, b: 0.0, c: 0.0, d: 0.0, e: 0.0, f: 0.0 },
        Affine { a: 1e300, b: 0.0, c: 0.0, d: 1.0, e: 0.0, f: 0.0 },
        Affine { a: f64::NAN, b: 0.0, c: 0.0, d: 1.0, e: 0.0, f: 0.0 },
    ] {
        let mut shape = stroke_shape(&edges, false, st);
        shape.transform = transform;
        expect_invalid(shape);
    }
    // Subpath ranges must fit the edge list and match the closed flags.
    let ranges = [0..9];
    let closed = [false];
    expect_invalid(with_subpaths(stroke_shape(&edges, false, st), &ranges, &closed));
    expect_invalid(with_subpaths(stroke_shape(&edges, false, st), &ranges, &[]));
}

#[test]
fn view_and_tile_size_mismatch_is_invalid() {
    let vb = ViewBox { x: 0.0, y: 0.0, width: 4.0, height: 4.0 };
    assert_eq!(ViewMap::from_viewbox(vb, 0, 4, None), Err(RenderError::InvalidGeometry));
    for bad in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        let broken = ViewBox { width: bad, ..vb };
        assert_eq!(ViewMap::from_viewbox(broken, 4, 4, None), Err(RenderError::InvalidGeometry));
    }
    let view = ViewMap::from_viewbox(vb, 4, 4, None).unwrap();
    let mut tile = empty_tile(5, 4);
    assert_eq!(render_shapes(&[], view, &mut tile, NEVER), Err(RenderError::InvalidGeometry));
}

#[test]
fn over_limit_input_returns_limit_exceeded() {
    // More edges than the C cap (SVG_SHAPE_MAX_EDGES = 4 Mi).
    let many = vec![edge(0.0, 0.0, 0.0, 0.0); vexel_render::MAX_EDGES + 1];
    assert_eq!(
        try_render(&[fill_shape(&many, RED, false)], 4).map(|_| ()),
        Err(RenderError::LimitExceeded)
    );
    // Stroke work beyond the dash-cache step budget. WO-RUST-LIMITS-03 moved this pin
    // from 600,000 edges (1.2M steps, refused at the old 1,000,000 cap) to 2,000,001
    // edges (4,000,002 steps, refused at the then-new 4,000,000 cap). The step cap is now 6M
    // and the 192 MiB byte budget binds solid rings at 2,287,801 edges, so the pin is 2,287,802. The exact boundary is
    // pinned in validate.rs (limit03_tests).
    let ring = vec![edge(1.0, 1.0, 2.0, 1.0); 2_287_802];
    let st = style(RED, 1.0, Linecap::Butt, Linejoin::Miter, 4.0);
    assert_eq!(
        try_render(&[stroke_shape(&ring, true, st)], 4).map(|_| ()),
        Err(RenderError::LimitExceeded)
    );
    // Tile axes are bounded by vexel-raster (4096).
    assert_eq!(vexel_render::offscreen_tile_size(4097.0, 4.0), Err(RenderError::InvalidGeometry));
}

#[test]
fn cancellation_stops_at_row_granularity() {
    let edges = polygon(&[(0.0, 0.0), (16.0, 0.0), (16.0, 16.0), (0.0, 16.0)]);
    let shape = fill_shape(&edges, RED, false);
    let mut tile = empty_tile(16, 16);
    assert_eq!(render_shapes(&[shape], unit_view(16), &mut tile, &|| true), Err(RenderError::Cancelled));
    // Cancel after the shape check and five rows.
    let polls = Cell::new(0);
    let mut tile = empty_tile(16, 16);
    let control = || {
        polls.set(polls.get() + 1);
        polls.get() > 6
    };
    assert_eq!(render_shapes(&[shape], unit_view(16), &mut tile, &control), Err(RenderError::Cancelled));
    assert_eq!(polls.get(), 7, "one poll per shape plus one per row, stopping at the first true");
    let first_rows_painted = tile.rgba()[..16 * 4 * 5].iter().any(|b| *b != 0);
    let later_rows_empty = tile.rgba()[16 * 4 * 5..].iter().all(|b| *b == 0);
    assert!(first_rows_painted && later_rows_empty, "rows before the poll were painted, later rows were not");
}

#[test]
fn empty_inputs_are_valid_and_leave_the_tile_empty() {
    let tile = try_render(&[], 8).unwrap();
    assert!(tile.rgba().iter().all(|b| *b == 0));
    let no_edges: [FlatEdge; 0] = [];
    let tile = try_render(&[fill_shape(&no_edges, RED, false)], 8).unwrap();
    assert!(tile.rgba().iter().all(|b| *b == 0));
}
