//! Playback worker, prepared inputs and async host ticks.

use std::time::{Duration, Instant};

use vexel_timeline::{EditLimits, EmptyTimelineSettings, LoopMode};

use super::host::NativeAnimationHost;
use crate::animation::native_export::NativeExportSource;
use crate::animation::playback::ManualClock;
use crate::compose::Frame;
use crate::preview::Control;
use crate::session::AppDocument;

fn doc() -> AppDocument {
    let mut svg = String::from(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"64\" height=\"64\" viewBox=\"0 0 64 64\">",
    );
    for i in 0..9u32 {
        svg.push_str(&format!(
            "<rect id=\"r{i}\" x=\"{}\" y=\"{}\" width=\"16\" height=\"16\" fill=\"#3366cc\"/>",
            (i % 3) * 20,
            (i / 3) * 20
        ));
    }
    svg.push_str("</svg>");
    let path = std::env::temp_dir().join(format!(
        "vexel-play-{}-{}.svg",
        std::process::id(),
        Instant::now().elapsed().as_nanos()
    ));
    std::fs::write(&path, svg).unwrap();
    let mut doc = AppDocument::new(64, 64).unwrap();
    doc.open_path(&path).unwrap();
    let _ = std::fs::remove_file(&path);
    let settings = EmptyTimelineSettings {
        ticks_per_second: 24,
        duration_ticks: 48,
        fps_numerator: 24,
        fps_denominator: 1,
        loop_mode: LoopMode::Repeat,
        onion_before: 0,
        onion_after: 0,
        onion_opacity: 0.25,
    };
    doc.edit_timeline(settings, EditLimits::default(), |c| {
        c.add_property_track(b"t", b"r4", b"opacity", b"scalar", b"linear", b"1", b"t")?
            .set_property_key(b"t", 24, b"0.2")
    })
    .unwrap();
    doc
}

fn wait(host: &mut NativeAnimationHost<ManualClock>, doc: &AppDocument, size: (u32, u32)) -> Frame {
    let start = Instant::now();
    loop {
        if let Some(frame) = host.tick_async(doc, size, size, true) {
            return frame;
        }
        assert!(start.elapsed() < Duration::from_secs(20), "no frame");
        std::thread::sleep(Duration::from_millis(2));
    }
}

#[test]
fn playback_frames_equal_export_frames() {
    let doc = doc();
    let source = NativeExportSource::from_document(&doc).unwrap();
    for tick in [0u32, 7, 24, 40] {
        let mut host = NativeAnimationHost::new(ManualClock::default());
        host.mcp_control(&doc, Control::Seek(f64::from(tick) / 24.0), true)
            .unwrap();
        let frame = host.tick(&doc, (64, 64)).unwrap();
        let (w, h, rgba) = source
            .render_still(f64::from(tick) / 24.0, |s| Ok(s))
            .unwrap();
        assert_eq!((frame.width, frame.height), (w, h));
        assert!(frame.rgba == rgba, "tick {tick} differs from export");
    }
}

#[test]
fn async_tick_delivers_each_requested_frame_once() {
    let doc = doc();
    let mut host = NativeAnimationHost::new(ManualClock::default());
    host.mcp_control(&doc, Control::Seek(0.5), true).unwrap();
    let first = wait(&mut host, &doc, (64, 64));
    assert_eq!((first.width, first.height), (64, 64));
    // Nothing new is posted or returned while the clock stays put.
    for _ in 0..20 {
        assert!(host.tick_async(&doc, (64, 64), (64, 64), true).is_none());
        std::thread::sleep(Duration::from_millis(2));
    }
}

#[test]
fn seek_drops_frames_of_the_previous_position() {
    let doc = doc();
    let mut host = NativeAnimationHost::new(ManualClock::default());
    host.mcp_control(&doc, Control::Seek(0.0), true).unwrap();
    assert!(host.tick_async(&doc, (64, 64), (64, 64), true).is_none());
    // Scrub before the worker is polled: the tick-0 frame must never surface.
    host.mcp_control(&doc, Control::Seek(1.0), true).unwrap();
    let frame = wait(&mut host, &doc, (64, 64));
    let source = NativeExportSource::from_document(&doc).unwrap();
    let (_, _, want) = source.render_still(1.0, |s| Ok(s)).unwrap();
    assert!(frame.rgba == want, "stale frame surfaced after a seek");
}

#[test]
fn playing_renders_at_the_playback_size_and_pausing_at_full_size() {
    let doc = doc();
    let mut host = NativeAnimationHost::new(ManualClock::default());
    host.mcp_control(&doc, Control::Play, true).unwrap();
    let start = Instant::now();
    let small = loop {
        if let Some(frame) = host.tick_async(&doc, (64, 64), (32, 32), true) {
            break frame;
        }
        assert!(start.elapsed() < Duration::from_secs(20), "no frame");
        std::thread::sleep(Duration::from_millis(2));
    };
    assert_eq!((small.width, small.height), (32, 32));
    host.mcp_control(&doc, Control::Pause, true).unwrap();
    let full = wait(&mut host, &doc, (64, 64));
    assert_eq!((full.width, full.height), (64, 64));
}

#[test]
fn an_edit_cancels_work_for_the_old_revision() {
    let mut doc = doc();
    let mut host = NativeAnimationHost::new(ManualClock::default());
    host.mcp_control(&doc, Control::Seek(0.5), true).unwrap();
    assert!(host.tick_async(&doc, (64, 64), (64, 64), true).is_none());
    let settings = EmptyTimelineSettings {
        ticks_per_second: 24,
        duration_ticks: 48,
        fps_numerator: 24,
        fps_denominator: 1,
        loop_mode: LoopMode::Repeat,
        onion_before: 0,
        onion_after: 0,
        onion_opacity: 0.25,
    };
    doc.edit_timeline(settings, EditLimits::default(), |c| {
        c.set_property_key(b"t", 12, b"0.9")
    })
    .unwrap();
    host.mcp_control(&doc, Control::Seek(0.5), true).unwrap();
    let frame = wait(&mut host, &doc, (64, 64));
    let source = NativeExportSource::from_document(&doc).unwrap();
    let (_, _, want) = source.render_still(0.5, |s| Ok(s)).unwrap();
    assert!(frame.rgba == want, "frame of the old revision surfaced");
}

#[test]
fn view_paints_a_small_playback_frame_and_restores_the_static_one() {
    use crate::compose::DocumentView;
    let doc = doc();
    let mut view = DocumentView::new();
    view.set_name("doc");
    view.show(&doc.source_bytes().unwrap(), doc.revision(), (64, 64));
    let start = Instant::now();
    while view.composed().is_none() {
        assert!(
            start.elapsed() < Duration::from_secs(20),
            "compose timed out"
        );
        view.poll();
        std::thread::sleep(Duration::from_millis(2));
    }
    let static_rgba = view.composed().unwrap().frame.rgba.clone();
    let small = Frame {
        width: 32,
        height: 32,
        rgba: vec![9; 32 * 32 * 4],
        disclosures: vec![],
    };
    assert!(!view.present_native(doc.revision() + 1, small.clone()));
    assert!(view.present_native(doc.revision(), small));
    assert!(view.animated_shown());
    let painted = view.stage_paint().unwrap().frame.unwrap();
    assert_eq!((painted.width, painted.height), (32, 32));
    let full = Frame {
        width: 64,
        height: 64,
        rgba: vec![7; 64 * 64 * 4],
        disclosures: vec![],
    };
    assert!(view.present_native(doc.revision(), full));
    let painted = view.stage_paint().unwrap().frame.unwrap();
    assert_eq!(
        (painted.width, painted.height, painted.rgba[0]),
        (64, 64, 7)
    );
    assert!(view.restore_static());
    assert!(view.composed().unwrap().frame.rgba == static_rgba);
    assert!(!view.animated_shown());
}
