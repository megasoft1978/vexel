use std::ffi::OsStr;
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use vexel_platform_macos::{
    AtomicFileCreateNewError, AtomicFileCreateNewHooks, AtomicFileReplaceError,
    AtomicFileReplaceHooks, MAX_ATOMIC_FILE_PATH_BYTES, atomic_file_create_new,
    atomic_file_replace,
};
use vexel_project::{self as project, BundleInput, BundleView};

pub const RECOVERY_VERSION: u32 = 1;
pub const RECOVERY_HEADER_BYTES: usize = 72;
pub const RECOVERY_MAX_PATH_BYTES: usize = 1023;
pub const RECOVERY_MAX_BYTES: usize =
    project::MAX_BYTES + RECOVERY_HEADER_BYTES + RECOVERY_MAX_PATH_BYTES;
const MAGIC: &[u8; 8] = b"VXRECOV\x01";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecoveryError {
    Ok,
    InvalidArgument,
    Limit,
    OutOfMemory,
    PathLimit,
    AlreadyExists,
    Io,
    Durability,
    Corrupt,
    UnsupportedVersion,
    PathMismatch,
    InvalidProject,
}

impl RecoveryError {
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
            Self::Corrupt => "corrupt",
            Self::UnsupportedVersion => "unsupported_version",
            Self::PathMismatch => "path_mismatch",
            Self::InvalidProject => "invalid_project",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RecoveryResult {
    pub error: RecoveryError,
    pub bundle_error: project::Error,
    pub replace_error: AtomicFileReplaceError,
    pub create_error: AtomicFileCreateNewError,
    pub encoded_size: usize,
    pub bytes_written: usize,
    pub error_offset: usize,
    pub published: bool,
    pub directory_fsynced: bool,
}

/// Failure and operation state matching the original C result contract.
pub type RecoveryFailure = RecoveryResult;

impl RecoveryResult {
    const fn error(error: RecoveryError) -> Self {
        Self {
            error,
            bundle_error: project::Error::Ok,
            replace_error: AtomicFileReplaceError::Ok,
            create_error: AtomicFileCreateNewError::Ok,
            encoded_size: 0,
            bytes_written: 0,
            error_offset: 0,
            published: false,
            directory_fsynced: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RecoveryInput<'a> {
    pub source_path: &'a [u8],
    pub source_size: u64,
    pub source_modified_seconds: i64,
    pub source_modified_nanoseconds: u32,
    pub captured_time_nanoseconds: u64,
    pub project: BundleInput<'a>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RecoveryView<'a> {
    pub source_path: &'a [u8],
    pub source_size: u64,
    pub source_modified_seconds: i64,
    pub source_modified_nanoseconds: u32,
    pub captured_time_nanoseconds: u64,
    pub project: BundleView<'a>,
}

/// Measures the exact v1 capsule size after validating path, timestamp, and
/// embedded bundle constraints.
pub fn project_recovery_encoded_size(input: &RecoveryInput<'_>) -> RecoveryResult {
    if !valid_source_path(input.source_path) || input.source_modified_nanoseconds >= 1_000_000_000 {
        return RecoveryResult::error(RecoveryError::InvalidArgument);
    }
    let bundle_size = match project::encoded_size(&input.project) {
        Ok(size) => size,
        Err(failure) => return bundle_input_failure(failure.error),
    };
    let Some(size) = RECOVERY_HEADER_BYTES
        .checked_add(input.source_path.len())
        .and_then(|size| size.checked_add(bundle_size))
    else {
        return RecoveryResult::error(RecoveryError::Limit);
    };
    if size > RECOVERY_MAX_BYTES {
        return RecoveryResult::error(RecoveryError::Limit);
    }
    let mut result = RecoveryResult::error(RecoveryError::Ok);
    result.encoded_size = size;
    result
}

/// Writes the full capsule to caller storage after all input constraints and
/// destination capacity have been checked. The embedded project is encoded
/// directly into the destination; no input payload is cloned.
pub fn project_recovery_encode_into(
    input: &RecoveryInput<'_>,
    output: &mut [u8],
) -> RecoveryResult {
    let mut measured = project_recovery_encoded_size(input);
    if measured.error != RecoveryError::Ok {
        return measured;
    }
    if output.len() < measured.encoded_size {
        measured.error = RecoveryError::InvalidArgument;
        return measured;
    }

    output[..8].copy_from_slice(MAGIC);
    put_u32(output, 8, RECOVERY_VERSION);
    put_u32(output, 12, RECOVERY_HEADER_BYTES as u32);
    put_u32(output, 16, input.source_path.len() as u32);
    put_u32(output, 20, 0);
    put_u64(output, 24, input.source_size);
    put_u64(output, 32, input.source_modified_seconds as u64);
    put_u32(output, 40, input.source_modified_nanoseconds);
    let bundle_size = measured.encoded_size - RECOVERY_HEADER_BYTES - input.source_path.len();
    put_u64(output, 44, bundle_size as u64);
    put_u32(output, 52, 0);
    put_u32(output, 56, crc32(input.source_path));
    put_u32(output, 60, 0);
    put_u64(output, 64, input.captured_time_nanoseconds);
    let path_end = RECOVERY_HEADER_BYTES + input.source_path.len();
    output[RECOVERY_HEADER_BYTES..path_end].copy_from_slice(input.source_path);
    let bundle_result = project::encode_into(
        &input.project,
        &mut output[path_end..path_end + bundle_size],
    );
    match bundle_result {
        Ok(written) if written == bundle_size => {}
        Ok(_) => {
            measured.error = RecoveryError::InvalidProject;
            measured.bundle_error = project::Error::InvalidArgument;
            return measured;
        }
        Err(failure) => {
            measured.error = match failure.error {
                project::Error::Limit => RecoveryError::Limit,
                _ => RecoveryError::InvalidProject,
            };
            measured.bundle_error = failure.error;
            return measured;
        }
    }
    put_u32(output, 52, crc32(&output[path_end..path_end + bundle_size]));
    let record_crc = crc32_update(u32::MAX, &output[..RECOVERY_HEADER_BYTES]);
    let record_crc = crc32_update(
        record_crc,
        &output[RECOVERY_HEADER_BYTES..measured.encoded_size],
    );
    put_u32(output, 60, !record_crc);
    measured
}

/// Validates a full capsule and returns borrowed path/project views. Pass
/// `None` or an empty slice to skip source identity comparison.
pub fn project_recovery_decode<'a>(
    encoded: &'a [u8],
    expected_source_path: Option<&[u8]>,
) -> Result<RecoveryView<'a>, RecoveryFailure> {
    if encoded.len() < RECOVERY_HEADER_BYTES {
        return Err(RecoveryResult::error(RecoveryError::InvalidArgument));
    }
    if encoded.get(..8) != Some(MAGIC.as_slice())
        || read_u32(encoded, 12) != Some(RECOVERY_HEADER_BYTES as u32)
        || read_u32(encoded, 20) != Some(0)
    {
        return Err(RecoveryResult::error(RecoveryError::Corrupt));
    }
    let Some(version) = read_u32(encoded, 8) else {
        return Err(RecoveryResult::error(RecoveryError::Corrupt));
    };
    if version != RECOVERY_VERSION {
        return Err(RecoveryResult::error(RecoveryError::UnsupportedVersion));
    }
    let path_length = read_u32(encoded, 16).unwrap_or_default() as usize;
    let raw_bundle_length = read_u64(encoded, 44).unwrap_or(u64::MAX);
    let nanoseconds = read_u32(encoded, 40).unwrap_or(u32::MAX);
    if path_length == 0
        || path_length > RECOVERY_MAX_PATH_BYTES
        || nanoseconds >= 1_000_000_000
        || raw_bundle_length > project::MAX_BYTES as u64
        || raw_bundle_length > usize::MAX as u64
    {
        return Err(RecoveryResult::error(RecoveryError::Corrupt));
    }
    let bundle_length = raw_bundle_length as usize;
    let Some(expected_length) = RECOVERY_HEADER_BYTES
        .checked_add(path_length)
        .and_then(|length| length.checked_add(bundle_length))
    else {
        return Err(RecoveryResult::error(RecoveryError::Corrupt));
    };
    if expected_length != encoded.len() || expected_length > RECOVERY_MAX_BYTES {
        return Err(RecoveryResult::error(RecoveryError::Corrupt));
    }
    let path_end = RECOVERY_HEADER_BYTES + path_length;
    let source_path = &encoded[RECOVERY_HEADER_BYTES..path_end];
    let bundle_bytes = &encoded[path_end..expected_length];
    let mut checksum_header = [0; RECOVERY_HEADER_BYTES];
    checksum_header.copy_from_slice(&encoded[..RECOVERY_HEADER_BYTES]);
    checksum_header[60..64].fill(0);
    let record_crc = crc32_update(u32::MAX, &checksum_header);
    let record_crc = crc32_update(record_crc, &encoded[RECOVERY_HEADER_BYTES..]);
    if !valid_source_path(source_path)
        || crc32(source_path) != read_u32(encoded, 56).unwrap_or_default()
        || crc32(bundle_bytes) != read_u32(encoded, 52).unwrap_or_default()
        || !record_crc != read_u32(encoded, 60).unwrap_or_default()
    {
        return Err(RecoveryResult::error(RecoveryError::Corrupt));
    }
    if let Some(expected) = expected_source_path.filter(|path| !path.is_empty()) {
        if expected != source_path {
            return Err(RecoveryResult::error(RecoveryError::PathMismatch));
        }
    }
    let decoded = project::decode(bundle_bytes).map_err(|failure| {
        let mut result =
            RecoveryResult::error(if failure.error == project::Error::UnsupportedVersion {
                RecoveryError::UnsupportedVersion
            } else {
                RecoveryError::InvalidProject
            });
        result.bundle_error = failure.error;
        result.error_offset = failure.error_offset;
        result
    })?;
    Ok(RecoveryView {
        source_path,
        source_size: read_u64(encoded, 24).unwrap_or_default(),
        source_modified_seconds: read_u64(encoded, 32).unwrap_or_default() as i64,
        source_modified_nanoseconds: nanoseconds,
        captured_time_nanoseconds: read_u64(encoded, 64).unwrap_or_default(),
        project: decoded,
    })
}

/// Atomically publishes a capsule to a path distinct from the source path.
/// It does not alter the source file or saved-state metadata.
pub fn project_recovery_save(path: &[u8], input: &RecoveryInput<'_>) -> RecoveryResult {
    if path.is_empty() || path.contains(&0) || input.source_path.is_empty() {
        return RecoveryResult::error(RecoveryError::InvalidArgument);
    }
    if path.len() > MAX_ATOMIC_FILE_PATH_BYTES {
        return RecoveryResult::error(RecoveryError::PathLimit);
    }
    let measured = project_recovery_encoded_size(input);
    if measured.error != RecoveryError::Ok {
        return measured;
    }
    if path == input.source_path {
        return RecoveryResult::error(RecoveryError::InvalidArgument);
    }
    let mut bytes = Vec::new();
    if bytes.try_reserve_exact(measured.encoded_size).is_err() {
        return RecoveryResult::error(RecoveryError::OutOfMemory);
    }
    bytes.resize(measured.encoded_size, 0);
    let encoded = project_recovery_encode_into(input, &mut bytes);
    if encoded.error != RecoveryError::Ok {
        return encoded;
    }
    recovery_save_encoded_with_hooks(path, &bytes, None, None)
}

pub(super) fn recovery_save_encoded_with_hooks(
    path: &[u8],
    bytes: &[u8],
    replace_hooks: Option<&mut dyn AtomicFileReplaceHooks>,
    create_hooks: Option<&mut dyn AtomicFileCreateNewHooks>,
) -> RecoveryResult {
    let target = Path::new(OsStr::from_bytes(path));
    let metadata = fs::symlink_metadata(target);
    let mut result = RecoveryResult::error(RecoveryError::Io);
    match metadata {
        Ok(metadata) if metadata.file_type().is_file() => {
            let written = atomic_file_replace(path, bytes, replace_hooks);
            result.replace_error = written.error;
            result.published = written.renamed;
            result.directory_fsynced = written.directory_synced;
            result.bytes_written = bytes.len();
            result.error = match written.error {
                AtomicFileReplaceError::Ok => RecoveryError::Ok,
                AtomicFileReplaceError::Durability => RecoveryError::Durability,
                _ => RecoveryError::Io,
            };
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let written = atomic_file_create_new(path, bytes, create_hooks);
            result.create_error = written.error;
            result.published = written.published;
            result.directory_fsynced = written.directory_synced;
            result.bytes_written = bytes.len();
            result.error = match written.error {
                AtomicFileCreateNewError::Ok => RecoveryError::Ok,
                AtomicFileCreateNewError::Exists => RecoveryError::AlreadyExists,
                AtomicFileCreateNewError::Durability => RecoveryError::Durability,
                _ => RecoveryError::Io,
            };
        }
        _ => {}
    }
    result
}

fn valid_source_path(path: &[u8]) -> bool {
    !path.is_empty()
        && path.len() <= RECOVERY_MAX_PATH_BYTES
        && path[0] == b'/'
        && !path.contains(&0)
}

fn bundle_input_failure(error: project::Error) -> RecoveryResult {
    let mut result = RecoveryResult::error(match error {
        project::Error::Limit => RecoveryError::Limit,
        _ => RecoveryError::InvalidProject,
    });
    result.bundle_error = error;
    result
}

fn crc32_update(mut crc: u32, bytes: &[u8]) -> u32 {
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            let mask = 0u32.wrapping_sub(crc & 1);
            crc = (crc >> 1) ^ (0xedb8_8320 & mask);
        }
    }
    crc
}

fn crc32(bytes: &[u8]) -> u32 {
    !crc32_update(u32::MAX, bytes)
}

fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn put_u64(bytes: &mut [u8], offset: usize, value: u64) {
    bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

fn read_u32(bytes: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        bytes.get(offset..offset.checked_add(4)?)?.try_into().ok()?,
    ))
}

fn read_u64(bytes: &[u8], offset: usize) -> Option<u64> {
    Some(u64::from_le_bytes(
        bytes.get(offset..offset.checked_add(8)?)?.try_into().ok()?,
    ))
}

pub const fn project_recovery_error_name(error: RecoveryError) -> &'static str {
    error.name()
}
