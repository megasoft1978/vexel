use std::ffi::OsStr;
use std::fs::{self, OpenOptions};
use std::io::{self, Read};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;
use vexel_platform_macos::MAX_ATOMIC_FILE_PATH_BYTES;
use vexel_project::{self as project, BundleFailure, BundleView};

/// Failure to load a native project. Decode errors retain their codec category,
/// encoded-size field, and byte offset; file errors never expose partial data.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LoadFailure {
    pub file_error: super::FileError,
    pub bundle_failure: Option<BundleFailure>,
}

impl LoadFailure {
    const fn file(file_error: super::FileError) -> Self {
        Self {
            file_error,
            bundle_failure: None,
        }
    }

    const fn bundle(failure: BundleFailure) -> Self {
        let file_error = match failure.error {
            project::Error::OutOfMemory => super::FileError::OutOfMemory,
            project::Error::Limit => super::FileError::Limit,
            _ => super::FileError::InvalidArgument,
        };
        Self {
            file_error,
            bundle_failure: Some(failure),
        }
    }
}

/// A fully validated project bundle backed by its exact encoded file bytes.
/// The format permits files up to 1 GiB; loading owns that encoded buffer and
/// decoding currently requires the complete bundle in memory.
#[derive(Debug)]
pub struct LoadedProject {
    encoded: Vec<u8>,
}

impl LoadedProject {
    /// The exact bytes read from the project file.
    pub fn encoded_bytes(&self) -> &[u8] {
        &self.encoded
    }

    /// Reopens a validated borrowed view whose lifetime is tied to this owner.
    pub fn view(&self) -> BundleView<'_> {
        // Construction validates the immutable bytes before `LoadedProject`
        // can be returned, so this decode cannot fail absent memory corruption.
        project::decode(&self.encoded).expect("loaded bundle remains validated")
    }
}

/// Reads, bounds, and validates a `.vexel` bundle before returning owned data.
/// `path` is a macOS path byte sequence and is validated before filesystem access.
pub fn project_file_load(path: &[u8]) -> Result<LoadedProject, LoadFailure> {
    if path.is_empty() || path.contains(&0) {
        return Err(LoadFailure::file(super::FileError::InvalidArgument));
    }
    if path.len() > MAX_ATOMIC_FILE_PATH_BYTES {
        return Err(LoadFailure::file(super::FileError::PathLimit));
    }
    let path = Path::new(OsStr::from_bytes(path));
    let path_metadata =
        fs::symlink_metadata(path).map_err(|_| LoadFailure::file(super::FileError::Io))?;
    if !path_metadata.file_type().is_file() {
        return Err(LoadFailure::file(super::FileError::InvalidArgument));
    }
    // Nonblocking open avoids hanging if the checked path is raced with a FIFO;
    // no-follow rejects a raced symlink. Handle metadata below validates again.
    const DARWIN_O_NONBLOCK: i32 = 0x0004;
    const DARWIN_O_NOFOLLOW: i32 = 0x0100;
    let mut options = OpenOptions::new();
    options
        .read(true)
        .custom_flags(DARWIN_O_NONBLOCK | DARWIN_O_NOFOLLOW);
    let file = options
        .open(path)
        .map_err(|_| LoadFailure::file(super::FileError::Io))?;
    let metadata = file
        .metadata()
        .map_err(|_| LoadFailure::file(super::FileError::Io))?;
    if !metadata.file_type().is_file() {
        return Err(LoadFailure::file(super::FileError::InvalidArgument));
    }
    let declared = usize::try_from(metadata.len())
        .map_err(|_| LoadFailure::bundle(limit_failure(project::MAX_BYTES)))?;
    if declared > project::MAX_BYTES {
        return Err(LoadFailure::bundle(limit_failure(declared)));
    }
    let mut reader = io::BufReader::new(file);
    let encoded = read_bounded(&mut reader, project::MAX_BYTES).map_err(|error| match error {
        ReadFailure::Limit(size) => LoadFailure::bundle(limit_failure(size)),
        ReadFailure::Io => LoadFailure::file(super::FileError::Io),
        ReadFailure::OutOfMemory => LoadFailure::file(super::FileError::OutOfMemory),
    })?;
    project::decode(&encoded).map_err(LoadFailure::bundle)?;
    Ok(LoadedProject { encoded })
}

const fn limit_failure(size: usize) -> BundleFailure {
    BundleFailure {
        error: project::Error::Limit,
        encoded_size: size,
        error_offset: size,
    }
}

#[derive(Debug)]
pub(super) enum ReadFailure {
    Limit(usize),
    OutOfMemory,
    Io,
}

pub(super) fn read_bounded<R: Read>(reader: &mut R, limit: usize) -> Result<Vec<u8>, ReadFailure> {
    const CHUNK_BYTES: usize = 8192;
    let mut output = Vec::new();
    let mut chunk = [0u8; CHUNK_BYTES];
    loop {
        let count = match reader.read(&mut chunk) {
            Ok(0) => return Ok(output),
            Ok(count) => count,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(_) => return Err(ReadFailure::Io),
        };
        let new_len = output
            .len()
            .checked_add(count)
            .ok_or(ReadFailure::Limit(usize::MAX))?;
        if new_len > limit {
            return Err(ReadFailure::Limit(new_len));
        }
        output
            .try_reserve_exact(count)
            .map_err(|_| ReadFailure::OutOfMemory)?;
        output.extend_from_slice(&chunk[..count]);
    }
}
