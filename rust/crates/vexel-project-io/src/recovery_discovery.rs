use super::recovery::{project_recovery_decode, RecoveryError, RECOVERY_MAX_BYTES};
use std::ffi::OsStr;
use std::fs::{self, File, Metadata};
use std::io::Read;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::path::Path;

pub enum RecoveryDiscovery {
    Absent,
    Matching { capsule_bytes: Vec<u8> },
    Conflict { capsule_bytes: Vec<u8> },
    Invalid { error: RecoveryError },
}

/// Finds and validates a recovery capsule beside a source path supplied by the caller.
/// This only reads metadata and capsule bytes; it never reads the source body or writes files.
pub fn project_recovery_discover(source_path: &[u8], capsule_path: &[u8]) -> RecoveryDiscovery {
    project_recovery_discover_with_after_capsule_read(source_path, capsule_path, || {})
}

fn project_recovery_discover_with_after_capsule_read(
    source_path: &[u8],
    capsule_path: &[u8],
    after_capsule_read: impl FnOnce(),
) -> RecoveryDiscovery {
    if !valid_path(source_path) || !valid_path(capsule_path) || source_path == capsule_path {
        return invalid(RecoveryError::InvalidArgument);
    }

    let source_path_os = Path::new(OsStr::from_bytes(source_path));
    let capsule_path_os = Path::new(OsStr::from_bytes(capsule_path));
    let source_link_metadata = match fs::symlink_metadata(source_path_os) {
        Ok(metadata) if metadata.file_type().is_file() => metadata,
        _ => return invalid(RecoveryError::InvalidArgument),
    };
    let source_file = match File::open(source_path_os) {
        Ok(file) => file,
        Err(_) => return invalid(RecoveryError::Io),
    };
    let source_metadata = match source_file.metadata() {
        Ok(metadata) if metadata.file_type().is_file() => metadata,
        _ => return invalid(RecoveryError::InvalidArgument),
    };
    if !same_file(&source_link_metadata, &source_metadata) {
        return invalid(RecoveryError::InvalidArgument);
    }

    match fs::symlink_metadata(capsule_path_os) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return RecoveryDiscovery::Absent;
        }
        Err(_) => return invalid(RecoveryError::Io),
        Ok(metadata) if !metadata.file_type().is_file() => {
            return invalid(RecoveryError::InvalidArgument);
        }
        Ok(_) => {}
    }

    let capsule_file = match File::open(capsule_path_os) {
        Ok(file) => file,
        Err(_) => return invalid(RecoveryError::Io),
    };
    let capsule_metadata = match capsule_file.metadata() {
        Ok(metadata) if metadata.file_type().is_file() => metadata,
        _ => return invalid(RecoveryError::InvalidArgument),
    };
    if same_file(&source_metadata, &capsule_metadata) {
        return invalid(RecoveryError::InvalidArgument);
    }
    let capsule_path_metadata = match fs::symlink_metadata(capsule_path_os) {
        Ok(metadata) if metadata.file_type().is_file() => metadata,
        _ => return invalid(RecoveryError::InvalidArgument),
    };
    if !same_file(&capsule_metadata, &capsule_path_metadata) {
        return invalid(RecoveryError::InvalidArgument);
    }
    if capsule_metadata.len() > RECOVERY_MAX_BYTES as u64 {
        return invalid(RecoveryError::Limit);
    }

    let mut capsule_bytes = Vec::new();
    if capsule_bytes
        .try_reserve_exact(capsule_metadata.len() as usize)
        .is_err()
    {
        return invalid(RecoveryError::OutOfMemory);
    }
    let read_limit = RECOVERY_MAX_BYTES as u64 + 1;
    let mut bounded = (&capsule_file).take(read_limit);
    if bounded.read_to_end(&mut capsule_bytes).is_err() {
        return invalid(RecoveryError::Io);
    }
    after_capsule_read();
    if capsule_bytes.len() > RECOVERY_MAX_BYTES {
        return invalid(RecoveryError::Limit);
    }
    let capsule_metadata_after = match capsule_file.metadata() {
        Ok(metadata) if metadata.file_type().is_file() => metadata,
        _ => return invalid(RecoveryError::InvalidArgument),
    };
    let capsule_path_after = match fs::symlink_metadata(capsule_path_os) {
        Ok(metadata) if metadata.file_type().is_file() => metadata,
        _ => return invalid(RecoveryError::InvalidArgument),
    };
    if capsule_metadata_after.len() > RECOVERY_MAX_BYTES as u64 {
        return invalid(RecoveryError::Limit);
    }
    if !same_file(&capsule_metadata_after, &capsule_path_after) {
        return invalid(RecoveryError::InvalidArgument);
    }

    let source_metadata_after = match source_file.metadata() {
        Ok(metadata) if metadata.file_type().is_file() => metadata,
        _ => return invalid(RecoveryError::InvalidArgument),
    };
    let source_path_after = match fs::symlink_metadata(source_path_os) {
        Ok(metadata) if metadata.file_type().is_file() => metadata,
        _ => return invalid(RecoveryError::InvalidArgument),
    };
    if !same_file(&source_metadata_after, &source_path_after) {
        return invalid(RecoveryError::InvalidArgument);
    }

    let view = match project_recovery_decode(&capsule_bytes, Some(source_path)) {
        Ok(view) => view,
        Err(failure) => return invalid(failure.error),
    };
    let stamp_matches = view.source_size == source_metadata_after.len()
        && view.source_modified_seconds == source_metadata_after.mtime()
        && view.source_modified_nanoseconds == source_metadata_after.mtime_nsec() as u32;
    if stamp_matches {
        RecoveryDiscovery::Matching { capsule_bytes }
    } else {
        RecoveryDiscovery::Conflict { capsule_bytes }
    }
}

fn valid_path(path: &[u8]) -> bool {
    !path.is_empty() && !path.contains(&0)
}

fn same_file(left: &Metadata, right: &Metadata) -> bool {
    left.dev() == right.dev() && left.ino() == right.ino()
}

fn invalid(error: RecoveryError) -> RecoveryDiscovery {
    RecoveryDiscovery::Invalid { error }
}

#[cfg(test)]
#[path = "recovery_discovery_tests.rs"]
mod tests;
