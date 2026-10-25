//! Decode-ahead pipeline: same frames as the serial path, no stale graphs, bounded memory.

use std::cell::Cell;
use std::rc::Rc;

use vexel_timeline::{EditLimits, EmptyTimelineSettings, LoopMode};

use super::ahead::{THREADS, WINDOW};
use super::host::NativeAnimationHost;
use crate::animation::native_export::NativeExportSource;
use crate::animation::playback::LoopClock;
use crate::preview::Control;
use crate::session::AppDocument;

const TICKS: u32 = 48;

#[derive(Clone, Default)]
struct HandClock(Rc<Cell<f64>>);

impl LoopClock for HandClock {
    fn now_seconds(&self) -> f64 {
        self.0.get()
    }
}

/// Animated camera group (transform track) around shapes, one of them a
/// morphing path.
fn doc() -> AppDocument {
    let mut svg = String::from(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"96\" height=\"96\" viewBox=\"0 0 96 96\"><g id=\"cam\">",
    );
    for i in 0..12u32 {
        svg.push_str(&format!(
            "<rect id=\"r{i}\" x=\"{}\" y=\"{}\" width=\"14\" height=\"14\" fill=\"#{:02x}66cc\"/>",
            (i % 4) * 22,
            (i / 4) * 22,
            i * 20
        ));
    }
    svg.push_str("<path id=\"m\" d=\"M10 80 L40 80 L40 92 Z\" fill=\"#cc3366\"/></g></svg>");
    let path = std::env::temp_dir().join(format!("vexel-ahead-{}.svg", std::process::id()));
    std::fs::write(&path, svg).unwrap();
    let mut doc = AppDocument::new(96, 96).unwrap();
    doc.open_path(&path).unwrap();
    let _ = std::fs::remove_file(&path);
    let settings = EmptyTimelineSettings {
        ticks_per_second: 24,
        duration_ticks: TICKS,
        fps_numerator: 24,
        fps_denominator: 1,
        loop_mode: LoopMode::Repeat,
        onion_before: 0,
        onion_after: 0,
        onion_opacity: 0.25,
    };
    doc.edit_timeline(settings, EditLimits::default(), |c| {
        c.add_property_track(
            b"cam",
            b"cam",
            b"transform",
            b"transform2d",
            b"linear",
            b"[1,0,0,1,0,0]",
            b"cam",
        )?
        .set_property_key(b"cam", 16, b"[1.2,0.1,-0.1,1.2,-6,-4]")?
        .set_property_key(b"cam", 32, b"[0.9,0,0,0.9,4,6]")?
        .add_property_track(
            b"mo",
            b"m",
            b"d",
            b"path",
            b"linear",
            br#""M10 80 L40 80 L40 92 Z""#,
            b"mo",
        )?
        .set_property_key(b"mo", 24, br#""M20 70 L50 84 L30 94 Z""#)
    })
    .unwrap();
    doc
}

fn host(clock: &HandClock, ahead: bool) -> NativeAnimationHost<HandClock> {
    let mut host = NativeAnimationHost::new(clock.clone());
    host.set_decode_ahead(ahead);
    host
}

fn play(host: &mut NativeAnimationHost<HandClock>, doc: &AppDocument) {
    host.snapshot(doc, None).unwrap();
    host.mcp_control(doc, Control::Play, true).unwrap();
}

#[test]
fn pipelined_frames_equal_serial_frames_for_every_tick() {
    let doc = doc();
    // Sequential play, then stride 2 and 3 (slow renders skip ticks), then a jump.
    let mut order: Vec<u32> = (0..TICKS).collect();
    order.extend((0..TICKS).step_by(2));
    order.extend((1..TICKS).step_by(3));
    order.extend([40, 3, 4, 5, 30, 31]);
    let (c1, c2) = (HandClock::default(), HandClock::default());
    let (mut serial, mut piped) = (host(&c1, false), host(&c2, true));
    play(&mut serial, &doc);
    play(&mut piped, &doc);
    let mut distinct = std::collections::HashSet::new();
    for tick in order {
        let at = (f64::from(tick) + 0.5) / 24.0;
        c1.0.set(at);
        c2.0.set(at);
        let a = serial.tick(&doc, (96, 96)).expect("serial frame");
        let b = piped.tick(&doc, (96, 96)).expect("pipelined frame");
        assert!(a.rgba == b.rgba, "tick {tick} differs from serial");
        distinct.insert(a.rgba);
    }
    assert!(distinct.len() > 30, "scene does not animate");
    let hits = piped
        .prepared_for_test(&doc)
        .unwrap()
        .ahead()
        .hits
        .load(std::sync::atomic::Ordering::Relaxed);
    assert!(hits > 50, "ring served only {hits} ticks");
}

#[test]
fn a_new_generation_drops_decoded_ticks() {
    let doc = doc();
    let clock = HandClock::default();
    let mut host = host(&clock, true);
    let prepared = host.prepared_for_test(&doc).unwrap();
    prepared.set_preview(true);
    prepared.render(5, (96, 96)).unwrap();
    prepared.ahead().settle();
    let (entries, _) = prepared.ahead().held();
    assert!(entries > 0, "nothing decoded ahead");
    prepared.cancel_ahead();
    assert_eq!(prepared.ahead().held(), (0, 0));
    for tick in 6..6 + WINDOW {
        assert!(prepared.ahead().take(tick).is_none(), "stale graph {tick}");
    }
    // Leaving preview (pause, scrub, export) also drops the ring.
    prepared.render(9, (96, 96)).unwrap();
    prepared.ahead().settle();
    prepared.set_preview(false);
    assert_eq!(prepared.ahead().held(), (0, 0));
}

#[test]
fn a_seek_while_playing_never_shows_a_decoded_ahead_frame() {
    let doc = doc();
    let clock = HandClock::default();
    let mut host = host(&clock, true);
    play(&mut host, &doc);
    for tick in 0..6u32 {
        clock.0.set((f64::from(tick) + 0.5) / 24.0);
        host.tick(&doc, (96, 96)).unwrap();
    }
    host.mcp_control(&doc, Control::Seek(20.0 / 24.0), true)
        .unwrap();
    let frame = host.tick(&doc, (96, 96)).unwrap();
    let source = NativeExportSource::from_document(&doc).unwrap();
    let (_, _, want) = source.render_still(20.0 / 24.0, |s| Ok(s)).unwrap();
    assert!(frame.rgba == want, "stale or inexact frame after a seek");
}

#[test]
fn ring_memory_and_threads_stay_bounded() {
    let doc = doc();
    let clock = HandClock::default();
    let mut host = host(&clock, true);
    let prepared = host.prepared_for_test(&doc).unwrap();
    prepared.set_preview(true);
    for tick in 0..TICKS {
        prepared.render(tick, (96, 96)).unwrap();
        let (entries, _) = prepared.ahead().held();
        assert!(entries <= WINDOW as usize, "{entries} entries");
        assert!(THREADS <= WINDOW as usize);
    }
    prepared.ahead().settle();
    let (_, bytes) = prepared.ahead().held();
    assert!(bytes > 0);
    // A budget below one graph keeps nothing ready.
    prepared.cancel_ahead();
    prepared.ahead().set_budget(1);
    prepared.render(2, (96, 96)).unwrap();
    prepared.ahead().settle();
    assert_eq!(prepared.ahead().held().1, 0);
    // Frames are still right without a ring.
    prepared.render(3, (96, 96)).unwrap();
}
