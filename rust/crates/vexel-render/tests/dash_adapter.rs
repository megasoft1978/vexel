//! `from_scene_shape_with_dashes` and the unchanged `from_scene_shape` Dash rejection.
use vexel_geometry::affine_identity;
use vexel_render::{RenderError, UnsupportedKind, from_scene_shape, from_scene_shape_with_dashes};
use vexel_scene::{Shape, SourceMap, Span, TableRange};

const RED: [u8; 4] = [255, 0, 0, 255];

fn dashed_scene_shape(count: usize) -> Shape {
    let mut shape = Shape::new(SourceMap {
        element: Span { start: 0, end: 0 },
        transform: affine_identity(),
        transform_node: None,
    });
    shape.stroke.dash_values = TableRange { first: 0, count };
    shape.stroke.dash_offset = 1.5;
    shape
}

#[test]
fn with_dashes_carries_the_pattern_and_offset() {
    let shape = dashed_scene_shape(2);
    let dashes = [3.0, 1.0];
    let item = from_scene_shape_with_dashes(&shape, RED, &[], &[], &[], &[], &[], &dashes).unwrap();
    assert_eq!(item.dashes, &dashes);
    assert_eq!(item.shape.stroke.unwrap().dash_offset, 1.5);
}

#[test]
fn with_dashes_requires_a_matching_slice_and_plain_conversion_still_rejects_dashes() {
    let shape = dashed_scene_shape(2);
    let mismatch = from_scene_shape_with_dashes(&shape, RED, &[], &[], &[], &[], &[], &[1.0]);
    assert_eq!(mismatch.map(|_| ()), Err(RenderError::InvalidGeometry));
    let plain = from_scene_shape(&shape, RED, &[], &[], &[], &[], &[]);
    assert_eq!(
        plain.map(|_| ()),
        Err(RenderError::Unsupported(UnsupportedKind::Dash))
    );
    // Undashed scene shapes convert through both entry points.
    let solid = dashed_scene_shape(0);
    assert!(from_scene_shape_with_dashes(&solid, RED, &[], &[], &[], &[], &[], &[]).is_ok());
}
