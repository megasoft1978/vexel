//! Parallel exports must be byte-identical to the serial (one worker) export:
//! GIF bytes, PNG frame bytes and order, and decoded MP4 frames.

use super::NativeExportSource;
use super::ordered::FORCED_WORKERS;
use crate::animation::native_export_frames::NativeFramesRequest;
use crate::animation::native_export_video::NativeVideoRequest;
use crate::project_session::ProjectTimelineCandidate;
use std::sync::atomic::Ordering;
use vexel_platform_apple::VideoCodec;
use vexel_timeline::{EditLimits, EmptyTimelineSettings, LoopMode};

const TICKS: u32 = 24;
const SCENE: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 48 48"><defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#ff3366"/><stop offset="1" stop-color="#2255ff"/></linearGradient></defs><circle id="dot" cx="24" cy="24" r="14" fill="url(#g)" stroke="#102030" stroke-width="3"/><rect id="bar" x="4" y="4" width="20" height="6" fill="#33aa55"/></svg>"##;

fn source() -> NativeExportSource {
    let timeline = format!(
        r#"{{"schema":"vexel.animation","version":1,"ticks_per_second":24,"duration_ticks":{TICKS},"fps":{{"numerator":24,"denominator":1}},"loop":"ping_pong","onion":{{"before":0,"after":0,"opacity":0}},"tracks":[{{"id":"fade","kind":"property","visible":true,"target":"dot","property":"opacity","value_type":"scalar","interpolation":"linear","keys":[{{"tick":0,"value":0.2}},{{"tick":23,"value":1}}]}},{{"id":"paint","kind":"property","visible":true,"target":"bar","property":"fill","value_type":"rgba","interpolation":"linear","keys":[{{"tick":0,"value":[1,0,0,1]}},{{"tick":23,"value":[0,0,1,1]}}]}}]}}"#
    );
    let settings = EmptyTimelineSettings {
        ticks_per_second: 24,
        duration_ticks: TICKS,
        fps_numerator: 24,
        fps_denominator: 1,
        loop_mode: LoopMode::PingPong,
        onion_before: 0,
        onion_after: 0,
        onion_opacity: 0.25,
    };
    let candidate = ProjectTimelineCandidate::open(
        timeline.as_bytes(),
        vec![b"dot".to_vec(), b"bar".to_vec()],
        Vec::new(),
        settings,
        EditLimits::default(),
    )
    .expect("fixture timeline");
    NativeExportSource::new(SCENE.to_vec(), candidate)
}

fn fnv(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| {
        (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// (GIF hash, per-frame PNG-source hashes with indexes, decoded MP4 hashes).
fn run(workers: usize) -> (u64, Vec<(usize, u64)>, Vec<u64>) {
    FORCED_WORKERS.store(workers, Ordering::Relaxed);
    let src = source();
    let gif = fnv(&src.export_gif().unwrap().bytes);
    let mut frames = Vec::new();
    let req = NativeFramesRequest {
        size: Some((96, 96)),
        background: Some([250, 250, 250]),
        fps: None,
    };
    src.render_frames(&req, |i, rgba| {
        frames.push((i, fnv(rgba)));
        Ok::<(), ()>(())
    })
    .unwrap();
    let path = std::env::temp_dir().join(format!("vexel-par-{}-{workers}.mp4", std::process::id()));
    src.export_video(&NativeVideoRequest {
        path: path.clone(),
        codec: VideoCodec::H264,
        size: Some((96, 96)),
        background: [255, 255, 255],
        alpha: false,
        bit_rate: Some(4_000_000),
        quality: None,
        fps: None,
        loops: 1,
    })
    .unwrap();
    let decoded = vexel_platform_apple::video_reader::decode(&path).unwrap();
    let _ = std::fs::remove_file(&path);
    let video = decoded.frames.iter().map(|f| fnv(&f.rgba)).collect();
    FORCED_WORKERS.store(0, Ordering::Relaxed);
    (gif, frames, video)
}

#[test]
fn parallel_exports_match_serial_exports() {
    let serial = run(1);
    assert_eq!(serial.1.len(), 2 * TICKS as usize - 2);
    assert_eq!(serial.2.len(), 2 * TICKS as usize - 2);
    assert!(
        serial.1.windows(2).all(|w| w[0].1 != w[1].1),
        "frames differ"
    );
    for workers in [2, 4, 8] {
        let parallel = run(workers);
        assert_eq!(parallel.0, serial.0, "gif, {workers} workers");
        assert_eq!(parallel.1, serial.1, "frames, {workers} workers");
        assert_eq!(parallel.2, serial.2, "mp4, {workers} workers");
    }
}

/// Wall-clock probe, run by hand:
/// `VEXEL_BENCH_FILE=x.vexel VEXEL_BENCH_WORKERS=4 cargo test -p vexel-app --lib -- --ignored --nocapture bench_exports`
#[test]
#[ignore]
fn bench_exports() {
    use std::time::Instant;
    let Ok(file) = std::env::var("VEXEL_BENCH_FILE") else {
        return;
    };
    let workers = std::env::var("VEXEL_BENCH_WORKERS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    FORCED_WORKERS.store(workers, Ordering::Relaxed);
    let mut doc = crate::session::AppDocument::new(8, 8).unwrap();
    doc.open_path(std::path::Path::new(&file)).unwrap();
    let src = NativeExportSource::from_document(&doc).expect("native timeline");
    let side = Some((512, 512));
    let t = Instant::now();
    let gif = src.export_gif().unwrap();
    println!(
        "BENCH gif {:?} frames={} {}x{}",
        t.elapsed(),
        gif.frames,
        gif.width,
        gif.height
    );
    let req = NativeFramesRequest {
        size: side,
        background: None,
        fps: None,
    };
    let t = Instant::now();
    let mut pixels = Vec::new();
    let done = src
        .render_frames(&req, |_, rgba| {
            pixels.push(rgba.to_vec());
            Ok::<(), ()>(())
        })
        .unwrap();
    let frames: Vec<_> = pixels
        .iter()
        .map(|rgba| vexel_png::ApngFrame {
            rgba,
            delay_num: 1,
            delay_den: 24,
        })
        .collect();
    vexel_png::encode_apng(&frames, done.width, done.height, 0).unwrap();
    println!("BENCH apng {:?} frames={}", t.elapsed(), done.frames);
    let path = std::env::temp_dir().join(format!("vexel-bench-{}.mp4", std::process::id()));
    let t = Instant::now();
    let video = src
        .export_video(&NativeVideoRequest {
            path: path.clone(),
            codec: VideoCodec::H264,
            size: side,
            background: [255; 3],
            alpha: false,
            bit_rate: None,
            quality: None,
            fps: None,
            loops: 1,
        })
        .unwrap();
    println!("BENCH mp4 {:?} frames={}", t.elapsed(), video.frames);
    let _ = std::fs::remove_file(&path);
}
