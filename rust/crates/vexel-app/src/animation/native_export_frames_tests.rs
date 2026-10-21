use super::native_export_frames::{NativeFramesError, NativeFramesRequest};
use super::native_export_video_tests::timeline_source;

fn request(size: Option<(u32, u32)>, fps: Option<u32>) -> NativeFramesRequest {
    NativeFramesRequest {
        size,
        background: None,
        fps,
    }
}

#[test]
fn frames_sample_by_fps_and_cap_at_1440() {
    let src = timeline_source(10, 4, "loop", 2);
    let plan = src.plan_frames(&request(Some((8, 8)), Some(5))).unwrap();
    assert_eq!((plan.frames, plan.stride), (2, 2));
    // An fps above the tick rate still shows one frame per tick.
    let plan = src.plan_frames(&request(Some((8, 8)), Some(60))).unwrap();
    assert_eq!((plan.frames, plan.stride), (4, 1));
    // 3000 ticks exceed the cap, so every 3rd is kept.
    let long = timeline_source(30, 3000, "loop", 2);
    let plan = long.plan_frames(&request(Some((8, 8)), None)).unwrap();
    assert_eq!((plan.frames, plan.stride), (1000, 3));
    let plan = long.plan_frames(&request(None, None)).unwrap();
    assert_eq!((plan.width, plan.height), (2, 2));
}

#[test]
fn frames_render_in_order_with_alpha_or_background() {
    let src = timeline_source(10, 4, "loop", 2);
    let mut seen = Vec::new();
    let done = src
        .render_frames(&request(Some((4, 4)), None), |i, rgba| {
            seen.push((i, rgba.len(), rgba[..4].to_vec()));
            Ok::<(), ()>(())
        })
        .unwrap();
    assert_eq!((done.frames, done.width, done.height), (4, 4, 4));
    assert_eq!(seen.len(), 4);
    assert!(seen.iter().enumerate().all(|(n, f)| f.0 == n && f.1 == 64));
    // Red before the flip, green after (step interpolation).
    assert!(seen[0].2[0] > 200 && seen[3].2[1] > 200, "{seen:?}");
    let mut req = request(Some((4, 4)), None);
    req.background = Some([1, 2, 3]);
    src.render_frames(&req, |_, rgba| {
        assert!(rgba.chunks_exact(4).all(|p| p[3] == 255));
        Ok::<(), ()>(())
    })
    .unwrap();
}

#[test]
fn frames_reject_bad_sizes_and_stop_on_a_sink_error() {
    let src = timeline_source(10, 4, "loop", 2);
    assert_eq!(
        src.plan_frames(&request(Some((4097, 8)), None))
            .unwrap_err(),
        NativeFramesError::BadSize
    );
    let mut calls = 0;
    let error = src
        .render_frames(&request(Some((4, 4)), None), |_, _| {
            calls += 1;
            Err("stop")
        })
        .unwrap_err();
    assert_eq!((calls, error), (1, NativeFramesError::Sink("stop")));
}
