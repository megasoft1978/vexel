//! Headless tests for playback (AC5), export (AC6) and edit history (AC7).
//! No window, no wall clock.

#[path = "../../../vexel-gif/tests/support/mod.rs"]
mod gif_support;

use super::export::{ExportError, ExportRequest, export_gif, frame_count};
use super::frame::{AnimatedScene, AnimationError, decode, over_white};
use super::playback::{FrameQueue, ManualClock, Playback};
use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use vexel_gif::MAX_QUANT_ERROR;
use vexel_mcp_access::{AccessError, AccessPolicy};
use vexel_scene::EvaluatedFrame;

const LOOP: &[u8] = include_bytes!("../../../../fixtures/v1/anim/loop.svg");
const NEVER: AtomicBool = AtomicBool::new(false);

fn scene() -> AnimatedScene {
    decode(LOOP).expect("decode fixture")
}

/// Leftmost edge x of shape `index` and its first edge colour alpha.
fn probe(frame: &EvaluatedFrame<'_>, index: usize) -> (f64, u8) {
    let view = frame.view();
    let shape = &view.shapes[index];
    let edges = shape.edges.slice(view.edges).unwrap();
    let min_x = edges
        .iter()
        .flat_map(|e| [e.a.x, e.b.x])
        .fold(f64::INFINITY, f64::min);
    (min_x, view.edge_colors[shape.edges.first][3])
}

fn at(scene: &AnimatedScene, t: f64) -> ((f64, u8), (f64, u8)) {
    let frame = scene.evaluate(t, &NEVER).unwrap();
    (probe(&frame, 0), probe(&frame, 1))
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}

// --- AC5 -------------------------------------------------------------------

#[test]
fn evaluate_frame_values_at_start_mid_end_and_wrap() {
    let s = scene();
    assert!(near(s.loop_seconds(), 1.0));
    // box x: 4 -> 40 -> 4 ; dot opacity 1 -> 0.25 -> 1 (alpha 255 -> 64 -> 255).
    let ((x0, _), (_, a0)) = at(&s, 0.0);
    assert!(near(x0, 4.0) && a0 == 255, "t=0: {x0} {a0}");
    let ((xq, _), (_, aq)) = at(&s, 0.25);
    assert!(
        near(xq, 22.0) && (159..=161).contains(&aq),
        "t=.25: {xq} {aq}"
    );
    let ((xm, _), (_, am)) = at(&s, 0.5);
    assert!(near(xm, 40.0) && (63..=65).contains(&am), "t=.5: {xm} {am}");
    let ((xe, _), (_, ae)) = at(&s, 0.999);
    assert!((xe - 4.0).abs() < 0.1 && ae >= 254, "t~end: {xe} {ae}");
    // The loop boundary repeats frame 0 (repeatCount indefinite).
    let ((xw, _), (_, aw)) = at(&s, 1.0);
    assert!(near(xw, x0) && aw == a0, "t=1.0 wraps to t=0: {xw} {aw}");
    let ((xw2, _), _) = at(&s, 1.5);
    assert!(near(xw2, 40.0));
}

#[test]
fn playback_follows_injected_clock_and_wraps_at_the_loop_boundary() {
    let s = scene();
    let clock = ManualClock::default();
    let mut p = Playback::new(clock, s.loop_seconds());
    assert!(!p.is_playing() && p.time() == 0.0);
    assert!(p.play() && p.is_playing());
    p.clock().advance(0.5);
    assert!(near(p.time(), 0.5));
    assert!(near(at(&s, p.time()).0.0, 40.0));
    p.clock().advance(0.5); // exactly one loop: wraps to 0
    assert!(near(p.time(), 0.0));
    assert!(near(at(&s, p.time()).0.0, 4.0));
    p.clock().advance(0.25 + 3.0); // three more loops and a quarter
    assert!(near(p.time(), 0.25));
}

#[test]
fn pause_scrub_and_resume_without_a_window() {
    let s = scene();
    let mut p = Playback::new(ManualClock::default(), s.loop_seconds());
    p.play();
    p.clock().advance(0.3);
    p.pause();
    assert!(!p.is_playing() && near(p.time(), 0.3));
    p.clock().advance(5.0); // paused: time does not move
    assert!(near(p.time(), 0.3));
    assert!(p.seek(0.5) && near(p.time(), 0.5) && !p.is_playing());
    assert!(near(at(&s, p.time()).0.0, 40.0));
    assert!(
        p.seek(99.0) && near(p.time(), 1.0),
        "clamped to the loop end"
    );
    assert!(
        !p.seek(f64::NAN) && near(p.time(), 1.0),
        "invalid seek changes nothing"
    );
    assert!(p.seek(0.75));
    p.play();
    p.clock().advance(0.5); // 0.75 + 0.5 wraps to 0.25
    assert!(p.is_playing() && near(p.time(), 0.25));
    assert!(
        p.seek(0.5) && p.is_playing(),
        "scrub while playing keeps playing"
    );
    p.clock().advance(0.25);
    assert!(near(p.time(), 0.75));
}

#[test]
fn nothing_to_play_without_a_loop() {
    let mut p = Playback::new(ManualClock::default(), 0.0);
    assert!(!p.play() && !p.is_playing() && p.time() == 0.0);
    assert!(p.seek(3.0) && p.time() == 0.0);
}

#[test]
fn frame_queue_is_bounded_and_drops_oldest() {
    let mut q = FrameQueue::new(3);
    assert_eq!((q.push(1), q.push(2), q.push(3)), (None, None, None));
    assert_eq!(q.push(4), Some(1));
    assert_eq!(q.len(), 3);
    assert_eq!(
        (q.pop(), q.pop(), q.pop(), q.pop()),
        (Some(2), Some(3), Some(4), None)
    );
    assert_eq!(
        FrameQueue::<u8>::new(0).push(1),
        None,
        "capacity is at least one"
    );
}

#[test]
fn rendered_frames_differ_over_time_and_loop_back() {
    let s = scene();
    let f = |t: f64| over_white(&s.render(t, &NEVER).unwrap());
    let (a, b, c) = (f(0.0), f(0.5), f(1.0));
    assert_eq!(a.len(), 64 * 48 * 4);
    assert_ne!(a, b);
    assert_eq!(a, c, "loop boundary renders the first frame again");
    // box left at x=4..20, y=8..24 at t=0; moved to 40..56 at t=0.5.
    let px = |v: &[u8], x: usize, y: usize| v[(y * 64 + x) * 4..(y * 64 + x) * 4 + 3].to_vec();
    assert_eq!(px(&a, 10, 16), [0xe0, 0x30, 0x30]);
    assert_eq!(px(&a, 48, 16), [255, 255, 255]);
    assert_eq!(px(&b, 48, 16), [0xe0, 0x30, 0x30]);
    assert_eq!(px(&b, 10, 16), [255, 255, 255]);
}

#[test]
fn unsupported_or_invalid_inputs_are_typed_not_approximated() {
    let s = scene();
    assert!(matches!(
        s.evaluate(f64::NAN, &NEVER),
        Err(AnimationError::Evaluate(_))
    ));
    assert!(matches!(
        s.evaluate(-1.0, &NEVER),
        Err(AnimationError::Evaluate(_))
    ));
    let stop = AtomicBool::new(true);
    assert_eq!(s.render(0.0, &stop).unwrap_err(), AnimationError::Cancelled);
    let dashed = br#"<svg xmlns="http://www.w3.org/2000/svg" width="8" height="8"><rect x="1" y="1" width="4" height="4" fill="none" stroke="red" stroke-dasharray="2 1"><animate attributeName="x" values="1;2" dur="1s"/></rect></svg>"#;
    let scene = decode(dashed).unwrap();
    assert!(matches!(
        scene.render(0.0, &NEVER),
        Err(AnimationError::Render(_))
    ));
    assert!(decode(b"<svg").is_err());
}

// --- AC6 -------------------------------------------------------------------

static DIRS: AtomicUsize = AtomicUsize::new(0);

fn grant() -> (PathBuf, AccessPolicy) {
    let dir = std::env::temp_dir().join(format!(
        "vexel-anim-{}-{}",
        std::process::id(),
        DIRS.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let dir = std::fs::canonicalize(dir).unwrap();
    let policy = AccessPolicy::new(Some(OsString::from(&dir)), None);
    (dir, policy)
}

/// Copies an exported GIF to `VEXEL_ANIM_EVIDENCE_DIR` when set (for opening
/// it in an independent viewer); a no-op otherwise.
fn keep_evidence(file: &std::path::Path, name: &str) {
    if let Some(dir) = std::env::var_os("VEXEL_ANIM_EVIDENCE_DIR") {
        std::fs::copy(file, PathBuf::from(dir).join(name)).unwrap();
    }
}

fn request<'a>(path: &'a std::ffi::OsStr, fps: u32, duration: f64) -> ExportRequest<'a> {
    ExportRequest {
        path,
        fps,
        duration,
        overwrite: false,
    }
}

fn names(dir: &PathBuf) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    v.sort();
    v
}

#[test]
fn export_writes_gif_with_duration_times_fps_frames_matching_rendered_frames() {
    let (dir, policy) = grant();
    let out = dir.join("loop.gif");
    let mut seen = Vec::new();
    let summary = export_gif(
        LOOP,
        &policy,
        &request(out.as_os_str(), 10, 1.0),
        &NEVER,
        &mut |done, total| seen.push((done, total)),
    )
    .unwrap();
    assert_eq!(
        (
            summary.frames,
            summary.delay_cs,
            summary.width,
            summary.height
        ),
        (10, 10, 64, 48)
    );
    assert_eq!(seen.first(), Some(&(1, 10)));
    assert_eq!(seen.last(), Some(&(10, 10)));
    let bytes = std::fs::read(&out).unwrap();
    keep_evidence(&out, "loop.gif");
    assert_eq!(bytes.len(), summary.bytes);
    assert_eq!(names(&dir), ["loop.gif"], "no temporary file left behind");
    let d = gif_support::decode(&bytes).unwrap();
    assert_eq!((d.width, d.height), (64, 48));
    assert_eq!(d.frames.len(), 10);
    assert!(
        d.frames
            .iter()
            .all(|f| f.delay_cs == 10 && f.gce_count == 1)
    );
    assert_eq!(d.loop_count, Some(0));
    let s = scene();
    for (index, time) in [(0usize, 0.0), (9, 0.9)] {
        let want = over_white(&s.render(time, &NEVER).unwrap());
        let got = d.rgba(index);
        let worst = want
            .iter()
            .zip(&got)
            .map(|(a, b)| a.abs_diff(*b))
            .max()
            .unwrap();
        assert!(worst <= MAX_QUANT_ERROR, "frame {index}: {worst}");
    }
    assert_ne!(d.rgba(0), d.rgba(5), "the exported frames animate");
}

#[test]
fn frame_count_follows_ceil_duration_times_fps() {
    assert_eq!(frame_count(1.0, 10), 10);
    assert_eq!(frame_count(0.3, 10), 3);
    assert_eq!(frame_count(0.25, 10), 3);
    assert_eq!(frame_count(0.01, 1), 1);
    assert_eq!(frame_count(1.0, 30), 30);
    let (dir, policy) = grant();
    let out = dir.join("half.gif");
    let summary = export_gif(
        LOOP,
        &policy,
        &request(out.as_os_str(), 30, 0.5),
        &NEVER,
        &mut |_, _| {},
    )
    .unwrap();
    assert_eq!((summary.frames, summary.delay_cs), (15, 3));
    let d = gif_support::decode(&std::fs::read(&out).unwrap()).unwrap();
    assert_eq!((d.frames.len(), d.frames[0].delay_cs), (15, 3));
}

#[test]
fn path_outside_grant_is_denied_and_creates_nothing() {
    let (dir, policy) = grant();
    let (other, _) = grant();
    let target = other.join("escape.gif");
    let err = export_gif(
        LOOP,
        &policy,
        &request(target.as_os_str(), 10, 1.0),
        &NEVER,
        &mut |_, _| {},
    )
    .unwrap_err();
    assert_eq!(err, ExportError::Access(AccessError::NotGranted));
    assert!(names(&other).is_empty() && names(&dir).is_empty());
    let missing = dir.join("no-such-dir").join("x.gif");
    let err = export_gif(
        LOOP,
        &policy,
        &request(missing.as_os_str(), 10, 1.0),
        &NEVER,
        &mut |_, _| {},
    )
    .unwrap_err();
    assert_eq!(err, ExportError::Access(AccessError::NotFound));
}

#[test]
fn existing_file_is_kept_without_overwrite_and_replaced_with_it() {
    let (dir, policy) = grant();
    let out = dir.join("keep.gif");
    std::fs::write(&out, b"precious").unwrap();
    let err = export_gif(
        LOOP,
        &policy,
        &request(out.as_os_str(), 10, 1.0),
        &NEVER,
        &mut |_, _| {},
    )
    .unwrap_err();
    assert_eq!(err, ExportError::Exists);
    assert_eq!(std::fs::read(&out).unwrap(), b"precious");
    assert_eq!(names(&dir), ["keep.gif"]);
    let mut req = request(out.as_os_str(), 10, 1.0);
    req.overwrite = true;
    export_gif(LOOP, &policy, &req, &NEVER, &mut |_, _| {}).unwrap();
    assert_eq!(&std::fs::read(&out).unwrap()[..6], b"GIF89a");
    assert_eq!(names(&dir), ["keep.gif"]);
}

#[test]
fn cancellation_before_and_during_rendering_leaves_no_file() {
    let (dir, policy) = grant();
    let out = dir.join("cancel.gif");
    let stop = AtomicBool::new(true);
    let err = export_gif(
        LOOP,
        &policy,
        &request(out.as_os_str(), 10, 1.0),
        &stop,
        &mut |_, _| {},
    )
    .unwrap_err();
    assert_eq!(err, ExportError::Cancelled);
    let stop = AtomicBool::new(false);
    let err = export_gif(
        LOOP,
        &policy,
        &request(out.as_os_str(), 10, 1.0),
        &stop,
        &mut |done, _| {
            if done == 4 {
                stop.store(true, Ordering::Relaxed);
            }
        },
    )
    .unwrap_err();
    assert_eq!(err, ExportError::Cancelled);
    assert!(names(&dir).is_empty() && !out.exists());
}

#[test]
fn invalid_requests_are_typed_before_any_file_or_render() {
    let (dir, policy) = grant();
    let out = dir.join("bad.gif");
    let run = |fps: u32, duration: f64| {
        let req = request(out.as_os_str(), fps, duration);
        export_gif(LOOP, &policy, &req, &NEVER, &mut |_, _| {}).unwrap_err()
    };
    assert_eq!(run(0, 1.0), ExportError::InvalidFps);
    assert_eq!(run(31, 1.0), ExportError::InvalidFps);
    assert_eq!(run(10, 0.0), ExportError::InvalidDuration);
    assert_eq!(run(10, f64::NAN), ExportError::InvalidDuration);
    assert_eq!(run(10, 3601.0), ExportError::InvalidDuration);
    assert_eq!(run(10, 1.5), ExportError::DurationExceedsLoop);
    assert_eq!(
        run(30, 5.0),
        ExportError::Encode(vexel_gif::GifError::TooManyFrames)
    );
    let still = br#"<svg xmlns="http://www.w3.org/2000/svg" width="8" height="8"><rect width="4" height="4"/></svg>"#;
    let err = export_gif(
        still,
        &policy,
        &request(out.as_os_str(), 10, 1.0),
        &NEVER,
        &mut |_, _| {},
    )
    .unwrap_err();
    assert_eq!(err, ExportError::NotAnimated);
    assert!(names(&dir).is_empty());
}

// --- AC7 -------------------------------------------------------------------

#[test]
fn animation_fragment_undo_redo_restores_exact_source_bytes() {
    use vexel_v1_document::V1Document;
    let mut doc = V1Document::new(64, 48).unwrap();
    let base = doc.source().unwrap();
    let id = "box";
    doc.add_rectangle_with_id(4.0, 8.0, 20.0, 24.0, "#e03030", id)
        .unwrap();
    let with_shape = doc.source().unwrap();
    let fragment =
        br#"<animate attributeName="x" values="4;40;4" dur="1s" repeatCount="indefinite"/>"#;
    doc.add_animation_fragment(id, fragment).unwrap();
    let animated = doc.source().unwrap();
    assert_ne!(animated, with_shape);
    doc.undo().unwrap();
    assert_eq!(doc.source().unwrap(), with_shape);
    doc.redo().unwrap();
    assert_eq!(
        doc.source().unwrap(),
        animated,
        "redo restores the exact bytes"
    );
    doc.undo().unwrap();
    doc.undo().unwrap();
    assert_eq!(doc.source().unwrap(), base);
    // The animated source is playable through the real pipeline.
    doc.redo().unwrap();
    doc.redo().unwrap();
    let scene = decode(&doc.source().unwrap()).unwrap();
    assert!(near(scene.loop_seconds(), 1.0));
    assert!(near(
        probe(&scene.evaluate(0.5, &NEVER).unwrap(), 0).0,
        40.0
    ));
}

// --- app wiring ------------------------------------------------------------

fn wait(job: &mut super::job::ExportJob) -> String {
    for _ in 0..4000 {
        if let Some(status) = job.poll() {
            return status;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    panic!("export job did not finish");
}

#[test]
fn background_job_replies_once_with_json_or_a_fixed_error() {
    use super::job::ExportJob;
    use std::sync::mpsc::sync_channel;
    let (dir, policy) = grant();
    let out = dir.join("job.gif");
    let (tx, rx) = sync_channel(1);
    let mut job = ExportJob::spawn(
        LOOP.to_vec(),
        policy.clone(),
        out.clone().into(),
        10,
        1.0,
        tx,
    )
    .unwrap();
    assert_eq!(wait(&mut job), "Exported 10 GIF frames");
    let reply = rx.recv().unwrap().unwrap();
    assert!(
        reply.contains("\"frames\":10") && reply.contains("\"delayCs\":10"),
        "{reply}"
    );
    assert!(
        !reply.contains(dir.to_str().unwrap()),
        "no path in the reply"
    );
    assert_eq!(job.poll(), None, "reply is sent exactly once");
    // Existing file without overwrite: fixed message, file untouched.
    let before = std::fs::read(&out).unwrap();
    let (tx, rx) = sync_channel(1);
    let mut job = ExportJob::spawn(
        LOOP.to_vec(),
        policy.clone(),
        out.clone().into(),
        10,
        1.0,
        tx,
    )
    .unwrap();
    wait(&mut job);
    assert_eq!(
        rx.recv().unwrap().unwrap_err(),
        "export file already exists"
    );
    assert_eq!(std::fs::read(&out).unwrap(), before);
    // Outside the grant.
    let (other, _) = grant();
    let (tx, rx) = sync_channel(1);
    let mut job = ExportJob::spawn(
        LOOP.to_vec(),
        policy,
        other.join("x.gif").into(),
        10,
        1.0,
        tx,
    )
    .unwrap();
    wait(&mut job);
    assert_eq!(
        rx.recv().unwrap().unwrap_err(),
        "export path is not in a granted directory"
    );
    assert!(names(&other).is_empty());
}

#[test]
fn cancelled_job_reports_cancellation_and_writes_nothing_partial() {
    use super::job::ExportJob;
    use std::sync::mpsc::sync_channel;
    let (dir, policy) = grant();
    let (tx, rx) = sync_channel(1);
    let mut job =
        ExportJob::spawn(LOOP.to_vec(), policy, dir.join("c.gif").into(), 30, 1.0, tx).unwrap();
    job.cancel();
    wait(&mut job);
    // Cancelling right after spawn can race a very fast finish; either way the
    // directory holds nothing partial.
    match rx.recv().unwrap() {
        Err(message) => {
            assert_eq!(message, "export cancelled");
            assert!(names(&dir).is_empty());
        }
        Ok(_) => assert_eq!(names(&dir), ["c.gif"]),
    }
}

#[test]
fn preview_document_with_agent_added_animation_exports_a_gif() {
    use crate::preview::{Operation, PreviewState, Shape};
    let state = PreviewState::new().unwrap();
    let (state, _) = state
        .stage(&Operation::Add(Shape::Rect {
            x: 8.0,
            y: 8.0,
            width: 40.0,
            height: 40.0,
            rgb: 0xE03030,
        }))
        .unwrap();
    let target = state.element_ids()[0].clone();
    let xml = br#"<animate attributeName="x" values="8;200;8" dur="1s"/>"#.to_vec();
    let (state, _) = state
        .stage(&Operation::AnimationAdd {
            target_id: target,
            xml,
        })
        .unwrap();
    let (dir, policy) = grant();
    let out = dir.join("pip.gif");
    let summary = export_gif(
        &state.source().unwrap(),
        &policy,
        &request(out.as_os_str(), 5, 1.0),
        &NEVER,
        &mut |_, _| {},
    )
    .unwrap();
    assert_eq!(
        (summary.frames, summary.width, summary.height),
        (5, 512, 512)
    );
    keep_evidence(&out, "pip.gif");
    let d = gif_support::decode(&std::fs::read(&out).unwrap()).unwrap();
    assert_eq!(d.frames.len(), 5);
    assert_ne!(d.rgba(0), d.rgba(2), "the rectangle moves between frames");
    // The window raster path agrees with the export renderer at the same time.
    let shown = super::present::raster_at(&state.source().unwrap(), 0.4).unwrap();
    let rendered = decode(&state.source().unwrap())
        .unwrap()
        .render(0.4, &NEVER)
        .unwrap();
    assert_eq!(shown, over_white(&rendered));
}
