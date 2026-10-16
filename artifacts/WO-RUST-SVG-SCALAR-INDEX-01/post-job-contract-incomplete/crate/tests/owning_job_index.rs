use std::sync::Arc;
use vexel_scene::*;
use vexel_svg_scalar_index::{build_bounds_job, create_scanlines_job, mark_boundaries_job};

struct Run;
impl WorkControl for Run {
    fn charge(&mut self, _: WorkKind, _: u64) -> std::result::Result<(), Stop> {
        Ok(())
    }
    fn checkpoint(&mut self) -> std::result::Result<(), Stop> {
        Ok(())
    }
}

struct CancelOnCopy;
impl WorkControl for CancelOnCopy {
    fn charge(&mut self, kind: WorkKind, _: u64) -> std::result::Result<(), Stop> {
        if kind == WorkKind::SourceRead {
            Err(Stop::Canceled)
        } else {
            Ok(())
        }
    }
    fn checkpoint(&mut self) -> std::result::Result<(), Stop> {
        Ok(())
    }
}

fn fixture() -> SceneGraph {
    let source = Arc::new(Vec::from(&b"M1 1 L6 1 L6 6 Z"[..]));
    // The accepted animation parser accepts an empty token stream. SVG token
    // creation stays in the decoder scope; this fixture exercises real scene,
    // path, source ownership and animation-owner identity.
    let animation = vexel_svg_animation::parse_owned(&source, &[]).unwrap();
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
    let span = Span {
        start: 0,
        end: source.len(),
    };
    let path = vexel_svg_path::parse_path(
        &source,
        span,
        vexel_geometry::affine_identity(),
        Default::default(),
    )
    .unwrap();
    let edge_colors = (0..path.edges.len()).map(|_| [20, 40, 60, 255]).collect();
    let shape = Shape::new(SourceMap {
        element: span,
        transform: vexel_geometry::affine_identity(),
        transform_node: None,
    });
    let mut run = Run;
    builder
        .append_shape(
            builder.root_id(),
            ShapeInput {
                shape,
                path,
                edge_colors,
            },
            &mut run,
        )
        .unwrap();
    builder.freeze().unwrap()
}

#[test]
fn canonical_scene_path_uses_same_job_for_bounds_boundary_and_rows() {
    let graph = fixture();
    let view = graph.root();
    assert_eq!(view.shapes.len(), 1);
    assert!(view.edges.len() >= 3);
    let ledger = ChargeLedger::new(graph.source_owner(), 4 * 1024 * 1024, 128).unwrap();
    let mut job = RasterJob::new(ledger).unwrap();
    let mut run = Run;
    let scratch = JobRasterScratch::new(&mut job, &mut run).unwrap();
    let map = ViewMapping {
        scale_x: 1.0,
        scale_y: 1.0,
        offset_x: 0.0,
        offset_y: 0.0,
    };
    let pixel_count = build_bounds_job(view, map, &mut job, &scratch, &mut run).unwrap();
    let bounds = job.slice(scratch.bounds).unwrap();
    assert_eq!(bounds.len(), 1);
    assert!(bounds[0].pixels.visible);
    assert!(pixel_count > 0);
    let old = bounds[0];
    let old_capacity = job.capacity(scratch.bounds).unwrap();
    job.reserve(scratch.bounds, old_capacity + 1, &mut run)
        .unwrap();
    assert_eq!(job.slice(scratch.bounds).unwrap(), &[old]);
    let capacity = job.capacity(scratch.bounds).unwrap();
    let before_failed_growth = job.stats();
    assert!(job.reserve(scratch.bounds, usize::MAX, &mut run).is_err());
    assert_eq!(job.capacity(scratch.bounds).unwrap(), capacity);
    assert_eq!(job.slice(scratch.bounds).unwrap(), &[old]);
    assert_eq!(job.stats(), before_failed_growth);
    let marked_work = mark_boundaries_job(view, map, &mut job, &scratch, &mut run).unwrap();
    assert!(marked_work > 0);
    assert_eq!(
        job.boundary_work(scratch.boundary_bits).unwrap(),
        marked_work
    );
    let state =
        create_scanlines_job(view, scratch.bounds, map, &mut job, &scratch, &mut run).unwrap();
    assert_eq!(state.shape_count, 1);
    assert_eq!(state.edge_count, view.edges.len());
    assert_eq!(state.scanline_count, 40);
    assert!(!job.slice(scratch.scanline_events).unwrap().is_empty());
    assert!(job.stats().live_bytes > job.stats().source_bytes);
    assert!(job.stats().total_allocated_bytes > 0);
    scratch.release(&mut job).unwrap();
    assert_eq!(job.finish().unwrap().live_bytes, 0);
}

#[test]
fn canceled_job_boundary_initialization_publishes_no_partial_mask() {
    let graph = fixture();
    let view = graph.root();
    let ledger = ChargeLedger::new(graph.source_owner(), 4 * 1024 * 1024, 128).unwrap();
    let mut job = RasterJob::new(ledger).unwrap();
    let mut run = Run;
    let scratch = JobRasterScratch::new(&mut job, &mut run).unwrap();
    let map = ViewMapping {
        scale_x: 1.0,
        scale_y: 1.0,
        offset_x: 0.0,
        offset_y: 0.0,
    };
    let error = mark_boundaries_job(view, map, &mut job, &scratch, &mut CancelOnCopy).unwrap_err();
    assert_eq!(error.kind, ErrorKind::Canceled);
    assert!(job.slice(scratch.boundary_bits).unwrap().is_empty());
    assert!(job.capacity(scratch.boundary_bits).unwrap() > 0);
    assert!(job.stats().total_allocated_bytes > 0);
}
