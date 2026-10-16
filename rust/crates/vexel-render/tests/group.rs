//! Unit tests for `render_groups`: opacity, clip groups, masks, limits, cancellation.
mod common;
use common::*;
use std::cell::Cell;
use vexel_geometry::affine_identity;
use vexel_raster::Tile;
use vexel_render::{
    ClipShape, FlatEdge, MAX_MASK_INSTANCES, RenderClip, RenderError, RenderGroup, RenderMask,
    RenderNode, RenderShape, render_groups,
};

const SIZE: u32 = 16;

fn square(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<FlatEdge> {
    polygon(&[(x0, y0), (x1, y0), (x1, y1), (x0, y1)])
}

fn node<'a>(shape: RenderShape<'a>) -> RenderNode<'a> {
    RenderNode::Shape { shape, clip: None }
}

fn group<'a>(
    opacity: f64,
    mask: Option<&'a RenderMask<'a>>,
    children: &'a [RenderNode<'a>],
) -> RenderNode<'a> {
    RenderNode::Group(RenderGroup {
        opacity,
        clip: None,
        mask,
        children,
    })
}

fn run(nodes: &[RenderNode<'_>]) -> Result<Tile, RenderError> {
    let mut tile = empty_tile(SIZE, SIZE);
    render_groups(nodes, unit_view(SIZE), &mut tile, NEVER)?;
    Ok(tile)
}

fn mask<'a>(luminance: bool, region: [f64; 4], content: &'a [RenderNode<'a>]) -> RenderMask<'a> {
    RenderMask {
        luminance,
        region,
        region_transform: affine_identity(),
        content,
    }
}

#[test]
fn opacity_zero_half_and_one() {
    let edges = square(2.0, 2.0, 10.0, 10.0);
    let kids = [node(fill_shape(&edges, [200, 0, 0, 255], false))];
    assert_eq!(
        pixel(&run(&[group(0.0, None, &kids)]).unwrap(), 5, 5),
        [0, 0, 0, 0]
    );
    let half = pixel(&run(&[group(0.5, None, &kids)]).unwrap(), 5, 5);
    assert_eq!(half, [200, 0, 0, 128]);
    assert_eq!(
        pixel(&run(&[group(1.0, None, &kids)]).unwrap(), 5, 5),
        [200, 0, 0, 255]
    );
}

#[test]
fn nested_opacity_multiplies_and_overlap_is_isolated() {
    let edges = square(2.0, 2.0, 10.0, 10.0);
    let overlap = square(6.0, 6.0, 14.0, 14.0);
    let inner = [node(fill_shape(&overlap, [0, 0, 200, 255], false))];
    let outer = [
        node(fill_shape(&edges, [200, 0, 0, 255], false)),
        group(0.5, None, &inner),
    ];
    let tile = run(&[group(0.5, None, &outer)]).unwrap();
    assert_eq!(pixel(&tile, 3, 3)[3], 128, "single level 0.5");
    // Inner-only pixel: 0.5 * 0.5 of 255 = 63.75
    assert_eq!(pixel(&tile, 12, 12)[3], 64);
    // The group does not blend per child: inner over outer is composed in the layer first.
    let both = pixel(&tile, 8, 8);
    assert_eq!(
        both,
        [100, 0, 100, 128],
        "inner 50% blue over opaque red, then outer 50%"
    );
}

#[test]
fn clip_group_excludes_pixels_outside_the_clip() {
    let clip_edges = [
        FlatEdge {
            x0: 0.0,
            y0: 0.0,
            x1: 8.0,
            y1: 0.0,
        },
        FlatEdge {
            x0: 8.0,
            y0: 0.0,
            x1: 8.0,
            y1: 16.0,
        },
        FlatEdge {
            x0: 8.0,
            y0: 16.0,
            x1: 0.0,
            y1: 16.0,
        },
        FlatEdge {
            x0: 0.0,
            y0: 16.0,
            x1: 0.0,
            y1: 0.0,
        },
    ];
    let shapes = [ClipShape {
        edges: 0..4,
        evenodd: false,
        filled: true,
    }];
    let clip = RenderClip {
        edges: &clip_edges,
        shapes: &shapes,
        parent: None,
        object_bounding_box: false,
        bounds: [0.0; 4],
        user_inverse: affine_identity(),
    };
    let a = square(2.0, 2.0, 6.0, 6.0);
    let b = square(10.0, 2.0, 14.0, 6.0);
    let kids = [
        node(fill_shape(&a, [9, 9, 9, 255], false)),
        node(fill_shape(&b, [9, 9, 9, 255], false)),
    ];
    let tile = run(&[RenderNode::Group(RenderGroup {
        opacity: 1.0,
        clip: Some(&clip),
        mask: None,
        children: &kids,
    })])
    .unwrap();
    assert_eq!(pixel(&tile, 3, 3), [9, 9, 9, 255]);
    assert_eq!(pixel(&tile, 11, 3), [0, 0, 0, 0]);
}

fn masked(luminance: bool, content_rgba: [u8; 4]) -> [u8; 4] {
    let target = square(0.0, 0.0, 16.0, 16.0);
    let content_edges = square(0.0, 0.0, 16.0, 16.0);
    let content = [node(fill_shape(&content_edges, content_rgba, false))];
    let m = mask(luminance, [0.0, 0.0, 16.0, 16.0], &content);
    let kids = [node(fill_shape(&target, [10, 20, 30, 255], false))];
    pixel(&run(&[group(1.0, Some(&m), &kids)]).unwrap(), 8, 8)
}

#[test]
fn luminance_and_alpha_masks_differ() {
    // White at alpha 128: luminance 1.0 -> alpha 128; alpha mask -> 128.
    assert_eq!(masked(true, [255, 255, 255, 255])[3], 255);
    assert_eq!(
        masked(false, [0, 0, 0, 255])[3],
        255,
        "alpha mask ignores colour"
    );
    assert_eq!(
        masked(true, [0, 0, 0, 255]),
        [0, 0, 0, 0],
        "black luminance hides"
    );
    let mid = masked(true, [128, 128, 128, 255])[3];
    assert!(
        (50..=60).contains(&mid),
        "linear luminance of #808080 is about 0.216: {mid}"
    );
    assert_eq!(masked(false, [255, 255, 255, 128])[3], 128);
}

#[test]
fn mask_with_zero_area_region_renders_nothing() {
    let target = square(0.0, 0.0, 16.0, 16.0);
    let content = [node(fill_shape(&target, [255, 255, 255, 255], false))];
    let kids = [node(fill_shape(&target, [10, 20, 30, 255], false))];
    for region in [
        [4.0, 4.0, 0.0, 8.0],
        [4.0, 4.0, 8.0, 0.0],
        [4.0, 4.0, 0.0, 0.0],
    ] {
        let m = mask(false, region, &content);
        let tile = run(&[group(1.0, Some(&m), &kids)]).unwrap();
        assert!(tile.rgba().iter().all(|b| *b == 0), "{region:?}");
    }
}

#[test]
fn mask_region_restricts_to_its_rectangle() {
    let target = square(0.0, 0.0, 16.0, 16.0);
    let content = [node(fill_shape(&target, [255, 255, 255, 255], false))];
    let kids = [node(fill_shape(&target, [10, 20, 30, 255], false))];
    let m = mask(true, [4.0, 4.0, 8.0, 8.0], &content);
    let tile = run(&[group(1.0, Some(&m), &kids)]).unwrap();
    assert_eq!(pixel(&tile, 8, 8), [10, 20, 30, 255]);
    assert_eq!(pixel(&tile, 2, 2), [0, 0, 0, 0]);
    assert_eq!(pixel(&tile, 13, 8), [0, 0, 0, 0]);
}

#[test]
fn mask_instance_limit_is_typed() {
    let target = square(0.0, 0.0, 4.0, 4.0);
    let kids = [node(fill_shape(&target, [1, 2, 3, 255], false))];
    let m = mask(false, [0.0, 0.0, 16.0, 16.0], &[]);
    let groups =
        |n: usize| -> Vec<RenderNode<'_>> { (0..n).map(|_| group(1.0, Some(&m), &kids)).collect() };
    assert_eq!(MAX_MASK_INSTANCES, 1024);
    assert!(run(&groups(1024)).is_ok());
    assert_eq!(run(&groups(1025)).unwrap_err(), RenderError::LimitExceeded);
}

#[test]
fn mask_nesting_deeper_than_eight_is_a_limit_error() {
    let edges: &'static [FlatEdge] = Box::leak(square(0.0, 0.0, 4.0, 4.0).into_boxed_slice());
    let leaf = || node(fill_shape(edges, [1, 2, 3, 255], false));
    let mut content: Vec<RenderNode<'static>> = vec![leaf()];
    let build = |depth: usize| {
        let mut content = vec![leaf()];
        for _ in 0..depth {
            let inner: &'static [RenderNode<'static>] = Box::leak(content.into_boxed_slice());
            let m: &'static RenderMask<'static> =
                Box::leak(Box::new(mask(false, [0.0, 0.0, 16.0, 16.0], inner)));
            let kids: &'static [RenderNode<'static>] = Box::leak(vec![leaf()].into_boxed_slice());
            content = vec![group(1.0, Some(m), kids)];
        }
        content
    };
    content = build(8);
    assert!(run(&content).is_ok(), "eight nested levels render");
    assert_eq!(run(&build(9)).unwrap_err(), RenderError::LimitExceeded);
}

#[test]
fn cancellation_mid_layer_returns_cancelled() {
    let edges = square(2.0, 2.0, 10.0, 10.0);
    let kids = [node(fill_shape(&edges, [200, 0, 0, 255], false))];
    let polls = Cell::new(0u32);
    let control = || {
        polls.set(polls.get() + 1);
        polls.get() > 6
    };
    let mut tile: Tile = empty_tile(SIZE, SIZE);
    let result = render_groups(
        &[group(0.5, None, &kids)],
        unit_view(SIZE),
        &mut tile,
        &control,
    );
    assert_eq!(result.unwrap_err(), RenderError::Cancelled);
}

#[test]
fn invalid_opacity_and_non_empty_tile_are_rejected() {
    let edges = square(2.0, 2.0, 10.0, 10.0);
    let kids = [node(fill_shape(&edges, [1, 1, 1, 255], false))];
    for opacity in [-0.1, 1.5, f64::NAN] {
        assert_eq!(
            run(&[group(opacity, None, &kids)]).unwrap_err(),
            RenderError::InvalidGeometry
        );
    }
    let mut tile = empty_tile(SIZE, SIZE);
    tile.rgba_mut()[0] = 1;
    assert_eq!(
        render_groups(&kids, unit_view(SIZE), &mut tile, NEVER).unwrap_err(),
        RenderError::TileNotEmpty
    );
}
