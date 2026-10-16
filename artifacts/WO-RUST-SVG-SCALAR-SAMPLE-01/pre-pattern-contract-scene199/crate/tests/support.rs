#![allow(dead_code)]
use std::sync::Arc;
use vexel_scene::*;
pub struct Run;
impl WorkControl for Run {
    fn charge(&mut self, _: WorkKind, _: u64) -> std::result::Result<(), Stop> {
        Ok(())
    }
    fn checkpoint(&mut self) -> std::result::Result<(), Stop> {
        Ok(())
    }
}
pub struct StopAfter(pub usize);
impl WorkControl for StopAfter {
    fn charge(&mut self, _: WorkKind, _: u64) -> std::result::Result<(), Stop> {
        if self.0 == 0 {
            Err(Stop::Canceled)
        } else {
            self.0 -= 1;
            Ok(())
        }
    }
    fn checkpoint(&mut self) -> std::result::Result<(), Stop> {
        if self.0 == 0 {
            Err(Stop::Canceled)
        } else {
            Ok(())
        }
    }
}
pub fn viewport() -> Viewport {
    Viewport {
        width: 100,
        height: 100,
        view_x: 0.,
        view_y: 0.,
        view_width: 100.,
        view_height: 100.,
        aspect: Default::default(),
    }
}
pub fn owner() -> SourceOwner {
    let mut v = Vec::with_capacity(4096);
    v.extend_from_slice(br#"<svg><path d="M0 0 L1 1 2 3 Z"/><animate attributeName="opacity" values="0;1" dur="1s"/></svg>"#);
    Arc::new(v)
}
pub fn animation(source: &SourceOwner) -> OwnedAnimation {
    let (tokens, r) = vexel_xml::tokenize(source, vexel_xml::XmlLimits::default());
    assert_eq!(r.diagnostic.category, vexel_xml::XmlError::None);
    vexel_svg_animation::parse_owned(source, &tokens[..r.token_count]).unwrap()
}
pub fn builder() -> SceneBuilder {
    let source = owner();
    SceneBuilder::new(
        source.clone(),
        animation(&source),
        7,
        viewport(),
        Limits::default(),
    )
    .unwrap()
}
pub fn shape(source: &[u8]) -> ShapeInput {
    let start = source.windows(2).position(|x| x == b"M0").unwrap();
    let end = start + "M0 0 L1 1 2 3 Z".len();
    let path = vexel_svg_path::parse_path(
        source,
        Span { start, end },
        vexel_geometry::affine_identity(),
        Default::default(),
    )
    .unwrap();
    let colors = (0..path.edges.len())
        .map(|i| [i as u8, 2, 3, 255])
        .collect();
    ShapeInput {
        shape: Shape::new(SourceMap {
            element: Span { start: 5, end: 31 },
            transform: vexel_geometry::affine_identity(),
            transform_node: None,
        }),
        path,
        edge_colors: colors,
    }
}
pub fn transform(parent: Option<TransformId>) -> TransformFrame {
    let a = vexel_geometry::affine_identity();
    TransformFrame {
        source_offset: 5,
        parent,
        attribute: a,
        suffix: a,
        motion: a,
    }
}
pub fn image() -> Image {
    let mut rgba = Vec::with_capacity(113);
    rgba.extend_from_slice(&[1, 2, 3, 255, 99, 99, 99, 99]);
    Image {
        pixels: ImagePixels {
            rgba,
            byte_count: 8,
            row_stride: 8,
            width: 1,
            height: 1,
        },
        x: 0.,
        y: 0.,
        width: 1.,
        height: 1.,
        draw_x: 0.,
        draw_y: 0.,
        draw_width: 1.,
        draw_height: 1.,
    }
}
pub fn clip() -> Clip {
    Clip {
        geometry: ClipGeometry {
            edges: Vec::with_capacity(7),
            shapes: Vec::with_capacity(3),
        },
        parent: None,
        owner_node: None,
        owner_before_suffix: false,
        object_bounding_box: false,
        bounds: [0., 0., 1., 1.],
        user_inverse: vexel_geometry::affine_identity(),
    }
}
pub fn pattern() -> Pattern {
    let mut pixels = Vec::with_capacity(97);
    pixels.extend_from_slice(&[4, 3, 2, 255]);
    Pattern {
        tile: vexel_raster::Tile::from_owned_rgba(1, 1, pixels).unwrap(),
        definition: ElementId(1),
        tile_width: 1.,
        tile_height: 1.,
        content_transform: vexel_geometry::affine_identity(),
    }
}
pub fn mask(child: SceneId) -> Mask {
    Mask {
        content: child,
        luminance: false,
        region_object_bounding_box: false,
        custom_region: false,
        region: [0., 0., 1., 1.],
        region_transform: vexel_geometry::affine_identity(),
        source_offset: 5,
    }
}
pub fn filter() -> Filter {
    Filter {
        primitives: TableRange::EMPTY,
        linear_rgb: false,
        filter_units_object_bounding_box: false,
        primitive_units_object_bounding_box: false,
        custom_region: false,
        region: [0., 0., 1., 1.],
    }
}
pub fn primitive() -> FilterPrimitive {
    FilterPrimitive {
        kind: FilterKind::Flood,
        input: FilterInput::Graphic,
        input2: FilterInput::Previous,
        merge_inputs: [FilterInput::Previous; 8],
        merge_count: 0,
        mode: 0,
        rgba: [1, 2, 3, 255],
        values: [0.; 20],
        source_offset: 0,
    }
}
pub fn group() -> OpacityGroup {
    OpacityGroup {
        shapes: TableRange::EMPTY,
        parent: None,
        opacity: 1.,
        filter: None,
        mask: None,
        blend_mode: 0,
        isolation: false,
    }
}
pub fn paint() -> Paint {
    let a = vexel_geometry::affine_identity();
    Paint {
        kind: PaintKind::Solid,
        spread: Spread::Pad,
        object_bounding_box: false,
        coordinates: [0.; 6],
        bounds: [0., 0., 1., 1.],
        user_inverse: a,
        gradient_inverse: a,
        stops: TableRange::EMPTY,
        pattern: None,
        pattern_inverse: a,
        pattern_opacity: 1.,
    }
}
