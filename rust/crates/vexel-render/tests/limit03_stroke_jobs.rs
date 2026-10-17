//! WO-RUST-LIMITS-03 stroke-job pins. Each subpath is an open pair of edges, of which
//! one (128 units long) is stroked with dash [64, 64]; the range costs exactly 5 build
//! steps (2 for the pattern, 1 for the edge, 2 for the two dashes; measured in
//! dash/tests.rs). So 800,000 subpaths (4,000,000 steps) is the largest job accepted at
//! MAX_STROKE_STEPS = 4,000,000 and 800,001 subpaths (4,000,005 steps) is refused.
//! The 128-unit user space is mapped onto a 1x1 tile so the raster sweep stays cheap.
mod common;
use common::*;
use std::ops::Range;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use vexel_render::{
    DashedShape, FlatEdge, Linecap, Linejoin, RenderError, ViewBox, ViewMap, render_dashed_shapes,
};

const LARGEST_ACCEPTED_RANGES: usize = 800_000;

struct Job {
    edges: Vec<FlatEdge>,
    ranges: Vec<Range<usize>>,
    closed: Vec<bool>,
}

/// Each range is two edges: the stroked edge and the implicit closing edge, which
/// `StrokeRange::stroked_count` drops for an open subpath.
fn job(ranges: usize) -> Job {
    let mut edges = Vec::with_capacity(2 * ranges);
    for k in 0..ranges {
        let y = (k % 128) as f64;
        edges.push(edge(0.0, y, 128.0, y));
        edges.push(edge(128.0, y, 0.0, y));
    }
    Job {
        edges,
        ranges: (0..ranges).map(|k| 2 * k..2 * k + 2).collect(),
        closed: vec![false; ranges],
    }
}

fn render(job: &Job, control: &dyn Fn() -> bool) -> Result<(), RenderError> {
    let stroke = style([0, 0, 0, 255], 1.0, Linecap::Butt, Linejoin::Miter, 4.0);
    let shape = with_subpaths(stroke_shape(&job.edges, false, stroke), &job.ranges, &job.closed);
    let shapes = [DashedShape { shape, dashes: &[64.0, 64.0] }];
    let vb = ViewBox { x: 0.0, y: 0.0, width: 128.0, height: 128.0 };
    let view = ViewMap::from_viewbox(vb, 1, 1, None).unwrap();
    let mut tile = empty_tile(1, 1);
    render_dashed_shapes(&shapes, view, &mut tile, control)
}

#[test]
fn limit03_dash_job_admits_800000_ranges_and_refuses_800001() {
    assert!(render(&job(LARGEST_ACCEPTED_RANGES), NEVER).is_ok());
    assert_eq!(
        render(&job(LARGEST_ACCEPTED_RANGES + 1), NEVER),
        Err(RenderError::LimitExceeded)
    );
}

#[test]
fn limit03_cancel_during_largest_accepted_dash_job_is_prompt() {
    let job = job(LARGEST_ACCEPTED_RANGES);
    let flag = AtomicBool::new(false);
    let (result, raised, returned) = std::thread::scope(|scope| {
        let flag_ref = &flag;
        let raiser = scope.spawn(move || {
            std::thread::sleep(Duration::from_millis(50));
            let raised = Instant::now();
            flag_ref.store(true, Ordering::Release);
            raised
        });
        let control = || flag.load(Ordering::Acquire);
        let result = render(&job, &control);
        (result, raiser.join().unwrap(), Instant::now())
    });
    assert_eq!(result, Err(RenderError::Cancelled), "job finished before the cancel");
    let latency = returned.duration_since(raised);
    eprintln!("MEASURE limit03_cancel_latency_us={}", latency.as_micros());
    // Release (the shipped profile) is held to 250 ms. The debug profile builds the
    // stroke primitives several times slower and `control` is polled only per shape and
    // per pixel row, so it gets 1 s; the release number is the evidence (result.md).
    let bound = if cfg!(debug_assertions) { Duration::from_millis(1000) } else { Duration::from_millis(250) };
    assert!(latency <= bound, "cancel latency {latency:?} over {bound:?}");
}
