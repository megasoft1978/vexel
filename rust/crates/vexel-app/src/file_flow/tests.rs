use super::*;
use crate::project_session::shortcut_action;
use std::{
    cell::RefCell,
    collections::VecDeque,
    fs,
    os::unix::{ffi::OsStrExt, fs::PermissionsExt},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU32, Ordering},
};
use vexel_platform_apple::Modifiers;
use vexel_v1_document::V1Document;

/// Scripted stand-in for the AppKit dialogs; records every call.
#[derive(Default)]
struct Fake {
    open: RefCell<VecDeque<PanelResult>>,
    save: RefCell<VecDeque<PanelResult>>,
    confirm: RefCell<VecDeque<UnsavedChoice>>,
    calls: RefCell<Vec<String>>,
    alerts: RefCell<Vec<(String, String)>>,
}
impl PanelBackend for Fake {
    fn open_panel(&self, extensions: &[&str]) -> PanelResult {
        self.calls
            .borrow_mut()
            .push(format!("open:{}", extensions.join(",")));
        self.open
            .borrow_mut()
            .pop_front()
            .expect("unscripted open panel")
    }
    fn save_panel(&self, suggested_name: &str) -> PanelResult {
        self.calls
            .borrow_mut()
            .push(format!("save:{suggested_name}"));
        self.save
            .borrow_mut()
            .pop_front()
            .expect("unscripted save panel")
    }
    fn confirm_unsaved(&self, text: &UnsavedText<'_>) -> UnsavedChoice {
        self.calls
            .borrow_mut()
            .push(format!("confirm:{}", text.save));
        self.confirm
            .borrow_mut()
            .pop_front()
            .expect("unscripted confirm")
    }
    fn show_error(&self, title: &str, message: &str) {
        self.alerts
            .borrow_mut()
            .push((title.to_owned(), message.to_owned()));
    }
}
impl Fake {
    fn calls(&self) -> Vec<String> {
        self.calls.borrow().clone()
    }
    fn picks_open(&self, path: &Path) {
        self.open
            .borrow_mut()
            .push_back(PanelResult::Path(path.as_os_str().as_bytes().to_vec()));
    }
    fn picks_save(&self, path: &Path) {
        self.save
            .borrow_mut()
            .push_back(PanelResult::Path(path.as_os_str().as_bytes().to_vec()));
    }
}

struct Dir(PathBuf);
impl Dir {
    fn new() -> Self {
        static N: AtomicU32 = AtomicU32::new(0);
        let path = std::env::temp_dir().join(format!(
            "vexel-flow-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
    fn file(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}
impl Drop for Dir {
    fn drop(&mut self) {
        let _ = fs::set_permissions(&self.0, fs::Permissions::from_mode(0o755));
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn bytes(path: &Path) -> &[u8] {
    path.as_os_str().as_bytes()
}

fn drawn() -> ProjectSession {
    let mut doc = V1Document::new(200, 100).unwrap();
    doc.add_rectangle_with_id(10.0, 10.0, 60.0, 50.0, "#ff0000", "r1")
        .unwrap();
    ProjectSession::new(doc)
}
fn blank() -> ProjectSession {
    ProjectSession::new(V1Document::new(64, 64).unwrap())
}
/// Writes a valid project and returns its bytes and the scene source saved.
fn fixture(dir: &Dir, name: &str) -> (PathBuf, Vec<u8>, Vec<u8>) {
    let path = dir.file(name);
    let mut session = drawn();
    session.save_as(bytes(&path)).unwrap();
    let source = session.document().source().unwrap();
    (path.clone(), fs::read(&path).unwrap(), source)
}

// AC1

#[test]
fn save_without_path_shows_save_panel_then_writes() {
    let dir = Dir::new();
    let target = dir.file("new.vexel");
    let backend = Fake::default();
    backend.picks_save(&target);
    let mut session = drawn();
    assert_eq!(
        handle(FileAction::Save, &mut session, &backend),
        FlowOutcome::Saved
    );
    assert_eq!(backend.calls(), ["save:Untitled.vexel"]);
    assert_eq!(session.path(), Some(bytes(&target)));
    assert!(!session.is_dirty());
    let reopened = ProjectSession::open(bytes(&target)).unwrap();
    assert_eq!(
        reopened.document().source().unwrap(),
        session.document().source().unwrap()
    );
}

#[test]
fn save_with_path_writes_without_any_panel() {
    let dir = Dir::new();
    let target = dir.file("a.vexel");
    let mut session = drawn();
    session.save_as(bytes(&target)).unwrap();
    session
        .document_mut()
        .add_ellipse_with_id(1.0, 1.0, 9.0, 9.0, "#00ff00", "e1")
        .unwrap();
    assert!(session.is_dirty());
    let backend = Fake::default();
    assert_eq!(
        handle(FileAction::Save, &mut session, &backend),
        FlowOutcome::Saved
    );
    assert!(backend.calls().is_empty());
    assert!(!session.is_dirty());
    let reopened = ProjectSession::open(bytes(&target)).unwrap();
    assert_eq!(
        reopened.document().source().unwrap(),
        session.document().source().unwrap()
    );
}

#[test]
fn save_as_always_shows_the_panel_and_updates_the_path() {
    let dir = Dir::new();
    let (first, _, _) = fixture(&dir, "first.vexel");
    let second = dir.file("second.vexel");
    let mut session = ProjectSession::open(bytes(&first)).unwrap();
    let backend = Fake::default();
    backend.picks_save(&second);
    assert_eq!(
        handle(FileAction::SaveAs, &mut session, &backend),
        FlowOutcome::Saved
    );
    assert_eq!(backend.calls(), ["save:first.vexel"]);
    assert_eq!(session.path(), Some(bytes(&second)));
    assert!(second.exists());
}

#[test]
fn cancelling_any_panel_changes_nothing() {
    let dir = Dir::new();
    let (path, on_disk, _) = fixture(&dir, "keep.vexel");
    let mut session = ProjectSession::open(bytes(&path)).unwrap();
    session
        .document_mut()
        .add_ellipse_with_id(1.0, 1.0, 9.0, 9.0, "#00ff00", "e1")
        .unwrap();
    let before = session.document().source().unwrap();
    for (action, script) in [(FileAction::SaveAs, "save"), (FileAction::Open, "open")] {
        let backend = Fake::default();
        if script == "save" {
            backend.save.borrow_mut().push_back(PanelResult::Cancelled);
        } else {
            backend
                .confirm
                .borrow_mut()
                .push_back(UnsavedChoice::DontSave);
            backend.open.borrow_mut().push_back(PanelResult::Cancelled);
        }
        assert_eq!(
            handle(action, &mut session, &backend),
            FlowOutcome::Cancelled
        );
        assert_eq!(session.path(), Some(bytes(&path)));
        assert_eq!(session.document().source().unwrap(), before);
        assert!(session.is_dirty());
        assert_eq!(fs::read(&path).unwrap(), on_disk);
        assert!(backend.alerts.borrow().is_empty());
    }
    // Save on a path-less document whose panel is cancelled stays dirty, unsaved.
    let mut fresh = drawn();
    let backend = Fake::default();
    backend.save.borrow_mut().push_back(PanelResult::Cancelled);
    assert_eq!(
        handle(FileAction::Save, &mut fresh, &backend),
        FlowOutcome::Cancelled
    );
    assert_eq!(fresh.path(), None);
    assert!(fresh.is_dirty());
}

// AC2

#[test]
fn open_on_a_clean_session_loads_the_fixture() {
    let dir = Dir::new();
    let (path, _, saved) = fixture(&dir, "fx.vexel");
    let mut session = blank();
    assert!(!session.is_dirty());
    let backend = Fake::default();
    backend.picks_open(&path);
    assert_eq!(
        handle(FileAction::Open, &mut session, &backend),
        FlowOutcome::Opened
    );
    assert_eq!(backend.calls(), ["open:vexel"]);
    assert_eq!(session.document().source().unwrap(), saved);
    assert_eq!(session.path(), Some(bytes(&path)));
    assert!(!session.is_dirty());
}

#[test]
fn open_on_a_dirty_session_prompts_first() {
    let dir = Dir::new();
    let (target, _, saved) = fixture(&dir, "fx.vexel");

    // Don't Save: the edit is discarded and the file opens.
    let mut session = drawn();
    let backend = Fake::default();
    backend
        .confirm
        .borrow_mut()
        .push_back(UnsavedChoice::DontSave);
    backend.picks_open(&target);
    assert_eq!(
        handle(FileAction::Open, &mut session, &backend),
        FlowOutcome::Opened
    );
    assert_eq!(backend.calls(), ["confirm:Save As\u{2026}", "open:vexel"]);
    assert_eq!(session.document().source().unwrap(), saved);

    // Cancel: nothing happens, no panel.
    let mut session = drawn();
    let before = session.document().source().unwrap();
    let backend = Fake::default();
    backend
        .confirm
        .borrow_mut()
        .push_back(UnsavedChoice::Cancel);
    assert_eq!(
        handle(FileAction::Open, &mut session, &backend),
        FlowOutcome::Cancelled
    );
    assert_eq!(backend.calls(), ["confirm:Save As\u{2026}"]);
    assert_eq!(session.document().source().unwrap(), before);
    assert!(session.is_dirty() && session.path().is_none());

    // Save (no path): save panel, write, then the open panel.
    let mut session = drawn();
    let edited = session.document().source().unwrap();
    let saved_to = dir.file("mine.vexel");
    let backend = Fake::default();
    backend.confirm.borrow_mut().push_back(UnsavedChoice::Save);
    backend.picks_save(&saved_to);
    backend.picks_open(&target);
    assert_eq!(
        handle(FileAction::Open, &mut session, &backend),
        FlowOutcome::Opened
    );
    assert_eq!(
        backend.calls(),
        [
            "confirm:Save As\u{2026}",
            "save:Untitled.vexel",
            "open:vexel"
        ]
    );
    assert_eq!(
        ProjectSession::open(bytes(&saved_to))
            .unwrap()
            .document()
            .source()
            .unwrap(),
        edited
    );
    assert_eq!(session.path(), Some(bytes(&target)));
}

#[test]
fn failed_save_during_open_keeps_the_document_and_aborts() {
    let dir = Dir::new();
    let (target, _, _) = fixture(&dir, "fx.vexel");
    let locked = dir.file("locked");
    fs::create_dir(&locked).unwrap();
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o555)).unwrap();
    let mut session = drawn();
    let before = session.document().source().unwrap();
    let backend = Fake::default();
    backend.confirm.borrow_mut().push_back(UnsavedChoice::Save);
    backend.picks_save(&locked.join("x.vexel"));
    let outcome = handle(FileAction::Open, &mut session, &backend);
    assert_eq!(outcome, FlowOutcome::Failed(SessionError::WriteFailed));
    assert!(!backend.calls().iter().any(|c| c.starts_with("open:")));
    assert_eq!(session.document().source().unwrap(), before);
    assert!(session.is_dirty() && session.path().is_none());
    assert_eq!(backend.alerts.borrow().len(), 1);
    assert!(target.exists());
}

// AC3

#[test]
fn close_request_follows_the_dirty_state() {
    let backend = Fake::default();
    assert_eq!(
        handle_close(&mut blank(), &backend),
        FlowOutcome::CloseWindow
    );
    assert!(backend.calls().is_empty());

    let mut dirty = drawn();
    backend
        .confirm
        .borrow_mut()
        .push_back(UnsavedChoice::Cancel);
    assert_eq!(handle_close(&mut dirty, &backend), FlowOutcome::KeepOpen);
    assert!(dirty.is_dirty());

    backend
        .confirm
        .borrow_mut()
        .push_back(UnsavedChoice::DontSave);
    assert_eq!(handle_close(&mut dirty, &backend), FlowOutcome::CloseWindow);

    let dir = Dir::new();
    let target = dir.file("c.vexel");
    backend.confirm.borrow_mut().push_back(UnsavedChoice::Save);
    backend.picks_save(&target);
    assert_eq!(handle_close(&mut dirty, &backend), FlowOutcome::CloseWindow);
    assert!(target.exists() && !dirty.is_dirty());

    // Save panel cancelled during close keeps the window open.
    let mut other = drawn();
    backend.confirm.borrow_mut().push_back(UnsavedChoice::Save);
    backend.save.borrow_mut().push_back(PanelResult::Cancelled);
    assert_eq!(handle_close(&mut other, &backend), FlowOutcome::KeepOpen);
    assert!(other.is_dirty());
}

// AC4

#[test]
fn failures_alert_with_the_user_message_and_change_nothing() {
    let dir = Dir::new();
    let (_, good, _) = fixture(&dir, "good.vexel");
    let mut cases: Vec<(PathBuf, SessionError)> = Vec::new();
    let truncated = dir.file("trunc.vexel");
    fs::write(&truncated, &good[..good.len() - 5]).unwrap();
    cases.push((truncated, SessionError::Corrupt { offset: 0 }));
    let mut bad = good.clone();
    bad[0] ^= 0xff;
    let magic = dir.file("magic.vexel");
    fs::write(&magic, &bad).unwrap();
    cases.push((magic, SessionError::NotAProject));
    let mut bad = good.clone();
    bad[8] = 2;
    let version = dir.file("version.vexel");
    fs::write(&version, &bad).unwrap();
    cases.push((version, SessionError::UnsupportedVersion));
    cases.push((dir.file("missing.vexel"), SessionError::Unreadable));

    let mut session = drawn();
    session.save_as(bytes(&dir.file("live.vexel"))).unwrap();
    let live_bytes = fs::read(dir.file("live.vexel")).unwrap();
    let live_source = session.document().source().unwrap();
    for (path, expected) in cases {
        let backend = Fake::default();
        backend.picks_open(&path);
        let outcome = handle(FileAction::Open, &mut session, &backend);
        let FlowOutcome::Failed(error) = outcome else {
            panic!("expected failure for {path:?}, got {outcome:?}");
        };
        assert_eq!(
            std::mem::discriminant(&error),
            std::mem::discriminant(&expected)
        );
        let alerts = backend.alerts.borrow();
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].1, error.user_message());
        assert_eq!(outcome.status(), Some(error.user_message()));
        assert_eq!(session.document().source().unwrap(), live_source);
        assert_eq!(session.path(), Some(bytes(&dir.file("live.vexel"))));
        assert_eq!(fs::read(dir.file("live.vexel")).unwrap(), live_bytes);
    }

    // Unwritable directory on Save As.
    let locked = dir.file("locked");
    fs::create_dir(&locked).unwrap();
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o555)).unwrap();
    let backend = Fake::default();
    backend.picks_save(&locked.join("x.vexel"));
    let outcome = handle(FileAction::SaveAs, &mut session, &backend);
    assert_eq!(outcome, FlowOutcome::Failed(SessionError::WriteFailed));
    assert_eq!(
        backend.alerts.borrow()[0].1,
        SessionError::WriteFailed.user_message()
    );
    assert_eq!(session.path(), Some(bytes(&dir.file("live.vexel"))));
    assert_eq!(fs::read(dir.file("live.vexel")).unwrap(), live_bytes);
}

#[test]
fn a_panel_error_alerts_instead_of_acting() {
    let backend = Fake::default();
    backend.save.borrow_mut().push_back(PanelResult::Error);
    let mut session = drawn();
    let outcome = handle(FileAction::SaveAs, &mut session, &backend);
    assert_eq!(outcome, FlowOutcome::Failed(SessionError::InvalidPath));
    assert_eq!(backend.alerts.borrow().len(), 1);
    assert!(session.is_dirty() && session.path().is_none());
}

// AC5

fn mods(shift: bool) -> Modifiers {
    Modifiers(Modifiers::COMMAND | if shift { Modifiers::SHIFT } else { 0 })
}

#[test]
fn menu_table_drives_shortcuts_titles_and_keys() {
    for (tag, entry) in FILE_MENU.iter().enumerate() {
        assert_eq!(
            shortcut_action(entry.key_code, mods(entry.shift)),
            Some(entry.action)
        );
        assert_eq!(action_for_tag(tag as u32), Some(entry.action));
        let spec = menu_specs()[tag];
        assert_eq!(
            (spec.title, spec.key, spec.shift, spec.tag),
            (entry.title, entry.key, entry.shift, tag as u32)
        );
    }
    let titles: Vec<_> = FILE_MENU
        .iter()
        .map(|e| (e.title, e.key, e.shift))
        .collect();
    assert_eq!(
        titles,
        [
            ("Open\u{2026}", "o", false),
            ("Save", "s", false),
            ("Save As\u{2026}", "s", true)
        ]
    );
    assert_eq!(action_for_tag(3), None);
}

#[test]
fn near_miss_shortcuts_map_to_nothing() {
    let s = 1;
    assert_eq!(shortcut_action(s, mods(false)), Some(FileAction::Save));
    assert_eq!(shortcut_action(s, mods(true)), Some(FileAction::SaveAs));
    assert_eq!(shortcut_action(31, mods(false)), Some(FileAction::Open));
    let cmd_opt = Modifiers(Modifiers::COMMAND | Modifiers::OPTION);
    let ctrl = Modifiers(Modifiers::CONTROL);
    assert_eq!(shortcut_action(s, cmd_opt), None);
    assert_eq!(shortcut_action(s, ctrl), None);
    assert_eq!(shortcut_action(s, Modifiers::default()), None);
    assert_eq!(shortcut_action(31, ctrl), None);
    assert_eq!(shortcut_action(31, mods(true)), None);
}

#[test]
fn menu_enablement_table() {
    let dir = Dir::new();
    let path = dir.file("e.vexel");
    // (dirty, has path) -> [Open, Save, Save As]
    let untitled_clean = blank();
    assert_eq!(menu_enabled(&untitled_clean), [true, true, true]);
    let untitled_dirty = drawn();
    assert_eq!(menu_enabled(&untitled_dirty), [true, true, true]);
    let mut saved = drawn();
    saved.save_as(bytes(&path)).unwrap();
    assert_eq!(menu_enabled(&saved), [true, false, true]);
    saved
        .document_mut()
        .add_ellipse_with_id(1.0, 1.0, 9.0, 9.0, "#00ff00", "e1")
        .unwrap();
    assert_eq!(menu_enabled(&saved), [true, true, true]);
}
