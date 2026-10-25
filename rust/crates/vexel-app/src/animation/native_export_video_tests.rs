use crate::animation::native_export::NativeExportSource;
use crate::animation::native_export_video::{MAX_VIDEO_SIDE, NativeVideoError, NativeVideoRequest};
use crate::project_session::ProjectTimelineCandidate;
use vexel_platform_apple::VideoCodec;
use vexel_timeline::{EditLimits, EmptyTimelineSettings, LoopMode};

const SCENE: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" width="2" height="2" viewBox="0 0 2 2"><rect id="shape" width="2" height="2" fill="#0000ff"/></svg>"##;

/// A square fill timeline of `duration` ticks at `tps`, with a key flip at
/// `flip` ticks.
pub(super) fn timeline_source(
    tps: u32,
    duration: u32,
    loop_mode: &str,
    flip: u32,
) -> NativeExportSource {
    timeline_source_with(SCENE.to_vec(), tps, duration, loop_mode, flip)
}

fn timeline_source_with(
    scene: Vec<u8>,
    tps: u32,
    duration: u32,
    loop_mode: &str,
    flip: u32,
) -> NativeExportSource {
    let timeline = format!(
        r#"{{"schema":"vexel.animation","version":1,"ticks_per_second":{tps},"duration_ticks":{duration},"fps":{{"numerator":{tps},"denominator":1}},"loop":"{loop_mode}","onion":{{"before":0,"after":0,"opacity":0}},"tracks":[{{"id":"paint","kind":"property","target":"shape","visible":true,"property":"fill","value_type":"rgba","interpolation":"step","keys":[{{"tick":0,"value":[1,0,0,1]}},{{"tick":{flip},"value":[0,1,0,1]}}]}}]}}"#
    );
    let settings = EmptyTimelineSettings {
        ticks_per_second: tps,
        duration_ticks: duration,
        fps_numerator: tps,
        fps_denominator: 1,
        loop_mode: LoopMode::Repeat,
        onion_before: 0,
        onion_after: 0,
        onion_opacity: 0.25,
    };
    let candidate = ProjectTimelineCandidate::open(
        timeline.as_bytes(),
        vec![b"shape".to_vec()],
        Vec::new(),
        settings,
        EditLimits::default(),
    )
    .expect("fixture timeline");
    NativeExportSource::new(scene, candidate)
}

fn request(name: &str, size: Option<(u32, u32)>) -> NativeVideoRequest {
    let dir = std::env::temp_dir().join(format!("vexel-native-video-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    NativeVideoRequest {
        path: dir.join(name),
        codec: VideoCodec::H264,
        size,
        background: [255, 255, 255],
        alpha: false,
        bit_rate: Some(2_000_000),
        quality: None,
        fps: None,
        loops: 1,
    }
}

fn is_mp4(path: &std::path::Path) -> bool {
    let bytes = std::fs::read(path).unwrap();
    bytes.len() > 100 && &bytes[4..8] == b"ftyp"
}

#[test]
fn video_export_writes_one_frame_per_tick() {
    let req = request("loop.mp4", Some((64, 64)));
    let video = timeline_source(10, 4, "loop", 2)
        .export_video(&req)
        .unwrap();
    assert_eq!(
        (
            video.frames,
            video.width,
            video.height,
            video.ticks_per_frame
        ),
        (4, 64, 64, 1)
    );
    assert!(is_mp4(&req.path));
    let _ = std::fs::remove_file(&req.path);
}

#[test]
fn video_export_ping_pong_reflects_without_repeating_ends() {
    let req = request("pp.mp4", Some((64, 64)));
    let video = timeline_source(10, 6, "ping_pong", 2)
        .export_video(&req)
        .unwrap();
    assert_eq!(video.frames, 10);
    let _ = std::fs::remove_file(&req.path);
}

#[test]
fn video_export_strides_past_the_frame_cap() {
    let req = request("long.mp4", Some((16, 16)));
    let video = timeline_source(30, 1441, "loop", 2)
        .export_video(&req)
        .unwrap();
    assert_eq!((video.frames, video.ticks_per_frame), (721, 2));
    let _ = std::fs::remove_file(&req.path);
}

#[test]
fn video_export_rounds_odd_sizes_up_and_rejects_oversize() {
    let src = timeline_source(10, 2, "loop", 1);
    let req = request("odd.mp4", Some((17, 15)));
    let video = src.export_video(&req).unwrap();
    assert_eq!((video.width, video.height), (18, 16));
    let _ = std::fs::remove_file(&req.path);
    for size in [(0, 8), (MAX_VIDEO_SIDE + 1, 8)] {
        let err = src
            .export_video(&request("bad.mp4", Some(size)))
            .unwrap_err();
        assert!(matches!(err, NativeVideoError::BadSize { .. }), "{err:?}");
    }
}

#[test]
fn video_export_fps_loops_and_cap_choose_the_stride() {
    let src = timeline_source(10, 4, "loop", 2);
    let mut req = request("fps.mp4", Some((16, 16)));
    req.fps = Some(5);
    let video = src.export_video(&req).unwrap();
    assert_eq!((video.frames, video.ticks_per_frame), (2, 2));
    // An fps above the tick rate still shows one frame per tick.
    req.fps = Some(60);
    req.loops = 3;
    let video = src.export_video(&req).unwrap();
    assert_eq!((video.frames, video.ticks_per_frame), (12, 1));
    // Loops count towards the frame cap: 3000 frames become every 3rd.
    let long = timeline_source(30, 1000, "loop", 2);
    req.fps = None;
    let video = long.export_video(&req).unwrap();
    assert_eq!((video.frames, video.ticks_per_frame), (1000, 3));
    req.loops = 0;
    assert_eq!(
        src.export_video(&req).unwrap_err(),
        NativeVideoError::BadLoops
    );
    let _ = std::fs::remove_file(&req.path);
}

#[test]
fn video_export_accepts_quality_in_place_of_a_bit_rate() {
    let src = timeline_source(10, 4, "loop", 2);
    let mut req = request("quality.mp4", Some((64, 64)));
    req.bit_rate = None;
    for quality in [0.0, 1.0] {
        req.quality = Some(quality);
        assert_eq!(src.export_video(&req).unwrap().frames, 4);
        assert!(is_mp4(&req.path));
    }
    let _ = std::fs::remove_file(&req.path);
}

#[test]
fn quality_level_is_monotonic_and_clamped() {
    use crate::animation::native_export_video::{DEFAULT_QUALITY, quality_level};
    assert_eq!(quality_level(0.0), 0.6);
    assert_eq!(quality_level(1.0), 1.0);
    assert_eq!(quality_level(-3.0), quality_level(0.0));
    assert_eq!(quality_level(9.0), quality_level(1.0));
    let steps = [0.0, 0.25, DEFAULT_QUALITY, 0.75, 1.0].map(quality_level);
    assert!(steps.windows(2).all(|w| w[0] < w[1]), "{steps:?}");
}

#[test]
fn video_export_file_size_orders_with_quality() {
    let mut rects = String::new();
    for i in 0..400u32 {
        let (x, y) = (i % 20 * 12, i / 20 * 12);
        rects += &format!(
            r##"<rect x="{x}" y="{y}" width="11" height="11" fill="#{:06x}"/>"##,
            i.wrapping_mul(2_654_435_761) & 0xff_ffff
        );
    }
    let svg = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="240" height="240" viewBox="0 0 240 240"><rect id="shape" width="240" height="240" fill="#ff0000"/>{rects}</svg>"##
    );
    let src = timeline_source_with(svg.into_bytes(), 30, 30, "loop", 15);
    let mut req = request("size-order.mp4", Some((480, 480)));
    req.bit_rate = None;
    let mut sizes = Vec::new();
    for quality in [Some(0.0), None, Some(1.0)] {
        req.quality = quality;
        src.export_video(&req).unwrap();
        sizes.push(std::fs::metadata(&req.path).unwrap().len());
        let _ = std::fs::remove_file(&req.path);
    }
    assert!(sizes[0] < sizes[1] && sizes[1] < sizes[2], "{sizes:?}");
}
