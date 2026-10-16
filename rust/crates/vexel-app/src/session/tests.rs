//! Headless session tests: single source, open, dirty/title, undo/redo, hooks.

use super::*;
use crate::compose::{DocumentView, compose_document, compose_svg};
use crate::file_flow::{FlowOutcome, handle};
use crate::project_session::FileAction;
use crate::tools::{CanvasMap, Key, Outcome, PointerInput, ToolController};
use std::cell::Cell;
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};
use vexel_platform_apple::{PanelBackend, PanelResult, UnsavedChoice, UnsavedText};
use vexel_ui::Rect;

pub(super) fn fixture(name: &str) -> Vec<u8> {
    let path = format!("{}/../../fixtures/v1/{name}", env!("CARGO_MANIFEST_DIR"));
    fs::read(&path).unwrap_or_else(|e| panic!("fixture {path}: {e}"))
}

pub(super) fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(format!(
        "{}/../../fixtures/v1/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
}

pub(super) struct Dir(pub PathBuf);
impl Dir {
    pub fn new() -> Self {
        static N: AtomicU32 = AtomicU32::new(0);
        let path = std::env::temp_dir().join(format!(
            "vexel-session-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
    pub fn file(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}
impl Drop for Dir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// FNV-1a over the frame bytes.
pub(super) fn hash(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x0100_0000_01b3)
    })
}

pub(super) fn frame_hash(source: &[u8], size: (u32, u32)) -> u64 {
    hash(&compose_svg(source, size.0, size.1).unwrap().rgba)
}

/// Shows the session through a real view and waits for its frame hash.
pub(super) fn shown_hash(view: &mut DocumentView, app: &AppDocument, size: (u32, u32)) -> u64 {
    view.show(&app.source_bytes().unwrap(), app.revision(), size);
    let deadline = Instant::now() + Duration::from_secs(20);
    while !(view.is_current(app.revision(), size) && view.composed().is_some()) {
        assert!(Instant::now() < deadline, "timed out: {:?}", view.status());
        view.poll();
        std::thread::sleep(Duration::from_millis(2));
    }
    hash(&view.composed().unwrap().frame.rgba)
}

fn rect(app: &mut AppDocument, id: &str, x: f64) {
    app.edit(|d| d.add_rectangle_with_id(x, 10.0, x + 40.0, 50.0, "#ff0000", id))
        .unwrap();
}

#[test]
fn edit_renders_from_the_single_session_source() {
    let mut app = AppDocument::new(160, 120).unwrap();
    let mut view = DocumentView::new();
    assert_eq!(app.revision(), 0);
    rect(&mut app, "r1", 10.0);
    assert_eq!(app.revision(), 1);
    let expected = frame_hash(&app.source_bytes().unwrap(), (160, 120));
    // `DocumentView::show` takes bytes and a revision; the view has no source
    // field to keep a second copy in.
    assert_eq!(shown_hash(&mut view, &app, (160, 120)), expected);
}

// ---- open -------------------------------------------------------------------

struct Recorder {
    save_panel_calls: Cell<u32>,
}
impl PanelBackend for Recorder {
    fn open_panel(&self, _: &[&str]) -> PanelResult {
        PanelResult::Cancelled
    }
    fn save_panel(&self, _: &str) -> PanelResult {
        self.save_panel_calls.set(self.save_panel_calls.get() + 1);
        PanelResult::Cancelled
    }
    fn confirm_unsaved(&self, _: &UnsavedText<'_>) -> UnsavedChoice {
        UnsavedChoice::Cancel
    }
    fn show_error(&self, _: &str, _: &str) {}
}

#[test]
fn open_svg_is_untitled_clean_and_renders_like_the_svg() {
    let svg = fixture("compose/compose-v1.svg");
    let mut app = AppDocument::new(10, 10).unwrap();
    app.open_path(&fixture_path("compose/compose-v1.svg"))
        .unwrap();
    assert_eq!(app.revision(), 1);
    assert_eq!(app.path(), None);
    assert!(!app.is_dirty());
    assert_eq!(app.title(), "Vexel");
    assert_eq!(app.display_name().as_deref(), Some("compose-v1.svg"));
    assert_eq!(app.source_bytes().unwrap(), svg);
    let mut view = DocumentView::new();
    assert_eq!(
        shown_hash(&mut view, &app, (160, 120)),
        frame_hash(&svg, (160, 120))
    );
    // Save on an untitled import asks for a name and never touches the SVG.
    let backend = Recorder {
        save_panel_calls: Cell::new(0),
    };
    let outcome = handle(FileAction::Save, app.session_for_flow(), &backend);
    assert_eq!(outcome, FlowOutcome::Cancelled);
    assert_eq!(backend.save_panel_calls.get(), 1);
    assert_eq!(
        fs::read(fixture_path("compose/compose-v1.svg")).unwrap(),
        svg
    );
}

#[test]
fn open_vexel_renders_like_its_scene_bytes_and_is_clean() {
    let dir = Dir::new();
    let path = dir.file("a.vexel");
    let mut src = AppDocument::new(200, 100).unwrap();
    rect(&mut src, "r1", 10.0);
    src.edit(|d| d.add_ellipse_with_id(70.0, 20.0, 150.0, 90.0, "#00ff00", "e1"))
        .unwrap();
    src.save_as(path.as_os_str().as_bytes()).unwrap();
    let scene = src.source_bytes().unwrap();

    let mut app = AppDocument::new(10, 10).unwrap();
    app.open_path(&path).unwrap();
    assert_eq!(app.source_bytes().unwrap(), scene);
    assert!(!app.is_dirty());
    assert_eq!(app.title(), "Vexel \u{2014} a.vexel");
    let mut view = DocumentView::new();
    assert_eq!(
        shown_hash(&mut view, &app, (200, 100)),
        frame_hash(&scene, (200, 100))
    );
}

#[test]
fn failed_open_is_typed_and_changes_nothing() {
    let dir = Dir::new();
    let good = dir.file("good.vexel");
    let mut src = AppDocument::new(100, 100).unwrap();
    rect(&mut src, "r1", 5.0);
    src.save_as(good.as_os_str().as_bytes()).unwrap();
    let bytes = fs::read(&good).unwrap();

    let mut cases: Vec<(PathBuf, fn(SessionError) -> bool)> = Vec::new();
    let corrupt = dir.file("corrupt.vexel");
    fs::write(&corrupt, &bytes[..bytes.len() - 5]).unwrap();
    cases.push((corrupt, |e| matches!(e, SessionError::Corrupt { .. })));
    let mut bad = bytes.clone();
    bad[8] = 2;
    let version = dir.file("version.vexel");
    fs::write(&version, &bad).unwrap();
    cases.push((version, |e| e == SessionError::UnsupportedVersion));
    let big = dir.file("big.vexel");
    fs::File::create(&big)
        .unwrap()
        .set_len(vexel_project::MAX_BYTES as u64 + 1)
        .unwrap();
    cases.push((big, |e| e == SessionError::TooLarge));
    cases.push((dir.file("missing.vexel"), |e| e == SessionError::Unreadable));
    let junk = dir.file("junk.svg");
    fs::write(&junk, b"<svg \xff").unwrap();
    cases.push((junk, |e| matches!(e, SessionError::InvalidScene(_))));
    cases.push((dir.file("absent.svg"), |e| e == SessionError::Unreadable));

    let mut app = AppDocument::new(160, 120).unwrap();
    rect(&mut app, "mine", 20.0);
    let (source, revision, dirty) = (app.source_bytes().unwrap(), app.revision(), app.is_dirty());
    for (path, expected) in cases {
        let error = app.open_path(&path).unwrap_err();
        assert!(expected(error), "{path:?}: {error:?}");
        assert_eq!(app.source_bytes().unwrap(), source);
        assert_eq!(app.revision(), revision);
        assert_eq!(app.path(), None);
        assert_eq!(app.is_dirty(), dirty);
    }
}

// ---- dirty and title --------------------------------------------------------

#[test]
fn window_title_is_pure() {
    assert_eq!(window_title(None, false), "Vexel");
    assert_eq!(window_title(None, true), "Vexel \u{2014} Untitled \u{2022}");
    assert_eq!(
        window_title(Some("a.vexel"), false),
        "Vexel \u{2014} a.vexel"
    );
    assert_eq!(
        window_title(Some("a.vexel"), true),
        "Vexel \u{2014} a.vexel \u{2022}"
    );
}

#[test]
fn dirty_follows_edit_save_undo_redo_and_reopen() {
    let dir = Dir::new();
    let path = dir.file("d.vexel");
    let mut app = AppDocument::new(100, 100).unwrap();
    assert!(!app.is_dirty());
    rect(&mut app, "r1", 5.0);
    assert!(app.is_dirty());
    assert_eq!(app.title(), "Vexel \u{2014} Untitled \u{2022}");
    app.save_as(path.as_os_str().as_bytes()).unwrap();
    assert!(!app.is_dirty());
    assert_eq!(app.title(), "Vexel \u{2014} d.vexel");
    app.undo().unwrap();
    assert!(app.is_dirty(), "undo past the saved point");
    assert_eq!(app.title(), "Vexel \u{2014} d.vexel \u{2022}");
    app.redo().unwrap();
    assert!(!app.is_dirty(), "redo back to the saved point");
    rect(&mut app, "r2", 50.0);
    assert!(app.is_dirty());
    app.open_path(&path).unwrap();
    assert!(!app.is_dirty());
}

// ---- undo / redo ------------------------------------------------------------

#[test]
fn undo_and_redo_bump_revision_and_restore_frames() {
    let size = (160, 120);
    let mut app = AppDocument::new(160, 120).unwrap();
    let mut view = DocumentView::new();
    let blank = app.source_bytes().unwrap();
    rect(&mut app, "r1", 10.0);
    let one = frame_hash(&app.source_bytes().unwrap(), size);
    let r1 = app.revision();
    rect(&mut app, "r2", 80.0);
    let two_source = app.source_bytes().unwrap();
    let two = frame_hash(&two_source, size);
    assert_ne!(one, two);

    app.undo().unwrap();
    assert!(app.revision() > r1 + 1 - 1 && app.revision() > r1);
    let after_undo = app.revision();
    assert_eq!(shown_hash(&mut view, &app, size), one);
    app.redo().unwrap();
    assert!(app.revision() > after_undo);
    assert_eq!(app.source_bytes().unwrap(), two_source);
    assert_eq!(shown_hash(&mut view, &app, size), two);
    // Nothing to redo: an error, no revision change.
    let before = app.revision();
    assert!(app.redo().is_err());
    assert_eq!(app.revision(), before);
    assert_ne!(blank, two_source);
}

// ---- hooks ------------------------------------------------------------------

struct AddRect(&'static str);
impl DocumentEditor for AddRect {
    type Error = vexel_v1_document::V1DocumentError;
    fn apply(&mut self, doc: &mut vexel_v1_document::V1Document) -> Result<(), Self::Error> {
        doc.add_rectangle_with_id(1.0, 1.0, 20.0, 20.0, "#123456", self.0)
    }
}
struct Nothing;
impl DocumentEditor for Nothing {
    type Error = ();
    fn apply(&mut self, _: &mut vexel_v1_document::V1Document) -> Result<(), ()> {
        Ok(())
    }
}
struct Fails;
impl DocumentEditor for Fails {
    type Error = ();
    fn apply(&mut self, _: &mut vexel_v1_document::V1Document) -> Result<(), ()> {
        Err(())
    }
}

#[derive(Default)]
struct FakeHost(Vec<(Vec<u8>, u64, f64)>);
impl PlaybackHost for FakeHost {
    fn request_frame(&mut self, source: &[u8], revision: u64, seconds: f64) {
        self.0.push((source.to_vec(), revision, seconds));
    }
}

#[test]
fn editors_drive_edit_and_noops_do_not_bump_revision() {
    let mut app = AppDocument::new(100, 100).unwrap();
    assert_eq!(app.apply(&mut AddRect("a")), Ok(EditOutcome::Changed));
    assert_eq!(app.revision(), 1);
    assert_eq!(app.apply(&mut Nothing), Ok(EditOutcome::Unchanged));
    assert_eq!(app.revision(), 1);
    assert_eq!(app.apply(&mut Fails), Err(()));
    assert_eq!(app.revision(), 1);
    // A duplicate stable id is rejected without changing the source.
    assert!(app.apply(&mut AddRect("a")).is_err());
    assert_eq!(app.revision(), 1);
    assert!(app.edit(|_| Ok::<(), ()>(())).is_ok());
    assert_eq!(app.revision(), 1);
}

#[test]
fn playback_host_requests_frames_from_the_session_source() {
    let mut app = AppDocument::new(100, 100).unwrap();
    app.apply(&mut AddRect("a")).unwrap();
    let mut host = FakeHost::default();
    app.request_frame(&mut host, 0.5).unwrap();
    app.request_frame(&mut host, 1.0).unwrap();
    assert_eq!(host.0.len(), 2);
    assert_eq!(host.0[0], (app.source_bytes().unwrap(), 1, 0.5));
    assert_eq!(host.0[1].2, 1.0);
}

#[test]
fn tool_controller_edits_through_the_session_with_the_compose_map() {
    let svg = fixture("compose/compose-v1.svg");
    let mut app = AppDocument::new(10, 10).unwrap();
    app.open_path(&fixture_path("compose/compose-v1.svg"))
        .unwrap();
    let stage = (160, 120);
    let composed = compose_document(&svg, stage.0, stage.1, &|| false).unwrap();
    let map = composed.map;
    let mut tools = ToolController::new();
    tools.set_tool(crate::model::Tool::Rectangle);
    let before = app.revision();
    for input in [
        PointerInput::Down { x: 20.0, y: 40.0 },
        PointerInput::Move { x: 60.0, y: 80.0 },
    ] {
        let result = app.edit(|doc| tools.pointer(input, doc, &map));
        assert!(result.is_ok());
    }
    assert_eq!(
        app.revision(),
        before,
        "a gesture in progress changes nothing"
    );
    let result = app.edit(|doc| tools.pointer(PointerInput::Up { x: 60.0, y: 80.0 }, doc, &map));
    assert_eq!(result, Ok(Outcome::Committed));
    assert_eq!(app.revision(), before + 1);
    assert!(app.is_dirty());
    // Escape with nothing selected leaves the source alone.
    let result = app.edit(|doc| tools.key(Key::Escape, doc));
    assert!(result.is_ok());
}

#[test]
fn point_map_as_canvas_map_agrees_with_hit_testing() {
    let svg = fixture("compose/hit-overlap.svg");
    let composed = compose_document(&svg, 200, 100, &|| false).unwrap();
    let map: &dyn CanvasMap = &composed.map;
    for (x, y) in [
        (40.0, 40.0),
        (100.0, 60.0),
        (150.0, 90.0),
        (-1.0, 5.0),
        (199.0, 99.0),
    ] {
        let via_trait = map.to_doc(x, y);
        assert_eq!(via_trait, composed.map.canvas_to_doc(x, y));
        // The point the tools see is the point the click status reports.
        let stage = Rect {
            x: 10,
            y: 20,
            width: 200,
            height: 100,
        };
        if x >= 0.0 {
            let mut view = DocumentView::new();
            view.set_name("hit");
            let canvas = DocumentView::canvas_point(stage, (10 + x as u32, 20 + y as u32));
            assert_eq!(canvas, Some((x, y)));
            let hit = composed.hit_test(x, y);
            let doc =
                via_trait.map(|(dx, dy)| composed.shapes.iter().rev().find(|s| s.hit(dx, dy)));
            assert_eq!(hit, doc.flatten().map(|s| s.scene_index));
        }
    }
}

#[test]
fn edit_shortcuts_map_undo_and_redo_only() {
    use vexel_platform_apple::Modifiers;
    let cmd = Modifiers(Modifiers::COMMAND);
    let shift_cmd = Modifiers(Modifiers::COMMAND | Modifiers::SHIFT);
    assert_eq!(edit_shortcut(6, cmd), Some(EditAction::Undo));
    assert_eq!(edit_shortcut(6, shift_cmd), Some(EditAction::Redo));
    assert_eq!(edit_shortcut(6, Modifiers(0)), None);
    assert_eq!(
        edit_shortcut(6, Modifiers(Modifiers::COMMAND | Modifiers::OPTION)),
        None
    );
    assert_eq!(edit_shortcut(1, cmd), None);
}
