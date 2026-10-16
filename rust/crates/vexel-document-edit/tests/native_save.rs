//! Original test_history_and_save fixtures use the required .cache/edit-* paths.
//! Files are retained as evidence (no delete/reset step).
mod common;
use common::*;
use std::{
    fs, io,
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
};
use vexel_document_edit::*;
use vexel_document_io::{SourceSaveError, SourceSaveHooks};
static NEXT: AtomicUsize = AtomicUsize::new(0);
fn directory() -> PathBuf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .unwrap();
    let path = root.join(format!(
        ".cache/edit-save-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&path).unwrap();
    path
}
fn path_bytes(path: &Path) -> &[u8] {
    use std::os::unix::ffi::OsStrExt;
    path.as_os_str().as_bytes()
}
#[test]
fn test_history_and_save() {
    let original = b"<svg><rect id='r' x='0'/></svg>";
    let mut edit = create(original);
    set(&mut edit, b"r", b"x", b"1");
    set(&mut edit, b"r", b"x", b"2");
    assert_eq!(edit.history_count(), 1);
    edit.undo().unwrap();
    assert_eq!(bytes(&edit), original);
    assert!(!edit.is_dirty());
    edit.redo().unwrap();
    assert!(contains(&bytes(&edit), b"x='2'"));
    edit.undo().unwrap();
    let dir = directory();
    let path = dir.join("existing.svg");
    fs::write(&path, original).unwrap();
    set(&mut edit, b"r", b"x", b"3");
    let saved = edit.save_as(path_bytes(&path), None);
    assert_eq!(saved.error, EditError::Ok);
    assert_eq!(saved.source_error, SourceSaveError::Ok);
    assert_eq!(saved.native_error_code, 0);
    assert!(saved.renamed && saved.file_synced && saved.directory_synced);
    assert!(!edit.is_dirty());
    assert!(contains(&fs::read(&path).unwrap(), b"x='3'"));
    let new_path = dir.join("new.svg");
    set(&mut edit, b"r", b"x", b"4");
    let saved = edit.save_as(path_bytes(&new_path), None);
    assert_eq!(saved.error, EditError::Ok);
    assert!(saved.renamed);
    assert!(!edit.is_dirty());
    assert!(contains(&fs::read(&new_path).unwrap(), b"x='4'"));
    edit.undo().unwrap();
    assert!(edit.is_dirty());
}
struct FailWrite;
impl SourceSaveHooks for FailWrite {
    fn before_write(&mut self, _: &fs::File, _: &[u8]) -> io::Result<usize> {
        Err(io::Error::other("injected write gate"))
    }
}
struct FailDirectory;
impl SourceSaveHooks for FailDirectory {
    fn before_sync_directory(&mut self, _: &Path) -> io::Result<()> {
        Err(io::Error::other("injected directory gate"))
    }
}
#[test]
fn save_failures_preserve_state_and_postrename_warning_marks_saved() {
    let original = b"<svg><rect id='r' x='0'/></svg>";
    let mut edit = create(original);
    let dir = directory();
    let path = dir.join("warning.svg");
    fs::write(&path, original).unwrap();
    set(&mut edit, b"r", b"x", b"1");
    let handle = id(&edit, b"r");
    let before = bytes(&edit);
    let cursor = edit.history_cursor();
    let rejected = edit.save_as(path_bytes(&path), Some(&mut FailWrite));
    assert_eq!(rejected.error, EditError::SaveFailed);
    assert_eq!(rejected.source_error, SourceSaveError::Destination);
    assert_eq!(rejected.native_error_code, 5);
    assert!(!rejected.renamed);
    assert!(edit.is_dirty());
    assert_eq!(fs::read(&path).unwrap(), original);
    assert_eq!(bytes(&edit), before);
    assert_eq!(edit.history_cursor(), cursor);
    assert!(edit.tag_name(handle).is_some());
    assert_eq!(rejected.diagnostic, vexel_xml::XmlDiagnostic::default());
    let warning = edit.save_as(path_bytes(&path), Some(&mut FailDirectory));
    assert_eq!(warning.error, EditError::SaveFailed);
    assert_eq!(warning.source_error, SourceSaveError::Durability);
    assert_eq!(warning.native_error_code, 6);
    assert!(warning.renamed && warning.file_synced);
    assert!(!warning.directory_synced);
    assert!(!edit.is_dirty());
    assert_eq!(fs::read(&path).unwrap(), before);
    assert_eq!(edit.tag_name(handle), Some(&b"rect"[..]));
    let empty = edit.save_as(b"", None);
    assert_eq!(empty.error, EditError::InvalidArgument);
    assert!(!empty.renamed);
    assert!(!edit.is_dirty());
    set(&mut edit, b"r", b"x", b"2");
    let missing = dir.join("missing-parent/file.svg");
    let rejected = edit.save_as(path_bytes(&missing), None);
    assert_eq!(rejected.error, EditError::SaveFailed);
    assert!(edit.is_dirty());
    let long = edit.save_as(&vec![b'x'; 1024], None);
    assert_eq!(long.source_error, SourceSaveError::PathLimit);
    assert_eq!(long.diagnostic, vexel_xml::XmlDiagnostic::default());
    assert!(edit.is_dirty());
}
