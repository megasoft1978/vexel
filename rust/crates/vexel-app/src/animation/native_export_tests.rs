use crate::animation::native_export::{NativeExportSource, loop_ticks};
use crate::project_session::ProjectTimelineCandidate;
use vexel_timeline::{EditLimits, EmptyTimelineSettings, LoopMode};

const SCENE: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" width="2" height="2" viewBox="0 0 2 2"><rect id="shape" width="2" height="2" fill="#0000ff"/></svg>"##;

fn source(loop_mode: &str) -> NativeExportSource {
    let timeline = format!(
        r#"{{"schema":"vexel.animation","version":1,"ticks_per_second":10,"duration_ticks":4,"fps":{{"numerator":10,"denominator":1}},"loop":"{loop_mode}","onion":{{"before":0,"after":0,"opacity":0}},"tracks":[{{"id":"paint","kind":"property","target":"shape","visible":true,"property":"fill","value_type":"rgba","interpolation":"step","keys":[{{"tick":0,"value":[1,0,0,1]}},{{"tick":2,"value":[0,1,0,1]}}]}}]}}"#
    );
    let settings = EmptyTimelineSettings {
        ticks_per_second: 10,
        duration_ticks: 4,
        fps_numerator: 10,
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
fn loop_ticks_follow_the_loop_mode() {
    assert_eq!(loop_ticks(4, LoopMode::Once), vec![0, 1, 2, 3]);
    assert_eq!(loop_ticks(4, LoopMode::PingPong), vec![0, 1, 2, 3, 2, 1]);
    assert_eq!(loop_ticks(1, LoopMode::PingPong), vec![0]);
}
