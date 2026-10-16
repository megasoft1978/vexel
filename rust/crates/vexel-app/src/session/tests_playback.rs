//! MCP playback through the shared driver and the production `PlaybackHost`
//! (WO-RUST-V1-PLAYBACK-01 AC7, AC9).

use super::*;
use crate::animation::driver::{Driver, TickOutcome};
use crate::animation::keys::{PlaybackKey, apply};
use crate::animation::playback::ManualClock;
use crate::preview::{Control, Operation, Shape};

const ANIM: &[u8] = b"<animate attributeName=\"x\" values=\"10;20\" dur=\"2s\"/>";
const RECT: Shape = Shape::Rect {
    x: 40.0,
    y: 50.0,
    width: 120.0,
    height: 80.0,
    rgb: 0x2a7fff,
};

fn reply(outcome: Result<McpOutcome, &'static str>) -> String {
    match outcome.unwrap() {
        McpOutcome::Reply(json) => json,
        other => panic!("expected a reply, got {other:?}"),
    }
}

fn animated() -> (AppDocument, McpHost) {
    let mut app = AppDocument::new(512, 512).unwrap();
    let mut host = McpHost::new();
    let mut driver = Driver::new(ManualClock::default());
    reply(host.handle_with(&mut app, Operation::Add(RECT), &mut driver));
    reply(host.handle_with(
        &mut app,
        Operation::AnimationAdd {
            target_id: "vexel-object-1".to_owned(),
            xml: ANIM.to_vec(),
        },
        &mut driver,
    ));
    (app, host)
}

fn control(
    host: &mut McpHost,
    app: &mut AppDocument,
    driver: &mut Driver<ManualClock>,
    control: Control,
) -> Result<McpOutcome, &'static str> {
    host.handle_with(app, Operation::Control(control), driver)
}

#[test]
fn mcp_controls_steer_the_shared_driver_with_the_integrate_replies() {
    let (mut app, mut host) = animated();
    let mut driver = Driver::new(ManualClock::default());

    let play = reply(control(&mut host, &mut app, &mut driver, Control::Play));
    assert_eq!(play, "{\"ok\":true,\"playing\":true,\"timeSeconds\":0.000}");
    assert!(driver.is_playing());
    driver.playback().clock().advance(0.5);
    let paused = reply(control(&mut host, &mut app, &mut driver, Control::Pause));
    assert_eq!(
        paused,
        "{\"ok\":true,\"playing\":false,\"timeSeconds\":0.500}"
    );
    assert!(!driver.is_playing());
    let seek = reply(control(
        &mut host,
        &mut app,
        &mut driver,
        Control::Seek(1.25),
    ));
    assert_eq!(
        seek,
        "{\"ok\":true,\"playing\":false,\"timeSeconds\":1.250}"
    );
    assert_eq!(driver.time(), 1.25);
    assert_eq!(
        control(&mut host, &mut app, &mut driver, Control::Seek(11.0)),
        Err("invalid preview animation time")
    );
    assert_eq!(
        control(&mut host, &mut app, &mut driver, Control::Seek(f64::NAN)),
        Err("invalid preview animation time")
    );
    assert_eq!(driver.time(), 1.25);
}

#[test]
fn keyboard_and_mcp_share_one_clock() {
    let (mut app, mut host) = animated();
    let mut driver = Driver::new(ManualClock::default());
    reply(control(&mut host, &mut app, &mut driver, Control::Play));
    driver.playback().clock().advance(0.5);
    // Space pauses what MCP started, at the shared time.
    assert!(apply(&mut driver, PlaybackKey::Toggle, false, false));
    assert!(!driver.is_playing());
    assert!((driver.time() - 0.5).abs() < 1e-9);
    let json = reply(control(
        &mut host,
        &mut app,
        &mut driver,
        Control::Seek(1.0),
    ));
    assert!(json.contains("\"playing\":false"), "{json}");
}

#[test]
fn play_on_a_static_document_keeps_the_integrate_error() {
    let mut app = AppDocument::new(512, 512).unwrap();
    let mut host = McpHost::new();
    let mut driver = Driver::new(ManualClock::default());
    assert_eq!(
        control(&mut host, &mut app, &mut driver, Control::Play),
        Err("no preview animation to play")
    );
    assert!(!driver.is_playing());
}

// ---- AC9 --------------------------------------------------------------------

#[test]
fn the_driver_receives_bytes_revision_and_seconds_from_the_session() {
    let (app, _host) = animated();
    let mut driver = Driver::new(ManualClock::default());
    app.request_frame(&mut driver, 0.0).unwrap();
    assert!(driver.knows(app.revision()));
    assert_eq!(driver.queue_len(), 1);
    assert!(driver.strip_visible());
    // The requested frame is the one the next tick hands out (no re-render).
    match driver.tick(app.revision(), &[]) {
        TickOutcome::Frame(frame) => {
            assert_eq!((frame.revision, frame.time), (app.revision(), 0.0));
        }
        other => panic!("expected the requested frame, got {other:?}"),
    }
}

#[test]
fn playback_never_dirties_the_document() {
    let (mut app, _host) = animated();
    let (revision, dirty, source) = (app.revision(), app.is_dirty(), app.source_bytes().unwrap());
    let mut driver = Driver::new(ManualClock::default());
    app.request_frame(&mut driver, 0.0).unwrap();
    assert!(driver.play());
    for _ in 0..5 {
        driver.playback().clock().advance(0.3);
        let _ = driver.tick(app.revision(), &app.source_bytes().unwrap());
    }
    driver.seek(1.5);
    let _ = driver.tick(app.revision(), &app.source_bytes().unwrap());
    driver.pause();
    assert_eq!(
        (app.revision(), app.is_dirty(), app.source_bytes().unwrap()),
        (revision, dirty, source)
    );
    assert_eq!(app.title(), "Vexel \u{2014} Untitled \u{2022}");
}
