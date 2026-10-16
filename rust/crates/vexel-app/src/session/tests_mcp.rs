//! MCP preview operations through the shared session.

use super::tests::{Dir, frame_hash, shown_hash};
use super::*;
use crate::animation::driver::Driver;
use crate::animation::playback::ManualClock;
use crate::compose::DocumentView;
use crate::preview::{Control, Operation, Shape};
use std::os::unix::ffi::OsStrExt;

const SIZE: (u32, u32) = (512, 512);
const ANIM: &[u8] = b"<animate attributeName=\"x\" values=\"10;20\" dur=\"2s\"/>";

fn reply(outcome: Result<McpOutcome, &'static str>) -> String {
    match outcome.unwrap() {
        McpOutcome::Reply(json) => json,
        other => panic!("expected a reply, got {other:?}"),
    }
}

const RECT: Shape = Shape::Rect {
    x: 40.0,
    y: 50.0,
    width: 120.0,
    height: 80.0,
    rgb: 0x2a7fff,
};

#[test]
fn shape_add_mutates_the_session_marks_dirty_and_renders() {
    let mut app = AppDocument::new(512, 512).unwrap();
    let mut host = McpHost::new();
    let mut driver = Driver::new(ManualClock::default());
    let mut view = DocumentView::new();
    let blank_revision = app.revision();
    let json = reply(host.handle_with(&mut app, Operation::Add(RECT), &mut driver));
    assert_eq!(
        json,
        "{\"ok\":true,\"shapeCount\":1,\"elementId\":\"vexel-object-1\",\"message\":\"Shape added\"}"
    );
    assert!(app.revision() > blank_revision);
    assert!(app.is_dirty());
    assert_eq!(app.title(), "Vexel \u{2014} Untitled \u{2022}");

    // Expected frame: the same shape added to an independent document.
    let mut expected = vexel_v1_document::V1Document::new(512, 512).unwrap();
    expected
        .add_rectangle_with_id(40.0, 50.0, 160.0, 130.0, "#2A7FFF", "vexel-object-1")
        .unwrap();
    assert_eq!(app.source_bytes().unwrap(), expected.source().unwrap());
    assert_eq!(
        shown_hash(&mut view, &app, SIZE),
        frame_hash(&expected.source().unwrap(), SIZE)
    );

    // Save and reopen: the bytes round-trip.
    let dir = Dir::new();
    let path = dir.file("mcp.vexel");
    app.save_as(path.as_os_str().as_bytes()).unwrap();
    assert!(!app.is_dirty());
    let saved = app.source_bytes().unwrap();
    let mut reopened = AppDocument::new(10, 10).unwrap();
    reopened.open_path(&path).unwrap();
    assert_eq!(reopened.source_bytes().unwrap(), saved);
}

#[test]
fn undo_redo_info_and_ids_go_through_the_document() {
    let mut app = AppDocument::new(512, 512).unwrap();
    let mut host = McpHost::new();
    let mut driver = Driver::new(ManualClock::default());
    reply(host.handle_with(&mut app, Operation::Add(RECT), &mut driver));
    let ellipse = Shape::Ellipse {
        cx: 300.0,
        cy: 300.0,
        rx: 50.0,
        ry: 30.0,
        rgb: 0xff0000,
    };
    let json = reply(host.handle_with(&mut app, Operation::Add(ellipse), &mut driver));
    assert!(json.contains("\"elementId\":\"vexel-object-2\""), "{json}");
    let info = reply(host.handle_with(&mut app, Operation::Info, &mut driver));
    assert!(
        info.contains("\"elementCount\":2") && info.contains("\"dirty\":true"),
        "{info}"
    );

    let revision = app.revision();
    let json = reply(host.handle_with(&mut app, Operation::Undo, &mut driver));
    assert!(json.contains("\"shapeCount\":1"), "{json}");
    assert!(app.revision() > revision);
    // The undone id is not reused.
    let json = reply(host.handle_with(&mut app, Operation::Add(ellipse), &mut driver));
    assert!(json.contains("\"elementId\":\"vexel-object-3\""), "{json}");
    assert_eq!(
        host.handle_with(&mut app, Operation::Redo, &mut driver),
        Err("nothing to redo")
    );
    let mut fresh = AppDocument::new(512, 512).unwrap();
    assert_eq!(
        host.handle_with(&mut fresh, Operation::Undo, &mut driver),
        Err("nothing to undo")
    );
}

#[test]
fn animation_fragment_is_applied_through_the_session() {
    let mut app = AppDocument::new(512, 512).unwrap();
    let mut host = McpHost::new();
    let mut driver = Driver::new(ManualClock::default());
    assert_eq!(
        host.handle_with(
            &mut app,
            Operation::AnimationAdd {
                target_id: "vexel-object-1".to_owned(),
                xml: ANIM.to_vec()
            },
            &mut driver
        ),
        Err("unknown preview animation target")
    );
    reply(host.handle_with(&mut app, Operation::Add(RECT), &mut driver));
    let revision = app.revision();
    let json = reply(host.handle_with(
        &mut app,
        Operation::AnimationAdd {
            target_id: "vexel-object-1".to_owned(),
            xml: ANIM.to_vec(),
        },
        &mut driver,
    ));
    assert!(json.contains("Animation added"), "{json}");
    assert!(app.revision() > revision);
    assert!(
        app.source_bytes()
            .unwrap()
            .windows(8)
            .any(|w| w == b"<animate")
    );
    let list = reply(host.handle_with(&mut app, Operation::AnimationList, &mut driver));
    assert!(list.contains("\"targetId\":\"vexel-object-1\""), "{list}");

    // Playback state follows the injected clock.
    let play = reply(host.handle_with(&mut app, Operation::Control(Control::Play), &mut driver));
    assert!(play.contains("\"playing\":true"), "{play}");
    assert!(driver.is_playing());
    driver.playback().clock().advance(0.5);
    let paused = reply(host.handle_with(&mut app, Operation::Control(Control::Pause), &mut driver));
    assert!(
        paused.contains("\"playing\":false") && paused.contains("0.500"),
        "{paused}"
    );
    let seek = reply(host.handle_with(
        &mut app,
        Operation::Control(Control::Seek(1.25)),
        &mut driver,
    ));
    assert!(seek.contains("1.250"), "{seek}");
    assert_eq!(
        host.handle_with(
            &mut app,
            Operation::Control(Control::Seek(11.0)),
            &mut driver
        ),
        Err("invalid preview animation time")
    );
    // Export hands the session source to the export job.
    let export = host
        .handle_with(
            &mut app,
            Operation::Export {
                path: "a.gif".to_owned(),
                fps: 10,
                duration: 1.0,
            },
            &mut driver,
        )
        .unwrap();
    assert!(matches!(export, McpOutcome::Export { fps: 10, .. }));
}

#[test]
fn invalid_requests_leave_the_session_unchanged() {
    let mut app = AppDocument::new(512, 512).unwrap();
    let mut host = McpHost::new();
    let mut driver = Driver::new(ManualClock::default());
    let bad = Shape::Rect {
        x: 0.0,
        y: 0.0,
        width: -5.0,
        height: 10.0,
        rgb: 0,
    };
    assert!(
        host.handle_with(&mut app, Operation::Add(bad), &mut driver)
            .is_err()
    );
    assert_eq!(app.revision(), 0);
    assert!(!app.is_dirty());
    let mut flat = AppDocument::new(512, 512).unwrap();
    assert_eq!(
        host.handle_with(&mut flat, Operation::Control(Control::Play), &mut driver),
        Err("no preview animation to play")
    );
}
