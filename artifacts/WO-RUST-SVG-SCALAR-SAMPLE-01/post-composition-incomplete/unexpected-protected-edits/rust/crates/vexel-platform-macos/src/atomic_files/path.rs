use super::MAX_ATOMIC_FILE_PATH_BYTES;
use crate::{SystemError, fill_random};
use std::ffi::{OsStr, OsString};
use std::fs::{File, OpenOptions};
use std::io;
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

const PATH_MAX_BYTES: usize = 1024;
const TEMP_ATTEMPTS: usize = 128;
const DARWIN_O_NOFOLLOW: i32 = 0x0100;
const TEMP_ALPHABET: &[u8; 62] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";

#[derive(Debug)]
pub(super) enum PathError {
    Invalid,
    Limit,
}

#[derive(Debug)]
pub(super) enum TempError {
    Limit,
    Io,
}

pub(super) fn target_path(bytes: &[u8]) -> Result<PathBuf, PathError> {
    if bytes.is_empty() {
        return Err(PathError::Invalid);
    }
    if bytes.len() > MAX_ATOMIC_FILE_PATH_BYTES || bytes.len() >= PATH_MAX_BYTES {
        return Err(PathError::Limit);
    }
    if bytes.contains(&0) {
        return Err(PathError::Invalid);
    }
    Ok(PathBuf::from(OsStr::from_bytes(bytes)))
}

pub(super) fn parent_bytes(target: &[u8]) -> &[u8] {
    match target.iter().rposition(|byte| *byte == b'/') {
        None => b".",
        Some(0) => b"/",
        Some(index) => &target[..index],
    }
}

pub(super) fn validate_new_name(target: &[u8]) -> Result<(), PathError> {
    let name = target
        .rsplit(|byte| *byte == b'/')
        .next()
        .unwrap_or_default();
    if name.is_empty() || name == b"." || name == b".." {
        return Err(PathError::Invalid);
    }
    Ok(())
}

pub(super) fn open_regular_nofollow(path: &Path) -> io::Result<File> {
    let mut options = OpenOptions::new();
    options.read(true).custom_flags(DARWIN_O_NOFOLLOW);
    options.open(path)
}

pub(super) fn create_temp(directory: &[u8], prefix: &[u8]) -> Result<(File, PathBuf), TempError> {
    let separator = usize::from(directory.last() != Some(&b'/'));
    let path_len = directory
        .len()
        .checked_add(separator)
        .and_then(|length| length.checked_add(prefix.len()))
        .and_then(|length| length.checked_add(6))
        .ok_or(TempError::Limit)?;
    if path_len >= PATH_MAX_BYTES {
        return Err(TempError::Limit);
    }

    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(path_len)
        .map_err(|_| TempError::Io)?;
    bytes.extend_from_slice(directory);
    if separator != 0 {
        bytes.push(b'/');
    }
    bytes.extend_from_slice(prefix);
    let prefix_len = bytes.len();

    for _ in 0..TEMP_ATTEMPTS {
        let mut random = [0_u8; 6];
        fill_random(&mut random).map_err(|error| match error {
            SystemError::RandomRequestTooLarge { .. } => TempError::Io,
            SystemError::RandomStatus(_) | SystemError::PeerCredentialsErrno(_) => TempError::Io,
        })?;

        bytes.truncate(prefix_len);
        for byte in random {
            bytes.push(TEMP_ALPHABET[(byte as usize) % TEMP_ALPHABET.len()]);
        }
        let path = PathBuf::from(OsString::from_vec(bytes.clone()));
        let mut options = OpenOptions::new();
        options.read(true).write(true).create_new(true).mode(0o600);
        match options.open(&path) {
            Ok(file) => return Ok((file, path)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(_) => return Err(TempError::Io),
        }
    }
    Err(TempError::Io)
}

pub(super) fn remove_temp(path: &Path) {
    let _ = std::fs::remove_file(path);
}
