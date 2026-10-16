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

fn canonical_scene() -> (SceneGraph, SourceOwner) {
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
    let colors = vec![[20, 40, 60, 255]; path.edges.len()];
    let shape = Shape::new(SourceMap {
        element: Span {
            start: 5,
            end: end + 2,
        },
        transform: affine_identity(),
        transform_node: None,
    });
    let scene = builder.root_id();
    let draft = builder.begin_draft(scene).unwrap();
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
    (builder.freeze().unwrap(), source)
}

#[test]
fn canonical_scene_uses_builder_path_animation_and_source_identity() {
    let (graph, source) = canonical_scene();
    let view = graph.root();
    assert_eq!(view.source_owner().as_slice(), source.as_slice());
    assert_eq!(view.shapes.len(), 1);
    assert!(!view.path_segments.is_empty());
    assert!(!view.animation().tracks.is_empty());
}

#[test]
fn job_scratch_is_same_source_owned_and_capacity_admitted() {
    let (_graph, source) = canonical_scene();
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
