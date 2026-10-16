use std::sync::Arc;
use vexel_geometry::{Edge, Point, affine_identity};
use vexel_scene::*;

struct Control(usize);
impl WorkControl for Control {
    fn charge(&mut self, _: WorkKind, units: u64) -> std::result::Result<(), Stop> {
        let n = usize::try_from(units).map_err(|_| Stop::Overflow)?;
        if n > self.0 {
            return Err(Stop::Canceled);
        }
        self.0 -= n;
        Ok(())
    }
    fn checkpoint(&mut self) -> std::result::Result<(), Stop> {
        if self.0 == 0 {
            Err(Stop::Canceled)
        } else {
            Ok(())
        }
    }
}

struct StopOnFillInterval;
impl WorkControl for StopOnFillInterval {
    fn charge(&mut self, kind: WorkKind, _: u64) -> std::result::Result<(), Stop> {
        if kind == WorkKind::FillIntervalTest {
            Err(Stop::Limit)
        } else {
            Ok(())
        }
    }
    fn checkpoint(&mut self) -> std::result::Result<(), Stop> {
        Ok(())
    }
}

fn canonical_scene() -> (SceneGraph, SourceOwner, ShapeId) {
    canonical_scene_with_order(0)
}

fn canonical_scene_with_order(paint_order: u8) -> (SceneGraph, SourceOwner, ShapeId) {
    canonical_scene_with_inverse(paint_order, affine_identity())
}

fn canonical_scene_with_inverse(
    paint_order: u8,
    stroke_inverse: vexel_geometry::Affine,
) -> (SceneGraph, SourceOwner, ShapeId) {
    let source: SourceOwner = Arc::new(
        br#"<svg><path d="M0 0 L8 0 L8 8 Z"/><animate attributeName="opacity" values="0;1" dur="1s"/></svg>"#.to_vec(),
    );
    let mut control = Control(100_000);
    let lexical = LexicalDocument::parse(source.clone(), 1 << 20, &mut control).unwrap();
    let animation = vexel_svg_animation::parse_owned(&source, lexical.tokens()).unwrap();
    let mut builder = SceneBuilder::new(
        source.clone(),
        animation,
        1,
        Viewport {
            width: 16,
            height: 16,
            view_x: 0.0,
            view_y: 0.0,
            view_width: 16.0,
            view_height: 16.0,
            aspect: Default::default(),
        },
        Limits::default(),
    )
    .unwrap();
    let data = source.as_slice();
    let d = data.windows(4).position(|w| w == b"M0 0").unwrap();
    let end = data[d..].iter().position(|b| *b == b'"').unwrap() + d;
    let path = vexel_svg_path::parse_path(
        data,
        Span { start: d, end },
        affine_identity(),
        Default::default(),
    )
    .unwrap();
    let colors = vec![[20, 40, 60, 128]; path.edges.len()];
    let mut shape = Shape::new(SourceMap {
        element: Span {
            start: 5,
            end: end + 2,
        },
        transform: affine_identity(),
        transform_node: None,
    });
    shape.fill_paint = None;
    shape.stroke.rgba = [10, 20, 30, 255];
    shape.stroke.width = 2.0;
    shape.stroke.paint_order = paint_order;
    shape.stroke_inverse = stroke_inverse;
    let scene = builder.root_id();
    let draft = builder.begin_draft(scene).unwrap();
    let shape_id = builder
        .append_shape(
            scene,
            ShapeInput {
                shape,
                path,
                edge_colors: colors,
            },
            &mut control,
        )
        .unwrap();
    builder.commit_draft(&draft, &mut control).unwrap();
    (builder.freeze().unwrap(), source, shape_id)
}

#[test]
fn canonical_scene_uses_builder_path_animation_and_source_identity() {
    let (graph, source, _) = canonical_scene();
    let view = graph.root();
    assert_eq!(view.source_owner().as_slice(), source.as_slice());
    assert_eq!(view.shapes.len(), 1);
    assert!(!view.path_segments.is_empty());
    assert!(!view.animation().tracks.is_empty());
}

#[test]
fn job_scratch_is_same_source_owned_and_capacity_admitted() {
    let (_graph, source, _) = canonical_scene();
    let ledger = ChargeLedger::new(&source, 1 << 20, 128).unwrap();
    let mut job = RasterJob::new(ledger).unwrap();
    let mut control = Control(10_000);
    let scratch = JobRasterScratch::new(&mut job, &mut control).unwrap();
    let handle = job
        .buffer::<Edge>(RasterPool::Scratch, 32, &mut control)
        .unwrap();
    job.extend(
        handle,
        &[Edge {
            a: Point { x: 1.0, y: 2.0 },
            b: Point { x: 3.0, y: 4.0 },
        }],
        &mut control,
    )
    .unwrap();
    assert_eq!(job.slice(handle).unwrap().len(), 1);
    assert!(job.capacity(handle).unwrap() >= 1);
    assert!(job.stats().live_bytes >= source.capacity());
    scratch.validate(&job, &mut control).unwrap();
}

struct FixtureResources;
impl ControlledKernelResources for FixtureResources {
    fn paint_sample(
        &self,
        _: &Paint,
        _: &[PaintStop],
        _: f64,
        _: f64,
        control: &mut dyn WorkControl,
    ) -> Result<[u8; 4]> {
        control
            .charge(WorkKind::PaintVisit, 1)
            .map_err(|stop| Diagnostic {
                kind: match stop {
                    Stop::Canceled => ErrorKind::Canceled,
                    Stop::Limit => ErrorKind::Limit(LimitKind::Work),
                    Stop::Overflow => ErrorKind::Limit(LimitKind::Arithmetic),
                },
                offset: 0,
            })?;
        Ok([80, 100, 120, 255])
    }
    fn clip_contains(
        &self,
        _: SceneView<'_>,
        _: ClipId,
        _: f64,
        _: f64,
        control: &mut dyn WorkControl,
    ) -> Result<bool> {
        control.checkpoint().map_err(|_| Diagnostic {
            kind: ErrorKind::Canceled,
            offset: 0,
        })?;
        Ok(true)
    }
    fn clip_boundary_edges(
        &self,
        _: SceneView<'_>,
        _: ClipId,
        _: &mut dyn FnMut(Edge) -> bool,
        control: &mut dyn WorkControl,
    ) -> Result<bool> {
        control.checkpoint().map_err(|_| Diagnostic {
            kind: ErrorKind::Canceled,
            offset: 0,
        })?;
        Ok(true)
    }
    fn clip_work_counts(
        &self,
        _: SceneView<'_>,
        _: ClipId,
        control: &mut dyn WorkControl,
    ) -> Result<ClipWorkCounts> {
        control.checkpoint().map_err(|_| Diagnostic {
            kind: ErrorKind::Canceled,
            offset: 0,
        })?;
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
        _: JobDashView<'_>,
        _: f64,
        _: f64,
        control: &mut dyn WorkControl,
    ) -> Result<(usize, bool, bool)> {
        control
            .charge(WorkKind::StrokeTest, 1)
            .map_err(|_| Diagnostic {
                kind: ErrorKind::Canceled,
                offset: 0,
            })?;
        Ok((1, false, true))
    }
}

#[test]
fn accepted_job_dash_grid_stroke_and_fill_share_one_composition() {
    let (graph, source, shape) = canonical_scene();
    let view = graph.root();
    let ledger = ChargeLedger::new(&source, 8 << 20, 256).unwrap();
    let mut job = RasterJob::new(ledger).unwrap();
    let mut control = Control(1_000_000);
    let mut scratch = JobRasterScratch::new(&mut job, &mut control).unwrap();
    job.extend(scratch.local_edges, view.edges, &mut control)
        .unwrap();
    let seams = vec![false; view.shapes.len() + view.path_subpaths.len()];
    job.extend(scratch.dash_seams, &seams, &mut control)
        .unwrap();
    let cache = vexel_svg_scalar_sample::build_dash_cache(
        view,
        view.edges,
        &seams,
        &mut job,
        &mut scratch,
        &mut control,
    )
    .unwrap();
    assert_eq!(job.slice(cache.shapes).unwrap().len(), view.shapes.len());
    assert_eq!(
        job.capacity(cache.segments).unwrap() >= job.slice(cache.segments).unwrap().len(),
        true
    );
    let cache_view = job.dash_view(cache, &mut control).unwrap();
    let total = cache_view.shapes[shape.index()].segments.count
        + cache_view.shapes[shape.index()].joins.count;
    let (probes, indexed, _) =
        vexel_svg_scalar_sample::sample_work(view, shape, cache_view, 0.5, 7.5);
    assert!(indexed);
    assert!(probes < total);
    let point = view.edges[0].a;
    let stroke_hit = job
        .compose(&source, cache, &mut control)
        .unwrap()
        .with_buffers(
            scratch.local_edges,
            scratch.dash_seams,
            &mut control,
            |cache, edges, seams, meter| {
                let input = vexel_svg_scalar_sample::RasterSampleInput {
                    resources: &FixtureResources,
                    active_edges: &[],
                    local_edges: edges,
                    dash_seams: seams,
                    dash_cache: cache,
                    px: point.x,
                    py: point.y,
                    paint_x: point.x,
                    paint_y: point.y,
                };
                vexel_svg_scalar_sample::stroke_contains(view, shape, &input, meter)
            },
        )
        .unwrap();
    assert!(stroke_hit);
    job.extend(scratch.boundary_bits, &[0u8; 32], &mut control)
        .unwrap();
    let layout = vexel_svg_scalar_sample::FillRowLayout {
        interval_counts: [1; 5],
        interval_stride: 1,
        max_edge_count: view.shapes[shape.index()].edges.count,
        run_count: 1,
    };
    job.extend(scratch.eligible_shapes, &[true], &mut control)
        .unwrap();
    job.extend(
        scratch.bounds,
        &[ShapeBounds {
            pixels: PixelBounds {
                x_first: 0,
                x_last: 15,
                y_first: 0,
                y_last: 15,
                visible: true,
            },
            ..ShapeBounds::default()
        }],
        &mut control,
    )
    .unwrap();
    let crossings = vec![
        FillCrossing {
            x: 0.0,
            winding_delta: 0,
            edge_index: 0
        };
        layout.max_edge_count
    ];
    job.extend(scratch.crossings, &crossings, &mut control)
        .unwrap();
    let intervals = [
        FillInterval {
            left: 2.125,
            right: 8.0,
        },
        FillInterval {
            left: 2.375,
            right: 8.0,
        },
        FillInterval {
            left: 2.5,
            right: 8.0,
        },
        FillInterval {
            left: 2.625,
            right: 8.0,
        },
        FillInterval {
            left: 2.875,
            right: 8.0,
        },
    ];
    job.extend(scratch.intervals, &intervals, &mut control)
        .unwrap();
    job.extend(
        scratch.pixel_runs,
        &[PixelRun {
            first_x: 0,
            end_x: 16,
            center_only: true,
        }],
        &mut control,
    )
    .unwrap();
    let work_before_fill = job.work();
    vexel_svg_scalar_sample::fill_row(
        view,
        shape,
        2,
        layout,
        &mut job,
        scratch,
        ViewMapping {
            scale_x: 1.0,
            scale_y: 1.0,
            offset_x: 0.0,
            offset_y: 0.0,
        },
        &FixtureResources,
        &mut control,
    )
    .unwrap();
    let center = job.slice(scratch.center_samples).unwrap();
    assert_eq!(center.len(), 16 * 4);
    assert_eq!(&center[6 * 4..6 * 4 + 4], &[20, 40, 60, 128]);
    let fill_work = job.work();
    assert_eq!(
        fill_work.fill_interval_tests - work_before_fill.fill_interval_tests,
        136
    );
    assert!(fill_work.shape_row_visits > work_before_fill.shape_row_visits);
    assert!(fill_work.shape_samples > work_before_fill.shape_samples);
    job.slice_mut(scratch.boundary_bits).unwrap().fill(0xff);
    job.slice_mut(scratch.pixel_runs).unwrap()[0].center_only = false;
    vexel_svg_scalar_sample::fill_row(
        view,
        shape,
        2,
        layout,
        &mut job,
        scratch,
        ViewMapping {
            scale_x: 1.0,
            scale_y: 1.0,
            offset_x: 0.0,
            offset_y: 0.0,
        },
        &FixtureResources,
        &mut control,
    )
    .unwrap();
    let subpixel = job.slice(scratch.subpixel_samples).unwrap();
    assert_eq!(subpixel.len(), 16 * 16 * 4);
    assert_eq!(
        &subpixel[(6 * 16 + 2 * 4 + 1) * 4..(6 * 16 + 2 * 4 + 1) * 4 + 4],
        &[20, 40, 60, 128]
    );
    for sy in 0..4 {
        for sx in 0..4 {
            let at = (6 * 16 + sy * 4 + sx) * 4;
            assert_eq!(&subpixel[at..at + 4], &[20, 40, 60, 128]);
        }
    }
    assert_eq!(
        job.work().fill_interval_tests - fill_work.fill_interval_tests,
        136
    );
    let committed_center = job.slice(scratch.center_samples).unwrap().to_vec();
    let committed_subpixel = job.slice(scratch.subpixel_samples).unwrap().to_vec();
    let malformed = vexel_svg_scalar_sample::FillRowLayout {
        interval_counts: [2; 5],
        ..layout
    };
    assert!(
        vexel_svg_scalar_sample::fill_row(
            view,
            shape,
            2,
            malformed,
            &mut job,
            scratch,
            ViewMapping {
                scale_x: 1.0,
                scale_y: 1.0,
                offset_x: 0.0,
                offset_y: 0.0
            },
            &FixtureResources,
            &mut control,
        )
        .is_err()
    );
    assert_eq!(job.slice(scratch.center_samples).unwrap(), committed_center);
    assert_eq!(
        job.slice(scratch.subpixel_samples).unwrap(),
        committed_subpixel
    );
    let stopped = vexel_svg_scalar_sample::fill_row(
        view,
        shape,
        2,
        layout,
        &mut job,
        scratch,
        ViewMapping {
            scale_x: 1.0,
            scale_y: 1.0,
            offset_x: 0.0,
            offset_y: 0.0,
        },
        &FixtureResources,
        &mut StopOnFillInterval,
    )
    .unwrap_err();
    assert_eq!(stopped.kind, ErrorKind::Limit(LimitKind::Work));
    assert_eq!(
        job.dash_view(cache, &mut control).unwrap().shapes.len(),
        view.shapes.len()
    );
}

#[test]
fn original_stroke_then_fill_paint_order_is_preserved() {
    let (graph, source, shape) = canonical_scene_with_order(1);
    let view = graph.root();
    let mut job = RasterJob::new(ChargeLedger::new(&source, 8 << 20, 256).unwrap()).unwrap();
    let mut control = Control(1_000_000);
    let mut scratch = JobRasterScratch::new(&mut job, &mut control).unwrap();
    job.extend(scratch.local_edges, view.edges, &mut control)
        .unwrap();
    let seams = vec![false; view.shapes.len() + view.path_subpaths.len()];
    job.extend(scratch.dash_seams, &seams, &mut control)
        .unwrap();
    let cache = vexel_svg_scalar_sample::build_dash_cache(
        view,
        view.edges,
        &seams,
        &mut job,
        &mut scratch,
        &mut control,
    )
    .unwrap();
    let sample = job
        .compose(&source, cache, &mut control)
        .unwrap()
        .with_buffers(
            scratch.local_edges,
            scratch.dash_seams,
            &mut control,
            |cache, edges, seams, meter| {
                let input = vexel_svg_scalar_sample::RasterSampleInput {
                    resources: &FixtureResources,
                    active_edges: &[],
                    local_edges: edges,
                    dash_seams: seams,
                    dash_cache: cache,
                    px: 6.5,
                    py: 5.5,
                    paint_x: 6.5,
                    paint_y: 5.5,
                };
                vexel_svg_scalar_sample::paint_sample(
                    view,
                    shape,
                    &input,
                    AlphaMode::SourceOver,
                    meter,
                )
            },
        )
        .unwrap();
    assert_eq!(sample, [15, 30, 45, 255]);
}

#[test]
fn original_non_scaling_stroke_uses_inverse_shape_coordinates() {
    let inverse = vexel_geometry::Affine {
        a: 0.5,
        b: 0.0,
        c: 0.0,
        d: 0.5,
        e: 0.0,
        f: 0.0,
    };
    let (graph, source, shape) = canonical_scene_with_inverse(0, inverse);
    let view = graph.root();
    let mut job = RasterJob::new(ChargeLedger::new(&source, 8 << 20, 256).unwrap()).unwrap();
    let mut control = Control(1_000_000);
    let mut scratch = JobRasterScratch::new(&mut job, &mut control).unwrap();
    job.extend(scratch.local_edges, view.edges, &mut control)
        .unwrap();
    let seams = vec![false; view.shapes.len() + view.path_subpaths.len()];
    job.extend(scratch.dash_seams, &seams, &mut control)
        .unwrap();
    let cache = vexel_svg_scalar_sample::build_dash_cache(
        view,
        view.edges,
        &seams,
        &mut job,
        &mut scratch,
        &mut control,
    )
    .unwrap();
    let (hit, miss) = job
        .compose(&source, cache, &mut control)
        .unwrap()
        .with_buffers(
            scratch.local_edges,
            scratch.dash_seams,
            &mut control,
            |cache, edges, seams, meter| {
                let test = |px, py, cache, meter: &mut dyn WorkControl| {
                    let input = vexel_svg_scalar_sample::RasterSampleInput {
                        resources: &FixtureResources,
                        active_edges: &[],
                        local_edges: edges,
                        dash_seams: seams,
                        dash_cache: cache,
                        px,
                        py,
                        paint_x: px,
                        paint_y: py,
                    };
                    vexel_svg_scalar_sample::stroke_contains(view, shape, &input, meter)
                };
                Ok((
                    test(13.0, 11.0, cache, meter)?,
                    test(13.0, 4.0, cache, meter)?,
                ))
            },
        )
        .unwrap();
    assert!(hit);
    assert!(!miss);
}

#[test]
fn canceled_dash_rebuild_keeps_published_cache_live() {
    let (graph, source, _) = canonical_scene();
    let view = graph.root();
    let mut job = RasterJob::new(ChargeLedger::new(&source, 8 << 20, 256).unwrap()).unwrap();
    let mut control = Control(1_000_000);
    let mut scratch = JobRasterScratch::new(&mut job, &mut control).unwrap();
    let seams = vec![false; view.shapes.len() + view.path_subpaths.len()];
    let cache = vexel_svg_scalar_sample::build_dash_cache(
        view,
        view.edges,
        &seams,
        &mut job,
        &mut scratch,
        &mut control,
    )
    .unwrap();
    let old_segments = job.slice(cache.segments).unwrap().to_vec();
    let before = job.stats().reference_index_bytes;
    let mut canceled = Control(32);
    assert_eq!(
        vexel_svg_scalar_sample::build_dash_cache(
            view,
            view.edges,
            &seams,
            &mut job,
            &mut scratch,
            &mut canceled
        )
        .unwrap_err()
        .kind,
        ErrorKind::Canceled
    );
    assert_eq!(job.slice(cache.segments).unwrap(), old_segments);
    assert_eq!(job.stats().reference_index_bytes, before);
    let replacement = vexel_svg_scalar_sample::build_dash_cache(
        view,
        view.edges,
        &seams,
        &mut job,
        &mut scratch,
        &mut control,
    )
    .unwrap();
    assert!(job.slice(cache.shapes).is_err());
    assert_eq!(
        job.slice(replacement.shapes).unwrap().len(),
        view.shapes.len()
    );
}

#[test]
fn original_large_dash_storage_case_stays_within_cache_and_step_limits() {
    let bytes = b"<svg width='16' height='16' viewBox='0 0 140000 20'><path d='M0 10L140000 10' stroke='red' stroke-dasharray='1 1'/></svg>";
    let source: SourceOwner = Arc::new(bytes.to_vec());
    let mut control = Control(100_000_000);
    let lexical = LexicalDocument::parse(source.clone(), 1 << 20, &mut control).unwrap();
    let animation = vexel_svg_animation::parse_owned(&source, lexical.tokens()).unwrap();
    let mut builder = SceneBuilder::new(
        source.clone(),
        animation,
        1,
        Viewport {
            width: 16,
            height: 16,
            view_x: 0.0,
            view_y: 0.0,
            view_width: 140_000.0,
            view_height: 20.0,
            aspect: Default::default(),
        },
        Limits::default(),
    )
    .unwrap();
    let scene = builder.root_id();
    let draft = builder.begin_draft(scene).unwrap();
    let dash = builder
        .append_dashes(scene, &[1.0, 1.0], &mut control)
        .unwrap();
    let start = bytes
        .windows(7)
        .position(|w| w == b"M0 10L0")
        .unwrap_or_else(|| bytes.windows(5).position(|w| w == b"M0 10").unwrap());
    let end = bytes[start..].iter().position(|b| *b == b'\'').unwrap() + start;
    let path = vexel_svg_path::parse_path(
        bytes,
        Span { start, end },
        affine_identity(),
        Default::default(),
    )
    .unwrap();
    let mut shape = Shape::new(SourceMap {
        element: Span {
            start: 0,
            end: bytes.len(),
        },
        transform: affine_identity(),
        transform_node: None,
    });
    shape.stroke.rgba = [255, 0, 0, 255];
    shape.stroke.width = 1.0;
    shape.stroke.dash_values = dash;
    let colors = vec![[255, 0, 0, 255]; path.edges.len()];
    builder
        .append_shape(
            scene,
            ShapeInput {
                shape,
                path,
                edge_colors: colors,
            },
            &mut control,
        )
        .unwrap();
    builder.commit_draft(&draft, &mut control).unwrap();
    let graph = builder.freeze().unwrap();
    let view = graph.root();
    let ledger = ChargeLedger::new(&source, 64 << 20, 512).unwrap();
    let mut job = RasterJob::new(ledger).unwrap();
    let mut scratch = JobRasterScratch::new(&mut job, &mut control).unwrap();
    let seams = vec![false; view.shapes.len() + view.path_subpaths.len()];
    job.extend(scratch.local_edges, view.edges, &mut control)
        .unwrap();
    job.extend(scratch.dash_seams, &seams, &mut control)
        .unwrap();
    let cache = vexel_svg_scalar_sample::build_dash_cache(
        view,
        view.edges,
        &seams,
        &mut job,
        &mut scratch,
        &mut control,
    )
    .unwrap();
    let segments = job.slice(cache.segments).unwrap();
    assert!(segments.len() > 65_536);
    assert!(cache.build_steps <= 1_000_000);
    let dash_bytes = job.capacity(cache.segments).unwrap() * std::mem::size_of::<DashSegment>()
        + job.capacity(cache.joins).unwrap() * std::mem::size_of::<DashJoin>()
        + job.capacity(cache.shapes).unwrap() * std::mem::size_of::<JobDashRange>();
    let range = job.slice(cache.shapes).unwrap()[0];
    let grid_bytes = job.capacity(range.grid_cells).unwrap() * std::mem::size_of::<GridCell>()
        + job.capacity(range.grid_refs).unwrap() * std::mem::size_of::<u32>();
    assert!(dash_bytes + grid_bytes <= vexel_svg_scalar_sample::DASH_CACHE_BYTE_LIMIT);
}

#[test]
fn canceled_job_growth_keeps_published_contents() {
    let source: SourceOwner = Arc::new(vec![0; 64]);
    let ledger = ChargeLedger::new(&source, 1 << 20, 64).unwrap();
    let mut job = RasterJob::new(ledger).unwrap();
    let mut control = Control(1000);
    let handle = job
        .buffer::<u32>(RasterPool::Scratch, 128, &mut control)
        .unwrap();
    job.extend(handle, &[7, 8], &mut control).unwrap();
    let old_capacity = job.capacity(handle).unwrap();
    let old_history = job.stats().total_allocated_bytes;
    let mut canceled = Control(1);
    assert_eq!(
        job.reserve(handle, 64, &mut canceled).unwrap_err().kind,
        ErrorKind::Canceled
    );
    assert_eq!(job.slice(handle).unwrap(), &[7, 8]);
    assert_eq!(job.capacity(handle).unwrap(), old_capacity);
    assert!(job.stats().total_allocated_bytes > old_history);
}
