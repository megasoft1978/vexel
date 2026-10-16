#![allow(dead_code)]

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use vexel_base::Span;
use vexel_geometry::{Affine, Edge, Point, affine_identity};
use vexel_scene::{
    Clip, ClipChild, ClipGeometry, LexicalDocument, Limits, Paint, PaintKind, SceneBuilder, Shape,
    ShapeInput, SourceMap, Spread, Stop, TableRange, TransformFrame, Viewport, WorkControl,
    WorkKind,
};
use vexel_svg_animation::parse_owned;
use vexel_svg_path::{ParseOptions, parse_path};

pub struct Control {
    pub canceled: AtomicBool,
    pub work: u64,
    limit: u64,
}
impl Control {
    pub fn new(limit: u64) -> Self {
        Self {
            canceled: AtomicBool::new(false),
            work: 0,
            limit,
        }
    }
}
impl WorkControl for Control {
    fn charge(&mut self, _: WorkKind, units: u64) -> Result<(), Stop> {
        if self.canceled.load(Ordering::Acquire) {
            return Err(Stop::Canceled);
        }
        let next = self.work.checked_add(units).ok_or(Stop::Overflow)?;
        if next > self.limit {
            return Err(Stop::Limit);
        }
        self.work = next;
        Ok(())
    }
    fn checkpoint(&mut self) -> Result<(), Stop> {
        if self.canceled.load(Ordering::Acquire) {
            Err(Stop::Canceled)
        } else {
            Ok(())
        }
    }
}

pub struct Fixture {
    pub graph: vexel_scene::SceneGraph,
    pub shape: vexel_scene::ShapeId,
    pub clip: Option<vexel_scene::ClipId>,
    pub source: Arc<Vec<u8>>,
}

pub fn fixture(animation: &str, path_data: &str, with_clip: bool) -> Fixture {
    fixture_with_options(animation, path_data, with_clip, false)
}

pub fn fixture_with_options(
    animation: &str,
    path_data: &str,
    with_clip: bool,
    with_object_paint: bool,
) -> Fixture {
    let element = if animation.contains("attributeName='d'") {
        format!("<path d='{path_data}'>{animation}</path>")
    } else {
        format!("<rect x='1' y='1' width='8' height='8' fill='red'>{animation}</rect>")
    };
    let svg =
        format!("<svg width='16' height='12'><metadata>{path_data}</metadata>{element}</svg>");
    let source = Arc::new(svg.into_bytes());
    let mut control = Control::new(u64::MAX);
    let lexical = LexicalDocument::parse(source.clone(), 1 << 20, &mut control).unwrap();
    let animation = parse_owned(&source, lexical.tokens()).unwrap();
    let target = animation.tracks()[0].target;
    let path_start = source
        .windows(path_data.len())
        .position(|window| window == path_data.as_bytes())
        .unwrap();
    let path = parse_path(
        &source,
        Span {
            start: path_start,
            end: path_start + path_data.len(),
        },
        affine_identity(),
        ParseOptions::default(),
    )
    .unwrap();
    let viewport = Viewport {
        width: 16,
        height: 12,
        view_x: 0.0,
        view_y: 0.0,
        view_width: 16.0,
        view_height: 12.0,
        aspect: Default::default(),
    };
    let mut builder =
        SceneBuilder::new(source.clone(), animation, 1, viewport, Limits::default()).unwrap();
    let scene = builder.root_id();
    let transform = builder
        .append_transform(
            scene,
            TransformFrame {
                source_offset: target.offset,
                parent: None,
                attribute: affine_identity(),
                suffix: affine_identity(),
                motion: affine_identity(),
            },
            &mut control,
        )
        .unwrap();
    let clip = if with_clip {
        Some(
            builder
                .append_clip(
                    scene,
                    Clip {
                        geometry: ClipGeometry {
                            edges: vec![Edge {
                                a: Point { x: 0.0, y: 0.0 },
                                b: Point { x: 1.0, y: 1.0 },
                            }],
                            shapes: vec![ClipChild {
                                edges: TableRange { first: 0, count: 1 },
                                evenodd: false,
                                filled: true,
                            }],
                        },
                        parent: None,
                        owner_node: Some(transform),
                        owner_before_suffix: true,
                        object_bounding_box: true,
                        bounds: [1.0, 1.0, 9.0, 9.0],
                        user_inverse: affine_identity(),
                    },
                    &mut control,
                )
                .unwrap(),
        )
    } else {
        None
    };
    let fill_paint = if with_object_paint {
        Some(
            builder
                .append_fill_paint(
                    scene,
                    Paint {
                        kind: PaintKind::Linear,
                        spread: Spread::Pad,
                        object_bounding_box: true,
                        coordinates: [0.0, 0.0, 1.0, 0.0, 0.0, 0.0],
                        bounds: [1.0, 1.0, 9.0, 9.0],
                        user_inverse: affine_identity(),
                        gradient_inverse: affine_identity(),
                        stops: TableRange::EMPTY,
                        pattern: None,
                        pattern_inverse: affine_identity(),
                        pattern_opacity: 1.0,
                    },
                    &mut control,
                )
                .unwrap(),
        )
    } else {
        None
    };
    let mut shape = Shape::new(SourceMap {
        element: Span {
            start: target.offset,
            end: target.offset + target.length,
        },
        transform: affine_identity(),
        transform_node: Some(transform),
    });
    shape.clip = clip;
    shape.fill_paint = fill_paint;
    let shape_id = builder
        .append_shape(
            scene,
            ShapeInput {
                shape,
                edge_colors: vec![[255, 0, 0, 255]; path.edges.len()],
                path,
            },
            &mut control,
        )
        .unwrap();
    Fixture {
        graph: builder.freeze().unwrap(),
        shape: shape_id,
        clip,
        source,
    }
}

pub fn clip_fixture() -> (
    vexel_scene::SceneGraph,
    vexel_scene::ShapeId,
    vexel_scene::ClipId,
    vexel_scene::ClipId,
    vexel_scene::ClipId,
) {
    let path_data = "M0 0 L2 0 L2 2 L0 2 Z";
    let source =
        Arc::new(format!("<svg><metadata>{path_data}</metadata><rect/></svg>").into_bytes());
    let mut control = Control::new(u64::MAX);
    let lexical = LexicalDocument::parse(source.clone(), 1 << 20, &mut control).unwrap();
    let animation = parse_owned(&source, lexical.tokens()).unwrap();
    let target = source
        .windows(5)
        .position(|window| window == b"<rect")
        .unwrap();
    let viewport = Viewport {
        width: 8,
        height: 8,
        view_x: 0.0,
        view_y: 0.0,
        view_width: 8.0,
        view_height: 8.0,
        aspect: Default::default(),
    };
    let mut builder =
        SceneBuilder::new(source.clone(), animation, 1, viewport, Limits::default()).unwrap();
    let scene = builder.root_id();
    let parent = builder
        .append_transform(
            scene,
            TransformFrame {
                source_offset: target,
                parent: None,
                attribute: Affine {
                    e: 5.0,
                    ..affine_identity()
                },
                suffix: Affine {
                    e: 7.0,
                    ..affine_identity()
                },
                motion: affine_identity(),
            },
            &mut control,
        )
        .unwrap();
    let child = builder
        .append_transform(
            scene,
            TransformFrame {
                source_offset: target,
                parent: Some(parent),
                attribute: Affine {
                    e: 3.0,
                    ..affine_identity()
                },
                suffix: Affine {
                    e: 4.0,
                    ..affine_identity()
                },
                motion: affine_identity(),
            },
            &mut control,
        )
        .unwrap();
    let add_clip = |builder: &mut SceneBuilder,
                    owner_node,
                    before_suffix,
                    parent_clip,
                    control: &mut Control| {
        builder
            .append_clip(
                scene,
                Clip {
                    geometry: ClipGeometry {
                        edges: vec![Edge {
                            a: Point { x: 0.0, y: 0.0 },
                            b: Point { x: 1.0, y: 1.0 },
                        }],
                        shapes: vec![ClipChild {
                            edges: TableRange { first: 0, count: 1 },
                            evenodd: false,
                            filled: true,
                        }],
                    },
                    parent: parent_clip,
                    owner_node: Some(owner_node),
                    owner_before_suffix: before_suffix,
                    object_bounding_box: true,
                    bounds: [0.0, 0.0, 2.0, 2.0],
                    user_inverse: affine_identity(),
                },
                control,
            )
            .unwrap()
    };
    let parent_clip = add_clip(&mut builder, parent, true, None, &mut control);
    let child_before_clip = add_clip(&mut builder, child, true, Some(parent_clip), &mut control);
    let child_full_clip = add_clip(
        &mut builder,
        child,
        false,
        Some(child_before_clip),
        &mut control,
    );
    let path_start = source
        .windows(path_data.len())
        .position(|window| window == path_data.as_bytes())
        .unwrap();
    let path = parse_path(
        &source,
        Span {
            start: path_start,
            end: path_start + path_data.len(),
        },
        Affine {
            e: 19.0,
            ..affine_identity()
        },
        ParseOptions::default(),
    )
    .unwrap();
    let mut shape = Shape::new(SourceMap {
        element: Span {
            start: target,
            end: target + 7,
        },
        transform: Affine {
            e: 19.0,
            ..affine_identity()
        },
        transform_node: Some(child),
    });
    shape.clip = Some(child_full_clip);
    let shape_id = builder
        .append_shape(
            scene,
            ShapeInput {
                edge_colors: vec![[0, 0, 0, 255]; path.edges.len()],
                shape,
                path: path.clone(),
            },
            &mut control,
        )
        .unwrap();
    let mut alias = Shape::new(SourceMap {
        element: Span {
            start: target,
            end: target + 7,
        },
        transform: Affine {
            e: 19.0,
            ..affine_identity()
        },
        transform_node: Some(child),
    });
    alias.clip = Some(child_full_clip);
    builder
        .append_shape(
            scene,
            ShapeInput {
                edge_colors: vec![[0, 0, 0, 255]; path.edges.len()],
                shape: alias,
                path,
            },
            &mut control,
        )
        .unwrap();
    let zero_path = parse_path(
        &source,
        Span {
            start: target,
            end: target,
        },
        affine_identity(),
        ParseOptions::default(),
    )
    .unwrap();
    let zero_shape = Shape::new(SourceMap {
        element: Span {
            start: target,
            end: target + 7,
        },
        transform: affine_identity(),
        transform_node: Some(child),
    });
    builder
        .append_shape(
            scene,
            ShapeInput {
                shape: zero_shape,
                edge_colors: Vec::new(),
                path: zero_path,
            },
            &mut control,
        )
        .unwrap();
    (
        builder.freeze().unwrap(),
        shape_id,
        parent_clip,
        child_before_clip,
        child_full_clip,
    )
}

pub fn edges(frame: &vexel_scene::EvaluatedFrame<'_>, shape: vexel_scene::ShapeId) -> Vec<Edge> {
    frame
        .view()
        .shape(shape)
        .unwrap()
        .edges
        .slice(frame.view().edges)
        .unwrap()
        .to_vec()
}
