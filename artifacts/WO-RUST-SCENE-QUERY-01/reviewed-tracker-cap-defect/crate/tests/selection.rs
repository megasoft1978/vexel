use std::sync::Arc;
use vexel_base::{Span, WorkControl, WorkKind};
use vexel_geometry::{Edge, affine_identity};
use vexel_scene::*;
use vexel_scene_query::{bounds_query, element_at_offset, hit_test, source_assign};

struct Run;
impl WorkControl for Run {
    fn charge(&mut self, _: WorkKind, _: u64) -> std::result::Result<(), Stop> {
        Ok(())
    }
    fn checkpoint(&mut self) -> std::result::Result<(), Stop> {
        Ok(())
    }
}
struct Cancel;
impl WorkControl for Cancel {
    fn charge(&mut self, _: WorkKind, _: u64) -> std::result::Result<(), Stop> {
        Err(Stop::Canceled)
    }
    fn checkpoint(&mut self) -> std::result::Result<(), Stop> {
        Err(Stop::Canceled)
    }
}
struct Exhaust(Stop);
impl WorkControl for Exhaust {
    fn charge(&mut self, _: WorkKind, _: u64) -> std::result::Result<(), Stop> {
        Err(self.0)
    }
    fn checkpoint(&mut self) -> std::result::Result<(), Stop> {
        Err(self.0)
    }
}

struct ClipFixture {
    result: bool,
}
impl KernelResources for ClipFixture {
    fn paint_sample(&self, _: &Paint, _: &[PaintStop], _: f64, _: f64) -> [u8; 4] {
        [0; 4]
    }
    fn clip_contains(
        &self,
        _: SceneView<'_>,
        _: ClipId,
        _: f64,
        _: f64,
        _: &mut dyn WorkControl,
    ) -> Result<bool> {
        Ok(self.result)
    }
    fn clip_boundary_edges(
        &self,
        _: SceneView<'_>,
        _: ClipId,
        _: &mut dyn FnMut(Edge) -> bool,
        _: &mut dyn WorkControl,
    ) -> Result<bool> {
        Ok(false)
    }
    fn clip_work_counts(
        &self,
        _: SceneView<'_>,
        _: ClipId,
        _: &mut dyn WorkControl,
    ) -> Result<ClipWorkCounts> {
        Ok(ClipWorkCounts {
            chain_elements: 0,
            child_shapes: 0,
            edges: 0,
        })
    }
    fn stroke_sample_work(
        &self,
        _: SceneView<'_>,
        _: ShapeId,
        _: &DashCache,
        _: f64,
        _: f64,
        _: &mut dyn WorkControl,
    ) -> Result<(usize, bool, bool)> {
        Ok((0, false, false))
    }
}

fn fixture() -> (SceneGraph, ClipId, ShapeId) {
    let source=Arc::new(b"<svg><path id='outer' d='M0 0 L20 0 L20 20 L0 20 Z'/><path id='inner' d='M0 0 L20 0 L20 20 L0 20 Z'/><path id='line' d='M0 30 L20 30 L20 50'/><path id='empty'/><path id='hole' d='M50 50 L70 50 L70 70 L50 70 Z M55 55 L65 55 L65 65 L55 65 Z'/><path id='image' d='M80 80 L90 80 L90 90 L80 90 Z'/></svg>".to_vec());
    let mut run = Run;
    let lexical = LexicalDocument::parse(source.clone(), 1 << 20, &mut run).unwrap();
    let animation = vexel_svg_animation::parse_owned(&source, lexical.tokens()).unwrap();
    let viewport = Viewport {
        width: 100,
        height: 100,
        view_x: 0.,
        view_y: 0.,
        view_width: 100.,
        view_height: 100.,
        aspect: Default::default(),
    };
    let mut builder =
        SceneBuilder::new(source.clone(), animation, 19, viewport, Limits::default()).unwrap();
    assert!(Arc::ptr_eq(builder.source_owner(), lexical.source_owner()));
    let root = builder.root_id();
    let clip = builder
        .append_clip(
            root,
            Clip {
                geometry: ClipGeometry {
                    edges: Vec::new(),
                    shapes: Vec::new(),
                },
                parent: None,
                owner_node: None,
                owner_before_suffix: false,
                object_bounding_box: false,
                bounds: [0., 0., 20., 20.],
                user_inverse: affine_identity(),
            },
            &mut run,
        )
        .unwrap();
    let src = source.as_slice();
    let add_path = |builder: &mut SceneBuilder,
                    element: &[u8],
                    d: &[u8],
                    fill: bool,
                    clip_id: Option<ClipId>,
                    run: &mut Run| {
        let start = src
            .windows(element.len())
            .position(|w| w == element)
            .unwrap();
        let dstart = src.windows(d.len()).position(|w| w == d).unwrap();
        let path = vexel_svg_path::parse_path(
            src,
            Span {
                start: dstart,
                end: dstart + d.len(),
            },
            affine_identity(),
            Default::default(),
        )
        .unwrap();
        let colors = vec![[200, 20, 10, 255]; path.edges.len()];
        let element_span = if element.windows(5).any(|w| w == b"outer") {
            Span {
                start: 0,
                end: src.len(),
            }
        } else {
            Span {
                start,
                end: start + element.len(),
            }
        };
        let mut shape = Shape::new(SourceMap {
            element: element_span,
            transform: affine_identity(),
            transform_node: None,
        });
        shape.filled = fill;
        shape.clip = clip_id;
        if !fill {
            shape.stroke.rgba = [1, 2, 3, 255];
            shape.stroke.width = 2.;
            shape.stroke_inverse.a = 0.5;
            shape.stroke_inverse.d = 0.5;
        }
        builder
            .append_shape(
                root,
                ShapeInput {
                    shape,
                    path,
                    edge_colors: colors,
                },
                run,
            )
            .unwrap()
    };
    let outer = b"<path id='outer' d='M0 0 L20 0 L20 20 L0 20 Z'/>";
    let inner = b"<path id='inner' d='M0 0 L20 0 L20 20 L0 20 Z'/>";
    let line = b"<path id='line' d='M0 30 L20 30 L20 50'/>";
    let mark = builder.mark(root).unwrap();
    let stale = add_path(
        &mut builder,
        outer,
        b"M0 0 L20 0 L20 20 L0 20 Z",
        true,
        None,
        &mut run,
    );
    builder.rollback(mark).unwrap();
    add_path(
        &mut builder,
        outer,
        b"M0 0 L20 0 L20 20 L0 20 Z",
        true,
        None,
        &mut run,
    );
    add_path(
        &mut builder,
        inner,
        b"M0 0 L20 0 L20 20 L0 20 Z",
        true,
        Some(clip),
        &mut run,
    );
    let line_id = add_path(
        &mut builder,
        line,
        b"M0 30 L20 30 L20 50",
        false,
        Some(clip),
        &mut run,
    );
    let empty_bytes = b"<path id='empty'/>";
    let empty_at = src
        .windows(empty_bytes.len())
        .position(|w| w == empty_bytes)
        .unwrap();
    let empty_span = Span {
        start: empty_at,
        end: empty_at + empty_bytes.len(),
    };
    let empty = ShapeInput {
        shape: Shape::new(SourceMap {
            element: empty_span,
            transform: affine_identity(),
            transform_node: None,
        }),
        path: vexel_svg_path::parse_path(
            src,
            Span {
                start: empty_span.start,
                end: empty_span.start,
            },
            affine_identity(),
            Default::default(),
        )
        .unwrap(),
        edge_colors: Vec::new(),
    };
    let empty_id = builder.append_shape(root, empty, &mut run).unwrap();
    let inner_at = src.windows(inner.len()).position(|w| w == inner).unwrap();
    let shared_source = Span {
        start: inner_at,
        end: inner_at + inner.len(),
    };
    let tied_empty = ShapeInput {
        shape: Shape::new(SourceMap {
            element: shared_source,
            transform: affine_identity(),
            transform_node: None,
        }),
        path: vexel_svg_path::parse_path(
            src,
            Span {
                start: empty_span.start,
                end: empty_span.start,
            },
            affine_identity(),
            Default::default(),
        )
        .unwrap(),
        edge_colors: Vec::new(),
    };
    let tied_id = builder.append_shape(root, tied_empty, &mut run).unwrap();
    let hole_element =
        b"<path id='hole' d='M50 50 L70 50 L70 70 L50 70 Z M55 55 L65 55 L65 65 L55 65 Z'/>";
    let hole_at = src
        .windows(hole_element.len())
        .position(|w| w == hole_element)
        .unwrap();
    let hole_span = Span {
        start: hole_at,
        end: hole_at + hole_element.len(),
    };
    let hole_path_data = b"M50 50 L70 50 L70 70 L50 70 Z M55 55 L65 55 L65 65 L55 65 Z";
    let hole_path_at = src
        .windows(hole_path_data.len())
        .position(|w| w == hole_path_data)
        .unwrap();
    let hole_path = vexel_svg_path::parse_path(
        src,
        Span {
            start: hole_path_at,
            end: hole_path_at + hole_path_data.len(),
        },
        affine_identity(),
        Default::default(),
    )
    .unwrap();
    let mut hole_shape = Shape::new(SourceMap {
        element: hole_span,
        transform: affine_identity(),
        transform_node: None,
    });
    hole_shape.evenodd = true;
    let hole_id = builder
        .append_shape(
            root,
            ShapeInput {
                shape: hole_shape,
                edge_colors: vec![[200, 20, 10, 255]; hole_path.edges.len()],
                path: hole_path,
            },
            &mut run,
        )
        .unwrap();
    let identity = affine_identity();
    let image_paint = builder
        .append_fill_paint(
            root,
            Paint {
                kind: PaintKind::Image,
                spread: Spread::Pad,
                object_bounding_box: false,
                coordinates: [0.; 6],
                bounds: [0., 0., 1., 1.],
                user_inverse: identity,
                gradient_inverse: identity,
                stops: TableRange::EMPTY,
                pattern: None,
                pattern_inverse: identity,
                pattern_opacity: 1.,
            },
            &mut run,
        )
        .unwrap();
    let image_data = b"M80 80 L90 80 L90 90 L80 90 Z";
    let image_path_at = src
        .windows(image_data.len())
        .position(|w| w == image_data)
        .unwrap();
    let image_path = vexel_svg_path::parse_path(
        src,
        Span {
            start: image_path_at,
            end: image_path_at + image_data.len(),
        },
        identity,
        Default::default(),
    )
    .unwrap();
    let image_element = b"<path id='image' d='M80 80 L90 80 L90 90 L80 90 Z'/>";
    let image_at = src
        .windows(image_element.len())
        .position(|w| w == image_element)
        .unwrap();
    let mut image_shape = Shape::new(SourceMap {
        element: Span {
            start: image_at,
            end: image_at + image_element.len(),
        },
        transform: identity,
        transform_node: None,
    });
    image_shape.fill_paint = Some(image_paint);
    let image_id = builder
        .append_shape(
            root,
            ShapeInput {
                shape: image_shape,
                edge_colors: vec![[0, 0, 0, 0]; image_path.edges.len()],
                path: image_path,
            },
            &mut run,
        )
        .unwrap();
    assert_eq!(line_id.index(), 2);
    assert_eq!(empty_id.index(), 3);
    assert_eq!(tied_id.index(), 4);
    assert_eq!(hole_id.index(), 5);
    assert_eq!(image_id.index(), 6);
    (builder.freeze().unwrap(), clip, stale)
}

#[test]
fn handles_include_zero_edge_records_and_reject_stale_or_foreign_ids() {
    let (graph, _, stale) = fixture();
    let view = graph.root();
    let empty = view.shape_id(3).unwrap();
    assert!(view.shape(stale).is_none());
    assert!(bounds_query(view, stale).is_none());
    let result = bounds_query(view, empty).unwrap();
    assert_eq!(result.user_bounds.width, 0.);
    let outer = bounds_query(view, view.shape_id(0).unwrap()).unwrap();
    assert_eq!(
        outer.user_bounds,
        Rect {
            x: 0.,
            y: 0.,
            width: 20.,
            height: 20.
        }
    );
    assert_eq!(
        outer.device_bounds,
        Rect {
            x: 0.,
            y: 0.,
            width: 20.,
            height: 20.
        }
    );
    assert_eq!(
        element_at_offset(view, result.source.start).unwrap().shape,
        empty
    );
    let inner = view.shapes[1].source.element;
    assert_eq!(
        element_at_offset(view, inner.start).unwrap().shape,
        view.shape_id(4).unwrap()
    );
    assert!(view.shape_id(7).is_none());
    let (other, _, _) = fixture();
    assert!(bounds_query(other.root(), empty).is_none());
}

#[test]
fn reverse_painter_order_clip_fill_stroke_tolerance_and_invalid_inputs() {
    let (graph, clip, _) = fixture();
    let view = graph.root();
    let mut run = Run;
    let top = hit_test(view, 5., 5., 0., &ClipFixture { result: true }, &mut run)
        .unwrap()
        .unwrap();
    assert_eq!(top.shape, view.shape_id(1).unwrap());
    assert_eq!(
        top.source.start,
        view.shape(view.shape_id(1).unwrap())
            .unwrap()
            .source
            .element
            .start
    );
    assert!(
        hit_test(view, 5., 5., 0., &ClipFixture { result: false }, &mut run)
            .unwrap()
            .is_some()
    );
    assert_eq!(
        hit_test(view, 51., 51., 0., &ClipFixture { result: true }, &mut run)
            .unwrap()
            .unwrap()
            .shape,
        view.shape_id(5).unwrap()
    );
    assert!(
        hit_test(view, 60., 60., 0., &ClipFixture { result: true }, &mut run)
            .unwrap()
            .is_none()
    );
    assert_eq!(
        hit_test(view, 85., 85., 0., &ClipFixture { result: true }, &mut run)
            .unwrap()
            .unwrap()
            .shape,
        view.shape_id(6).unwrap()
    );
    assert_eq!(clip.index(), 0);
    let stroke = hit_test(view, 5., 30., 0., &ClipFixture { result: true }, &mut run)
        .unwrap()
        .unwrap();
    assert_eq!(stroke.shape, view.shape_id(2).unwrap());
    assert!(
        hit_test(view, 5., 30., 0., &ClipFixture { result: false }, &mut run)
            .unwrap()
            .is_none()
    );
    assert!(
        hit_test(view, 25., 30., 0., &ClipFixture { result: true }, &mut run)
            .unwrap()
            .is_none()
    );
    assert!(
        hit_test(view, 5., 40., 0., &ClipFixture { result: true }, &mut run)
            .unwrap()
            .is_none()
    );
    assert_eq!(
        hit_test(view, 20., 40., 0., &ClipFixture { result: true }, &mut run)
            .unwrap()
            .unwrap()
            .shape,
        stroke.shape
    );
    assert!(
        hit_test(view, 5., 34., 0., &ClipFixture { result: true }, &mut run)
            .unwrap()
            .is_none()
    );
    assert_eq!(
        hit_test(view, 5., 34., 2., &ClipFixture { result: true }, &mut run)
            .unwrap()
            .unwrap()
            .shape,
        stroke.shape
    );
    assert_eq!(
        hit_test(
            view,
            f64::NAN,
            0.,
            0.,
            &ClipFixture { result: true },
            &mut run
        )
        .unwrap_err()
        .kind,
        ErrorKind::InvalidArgument
    );
    assert_eq!(
        hit_test(
            view,
            0.,
            0.,
            1_000_001.,
            &ClipFixture { result: true },
            &mut run
        )
        .unwrap_err()
        .kind,
        ErrorKind::InvalidArgument
    );
    for (stop, expected) in [
        (Stop::Canceled, ErrorKind::Canceled),
        (Stop::Limit, ErrorKind::Limit(LimitKind::Work)),
        (Stop::Overflow, ErrorKind::Limit(LimitKind::Arithmetic)),
    ] {
        let mut control = Exhaust(stop);
        assert_eq!(
            hit_test(
                view,
                0.,
                0.,
                0.,
                &ClipFixture { result: true },
                &mut control
            )
            .unwrap_err()
            .kind,
            expected
        );
    }
    let mut canceled = Cancel;
    assert_eq!(
        hit_test(
            view,
            0.,
            0.,
            0.,
            &ClipFixture { result: true },
            &mut canceled
        )
        .unwrap_err()
        .kind,
        ErrorKind::Canceled
    );
}

#[test]
fn source_assignment_is_checked_and_happens_before_append() {
    let source = b"<path d='M0 0 L1 0'/>";
    let span = Span {
        start: 0,
        end: source.len(),
    };
    let shape = Shape::new(SourceMap {
        element: Span { start: 0, end: 0 },
        transform: affine_identity(),
        transform_node: None,
    });
    let d = b"M0 0 L1 0";
    let start = source.windows(d.len()).position(|w| w == d).unwrap();
    let path = vexel_svg_path::parse_path(
        source,
        Span {
            start,
            end: start + d.len(),
        },
        affine_identity(),
        Default::default(),
    )
    .unwrap();
    let colors = vec![[1, 2, 3, 255]; path.edges.len()];
    let mut input = ShapeInput {
        shape: shape.clone(),
        path,
        edge_colors: colors,
    };
    source_assign(&mut input, source, span).unwrap();
    assert_eq!(input.shape.source.element, span);
    assert!(
        source_assign(
            &mut input,
            source,
            Span {
                start: usize::MAX,
                end: usize::MAX
            }
        )
        .is_err()
    );
    let owner = Arc::new(source.to_vec());
    let animation = vexel_svg_animation::parse_owned(&owner, &[]).unwrap();
    let viewport = Viewport {
        width: 10,
        height: 10,
        view_x: 0.,
        view_y: 0.,
        view_width: 10.,
        view_height: 10.,
        aspect: Default::default(),
    };
    let mut builder = SceneBuilder::new(owner, animation, 1, viewport, Limits::default()).unwrap();
    let root = builder.root_id();
    let id = builder.append_shape(root, input, &mut Run).unwrap();
    let graph = builder.freeze().unwrap();
    assert_eq!(graph.root().shape(id).unwrap().source.element, span);
}
