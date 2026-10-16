use std::ffi::OsStr;
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use vexel_platform_macos::{
    atomic_file_create_new, atomic_file_replace, AtomicFileCreateNewError,
    AtomicFileCreateNewHooks, AtomicFileReplaceError, AtomicFileReplaceHooks,
    MAX_ATOMIC_FILE_PATH_BYTES,
};
use vexel_project::{self as project, BundleInput};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FileError {
    Ok,
    InvalidArgument,
    Limit,
    OutOfMemory,
    PathLimit,
    AlreadyExists,
    Io,
    Durability,
}

impl FileError {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::InvalidArgument => "invalid_argument",
            Self::Limit => "limit",
            Self::OutOfMemory => "out_of_memory",
            Self::PathLimit => "path_limit",
            Self::AlreadyExists => "already_exists",
            Self::Io => "io",
            Self::Durability => "durability",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FileResult {
    pub error: FileError,
    pub bundle_error: project::Error,
    pub replace_error: AtomicFileReplaceError,
    pub create_error: AtomicFileCreateNewError,
    pub bytes_written: usize,
    pub published: bool,
    pub directory_fsynced: bool,
}

impl FileResult {
    const fn error(error: FileError) -> Self {
        Self {
            error,
            bundle_error: project::Error::Ok,
            replace_error: AtomicFileReplaceError::Ok,
            create_error: AtomicFileCreateNewError::Ok,
            bytes_written: 0,
            published: false,
            directory_fsynced: false,
        }
    }
}

/// Encodes and validates the complete bundle before atomically publishing it.
/// A missing destination is created without replacement; an existing regular
/// file is atomically replaced. `input` bytes remain borrowed by the caller.
pub fn project_file_save_as(path: &[u8], input: &BundleInput<'_>) -> FileResult {
    if path.is_empty() {
        return FileResult::error(FileError::InvalidArgument);
    }
    if path.len() > MAX_ATOMIC_FILE_PATH_BYTES {
        return FileResult::error(FileError::PathLimit);
    }
    if path.contains(&0) {
        return FileResult::error(FileError::InvalidArgument);
    }
    let size = match project::encoded_size(input) {
        Ok(size) => size,
        Err(failure) => return bundle_failure(failure.error),
    };
    let encoded = match project::encode(input) {
        Ok(bytes) if bytes.len() == size => bytes,
        Ok(_) => return bundle_failure(project::Error::InvalidArgument),
        Err(failure) => return bundle_failure(failure.error),
    };
    project_file_save_encoded_with_hooks(path, &encoded, None, None)
}

/// Atomically publishes caller-owned, checksum-valid bundle bytes. The input
/// buffer is borrowed and is never copied by this API.
pub fn project_file_save_encoded(path: &[u8], encoded: &[u8]) -> FileResult {
    project_file_save_encoded_with_hooks(path, encoded, None, None)
}

pub(super) fn project_file_save_encoded_with_hooks(
    path: &[u8],
    encoded: &[u8],
    replace_hooks: Option<&mut dyn AtomicFileReplaceHooks>,
    create_hooks: Option<&mut dyn AtomicFileCreateNewHooks>,
) -> FileResult {
    // The legacy encoded-save API maps every malformed path, including an
    // over-limit path, to INVALID_ARGUMENT.
    if path.is_empty() || path.len() > MAX_ATOMIC_FILE_PATH_BYTES
        || path.contains(&0) || encoded.is_empty()
    {
        return FileResult::error(FileError::InvalidArgument);
    }
    if let Err(failure) = project::decode(encoded) {
        return bundle_decode_failure(failure.error);
    }

    let target = Path::new(OsStr::from_bytes(path));
    let metadata = fs::symlink_metadata(target);
    let mut result = FileResult::error(FileError::Io);
    match metadata {
        Ok(metadata) if metadata.file_type().is_file() => {
            let written = atomic_file_replace(path, encoded, replace_hooks);
            result.replace_error = written.error;
            result.published = written.renamed;
            result.directory_fsynced = written.directory_synced;
            result.bytes_written = encoded.len();
            result.error = match written.error {
                AtomicFileReplaceError::Ok => FileError::Ok,
                AtomicFileReplaceError::Durability => FileError::Durability,
                _ => FileError::Io,
            };
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let written = atomic_file_create_new(path, encoded, create_hooks);
            result.create_error = written.error;
            result.published = written.published;
            result.directory_fsynced = written.directory_synced;
            result.bytes_written = encoded.len();
            result.error = match written.error {
                AtomicFileCreateNewError::Ok => FileError::Ok,
                AtomicFileCreateNewError::Exists => FileError::AlreadyExists,
                AtomicFileCreateNewError::Durability => FileError::Durability,
                _ => FileError::Io,
            };
        }
        _ => {}
    }
    result
}

pub(super) fn bundle_decode_failure(error: project::Error) -> FileResult {
    let mut result = FileResult::error(match error {
        project::Error::OutOfMemory => FileError::OutOfMemory,
        project::Error::Limit => FileError::Limit,
        _ => FileError::InvalidArgument,
    });
    result.bundle_error = error;
    result
}

fn bundle_failure(error: project::Error) -> FileResult {
    let mut result = FileResult::error(match error {
        project::Error::Limit => FileError::Limit,
        project::Error::OutOfMemory => FileError::OutOfMemory,
        _ => FileError::InvalidArgument,
    });
    result.bundle_error = error;
    result
}

pub const fn project_file_error_name(error: FileError) -> &'static str { error.name() }
