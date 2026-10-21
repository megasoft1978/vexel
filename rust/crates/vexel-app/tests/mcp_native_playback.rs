//! `animation_control` on a native timeline: play, pause and seek steer the
//! native animation host's clock and the reply reports the state it reports.
//! The document is built through the MCP rig; the host is driven directly
//! because the rig only owns the SVG driver.

mod common;

use std::cell::Cell;
use std::rc::Rc;

use common::{Rig, element_id, no_grants};
use vexel_app::animation::playback::LoopClock;
use vexel_app::native_animation::host::NativeAnimationHost;
use vexel_app::preview::Control;

/// A clock the test moves forward by hand; clones share the time.
#[derive(Clone, Default)]
struct SharedClock(Rc<Cell<f64>>);

impl SharedClock {
    fn set(&self, seconds: f64) {
        self.0.set(seconds);
    }
}

impl LoopClock for SharedClock {
    fn now_seconds(&self) -> f64 {
        self.0.get()
    }
}

/// A rect with one opacity track: keys 0 -> 0 and tick 12 -> 1 over the
/// default 48-tick, 24 ticks-per-second native timeline (2 s).
fn native_rig() -> Rig {
    let mut rig = Rig::new(no_grants());
    let id = element_id(
        rig.doc_tool(
            "element_add",
            r##""parentId":"root","template":"rect","attributes":{"x":"1","y":"1","width":"5","height":"5","fill":"#112233"}"##,
        )
        .ok(),
    );
    rig.doc_tool(
        "native_timeline_add_property_track",
        &format!(
            r#""trackId":"t1","targetId":"{id}","property":"opacity","valueType":"scalar","interpolation":"linear","value":0"#
        ),
    )
    .ok();
    rig.doc_tool(
        "native_timeline_set_property_key",
        r#""trackId":"t1","tick":12,"value":1"#,
    )
    .ok();
    rig
}

#[test]
fn native_timeline_is_detected_only_with_tracks() {
    let host = NativeAnimationHost::new(SharedClock::default());
    let empty = Rig::new(no_grants());
    assert!(!host.has_native_timeline(&empty.app));

    let rig = native_rig();
    assert!(host.has_native_timeline(&rig.app));
}

#[test]
fn play_then_pause_reports_playing_true_then_false() {
    let rig = native_rig();
    let clock = SharedClock::default();
    let mut host = NativeAnimationHost::new(clock.clone());

    let playing = host.mcp_control(&rig.app, Control::Play, true).unwrap();
    assert!(playing.contains("\"playing\":true"), "{playing}");
    assert!(playing.contains("\"tick\":0"), "{playing}");

    clock.set(0.25);
    let paused = host.mcp_control(&rig.app, Control::Pause, true).unwrap();
    assert!(paused.contains("\"playing\":false"), "{paused}");
    assert!(paused.contains("\"tick\":6"), "{paused}");
    assert!(paused.contains("\"timeSeconds\":0.250"), "{paused}");

    // A paused clock stays put: later time does not move the playhead.
    clock.set(1.5);
    let still = host.mcp_control(&rig.app, Control::Pause, true).unwrap();
    assert!(still.contains("\"playing\":false"), "{still}");
    assert!(still.contains("\"tick\":6"), "{still}");
}

#[test]
fn seek_sets_tick_and_keeps_play_state() {
    let rig = native_rig();
    let clock = SharedClock::default();
    let mut host = NativeAnimationHost::new(clock.clone());

    let seeked = host
        .mcp_control(&rig.app, Control::Seek(0.5), true)
        .unwrap();
    assert!(seeked.contains("\"playing\":false"), "{seeked}");
    assert!(seeked.contains("\"tick\":12"), "{seeked}");
    assert!(seeked.contains("\"timeSeconds\":0.500"), "{seeked}");

    clock.set(1.0);
    let playing = host.mcp_control(&rig.app, Control::Play, true).unwrap();
    assert!(playing.contains("\"playing\":true"), "{playing}");
    assert!(playing.contains("\"tick\":12"), "{playing}");

    clock.set(1.25);
    let paused = host.mcp_control(&rig.app, Control::Pause, true).unwrap();
    assert!(paused.contains("\"tick\":18"), "{paused}");
    assert!(paused.contains("\"timeSeconds\":0.750"), "{paused}");
}

#[test]
fn play_needs_the_animate_panel() {
    let rig = native_rig();
    let mut host = NativeAnimationHost::new(SharedClock::default());
    let error = host
        .mcp_control(&rig.app, Control::Play, false)
        .unwrap_err();
    assert_eq!(
        error,
        "native timeline plays in Animate mode on the Native timeline"
    );
    let reply = host.mcp_control(&rig.app, Control::Pause, false).unwrap();
    assert!(reply.contains("\"playing\":false"), "{reply}");
}

#[test]
fn seek_rejects_times_outside_the_tool_range() {
    let rig = native_rig();
    let mut host = NativeAnimationHost::new(SharedClock::default());
    for seconds in [-0.1, 10.5, f64::NAN] {
        assert_eq!(
            host.mcp_control(&rig.app, Control::Seek(seconds), true)
                .unwrap_err(),
            "invalid preview animation time"
        );
    }
}
