use crate::animation::native_export::{NativeExportSource, loop_ticks};
use crate::project_session::ProjectTimelineCandidate;
use vexel_timeline::{EditLimits, EmptyTimelineSettings, LoopMode};

const SCENE: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" width="2" height="2" viewBox="0 0 2 2"><rect id="shape" width="2" height="2" fill="#0000ff"/></svg>"##;

fn source(loop_mode: &str) -> NativeExportSource {
    timeline_source(10, 4, loop_mode, 2)
}

/// A square fill timeline of `duration` ticks at `tps`, with a key flip at
/// `flip` ticks.
fn timeline_source(tps: u32, duration: u32, loop_mode: &str, flip: u32) -> NativeExportSource {
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
    NativeExportSource::new(SCENE.to_vec(), candidate)
}

/// Delays of every graphic-control block, in file order.
fn delays(gif: &[u8]) -> Vec<u16> {
    gif.windows(8)
        .filter(|w| w[..3] == [0x21, 0xF9, 0x04] && w[7] <= 0xFF && w[3] & 0xE0 == 0)
        .map(|w| u16::from_le_bytes([w[4], w[5]]))
        .collect()
}

#[test]
fn native_export_gif_frames_and_delay() {
    let gif = source("loop").export_gif().unwrap();
    assert_eq!(&gif.bytes[..6], b"GIF89a");
    assert_eq!(
        (gif.frames, gif.delay_cs, gif.width, gif.height),
        (4, 10, 2, 2)
    );
    assert_eq!(delays(&gif.bytes), vec![10; 4]);

    let ping = source("ping_pong").export_gif().unwrap();
    assert_eq!(ping.frames, 6);
    assert_eq!(delays(&ping.bytes), vec![10; 6]);
}

#[test]
fn native_export_long_repeat_strides_to_frame_limit() {
    // 24 tps x 240 ticks: stride 2 keeps 120 frames, delay 4cs doubled to 8cs.
    let gif = timeline_source(24, 240, "loop", 120).export_gif().unwrap();
    assert_eq!(&gif.bytes[..6], b"GIF89a");
    assert_eq!((gif.frames, gif.delay_cs), (120, 8));
    assert_eq!(delays(&gif.bytes), vec![8; 120]);
}

#[test]
fn native_export_long_ping_pong_strides_to_frame_limit() {
    // 478 ping-pong steps: stride 4 keeps 120 frames, delay 4cs x 4 = 16cs.
    let gif = timeline_source(24, 240, "ping_pong", 120)
        .export_gif()
        .unwrap();
    assert!(gif.frames <= 120, "{}", gif.frames);
    assert_eq!((gif.frames, gif.delay_cs), (120, 16));
    assert_eq!(delays(&gif.bytes), vec![16; 120]);
}

#[test]
fn loop_ticks_follow_the_loop_mode() {
    assert_eq!(loop_ticks(4, LoopMode::Once), vec![0, 1, 2, 3]);
    assert_eq!(loop_ticks(4, LoopMode::PingPong), vec![0, 1, 2, 3, 2, 1]);
    assert_eq!(loop_ticks(1, LoopMode::PingPong), vec![0]);
}
