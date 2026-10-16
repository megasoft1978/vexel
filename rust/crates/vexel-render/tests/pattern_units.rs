//! Unit tests of pattern paint: tile alignment, transforms, limits,
//! cancellation, typed errors and byte-identity of solid rendering.
mod common;
use common::parity_data::{CASES, Case, EXTRA};
use common::*;
use std::cell::Cell;
use vexel_geometry::{Affine, affine_compose, affine_identity};
use vexel_raster::Tile;
use vexel_render::{
    FlatEdge, Linecap, Linejoin, PATTERN_MAX_INSTANCES, PatternPaint, PatternPaintShape,
    PatternTileSpec, PreserveAspect, RenderError, RenderShape, StrokeStyle, UnsupportedKind,
    ViewBox, ViewMap, from_scene_shape_with_pattern, pattern_pixel_size, render_pattern_shapes,
    render_pattern_tile, render_pattern_tiles, render_shapes,
};
use vexel_scene::{
    Affine as SceneAffine, Paint, PaintKind, PatternDescriptor, PatternMapping, Shape, SourceMap,
    Span, Spread, TableRange,
};

const RED: [u8; 4] = [255, 0, 0, 255];

fn spec(width: u32, height: u32, tile_width: f64, tile_height: f64) -> PatternTileSpec {
    PatternTileSpec {
        width,
        height,
        tile_width,
        tile_height,
    }
}

/// Tile whose left `red_width` user units are opaque red.
fn stripe_tile(width: u32, tile_width: f64, red_width: f64) -> Tile {
    let edges = polygon(&[
        (0.0, 0.0),
        (red_width, 0.0),
        (red_width, tile_width),
        (0.0, tile_width),
    ]);
    let child = PatternPaintShape::from(fill_shape(&edges, RED, false));
    render_pattern_tile(spec(width, width, tile_width, tile_width), &[child], NEVER)
        .unwrap()
        .unwrap()
}

fn scale(s: f64) -> Affine {
    Affine {
        a: s,
        b: 0.0,
        c: 0.0,
        d: s,
        e: 0.0,
        f: 0.0,
    }
}

/// Fills the whole `size` square view with the pattern.
fn paint(tile: &Tile, inverse: Affine, opacity: f64, size: u32) -> Result<Tile, RenderError> {
    let s = f64::from(size);
    let edges = polygon(&[(0.0, 0.0), (s, 0.0), (s, s), (0.0, s)]);
    let shape = PatternPaintShape {
        shape: fill_shape(&edges, [0, 0, 255, 255], false),
        fill: Some(PatternPaint {
            tile: Some(tile),
            inverse,
            opacity,
        }),
        stroke: None,
    };
    let mut out = empty_tile(size, size);
    render_pattern_shapes(&[shape], unit_view(size), &mut out, NEVER)?;
    Ok(out)
}

#[test]
fn integer_tile_boundaries_align_with_pixels() {
    let tile = stripe_tile(4, 4.0, 2.0);
    let out = paint(&tile, affine_identity(), 1.0, 16).unwrap();
    for y in 4..12 {
        for x in 4..12 {
            let expected = if x % 4 < 2 { RED } else { [0; 4] };
            assert_eq!(pixel(&out, x, y), expected, "pixel {x},{y}");
        }
    }
}

#[test]
fn fractional_tile_size_wraps_with_bilinear_blend() {
    // 2.5 unit tile at 4 px per unit: a 10 px tile, red for texels 0..5.
    let tile = stripe_tile(10, 2.5, 1.25);
    let out = paint(&tile, scale(4.0), 1.0, 24).unwrap();
    assert_eq!(pixel(&out, 5, 8), RED);
    assert_eq!(pixel(&out, 6, 8), [0; 4]);
    // Sample at texel 9.5 blends the transparent texel 9 with wrapped red texel 0.
    assert_eq!(pixel(&out, 7, 8), [255, 0, 0, 128]);
    assert_eq!(pixel(&out, 8, 8), RED);
    assert_eq!(pixel(&out, 10, 8), RED);
}

#[test]
fn scale_and_axis_swap_transforms_move_the_pattern() {
    let tile = stripe_tile(4, 4.0, 2.0);
    // Two tile pixels per document pixel: period two pixels.
    let out = paint(&tile, scale(2.0), 1.0, 16).unwrap();
    for x in 4..12 {
        let expected = if x % 2 == 0 { RED } else { [0; 4] };
        assert_eq!(pixel(&out, x, 8), expected, "scale x={x}");
    }
    // patternTransform rotate(90): inverse (x, y) -> (y, -x); the stripes turn horizontal.
    let rotate_inverse = Affine {
        a: 0.0,
        b: -1.0,
        c: 1.0,
        d: 0.0,
        e: 0.0,
        f: 0.0,
    };
    let out = paint(&tile, rotate_inverse, 1.0, 16).unwrap();
    for y in 4..12 {
        let expected = if y % 4 < 2 { RED } else { [0; 4] };
        for x in 4..12 {
            assert_eq!(pixel(&out, x, y), expected, "rotate {x},{y}");
        }
    }
}

fn mapping(origin: [f64; 2], pattern_transform: Affine, bounds: [f64; 4]) -> PatternMapping {
    let translate = Affine {
        a: 1.0,
        b: 0.0,
        c: 0.0,
        d: 1.0,
        e: -origin[0],
        f: -origin[1],
    };
    let inverse_pt = Affine {
        a: pattern_transform.d,
        b: -pattern_transform.b,
        c: -pattern_transform.c,
        d: pattern_transform.a,
        e: 0.0,
        f: 0.0,
    };
    PatternMapping {
        tile_origin: origin,
        user_to_base: affine_compose(translate, inverse_pt).unwrap(),
        pattern_transform,
        user_inverse: affine_identity(),
        viewport: [16.0, 16.0],
        bounds,
        source_map: SourceMap {
            element: Span { start: 0, end: 0 },
            transform: affine_identity(),
            transform_node: None,
        },
        opacity: 1.0,
    }
}

fn descriptor(width: u32, tile_width: f64) -> PatternDescriptor {
    PatternDescriptor {
        definition: vexel_scene::ElementId(0),
        width,
        height: width,
        tile_width,
        tile_height: tile_width,
        content_transform: affine_identity(),
        error_offset: 0,
    }
}

#[test]
fn object_bounding_box_and_user_space_units_give_the_same_pixels() {
    let tile = stripe_tile(4, 4.0, 2.0);
    // userSpaceOnUse x=3: tile origin 3. objectBoundingBox x=.1875 of a 16 wide
    // box at x=0 resolves to the same origin, so the decoder hands one mapping.
    let user = mapping([3.0, 0.0], affine_identity(), [0.0, 0.0, 16.0, 16.0]);
    let bbox_origin = 0.0 + 0.1875 * 16.0;
    let bbox = mapping(
        [bbox_origin, 0.0],
        affine_identity(),
        [0.0, 0.0, 16.0, 16.0],
    );
    let a = paint(
        &tile,
        user.pixel_inverse(descriptor(4, 4.0)).unwrap(),
        1.0,
        16,
    )
    .unwrap();
    let b = paint(
        &tile,
        bbox.pixel_inverse(descriptor(4, 4.0)).unwrap(),
        1.0,
        16,
    )
    .unwrap();
    assert_eq!(a, b);
    for x in 4..12 {
        let expected = if (x + 4 - 3) % 4 < 2 { RED } else { [0; 4] };
        assert_eq!(pixel(&a, x, 8), expected, "phase x={x}");
    }
    // A different origin shifts the phase (the box is not ignored).
    let moved = mapping(
        [bbox_origin + 1.0, 0.0],
        affine_identity(),
        [0.0, 0.0, 16.0, 16.0],
    );
    let c = paint(
        &tile,
        moved.pixel_inverse(descriptor(4, 4.0)).unwrap(),
        1.0,
        16,
    )
    .unwrap();
    assert_ne!(a, c);
    // rotate(90) patternTransform through the same mapping.
    let rotate = Affine {
        a: 0.0,
        b: 1.0,
        c: -1.0,
        d: 0.0,
        e: 0.0,
        f: 0.0,
    };
    let turned = mapping([0.0, 0.0], rotate, [0.0, 0.0, 16.0, 16.0]);
    let d = paint(
        &tile,
        turned.pixel_inverse(descriptor(4, 4.0)).unwrap(),
        1.0,
        16,
    )
    .unwrap();
    assert_eq!(pixel(&d, 8, 4), RED);
    assert_eq!(pixel(&d, 8, 6), [0; 4]);
    assert_eq!(pixel(&d, 4, 4), pixel(&d, 11, 4));
}

#[test]
fn zero_or_negative_tile_size_paints_nothing() {
    for (w, h) in [
        (0.0, 4.0),
        (4.0, 0.0),
        (-1.0, 4.0),
        (4.0, -2.0),
        (f64::NAN, 4.0),
    ] {
        assert!(
            render_pattern_tile(spec(4, 4, w, h), &[], NEVER)
                .unwrap()
                .is_none(),
            "{w}x{h}"
        );
    }
    // A pattern without a tile paints nothing, not even the solid colour.
    let edges = polygon(&[(0.0, 0.0), (8.0, 0.0), (8.0, 8.0), (0.0, 8.0)]);
    let shape = PatternPaintShape {
        shape: fill_shape(&edges, RED, false),
        fill: Some(PatternPaint {
            tile: None,
            inverse: affine_identity(),
            opacity: 1.0,
        }),
        stroke: None,
    };
    let mut out = empty_tile(8, 8);
    render_pattern_shapes(&[shape], unit_view(8), &mut out, NEVER).unwrap();
    assert!(out.rgba().iter().all(|b| *b == 0));
    assert_eq!(
        pattern_pixel_size(0.0, 4.0, 4.0),
        Err(RenderError::InvalidGeometry)
    );
    assert_eq!(
        pattern_pixel_size(-1.0, 4.0, 4.0),
        Err(RenderError::InvalidGeometry)
    );
}

#[test]
fn pattern_pixel_size_rounds_up_and_enforces_limits() {
    assert_eq!(pattern_pixel_size(2.5, 1.0, 4.0), Ok((10, 4)));
    assert_eq!(pattern_pixel_size(0.01, 0.01, 4.0), Ok((1, 1)));
    // Axis limit 4096: 1025 units at 4 px per unit is 4100 pixels.
    assert_eq!(
        pattern_pixel_size(1025.0, 1.0, 4.0),
        Err(RenderError::LimitExceeded)
    );
    assert_eq!(pattern_pixel_size(1024.0, 1.0, 4.0), Ok((4096, 4)));
    // Area limit 1048576 with both axes under 4096: 2000 x 600 = 1_200_000.
    assert_eq!(
        pattern_pixel_size(500.0, 150.0, 4.0),
        Err(RenderError::LimitExceeded)
    );
    assert_eq!(pattern_pixel_size(256.0, 256.0, 4.0), Ok((1024, 1024)));
    assert_eq!(
        pattern_pixel_size(70000.0, 1.0, 0.001),
        Err(RenderError::LimitExceeded)
    );
}

#[test]
fn over_axis_and_over_pixel_tiles_are_limit_exceeded() {
    assert_eq!(
        render_pattern_tile(spec(4097, 1, 4.0, 4.0), &[], NEVER).map(|_| ()),
        Err(RenderError::LimitExceeded)
    );
    assert_eq!(
        render_pattern_tile(spec(1, 4097, 4.0, 4.0), &[], NEVER).map(|_| ()),
        Err(RenderError::LimitExceeded)
    );
    // 2000 x 600 is within both axes but over 1048576 pixels.
    assert_eq!(
        render_pattern_tile(spec(2000, 600, 4.0, 4.0), &[], NEVER).map(|_| ()),
        Err(RenderError::LimitExceeded)
    );
    assert_eq!(
        render_pattern_tile(spec(4, 4, 65537.0, 4.0), &[], NEVER).map(|_| ()),
        Err(RenderError::LimitExceeded)
    );
    // A tile exactly at the area limit is accepted.
    let big = render_pattern_tile(spec(1024, 1024, 4.0, 4.0), &[], NEVER)
        .unwrap()
        .unwrap();
    assert_eq!((big.width(), big.height()), (1024, 1024));
}

#[test]
fn more_than_1024_instances_are_limit_exceeded() {
    let one = spec(1, 1, 1.0, 1.0);
    let ok: Vec<_> = (0..PATTERN_MAX_INSTANCES).map(|_| (one, &[][..])).collect();
    assert_eq!(render_pattern_tiles(&ok, NEVER).unwrap().len(), 1024);
    let over: Vec<_> = (0..=PATTERN_MAX_INSTANCES)
        .map(|_| (one, &[][..]))
        .collect();
    assert_eq!(
        render_pattern_tiles(&over, NEVER).map(|_| ()),
        Err(RenderError::LimitExceeded)
    );
    // 1025 distinct tiles referenced by one render are refused before painting.
    let tiles: Vec<Tile> = (0..=PATTERN_MAX_INSTANCES)
        .map(|_| empty_tile(1, 1))
        .collect();
    let edges = polygon(&[(0.0, 0.0), (4.0, 0.0), (4.0, 4.0), (0.0, 4.0)]);
    let shapes: Vec<_> = tiles
        .iter()
        .map(|t| PatternPaintShape {
            shape: fill_shape(&edges, RED, false),
            fill: Some(PatternPaint {
                tile: Some(t),
                inverse: affine_identity(),
                opacity: 1.0,
            }),
            stroke: None,
        })
        .collect();
    let mut out = empty_tile(4, 4);
    assert_eq!(
        render_pattern_shapes(&shapes, unit_view(4), &mut out, NEVER),
        Err(RenderError::LimitExceeded)
    );
    assert!(out.rgba().iter().all(|b| *b == 0));
    let mut out = empty_tile(4, 4);
    assert_eq!(
        render_pattern_shapes(&shapes[..1024], unit_view(4), &mut out, NEVER),
        Ok(())
    );
}

#[test]
fn cancellation_returns_cancelled() {
    let edges = polygon(&[(0.0, 0.0), (4.0, 0.0), (4.0, 4.0), (0.0, 4.0)]);
    let child = PatternPaintShape::from(fill_shape(&edges, RED, false));
    assert_eq!(
        render_pattern_tile(spec(4, 4, 4.0, 4.0), &[child], &|| true).map(|_| ()),
        Err(RenderError::Cancelled)
    );
    assert_eq!(
        render_pattern_tiles(&[(spec(4, 4, 4.0, 4.0), &[child][..])], &|| true).map(|_| ()),
        Err(RenderError::Cancelled)
    );
    let tile = stripe_tile(4, 4.0, 2.0);
    let polls = Cell::new(0);
    let control = || {
        polls.set(polls.get() + 1);
        polls.get() > 4
    };
    let square = polygon(&[(0.0, 0.0), (16.0, 0.0), (16.0, 16.0), (0.0, 16.0)]);
    let shape = PatternPaintShape {
        shape: fill_shape(&square, RED, false),
        fill: Some(PatternPaint {
            tile: Some(&tile),
            inverse: affine_identity(),
            opacity: 1.0,
        }),
        stroke: None,
    };
    let mut out = empty_tile(16, 16);
    assert_eq!(
        render_pattern_shapes(&[shape], unit_view(16), &mut out, &control),
        Err(RenderError::Cancelled)
    );
    assert_eq!(polls.get(), 5, "one poll per shape and row");
}

#[test]
fn opacity_scales_alpha_and_invalid_input_is_typed() {
    let tile = stripe_tile(4, 4.0, 4.0);
    let half = paint(&tile, affine_identity(), 0.5, 16).unwrap();
    assert_eq!(pixel(&half, 8, 8), [255, 0, 0, 128]);
    let none = paint(&tile, affine_identity(), 0.0, 16).unwrap();
    assert!(none.rgba().iter().all(|b| *b == 0));
    for bad in [-0.1, 1.5, f64::NAN] {
        assert_eq!(
            paint(&tile, affine_identity(), bad, 16).map(|_| ()),
            Err(RenderError::InvalidGeometry)
        );
    }
    let singular = Affine {
        a: 0.0,
        b: 0.0,
        c: 0.0,
        d: 0.0,
        e: 0.0,
        f: 0.0,
    };
    let non_finite = Affine {
        a: f64::NAN,
        ..affine_identity()
    };
    assert_eq!(
        paint(&tile, non_finite, 1.0, 16).map(|_| ()),
        Err(RenderError::InvalidGeometry)
    );
    // A singular element transform cannot be inverted to place the pattern.
    let edges = polygon(&[(0.0, 0.0), (4.0, 0.0), (4.0, 4.0), (0.0, 4.0)]);
    let mut shape = fill_shape(&edges, RED, false);
    shape.transform = singular;
    let entry = PatternPaintShape {
        shape,
        fill: Some(PatternPaint {
            tile: Some(&tile),
            inverse: affine_identity(),
            opacity: 1.0,
        }),
        stroke: None,
    };
    let mut out = empty_tile(4, 4);
    assert_eq!(
        render_pattern_shapes(&[entry], unit_view(4), &mut out, NEVER),
        Err(RenderError::InvalidGeometry)
    );
}

#[test]
fn stroke_pattern_paints_the_stroke_only() {
    let tile = stripe_tile(4, 4.0, 4.0);
    let edges = polyline(&[(1.0, 8.0), (15.0, 8.0)]);
    let mut shape = stroke_shape(
        &edges,
        false,
        style(RED, 6.0, Linecap::Butt, Linejoin::Miter, 4.0),
    );
    shape.stroke = Some(StrokeStyle {
        rgba: [0, 255, 0, 255],
        ..shape.stroke.unwrap()
    });
    let entry = PatternPaintShape {
        shape,
        fill: None,
        stroke: Some(PatternPaint {
            tile: Some(&tile),
            inverse: affine_identity(),
            opacity: 1.0,
        }),
    };
    let mut out = empty_tile(16, 16);
    render_pattern_shapes(&[entry], unit_view(16), &mut out, NEVER).unwrap();
    assert_eq!(
        pixel(&out, 8, 8),
        RED,
        "stroke centre uses the pattern colour, not the solid green"
    );
    assert_eq!(pixel(&out, 8, 1), [0; 4]);
}

fn scene_shape() -> Shape {
    Shape::new(SourceMap {
        element: Span { start: 0, end: 0 },
        transform: affine_identity(),
        transform_node: None,
    })
}

fn scene_paint(kind: PaintKind) -> Paint {
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

#[test]
fn scene_adapter_keeps_other_deferrals_and_rejects_unresolved_patterns() {
    let none = |_| None;
    let convert = |shape: &Shape, fill: &[Paint], stroke: &[Paint]| {
        from_scene_shape_with_pattern(shape, RED, fill, stroke, &none, &[], &[], &[]).map(|_| ())
    };
    let mut solid = scene_shape();
    solid.fill_paint = Some(0);
    assert_eq!(
        convert(&solid, &[scene_paint(PaintKind::Solid)], &[]),
        Ok(())
    );
    let mut pattern = scene_shape();
    pattern.fill_paint = Some(0);
    assert_eq!(
        convert(&pattern, &[scene_paint(PaintKind::Pattern)], &[]),
        Err(RenderError::InvalidGeometry)
    );
    let mut stroke = scene_shape();
    stroke.stroke_paint = Some(0);
    assert_eq!(
        convert(&stroke, &[], &[scene_paint(PaintKind::Pattern)]),
        Err(RenderError::InvalidGeometry)
    );
    let mut gradient = scene_shape();
    gradient.fill_paint = Some(0);
    assert_eq!(
        convert(&gradient, &[scene_paint(PaintKind::Linear)], &[]),
        Err(RenderError::Unsupported(UnsupportedKind::GradientPaint))
    );
    let mut dash = scene_shape();
    dash.stroke.dash_values = TableRange { first: 0, count: 2 };
    assert_eq!(
        convert(&dash, &[], &[]),
        Err(RenderError::Unsupported(UnsupportedKind::Dash))
    );
    let identity: SceneAffine = affine_identity();
    assert_eq!(identity, scene_shape().stroke_inverse);
}

fn built(case: &'static Case) -> Vec<(Vec<FlatEdge>, Vec<std::ops::Range<usize>>, Vec<bool>)> {
    case.shapes
        .iter()
        .map(|d| {
            (
                d.edges
                    .iter()
                    .map(|e| FlatEdge {
                        x0: e[0],
                        y0: e[1],
                        x1: e[2],
                        y1: e[3],
                    })
                    .collect(),
                d.subpaths.iter().map(|s| s.0..s.1).collect(),
                d.subpaths.iter().map(|s| s.2).collect(),
            )
        })
        .collect()
}

#[test]
fn solid_shapes_render_identically_through_both_entry_points() {
    for case in CASES.iter().chain(EXTRA) {
        let geometry = built(case);
        let shapes: Vec<RenderShape<'_>> = case
            .shapes
            .iter()
            .zip(&geometry)
            .map(|(d, (edges, ranges, closed))| {
                let m = d.transform_inverse;
                let det = m[0] * m[3] - m[1] * m[2];
                let r = 1.0 / det;
                let (a, b, c, dd) = (m[3] * r, -m[1] * r, -m[2] * r, m[0] * r);
                RenderShape {
                    edges,
                    subpaths: ranges,
                    subpath_closed: closed,
                    stroke_closed: d.stroke_closed,
                    evenodd: d.evenodd,
                    filled: d.filled,
                    fill_rgba: d.fill,
                    stroke: Some(StrokeStyle {
                        rgba: d.stroke,
                        width: d.stroke_width,
                        miter_limit: d.miter_limit,
                        dash_offset: 0.0,
                        cap: [Linecap::Butt, Linecap::Square, Linecap::Round][d.cap as usize],
                        join: [Linejoin::Miter, Linejoin::Bevel, Linejoin::Round][d.join as usize],
                        crisp_edges: false,
                    }),
                    transform: Affine {
                        a,
                        b,
                        c,
                        d: dd,
                        e: -(a * m[4] + c * m[5]),
                        f: -(b * m[4] + dd * m[5]),
                    },
                }
            })
            .collect();
        let aspect = case.aspect_set.then_some(PreserveAspect {
            align_x: case.align[0],
            align_y: case.align[1],
            none: case.aspect_none,
            slice: case.aspect_slice,
        });
        let view = ViewMap::from_viewbox(
            ViewBox {
                x: case.view[0],
                y: case.view[1],
                width: case.view[2],
                height: case.view[3],
            },
            case.width,
            case.height,
            aspect,
        )
        .unwrap();
        let mut plain = empty_tile(case.width, case.height);
        render_shapes(&shapes, view, &mut plain, NEVER).unwrap();
        let mut via = empty_tile(case.width, case.height);
        let wrapped: Vec<PatternPaintShape<'_>> = shapes.iter().copied().map(Into::into).collect();
        render_pattern_shapes(&wrapped, view, &mut via, NEVER).unwrap();
        assert_eq!(plain.rgba(), via.rgba(), "{}", case.id);
    }
}
