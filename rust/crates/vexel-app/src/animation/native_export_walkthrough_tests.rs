//! V1 walkthrough GIF step: a native timeline built through MCP-style element
//! add and `AppDocument::edit_timeline` exports one frame per loop step.

use crate::animation::driver::Driver;
use crate::animation::native_export::NativeExportSource;
use crate::animation::playback::ManualClock;
use crate::preview::parse_operation;
use crate::session::{AppDocument, McpHost, McpOutcome};
use vexel_json::JsonDocument;
use vexel_timeline::{EditLimits, EmptyTimelineSettings, LoopMode};

const SIZE: u32 = 64;

fn settings() -> EmptyTimelineSettings {
    EmptyTimelineSettings {
        ticks_per_second: 24,
        duration_ticks: 24,
        fps_numerator: 24,
        fps_denominator: 1,
        loop_mode: LoopMode::Once,
        onion_before: 0,
        onion_after: 0,
        onion_opacity: 0.25,
    }
}

/// Adds one rect through the MCP `element_add` handler; its id is
/// `vexel-object-1`.
fn add_rect(app: &mut AppDocument) {
    let json = r##"{"documentId":"preview-document","template":"rect","attributes":{"x":"1","y":"1","width":"5","height":"5","fill":"#112233"}}"##;
    let doc = JsonDocument::parse(json.as_bytes()).unwrap();
    let operation = parse_operation(&doc, doc.root(), "element_add").unwrap();
    let mut host = McpHost::new();
    let mut driver = Driver::new(ManualClock::default());
    match host.handle_with(app, operation, &mut driver).unwrap() {
        McpOutcome::Reply(reply) => assert!(reply.contains("vexel-object-1"), "{reply}"),
        other => panic!("unexpected {other:?}"),
    }
}

#[test]
fn v1_walkthrough_native_gif_frames_follow_timeline() {
    let mut app = AppDocument::new(SIZE, SIZE).unwrap();
    add_rect(&mut app);
    assert!(
        NativeExportSource::from_document(&app).is_none(),
        "no native tracks yet"
    );

    assert!(
        app.edit_timeline(settings(), EditLimits::default(), |candidate| {
            let added = candidate.add_property_track(
                b"fade",
                b"vexel-object-1",
                b"opacity",
                b"scalar",
                b"linear",
                b"0",
                b"",
            )?;
            let keyed = added.set_property_key(b"fade", 12, b"1")?;
            keyed.set_loop(LoopMode::PingPong)
        })
        .unwrap()
    );

    // Ping-pong over 24 ticks plays 0..24 then 22..1: 46 frames.
    let gif = NativeExportSource::from_document(&app)
        .expect("native timeline")
        .export_gif()
        .unwrap();
    assert_eq!(&gif.bytes[..6], b"GIF89a");
    assert_eq!(
        (gif.frames, gif.delay_cs, gif.width, gif.height),
        (46, 4, SIZE, SIZE)
    );

    // Repeat plays each of the 24 ticks once per loop.
    assert!(
        app.edit_timeline(settings(), EditLimits::default(), |candidate| {
            candidate.set_loop(LoopMode::Repeat)
        })
        .unwrap()
    );
    let looped = NativeExportSource::from_document(&app)
        .expect("native timeline")
        .export_gif()
        .unwrap();
    assert_eq!((looped.frames, looped.delay_cs), (24, 4));
}
