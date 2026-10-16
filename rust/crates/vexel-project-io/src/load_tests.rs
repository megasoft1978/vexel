use super::file_load::{ReadFailure, read_bounded};
use super::{FileError, project_file_load};
use std::fs::{self, File};
use std::io::{self, Cursor, Read, Write};
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use vexel_project::{Asset, BundleInput, Error as BundleError, MAX_BYTES};

static NEXT: AtomicUsize = AtomicUsize::new(0);

fn test_path(label: &str) -> Vec<u8> {
    let directory = Path::new(".cache/project-load-tests");
    fs::create_dir_all(directory).unwrap();
    directory
        .join(format!(
            "{label}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ))
        .as_os_str()
        .as_bytes()
        .to_vec()
}

fn valid_bundle() -> Vec<u8> {
    let assets = [Asset {
        name: b"sprite.png",
        media_type: b"image/png",
        data: b"\x00sprite",
    }];
    vexel_project::encode(&BundleInput {
        manifest_json: br#"{"title":"load"}"#,
        scene_svg: b"<svg><!--preserve--></svg>",
        timeline_json: br#"{"fps":24}"#,
        assets: &assets,
    })
    .unwrap()
}

fn write_path(path: &[u8], bytes: &[u8]) {
    let path = Path::new(std::ffi::OsStr::from_bytes(path));
    let mut file = File::create(path).unwrap();
    file.write_all(bytes).unwrap();
}

fn remove(path: &[u8]) {
    let _ = fs::remove_file(Path::new(std::ffi::OsStr::from_bytes(path)));
}

#[test]
fn valid_load_owns_exact_payload_and_exposes_borrowed_sections() {
    let path = test_path("valid");
    let encoded = valid_bundle();
    write_path(&path, &encoded);
    let loaded = project_file_load(&path).unwrap();
    assert_eq!(loaded.encoded_bytes(), encoded);
    let view = loaded.view();
    assert_eq!(view.manifest_json, br#"{"title":"load"}"#);
    assert_eq!(view.scene_svg, b"<svg><!--preserve--></svg>");
    assert_eq!(view.timeline_json, br#"{"fps":24}"#);
    assert_eq!(view.asset_at(0).unwrap().data, b"\x00sprite");
    remove(&path);
}

#[test]
fn path_validation_precedes_file_access_and_classifies_path_failures() {
    assert_eq!(
        project_file_load(b"").unwrap_err().file_error,
        FileError::InvalidArgument
    );
    assert_eq!(
        project_file_load(b"bad\0path").unwrap_err().file_error,
        FileError::InvalidArgument
    );
    assert_eq!(
        project_file_load(&vec![
            b'x';
            vexel_platform_macos::MAX_ATOMIC_FILE_PATH_BYTES + 1
        ])
        .unwrap_err()
        .file_error,
        FileError::PathLimit
    );
    let missing = test_path("missing");
    assert_eq!(
        project_file_load(&missing).unwrap_err().file_error,
        FileError::Io
    );
    let directory = test_path("directory");
    fs::create_dir(Path::new(std::ffi::OsStr::from_bytes(&directory))).unwrap();
    assert_eq!(
        project_file_load(&directory).unwrap_err().file_error,
        FileError::InvalidArgument
    );
    fs::remove_dir(Path::new(std::ffi::OsStr::from_bytes(&directory))).unwrap();
}

#[test]
fn oversized_sparse_file_rejected_by_metadata_before_payload_read() {
    let path = test_path("sparse");
    let file = File::create(Path::new(std::ffi::OsStr::from_bytes(&path))).unwrap();
    file.set_len((MAX_BYTES as u64) + 1).unwrap();
    let failure = project_file_load(&path).unwrap_err();
    assert_eq!(failure.file_error, FileError::Limit);
    let bundle = failure.bundle_failure.unwrap();
    assert_eq!(bundle.error, BundleError::Limit);
    assert_eq!(bundle.encoded_size, MAX_BYTES + 1);
    remove(&path);
}

#[test]
fn fifo_is_rejected_before_project_data_read() {
    let path = test_path("fifo");
    let status = Command::new("/usr/bin/mkfifo")
        .arg(Path::new(std::ffi::OsStr::from_bytes(&path)))
        .status()
        .expect("macOS mkfifo utility is available");
    assert!(status.success());
    let failure = project_file_load(&path).unwrap_err();
    assert_eq!(failure.file_error, FileError::InvalidArgument);
    fs::remove_file(Path::new(std::ffi::OsStr::from_bytes(&path))).unwrap();
}

#[test]
fn decode_failure_categories_and_offsets_are_preserved_without_loaded_bytes() {
    let path = test_path("bad");
    let mut bytes = valid_bundle();
    bytes[0] ^= 1;
    write_path(&path, &bytes);
    let failure = project_file_load(&path).unwrap_err();
    assert_eq!(failure.file_error, FileError::InvalidArgument);
    assert_eq!(failure.bundle_failure.unwrap().error, BundleError::BadMagic);

    let mut bytes = valid_bundle();
    bytes[8..12].copy_from_slice(&99u32.to_le_bytes());
    write_path(&path, &bytes);
    let failure = project_file_load(&path).unwrap_err();
    let bundle = failure.bundle_failure.unwrap();
    assert_eq!(bundle.error, BundleError::UnsupportedVersion);
    assert_eq!(bundle.error_offset, 8);

    let mut bytes = valid_bundle();
    *bytes.last_mut().unwrap() ^= 1;
    write_path(&path, &bytes);
    let failure = project_file_load(&path).unwrap_err();
    assert_eq!(failure.bundle_failure.unwrap().error, BundleError::Corrupt);
    remove(&path);
}

#[test]
fn bounded_reader_handles_short_reads_and_growth_past_private_limit() {
    let mut reader = ShortReader {
        inner: Cursor::new(b"abcdef"),
        max: 2,
    };
    assert_eq!(read_bounded(&mut reader, 6).unwrap(), b"abcdef");
    let mut reader = Cursor::new(b"abcdef");
    assert!(matches!(
        read_bounded(&mut reader, 5),
        Err(ReadFailure::Limit(6))
    ));
}

#[test]
fn read_failure_discards_partial_buffer() {
    let mut reader = FailsAfterPrefix { emitted: false };
    assert!(matches!(
        read_bounded(&mut reader, 32),
        Err(ReadFailure::Io)
    ));
}

struct ShortReader<R> {
    inner: R,
    max: usize,
}
impl<R: Read> Read for ShortReader<R> {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        let limit = out.len().min(self.max);
        self.inner.read(&mut out[..limit])
    }
}

struct FailsAfterPrefix {
    emitted: bool,
}
impl Read for FailsAfterPrefix {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        if self.emitted {
            return Err(io::Error::other("injected read failure"));
        }
        self.emitted = true;
        out[..3].copy_from_slice(b"abc");
        Ok(3)
    }
}
