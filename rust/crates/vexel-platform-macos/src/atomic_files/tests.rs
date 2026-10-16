use super::*;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let cache = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../.cache");
        fs::create_dir_all(&cache).expect("create repo-local cache directory");
        let id = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
        let root = cache.join(format!("atomic-{}-{id}", std::process::id()));
        fs::create_dir(&root).expect("create unique atomic fixture directory");
        Self(root)
    }

    fn path(&self, name: &str) -> (PathBuf, Vec<u8>) {
        let path = self.0.join(name);
        let bytes = path.as_os_str().as_bytes().to_vec();
        (path, bytes)
    }

    fn temporary_count(&self, prefix: &str) -> usize {
        fs::read_dir(&self.0)
            .expect("read fixture directory")
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().as_bytes().starts_with(prefix.as_bytes()))
            .count()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[derive(Default)]
struct Script {
    interrupt_write: bool,
    max_write: usize,
    fail_write: bool,
    zero_write: bool,
    fail_sync: bool,
    fail_close_at: usize,
    fail_rename: bool,
    fail_publish: bool,
    race_publish: bool,
    fail_directory_sync: bool,
    fail_remove_once: bool,
    writes: usize,
    closes: usize,
    file_syncs: usize,
    renames: usize,
    publishes: usize,
    removals: usize,
    directory_syncs: usize,
}

impl Script {
    fn before_write(&mut self, _file: &File, bytes: &[u8]) -> io::Result<usize> {
        self.writes += 1;
        if self.interrupt_write {
            self.interrupt_write = false;
            return Err(io::Error::from(io::ErrorKind::Interrupted));
        }
        if self.fail_write {
            return Err(io::Error::from(io::ErrorKind::Other));
        }
        if self.zero_write {
            return Ok(0);
        }
        let end = if self.max_write == 0 {
            bytes.len()
        } else {
            bytes.len().min(self.max_write)
        };
        Ok(end)
    }

    fn before_sync(&mut self, _file: &File) -> io::Result<()> {
        self.file_syncs += 1;
        if self.fail_sync {
            Err(io::Error::from(io::ErrorKind::Other))
        } else {
            Ok(())
        }
    }

    fn before_close(&mut self, _file: &File) -> io::Result<()> {
        self.closes += 1;
        if self.fail_close_at == self.closes {
            Err(io::Error::from(io::ErrorKind::Other))
        } else {
            Ok(())
        }
    }

    fn before_sync_dir(&mut self, _directory: &Path) -> io::Result<()> {
        self.directory_syncs += 1;
        if self.fail_directory_sync {
            Err(io::Error::from(io::ErrorKind::Other))
        } else {
            Ok(())
        }
    }
}

impl AtomicFileReplaceHooks for Script {
    fn before_write(&mut self, file: &File, bytes: &[u8]) -> io::Result<usize> {
        Script::before_write(self, file, bytes)
    }

    fn before_sync_file(&mut self, file: &File) -> io::Result<()> {
        self.before_sync(file)
    }

    fn before_close(&mut self, file: &File) -> io::Result<()> {
        Script::before_close(self, file)
    }

    fn before_rename(&mut self, _from: &Path, _to: &Path) -> io::Result<()> {
        self.renames += 1;
        if self.fail_rename {
            Err(io::Error::from(io::ErrorKind::Other))
        } else {
            Ok(())
        }
    }

    fn before_sync_directory(&mut self, directory: &Path) -> io::Result<()> {
        self.before_sync_dir(directory)
    }
}

impl AtomicFileCreateNewHooks for Script {
    fn before_write(&mut self, file: &File, bytes: &[u8]) -> io::Result<usize> {
        Script::before_write(self, file, bytes)
    }

    fn before_sync_file(&mut self, file: &File) -> io::Result<()> {
        self.before_sync(file)
    }

    fn before_close(&mut self, file: &File) -> io::Result<()> {
        Script::before_close(self, file)
    }

    fn before_publish(&mut self, _temporary: &Path, target: &Path) -> io::Result<()> {
        self.publishes += 1;
        if self.race_publish {
            let mut competitor = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(target)?;
            competitor.write_all(b"racing writer")?;
            crate::sys::close_checked(competitor.into())?;
        }
        if self.fail_publish {
            Err(io::Error::from(io::ErrorKind::Other))
        } else {
            Ok(())
        }
    }

    fn before_remove_temporary(&mut self, _temporary: &Path) -> io::Result<()> {
        self.removals += 1;
        if self.fail_remove_once {
            self.fail_remove_once = false;
            Err(io::Error::from(io::ErrorKind::Other))
        } else {
            Ok(())
        }
    }

    fn before_sync_directory(&mut self, directory: &Path) -> io::Result<()> {
        self.before_sync_dir(directory)
    }
}

fn replace_result(error: AtomicFileReplaceError) -> AtomicFileReplaceResult {
    AtomicFileReplaceResult {
        error,
        file_synced: false,
        renamed: false,
        directory_synced: false,
    }
}

fn create_result(error: AtomicFileCreateNewError) -> AtomicFileCreateNewResult {
    AtomicFileCreateNewResult {
        error,
        file_synced: false,
        published: false,
        directory_synced: false,
    }
}

#[test]
fn native_checked_close_consumes_a_real_owned_file() {
    let fixture = Fixture::new();
    let (path, _) = fixture.path("close-native");
    let file = File::create(path).expect("create checked-close fixture");
    crate::sys::close_checked(file.into()).expect("close the owned descriptor");
}

#[test]
fn byte_path_conversion_accepts_1023_bytes_and_rejects_1024() {
    let at_limit = vec![b'a'; MAX_ATOMIC_FILE_PATH_BYTES];
    assert!(super::path::target_path(&at_limit).is_ok());
    let over_limit = vec![b'a'; MAX_ATOMIC_FILE_PATH_BYTES + 1];
    assert!(matches!(
        super::path::target_path(&over_limit),
        Err(super::path::PathError::Limit)
    ));
    let non_utf8 = [0xff];
    let converted = super::path::target_path(&non_utf8).unwrap();
    assert_eq!(converted.as_os_str().as_bytes(), &non_utf8);
}

#[test]
fn replace_native_path_replaces_contents_and_preserves_mode() {
    let fixture = Fixture::new();
    let (path, path_bytes) = fixture.path("replace-native");
    fs::write(&path, b"old bytes").expect("seed replacement target");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();
    let old_inode = fs::metadata(&path).unwrap().ino();

    let result = atomic_file_replace(&path_bytes, b"new\0bytes", None);
    assert_eq!(
        result,
        AtomicFileReplaceResult {
            error: AtomicFileReplaceError::Ok,
            file_synced: true,
            renamed: true,
            directory_synced: true,
        }
    );
    assert_eq!(fs::read(&path).unwrap(), b"new\0bytes");
    assert_eq!(fs::metadata(&path).unwrap().permissions().mode() & 0o7777, 0o640);
    assert_ne!(fs::metadata(&path).unwrap().ino(), old_inode);
    assert_eq!(fixture.temporary_count(".vexel-save-"), 0);
}

#[test]
fn replace_successful_hooks_still_write_sync_close_and_rename_natively() {
    let fixture = Fixture::new();
    let (path, path_bytes) = fixture.path("replace-hook-success");
    fs::write(&path, b"old bytes").unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();
    let old_inode = fs::metadata(&path).unwrap().ino();
    let mut hooks = Script::default();

    assert_eq!(
        atomic_file_replace(&path_bytes, b"native through hooks", Some(&mut hooks)),
        AtomicFileReplaceResult {
            error: AtomicFileReplaceError::Ok,
            file_synced: true,
            renamed: true,
            directory_synced: true,
        }
    );
    let metadata = fs::metadata(&path).unwrap();
    assert_eq!(fs::read(&path).unwrap(), b"native through hooks");
    assert_eq!(metadata.permissions().mode() & 0o7777, 0o640);
    assert_ne!(metadata.ino(), old_inode);
    assert!(hooks.writes >= 1);
    assert_eq!(hooks.file_syncs, 1);
    assert_eq!(hooks.closes, 2);
    assert_eq!(hooks.renames, 1);
    assert_eq!(hooks.directory_syncs, 1);
    assert_eq!(fixture.temporary_count(".vexel-save-"), 0);
}

#[test]
fn replace_accepts_empty_content_and_preserves_accepted_error_classes() {
    let fixture = Fixture::new();
    let (path, path_bytes) = fixture.path("empty-replace");
    fs::write(&path, b"old").unwrap();
    assert_eq!(
        atomic_file_replace(&path_bytes, b"", None),
        AtomicFileReplaceResult {
            error: AtomicFileReplaceError::Ok,
            file_synced: true,
            renamed: true,
            directory_synced: true,
        }
    );
    assert_eq!(fs::read(&path).unwrap(), b"");

    assert_eq!(
        atomic_file_replace(b"", b"x", None),
        replace_result(AtomicFileReplaceError::Invalid)
    );
    assert_eq!(
        atomic_file_replace(b"bad\0path", b"x", None),
        replace_result(AtomicFileReplaceError::Invalid)
    );
    assert_eq!(
        atomic_file_replace(&vec![b'x'; MAX_ATOMIC_FILE_PATH_BYTES + 1], b"x", None),
        replace_result(AtomicFileReplaceError::PathLimit)
    );
    assert_eq!(
        atomic_file_replace(b"missing", b"x", None),
        replace_result(AtomicFileReplaceError::NotFound)
    );
    assert_eq!(fixture.temporary_count(".vexel-save-"), 0);
}

#[test]
fn replace_rejects_symlinks_and_nonregular_files() {
    let fixture = Fixture::new();
    let (regular, _) = fixture.path("regular");
    fs::write(&regular, b"regular").unwrap();
    let (link, link_bytes) = fixture.path("symlink");
    std::os::unix::fs::symlink(&regular, &link).unwrap();
    assert_eq!(
        atomic_file_replace(&link_bytes, b"changed", None),
        replace_result(AtomicFileReplaceError::NotRegular)
    );
    let (directory, directory_bytes) = fixture.path("directory");
    fs::create_dir(&directory).unwrap();
    assert_eq!(
        atomic_file_replace(&directory_bytes, b"changed", None),
        replace_result(AtomicFileReplaceError::NotRegular)
    );
    assert_eq!(fs::read(regular).unwrap(), b"regular");
    assert_eq!(fixture.temporary_count(".vexel-save-"), 0);
}

#[test]
fn replace_retries_interruptions_and_completes_short_writes() {
    let fixture = Fixture::new();
    let (path, path_bytes) = fixture.path("short-write");
    fs::write(&path, b"initial").unwrap();
    let mut script = Script {
        interrupt_write: true,
        max_write: 2,
        ..Script::default()
    };
    let result = atomic_file_replace(&path_bytes, b"abcdefgh", Some(&mut script));
    assert_eq!(
        result,
        AtomicFileReplaceResult {
            error: AtomicFileReplaceError::Ok,
            file_synced: true,
            renamed: true,
            directory_synced: true,
        }
    );
    assert_eq!(fs::read(path).unwrap(), b"abcdefgh");
    assert!(script.writes >= 5);
    assert_eq!(script.closes, 2);
}

#[test]
fn replace_write_sync_close_and_rename_failures_never_commit_and_clean_temp() {
    for failure in ["write", "zero", "sync", "source-close", "close", "rename"] {
        let fixture = Fixture::new();
        let (path, path_bytes) = fixture.path("failure-target");
        fs::write(&path, b"original").unwrap();
        let mut script = match failure {
            "write" => Script { fail_write: true, ..Script::default() },
            "zero" => Script { zero_write: true, ..Script::default() },
            "sync" => Script { fail_sync: true, ..Script::default() },
            "source-close" => Script { fail_close_at: 1, ..Script::default() },
            "close" => Script { fail_close_at: 2, ..Script::default() },
            _ => Script { fail_rename: true, ..Script::default() },
        };
        let mut expected = replace_result(AtomicFileReplaceError::Io);
        expected.file_synced = failure == "close" || failure == "rename";
        assert_eq!(
            atomic_file_replace(&path_bytes, b"replacement", Some(&mut script)),
            expected
        );
        assert_eq!(fs::read(path).unwrap(), b"original");
        assert_eq!(fixture.temporary_count(".vexel-save-"), 0, "{failure}");
    }
}

#[test]
fn replace_directory_sync_failure_reports_postcommit_state() {
    let fixture = Fixture::new();
    let (path, path_bytes) = fixture.path("replace-durability");
    fs::write(&path, b"old").unwrap();
    let mut script = Script { fail_directory_sync: true, ..Script::default() };
    assert_eq!(
        atomic_file_replace(&path_bytes, b"committed", Some(&mut script)),
        AtomicFileReplaceResult {
            error: AtomicFileReplaceError::Durability,
            file_synced: true,
            renamed: true,
            directory_synced: false,
        }
    );
    assert_eq!(fs::read(path).unwrap(), b"committed");
    assert_eq!(fixture.temporary_count(".vexel-save-"), 0);
}

#[test]
fn create_new_native_path_publishes_empty_content_with_private_mode() {
    let fixture = Fixture::new();
    let (path, path_bytes) = fixture.path("new-native");
    assert_eq!(
        atomic_file_create_new(&path_bytes, b"", None),
        AtomicFileCreateNewResult {
            error: AtomicFileCreateNewError::Ok,
            file_synced: true,
            published: true,
            directory_synced: true,
        }
    );
    assert_eq!(fs::read(&path).unwrap(), b"");
    assert_eq!(fs::metadata(path).unwrap().permissions().mode() & 0o777, 0o600);
    assert_eq!(fixture.temporary_count(".vexel-new-"), 0);
}

#[test]
fn create_new_successful_hooks_still_write_sync_close_and_publish_natively() {
    let fixture = Fixture::new();
    let (path, path_bytes) = fixture.path("new-hook-success");
    let mut hooks = Script::default();

    assert_eq!(
        atomic_file_create_new(&path_bytes, b"native through hooks", Some(&mut hooks)),
        AtomicFileCreateNewResult {
            error: AtomicFileCreateNewError::Ok,
            file_synced: true,
            published: true,
            directory_synced: true,
        }
    );
    let metadata = fs::metadata(&path).unwrap();
    assert_eq!(fs::read(&path).unwrap(), b"native through hooks");
    assert_eq!(metadata.permissions().mode() & 0o777, 0o600);
    assert!(hooks.writes >= 1);
    assert_eq!(hooks.file_syncs, 1);
    assert_eq!(hooks.closes, 1);
    assert_eq!(hooks.publishes, 1);
    assert_eq!(hooks.removals, 1);
    assert_eq!(hooks.directory_syncs, 1);
    assert_eq!(fixture.temporary_count(".vexel-new-"), 0);
}

#[test]
fn create_new_enforces_path_and_component_bounds_and_existing_collision() {
    let fixture = Fixture::new();
    let (existing, existing_bytes) = fixture.path("existing");
    fs::write(&existing, b"keep").unwrap();
    assert_eq!(
        atomic_file_create_new(&existing_bytes, b"replace", None),
        create_result(AtomicFileCreateNewError::Exists)
    );
    let (link, link_bytes) = fixture.path("existing-link");
    std::os::unix::fs::symlink(&existing, &link).unwrap();
    assert_eq!(
        atomic_file_create_new(&link_bytes, b"replace", None),
        create_result(AtomicFileCreateNewError::Exists)
    );
    for invalid in [&b""[..], b"bad\0path", b".", b"..", b"folder/", b"folder/.", b"folder/.."] {
        assert_eq!(
            atomic_file_create_new(invalid, b"x", None),
            create_result(AtomicFileCreateNewError::Invalid),
            "invalid path {:?}",
            invalid
        );
    }
    assert_eq!(
        atomic_file_create_new(&vec![b'x'; MAX_ATOMIC_FILE_PATH_BYTES + 1], b"x", None),
        create_result(AtomicFileCreateNewError::PathLimit)
    );
    assert_eq!(fs::read(existing).unwrap(), b"keep");
    assert_eq!(fixture.temporary_count(".vexel-new-"), 0);
}

#[test]
fn create_new_retries_interruptions_and_completes_short_writes() {
    let fixture = Fixture::new();
    let (path, path_bytes) = fixture.path("new-short-write");
    let mut script = Script {
        interrupt_write: true,
        max_write: 3,
        ..Script::default()
    };
    assert_eq!(
        atomic_file_create_new(&path_bytes, b"abcdefghij", Some(&mut script)),
        AtomicFileCreateNewResult {
            error: AtomicFileCreateNewError::Ok,
            file_synced: true,
            published: true,
            directory_synced: true,
        }
    );
    assert_eq!(fs::read(path).unwrap(), b"abcdefghij");
    assert!(script.writes >= 5);
    assert_eq!(script.closes, 1);
}

#[test]
fn create_new_failures_clean_temporary_and_do_not_publish_before_commit() {
    for failure in ["write", "zero", "sync", "close", "publish"] {
        let fixture = Fixture::new();
        let (path, path_bytes) = fixture.path("new-failure");
        let mut script = match failure {
            "write" => Script { fail_write: true, ..Script::default() },
            "zero" => Script { zero_write: true, ..Script::default() },
            "sync" => Script { fail_sync: true, ..Script::default() },
            "close" => Script { fail_close_at: 1, ..Script::default() },
            _ => Script { fail_publish: true, ..Script::default() },
        };
        let mut expected = create_result(AtomicFileCreateNewError::Io);
        expected.file_synced = failure == "close" || failure == "publish";
        assert_eq!(
            atomic_file_create_new(&path_bytes, b"data", Some(&mut script)),
            expected,
            "{failure}"
        );
        assert!(!path.exists(), "{failure}");
        assert_eq!(fixture.temporary_count(".vexel-new-"), 0, "{failure}");
    }
}

#[test]
fn create_new_race_collision_preserves_competing_file() {
    let fixture = Fixture::new();
    let (path, path_bytes) = fixture.path("new-race");
    let mut script = Script { race_publish: true, ..Script::default() };
    assert_eq!(
        atomic_file_create_new(&path_bytes, b"ours", Some(&mut script)),
        AtomicFileCreateNewResult {
            error: AtomicFileCreateNewError::Exists,
            file_synced: true,
            published: false,
            directory_synced: false,
        }
    );
    assert_eq!(fs::read(path).unwrap(), b"racing writer");
    assert_eq!(fixture.temporary_count(".vexel-new-"), 0);
}

#[test]
fn create_new_unlink_and_directory_durability_failures_report_commit_state() {
    let fixture = Fixture::new();
    let (remove_path, remove_bytes) = fixture.path("remove-after-publish");
    let mut remove_script = Script { fail_remove_once: true, ..Script::default() };
    assert_eq!(
        atomic_file_create_new(&remove_bytes, b"published", Some(&mut remove_script)),
        AtomicFileCreateNewResult {
            error: AtomicFileCreateNewError::Io,
            file_synced: true,
            published: true,
            directory_synced: false,
        }
    );
    assert_eq!(fs::read(remove_path).unwrap(), b"published");
    assert_eq!(fixture.temporary_count(".vexel-new-"), 0);

    let (durability_path, durability_bytes) = fixture.path("new-durability");
    let mut durability_script = Script { fail_directory_sync: true, ..Script::default() };
    assert_eq!(
        atomic_file_create_new(&durability_bytes, b"committed", Some(&mut durability_script)),
        AtomicFileCreateNewResult {
            error: AtomicFileCreateNewError::Durability,
            file_synced: true,
            published: true,
            directory_synced: false,
        }
    );
    assert_eq!(fs::read(durability_path).unwrap(), b"committed");
    assert_eq!(fixture.temporary_count(".vexel-new-"), 0);
}

#[test]
fn replace_error_names_match_the_native_c_names() {
    let cases = [
        (AtomicFileReplaceError::Ok, "ok"),
        (AtomicFileReplaceError::Invalid, "invalid"),
        (AtomicFileReplaceError::PathLimit, "path_limit"),
        (AtomicFileReplaceError::NotFound, "not_found"),
        (AtomicFileReplaceError::NotRegular, "not_regular"),
        (AtomicFileReplaceError::Io, "io"),
        (AtomicFileReplaceError::Durability, "durability"),
        (AtomicFileReplaceError::Unknown, "unknown"),
    ];
    for (error, expected) in cases {
        assert_eq!(error.to_string(), expected);
    }
}
