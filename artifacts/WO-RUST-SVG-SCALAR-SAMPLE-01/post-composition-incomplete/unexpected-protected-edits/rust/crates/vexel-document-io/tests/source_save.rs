use std::fs::{self, File};
use std::io;
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use vexel_document_io::{
    SourceSaveError, SourceSaveHooks, source_save_error_name, source_save_noop,
};
use vexel_platform_macos::{AtomicFileReplaceError, MAX_ATOMIC_FILE_PATH_BYTES};
use vexel_xml::{MAX_INPUT_BYTES, XmlError};

const FIXTURE: &[u8] = b"<?xml version='1.0' encoding=\"UTF-8\"?>\r\n<!-- authored preservation fixture -->\n<svg xmlns='http://www.w3.org/2000/svg' xmlns:x='urn:x' data-q=\"a'b\">\r  <metadata>  keep  whitespace\r\ninside </metadata>\n  <x:unknown x:quoted='single' note=\"double\"/>\r\n</svg>\n";
static NEXT_ID: AtomicU64 = AtomicU64::new(0);

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../.cache/source-save-rust-tests");
        fs::create_dir_all(&root).expect("create repository-local source-save fixture root");
        let path = root.join(format!(
            "{}-{}",
            std::process::id(),
            NEXT_ID.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).expect("create isolated source-save fixture");
        Self(path)
    }
    fn path(&self, name: &str) -> Vec<u8> {
        self.0.join(name).as_os_str().as_bytes().to_vec()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[derive(Default)]
struct Gates {
    writes: usize,
    renames: usize,
    sync_files: usize,
    sync_dirs: usize,
    closes: usize,
    publishes: usize,
    removes: usize,
    short_writes: bool,
    fail_write: bool,
    fail_rename: bool,
    fail_directory_sync: bool,
    collision_at_publish: Option<PathBuf>,
}
impl SourceSaveHooks for Gates {
    fn before_write(&mut self, _file: &File, remaining: &[u8]) -> io::Result<usize> {
        self.writes += 1;
        if let Some(path) = self.collision_at_publish.take() {
            fs::write(path, b"race winner").expect("publish competing destination");
        }
        if self.fail_write {
            self.fail_write = false;
            return Err(io::ErrorKind::Other.into());
        }
        Ok(if self.short_writes {
            remaining.len().min(3)
        } else {
            remaining.len()
        })
    }
    fn before_sync_file(&mut self, _file: &File) -> io::Result<()> {
        self.sync_files += 1;
        Ok(())
    }
    fn before_close(&mut self, _file: &File) -> io::Result<()> {
        self.closes += 1;
        Ok(())
    }
    fn before_rename(&mut self, _from: &Path, _to: &Path) -> io::Result<()> {
        self.renames += 1;
        if self.fail_rename {
            self.fail_rename = false;
            return Err(io::ErrorKind::Other.into());
        }
        Ok(())
    }
    fn before_publish(&mut self, _temporary: &Path, _target: &Path) -> io::Result<()> {
        self.publishes += 1;
        Ok(())
    }
    fn before_remove_temporary(&mut self, _temporary: &Path) -> io::Result<()> {
        self.removes += 1;
        Ok(())
    }
    fn before_sync_directory(&mut self, _directory: &Path) -> io::Result<()> {
        self.sync_dirs += 1;
        if self.fail_directory_sync {
            return Err(io::ErrorKind::Other.into());
        }
        Ok(())
    }
}

fn target(path: &[u8]) -> PathBuf {
    PathBuf::from(std::ffi::OsString::from_vec(path.to_vec()))
}

#[test]
fn replaces_existing_source_byte_for_byte_and_preserves_mode() {
    let fixture = Fixture::new();
    let path = fixture.path("document.svg");
    let target_path = target(&path);
    fs::write(&target_path, b"<svg>old</svg>").unwrap();
    fs::set_permissions(&target_path, fs::Permissions::from_mode(0o640)).unwrap();
    let mut gates = Gates {
        short_writes: true,
        ..Gates::default()
    };
    let result = source_save_noop(FIXTURE, &path, 128, Some(&mut gates));
    assert_eq!(result.error, SourceSaveError::Ok);
    assert_eq!(result.parse_diagnostic.category, XmlError::None);
    assert!(result.tokens.len() > 8);
    assert_eq!(result.tokens[0].source.slice(FIXTURE), Some(&FIXTURE[..38]));
    assert_eq!(fs::read(&target_path).unwrap(), FIXTURE);
    assert_eq!(
        fs::metadata(&target_path).unwrap().permissions().mode() & 0o7777,
        0o640
    );
    assert!(
        result.file_result.file_synced
            && result.file_result.renamed
            && result.file_result.directory_synced
    );
    assert!(gates.writes > 1 && gates.renames == 1);
    assert_no_temporary_files(&fixture.0);
}

#[test]
fn missing_target_uses_native_no_replace_create_and_reports_durability() {
    let fixture = Fixture::new();
    let path = fixture.path("new.svg");
    let target_path = target(&path);
    let mut gates = Gates {
        short_writes: true,
        ..Gates::default()
    };
    let result = source_save_noop(FIXTURE, &path, 128, Some(&mut gates));
    assert_eq!(result.error, SourceSaveError::Ok);
    assert_eq!(fs::read(&target_path).unwrap(), FIXTURE);
    assert_eq!(
        fs::metadata(&target_path).unwrap().permissions().mode() & 0o7777,
        0o600
    );
    assert!(
        result.file_result.file_synced
            && result.file_result.renamed
            && result.file_result.directory_synced
    );
    assert_eq!(
        gates.renames, 0,
        "create does not receive replace rename gate"
    );
    assert!(gates.writes > 1);
    assert_no_temporary_files(&fixture.0);

    let failed = fixture.path("sync-fails.svg");
    fs::write(target(&failed), b"old").unwrap();
    let mut gates = Gates {
        fail_directory_sync: true,
        ..Gates::default()
    };
    let result = source_save_noop(FIXTURE, &failed, 128, Some(&mut gates));
    assert_eq!(result.error, SourceSaveError::Durability);
    assert!(result.file_result.file_synced && result.file_result.renamed);
    assert!(!result.file_result.directory_synced);
    assert_eq!(fs::read(target(&failed)).unwrap(), FIXTURE);
}

#[test]
fn failures_keep_commit_state_and_clean_temporary_files() {
    let fixture = Fixture::new();
    let existing = fixture.path("existing.svg");
    fs::write(target(&existing), b"old").unwrap();
    let mut gates = Gates {
        fail_write: true,
        ..Gates::default()
    };
    let result = source_save_noop(FIXTURE, &existing, 128, Some(&mut gates));
    assert_eq!(result.error, SourceSaveError::Destination);
    assert_eq!(fs::read(target(&existing)).unwrap(), b"old");
    assert!(!result.file_result.file_synced && !result.file_result.renamed);
    assert_no_temporary_files(&fixture.0);

    let unpublished = fixture.path("unpublished.svg");
    let mut gates = Gates {
        fail_write: true,
        ..Gates::default()
    };
    let result = source_save_noop(FIXTURE, &unpublished, 128, Some(&mut gates));
    assert_eq!(result.error, SourceSaveError::Destination);
    assert!(!result.file_result.file_synced && !result.file_result.renamed);
    assert!(!target(&unpublished).exists());
    assert_no_temporary_files(&fixture.0);

    let mut gates = Gates {
        fail_rename: true,
        ..Gates::default()
    };
    let result = source_save_noop(FIXTURE, &existing, 128, Some(&mut gates));
    assert_eq!(result.error, SourceSaveError::Destination);
    assert!(result.file_result.file_synced && !result.file_result.renamed);
    assert_eq!(fs::read(target(&existing)).unwrap(), b"old");
    assert_no_temporary_files(&fixture.0);

    let race = fixture.path("race.svg");
    let mut gates = Gates {
        collision_at_publish: Some(target(&race)),
        ..Gates::default()
    };
    let result = source_save_noop(FIXTURE, &race, 128, Some(&mut gates));
    assert_eq!(result.error, SourceSaveError::Destination);
    assert_eq!(result.file_result.error, AtomicFileReplaceError::Io);
    assert!(result.file_result.file_synced && !result.file_result.renamed);
    assert_eq!(fs::read(target(&race)).unwrap(), b"race winner");
    assert_no_temporary_files(&fixture.0);
}

#[test]
fn validation_precedence_and_tokenizer_diagnostics_prevent_destination_calls() {
    let fixture = Fixture::new();
    let valid = b"<svg/>";
    let empty_path: &[u8] = b"";
    let mut gates = Gates::default();
    let result = source_save_noop(valid, empty_path, 4, Some(&mut gates));
    assert_eq!(result.error, SourceSaveError::Invalid);
    assert_eq!(source_save_error_name(result.error), "invalid");
    assert_eq!(gates.writes, 0);

    let long_path = vec![b'x'; MAX_ATOMIC_FILE_PATH_BYTES + 1];
    let oversized = vec![b'x'; MAX_INPUT_BYTES + 1];
    let result = source_save_noop(&oversized, &long_path, 4, Some(&mut gates));
    assert_eq!(
        result.error,
        SourceSaveError::PathLimit,
        "path limit precedes source limit"
    );
    assert_eq!(gates.writes, 0);
    let result = source_save_noop(&oversized, b"target", 4, Some(&mut gates));
    assert_eq!(result.error, SourceSaveError::InputLimit);
    assert_eq!(result.parse_diagnostic.category, XmlError::None);
    assert_eq!(result.parse_diagnostic.offset, 0);
    assert_eq!(result.file_result.error, AtomicFileReplaceError::Ok);
    assert!(!result.file_result.file_synced);
    assert!(!result.file_result.renamed);
    assert!(!result.file_result.directory_synced);
    assert_eq!(gates.writes, 0);

    let malformed = b"<svg><broken></svg>";
    let missing = fixture.path("must-not-exist.svg");
    let result = source_save_noop(malformed, &missing, 128, Some(&mut gates));
    assert_eq!(result.error, SourceSaveError::Tokenizer);
    assert_eq!(result.parse_diagnostic.category, XmlError::MismatchedTag);
    assert_eq!(result.parse_diagnostic.offset, 13);
    assert!(result.tokens.is_empty());
    assert!(!target(&missing).exists());
    assert_eq!(gates.writes, 0);

    let result = source_save_noop(valid, &missing, 0, Some(&mut gates));
    assert_eq!(result.error, SourceSaveError::Tokenizer);
    assert_eq!(result.parse_diagnostic.category, XmlError::OutputExhausted);
    assert_eq!(result.parse_diagnostic.offset, 0);
    assert!(!target(&missing).exists());

    let existing = fixture.path("empty-source-existing.svg");
    fs::write(target(&existing), b"keep unchanged").unwrap();
    let mut untouched = Gates::default();
    let result = source_save_noop(&[], &existing, 4, Some(&mut untouched));
    assert_eq!(result.error, SourceSaveError::Tokenizer);
    assert_eq!(
        result.parse_diagnostic.category,
        XmlError::IncompleteDocument
    );
    assert_eq!(result.parse_diagnostic.offset, 0);
    assert_eq!(fs::read(target(&existing)).unwrap(), b"keep unchanged");
    assert_eq!(untouched.writes, 0);
    assert_eq!(untouched.renames, 0);
    assert_eq!(untouched.sync_files, 0);
    assert_eq!(untouched.sync_dirs, 0);
    assert_eq!(untouched.closes, 0);
    assert_eq!(untouched.publishes, 0);
    assert_eq!(untouched.removes, 0);
}

#[test]
fn path_adapter_rejects_nul_and_nonregular_targets_after_tokenization() {
    let fixture = Fixture::new();
    let mut gates = Gates::default();
    let result = source_save_noop(b"<svg/>", b"bad\0path", 4, Some(&mut gates));
    assert_eq!(result.error, SourceSaveError::Destination);
    assert_eq!(result.file_result.error, AtomicFileReplaceError::Invalid);
    assert_eq!(result.parse_diagnostic.category, XmlError::None);
    assert_eq!(result.tokens.len(), 1);
    assert_eq!(gates.writes, 0);

    let directory = fixture.path("directory");
    fs::create_dir(target(&directory)).unwrap();
    let result = source_save_noop(b"<svg/>", &directory, 4, None);
    assert_eq!(result.error, SourceSaveError::Destination);
    assert_eq!(result.file_result.error, AtomicFileReplaceError::NotRegular);
    assert_no_temporary_files(&fixture.0);

    let original = fixture.path("symlink-target.svg");
    let link = fixture.path("symlink.svg");
    fs::write(target(&original), b"preserve target").unwrap();
    std::os::unix::fs::symlink(target(&original), target(&link)).unwrap();
    let result = source_save_noop(b"<svg/>", &link, 4, None);
    assert_eq!(result.error, SourceSaveError::Destination);
    assert_eq!(result.file_result.error, AtomicFileReplaceError::NotRegular);
    assert_eq!(fs::read(target(&original)).unwrap(), b"preserve target");
    assert_no_temporary_files(&fixture.0);
}

fn assert_no_temporary_files(directory: &Path) {
    let leftovers: Vec<_> = fs::read_dir(directory)
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| name.starts_with(".vexel-save-") || name.starts_with(".vexel-new-"))
        .collect();
    assert!(
        leftovers.is_empty(),
        "temporary files remain: {leftovers:?}"
    );
}
