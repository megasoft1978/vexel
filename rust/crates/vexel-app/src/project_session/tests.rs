use super::*;
use crate::preview::{Operation, PreviewState, Shape};
use std::{
    fs,
    os::unix::{ffi::OsStrExt, fs::PermissionsExt},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU32, Ordering},
};
use vexel_platform_apple::Modifiers;

const FOREIGN: &[u8] = include_bytes!("../../../../fixtures/v1/save/foreign.svg");
const ANIM: &[u8] = b"<animate attributeName=\"x\" values=\"10;20\" dur=\"1s\"/>";

struct Dir(PathBuf);
impl Dir {
    fn new() -> Self {
        static N: AtomicU32 = AtomicU32::new(0);
        let path = std::env::temp_dir().join(format!(
            "vexel-save-{}-{}",
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
    doc.add_ellipse_with_id(70.0, 20.0, 150.0, 90.0, "#00ff00", "e1")
        .unwrap();
    doc.add_animation_fragment("r1", ANIM).unwrap();
    ProjectSession::new(doc)
}

#[test]
fn round_trip_preserves_source_ids_order_and_animation() {
    let dir = Dir::new();
    let path = dir.file("a.vexel");
    let mut session = drawn();
    assert!(session.is_dirty());
    session.save_as(bytes(&path)).unwrap();
    assert!(!session.is_dirty());
    let saved = session.document().source().unwrap();
    let opened = ProjectSession::open(bytes(&path)).unwrap();
    assert_eq!(opened.document().source().unwrap(), saved);
    assert!(!opened.is_dirty());
    assert_eq!(opened.path(), Some(bytes(&path)));
    assert_eq!(
        (opened.document().width(), opened.document().height()),
        (200, 100)
    );
    let text = String::from_utf8(saved).unwrap();
    let r1 = text.find("id=\"r1\"").unwrap();
    let e1 = text.find("id=\"e1\"").unwrap();
    assert!(r1 < e1);
    assert!(text.contains(std::str::from_utf8(ANIM).unwrap()));
    assert!(text.find("<animate").unwrap() > r1 && text.find("<animate").unwrap() < e1);
}

#[test]
fn unknown_source_survives_byte_for_byte() {
    let dir = Dir::new();
    let path = dir.file("f.vexel");
    let mut doc = V1Document::from_source(FOREIGN).unwrap();
    assert_eq!(doc.source().unwrap(), FOREIGN);
    doc.add_rectangle_with_id(5.0, 5.0, 25.0, 25.0, "#abcdef", "new1")
        .unwrap();
    let mut session = ProjectSession::new(doc);
    session.save_as(bytes(&path)).unwrap();
    let opened = ProjectSession::open(bytes(&path)).unwrap();
    let after = opened.document().source().unwrap();
    assert_eq!(after, session.document().source().unwrap());
    // The edit touches only the range before the closing tag.
    let tail = b"</svg>\n";
    assert!(FOREIGN.ends_with(tail));
    let head = &FOREIGN[..FOREIGN.len() - tail.len()];
    assert!(after.starts_with(head), "prefix bytes changed");
    assert!(after.ends_with(tail), "suffix bytes changed");
    assert!(after.len() > FOREIGN.len());
    for marker in [
        &b"x:note=\"keep &amp; me\""[..],
        b"<inkscape:editor-state zoom=\"2.5\" a='single'/>",
        b"<x:unknown-thing  weird = \"1\"   data-z=\"9\">",
        b"<!-- leading comment -->",
    ] {
        assert!(after.windows(marker.len()).any(|w| w == marker));
    }
}

#[test]
fn unedited_foreign_source_saves_and_reopens_identically() {
    let dir = Dir::new();
    let path = dir.file("g.vexel");
    let mut session = ProjectSession::new(V1Document::from_source(FOREIGN).unwrap());
    session.save_as(bytes(&path)).unwrap();
    let opened = ProjectSession::open(bytes(&path)).unwrap();
    assert_eq!(opened.document().source().unwrap(), FOREIGN);
}

#[test]
fn undo_save_redo_and_history_cursor() {
    let dir = Dir::new();
    let path = dir.file("u.vexel");
    let mut session = drawn();
    let full = session.document().source().unwrap();
    assert_eq!(session.document().history_cursor(), 3);
    session.document_mut().undo().unwrap(); // drop animation
    let without_anim = session.document().source().unwrap();
    session.save_as(bytes(&path)).unwrap();
    // Saving neither resets nor corrupts the stack.
    assert_eq!(session.document().history_count(), 3);
    assert_eq!(session.document().history_cursor(), 2);
    assert!(!session.is_dirty());
    session.document_mut().redo().unwrap();
    assert_eq!(session.document().history_cursor(), 3);
    assert_eq!(session.document().source().unwrap(), full);
    assert!(session.is_dirty());
    session.document_mut().undo().unwrap();
    assert_eq!(session.document().source().unwrap(), without_anim);
    assert!(!session.is_dirty());
    // File holds the state at save time.
    let opened = ProjectSession::open(bytes(&path)).unwrap();
    assert_eq!(opened.document().source().unwrap(), without_anim);
}

#[test]
fn edit_undo_edit_save_reopen_yields_post_edit_state() {
    let dir = Dir::new();
    let path = dir.file("e.vexel");
    let mut doc = V1Document::new(100, 100).unwrap();
    doc.add_rectangle(1.0, 1.0, 20.0, 20.0, "#111111").unwrap();
    doc.undo().unwrap();
    doc.add_ellipse(30.0, 30.0, 60.0, 60.0, "#222222").unwrap();
    let post = doc.source().unwrap();
    let mut session = ProjectSession::new(doc);
    session.save_as(bytes(&path)).unwrap();
    let opened = ProjectSession::open(bytes(&path)).unwrap();
    assert_eq!(opened.document().source().unwrap(), post);
    // Reopen state: history is session-only, so it is reset.
    assert_eq!(opened.document().history_count(), 0);
    assert_eq!(opened.document().history_cursor(), 0);
    assert!(!opened.is_dirty());
}

#[test]
fn dirty_tracking_edit_save_and_undo_past_saved_point() {
    let dir = Dir::new();
    let path = dir.file("d.vexel");
    let mut session = ProjectSession::new(V1Document::new(100, 100).unwrap());
    assert!(!session.is_dirty());
    session
        .document_mut()
        .add_rectangle(1.0, 1.0, 9.0, 9.0, "#000000")
        .unwrap();
    assert!(session.is_dirty());
    session.save_as(bytes(&path)).unwrap();
    assert!(!session.is_dirty());
    session
        .document_mut()
        .add_ellipse(2.0, 2.0, 8.0, 8.0, "#000000")
        .unwrap();
    assert!(session.is_dirty());
    session.save().unwrap();
    assert!(!session.is_dirty());
    session.document_mut().undo().unwrap();
    assert!(session.is_dirty());
    session.document_mut().undo().unwrap();
    assert!(session.is_dirty());
    session.document_mut().redo().unwrap();
    session.document_mut().redo().unwrap();
    assert!(!session.is_dirty());
}

#[test]
fn save_without_path_is_typed_error_and_keeps_dirty() {
    let mut session = drawn();
    assert_eq!(session.save(), Err(SessionError::NoPath));
    assert!(session.is_dirty());
}

fn valid_file(dir: &Dir, name: &str) -> (PathBuf, Vec<u8>) {
    let path = dir.file(name);
    drawn().save_as(bytes(&path)).unwrap();
    let data = fs::read(&path).unwrap();
    (path, data)
}

fn assert_open_fails(path: &Path, expected: impl Fn(SessionError) -> bool) {
    let mut session = drawn();
    let source = session.document().source().unwrap();
    let (count, cursor, dirty) = (
        session.document().history_count(),
        session.document().history_cursor(),
        session.is_dirty(),
    );
    let error = session.open_replacing(bytes(path)).unwrap_err();
    assert!(expected(error), "{error:?}");
    assert!(!error.user_message().is_empty());
    assert_eq!(session.document().source().unwrap(), source);
    assert_eq!(session.document().history_count(), count);
    assert_eq!(session.document().history_cursor(), cursor);
    assert_eq!(session.is_dirty(), dirty);
    assert_eq!(session.path(), None);
}

#[test]
fn failed_opens_are_typed_and_leave_the_document_untouched() {
    let dir = Dir::new();
    let (_, good) = valid_file(&dir, "good.vexel");

    let truncated = dir.file("trunc.vexel");
    fs::write(&truncated, &good[..good.len() - 5]).unwrap();
    assert_open_fails(&truncated, |e| matches!(e, SessionError::Corrupt { .. }));
    let tiny = dir.file("tiny.vexel");
    fs::write(&tiny, &good[..20]).unwrap();
    assert_open_fails(&tiny, |e| matches!(e, SessionError::Corrupt { .. }));

    let mut bad = good.clone();
    bad[0] ^= 0xff;
    let magic = dir.file("magic.vexel");
    fs::write(&magic, &bad).unwrap();
    assert_open_fails(&magic, |e| e == SessionError::NotAProject);

    let mut bad = good.clone();
    bad[8] = 2;
    let version = dir.file("version.vexel");
    fs::write(&version, &bad).unwrap();
    assert_open_fails(&version, |e| e == SessionError::UnsupportedVersion);

    let mut bad = good.clone();
    let scene = project::MAX_SCENE_BYTES as u64 + 1;
    bad[24..32].copy_from_slice(&scene.to_le_bytes());
    let limit = dir.file("limit.vexel");
    fs::write(&limit, &bad).unwrap();
    assert_open_fails(&limit, |e| e == SessionError::TooLarge);

    let oversize = dir.file("big.vexel");
    let file = fs::File::create(&oversize).unwrap();
    file.set_len(project::MAX_BYTES as u64 + 1).unwrap(); // sparse
    drop(file);
    assert_open_fails(&oversize, |e| e == SessionError::TooLarge);

    assert_open_fails(&dir.file("missing.vexel"), |e| {
        e == SessionError::Unreadable
    });
    assert_open_fails(&dir.0, |e| e == SessionError::InvalidPath);
    assert_open_fails(Path::new(""), |e| e == SessionError::InvalidPath);
}

#[test]
fn valid_bundle_with_non_svg_scene_is_invalid_scene() {
    let dir = Dir::new();
    let path = dir.file("scene.vexel");
    write_project(bytes(&path), b"not xml", 10, 10).unwrap();
    assert_open_fails(&path, |e| matches!(e, SessionError::InvalidScene(_)));
}

#[test]
fn unwritable_directory_fails_without_touching_state_or_existing_file() {
    let dir = Dir::new();
    let path = dir.file("keep.vexel");
    let mut original = drawn();
    original.save_as(bytes(&path)).unwrap();
    let before = fs::read(&path).unwrap();

    let mut session = drawn();
    session
        .document_mut()
        .add_rectangle(1.0, 1.0, 5.0, 5.0, "#010101")
        .unwrap();
    let source = session.document().source().unwrap();
    session.path = Some(bytes(&path).to_vec());
    fs::set_permissions(&dir.0, fs::Permissions::from_mode(0o555)).unwrap();
    let replace = session.save();
    let create = session.save_as(bytes(&dir.file("new.vexel")));
    fs::set_permissions(&dir.0, fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(replace, Err(SessionError::WriteFailed));
    assert_eq!(create, Err(SessionError::WriteFailed));
    assert!(session.is_dirty());
    assert_eq!(session.document().source().unwrap(), source);
    assert_eq!(session.path(), Some(bytes(&path)));
    assert_eq!(fs::read(&path).unwrap(), before, "existing file changed");
    assert!(!dir.file("new.vexel").exists());
    let leftovers: Vec<_> = fs::read_dir(&dir.0).unwrap().collect();
    assert_eq!(leftovers.len(), 1, "temp file left behind");
}

#[test]
fn missing_directory_and_bad_paths_fail_typed() {
    let dir = Dir::new();
    let mut session = drawn();
    let nested = dir.file("nope/x.vexel");
    assert_eq!(
        session.save_as(bytes(&nested)),
        Err(SessionError::WriteFailed)
    );
    assert_eq!(session.save_as(b""), Err(SessionError::InvalidPath));
    assert_eq!(session.save_as(b"a\0b"), Err(SessionError::InvalidPath));
    assert!(session.is_dirty());
    assert_eq!(session.path(), None);
}

#[test]
fn resave_replaces_atomically_and_leaves_no_temp_files() {
    let dir = Dir::new();
    let path = dir.file("r.vexel");
    let mut session = drawn();
    session.save_as(bytes(&path)).unwrap();
    session
        .document_mut()
        .add_rectangle(1.0, 1.0, 5.0, 5.0, "#010101")
        .unwrap();
    session.save().unwrap();
    assert_eq!(fs::read_dir(&dir.0).unwrap().count(), 1);
    let opened = ProjectSession::open(bytes(&path)).unwrap();
    assert_eq!(
        opened.document().source().unwrap(),
        session.document().source().unwrap()
    );
}

#[test]
fn mcp_preview_state_keeps_working_after_save_and_open() {
    let dir = Dir::new();
    let path = dir.file("p.vexel");
    let shape = Shape::Rect {
        x: 10.0,
        y: 10.0,
        width: 50.0,
        height: 40.0,
        rgb: 0xff8800,
    };
    let (state, _) = PreviewState::new()
        .unwrap()
        .stage(&Operation::Add(shape))
        .unwrap();
    let source = state.source().unwrap();
    write_project(bytes(&path), &source, 512, 512).unwrap();
    let opened = ProjectSession::open(bytes(&path)).unwrap();
    assert_eq!(opened.document().source().unwrap(), source);
    let (state, _) = state.stage(&Operation::Add(shape)).unwrap();
    assert_eq!(state.shapes().len(), 2);
    let (state, _) = state.stage(&Operation::Undo).unwrap();
    assert_eq!(state.source().unwrap(), source);
    let (state, _) = state.stage(&Operation::Redo).unwrap();
    assert_eq!(state.shapes().len(), 2);
}

#[test]
fn shortcuts_map_to_file_actions() {
    let cmd = Modifiers(Modifiers::COMMAND);
    let cmd_shift = Modifiers(Modifiers::COMMAND | Modifiers::SHIFT);
    assert_eq!(shortcut_action(1, cmd), Some(FileAction::Save));
    assert_eq!(shortcut_action(1, cmd_shift), Some(FileAction::SaveAs));
    assert_eq!(shortcut_action(31, cmd), Some(FileAction::Open));
    assert_eq!(shortcut_action(1, Modifiers(0)), None);
    assert_eq!(
        shortcut_action(1, Modifiers(Modifiers::COMMAND | Modifiers::OPTION)),
        None
    );
    assert_eq!(shortcut_action(31, cmd_shift), None);
    assert_eq!(shortcut_action(2, cmd), None);
}

#[test]
fn unsaved_changes_prompt_text() {
    let named = unsaved_changes_prompt(Some("art.vexel"), FileAction::Open);
    assert_eq!(
        named.message,
        "Do you want to save the changes made to \u{201c}art.vexel\u{201d}?"
    );
    assert_eq!(named.confirm, "Save");
    assert_eq!(named.discard, "Don\u{2019}t Save");
    assert_eq!(named.cancel, "Cancel");
    assert!(named.detail.contains("opening another project"));
    let untitled = unsaved_changes_prompt(None, FileAction::Save);
    assert!(untitled.message.contains("Untitled"));
    assert_eq!(untitled.confirm, "Save As\u{2026}");
}
