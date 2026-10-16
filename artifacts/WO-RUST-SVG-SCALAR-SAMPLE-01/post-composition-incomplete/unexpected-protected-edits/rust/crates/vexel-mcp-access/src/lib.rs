#![forbid(unsafe_code)]
//! Filesystem access grants used by the MCP document and save operations.
//!
//! The policy owns the environment inputs so callers and tests do not need to
//! mutate process-global environment variables. Filesystem checks follow the
//! same canonicalize/stat sequence as the C implementation; concurrent path
//! replacement can therefore race these checks and later file operations.

use std::ffi::{OsStr, OsString};
use std::fs::{self, File};
use std::io::{BufReader, Read};
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

/// Largest accepted input and canonical output path, including the C NUL cap.
pub const PATH_CAP: usize = 1024;
/// Largest accepted `VEXEL_MCP_ALLOWED_DIRS` value, including the C NUL cap.
pub const ALLOWED_DIRS_CAP: usize = 8192;
const CONFIG_LINE_CAP: usize = PATH_CAP - 1;
const CONFIG_LINE_LIMIT: usize = 128;
const CONFIG_SUFFIX: &[u8] = b"/.config/vexel/mcp-allowed-dirs.txt";

/// Why a requested path could not be resolved under the access contract.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AccessError {
    /// Input is empty, contains an interior NUL, or exceeds the fixed cap.
    InvalidPath,
    /// The path does not exist or could not be canonicalized.
    NotFound,
    /// Existing targets must be regular files.
    NotRegularFile,
    /// A grant target or save parent must be a directory.
    NotDirectory,
    /// The canonical path is outside every configured grant.
    NotGranted,
}

/// Owned access inputs. `allowed_dirs` and `home` are snapshots, not live
/// references to process environment state.
#[derive(Clone, Debug)]
pub struct AccessPolicy {
    allowed_dirs: Option<OsString>,
    home: Option<PathBuf>,
    current_uid: u32,
}

impl AccessPolicy {
    /// Create an explicit policy, primarily for deterministic application
    /// composition and tests. The real uid is captured from the accepted
    /// native macOS identity interface.
    pub fn new(allowed_dirs: Option<OsString>, home: Option<PathBuf>) -> Self {
        Self {
            allowed_dirs,
            home,
            current_uid: vexel_platform_macos::current_uid(),
        }
    }

    /// Snapshot `VEXEL_MCP_ALLOWED_DIRS` and `HOME` from the current process.
    pub fn from_environment() -> Self {
        Self::new(
            std::env::var_os("VEXEL_MCP_ALLOWED_DIRS"),
            std::env::var_os("HOME").map(PathBuf::from),
        )
    }

    /// Resolve an existing regular file to its canonical absolute path.
    /// Inputs are raw macOS path bytes; `&str` and other `AsRef<OsStr>` values
    /// are accepted without conversion through UTF-8.
    pub fn resolve_existing(&self, path: impl AsRef<OsStr>) -> Result<PathBuf, AccessError> {
        let path = path.as_ref();
        validate_input(path)?;
        let canonical = fs::canonicalize(path).map_err(|_| AccessError::NotFound)?;
        ensure_output_bound(&canonical)?;
        let metadata = fs::metadata(&canonical).map_err(|_| AccessError::NotFound)?;
        if !metadata.is_file() {
            return Err(AccessError::NotRegularFile);
        }
        Ok(canonical)
    }

    /// Return whether an existing regular file is inside a configured grant.
    pub fn path_allowed(&self, path: impl AsRef<OsStr>) -> bool {
        let path = path.as_ref();
        let Ok(canonical) = self.resolve_existing(path) else {
            return false;
        };
        self.is_granted(&canonical)
    }

    /// Resolve an existing regular file or a new leaf in an existing,
    /// canonical directory, requiring a grant in either case.
    pub fn resolve_save_path(&self, path: impl AsRef<OsStr>) -> Result<PathBuf, AccessError> {
        let path = path.as_ref();
        validate_input(path)?;
        let requested = Path::new(path);
        match fs::symlink_metadata(requested) {
            Ok(_) => {
                let canonical = self.resolve_existing(path)?;
                if !self.is_granted(&canonical) {
                    return Err(AccessError::NotGranted);
                }
                Ok(canonical)
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                self.resolve_new_save_path(path)
            }
            Err(_) => Err(AccessError::NotFound),
        }
    }

    /// Resolve a granted directory to its canonical absolute path.
    pub fn resolve_granted_directory(
        &self,
        path: impl AsRef<OsStr>,
    ) -> Result<PathBuf, AccessError> {
        let path = path.as_ref();
        validate_input(path)?;
        let canonical = fs::canonicalize(path).map_err(|_| AccessError::NotFound)?;
        ensure_output_bound(&canonical)?;
        let metadata = fs::metadata(&canonical).map_err(|_| AccessError::NotFound)?;
        if !metadata.is_dir() {
            return Err(AccessError::NotDirectory);
        }
        if !self.is_granted(&canonical) {
            return Err(AccessError::NotGranted);
        }
        Ok(canonical)
    }

    fn is_granted(&self, path: &Path) -> bool {
        self.granted_by_environment(path) || self.granted_by_config(path)
    }

    fn granted_by_environment(&self, path: &Path) -> bool {
        let Some(value) = self.allowed_dirs.as_ref() else {
            return false;
        };
        let bytes = value.as_bytes();
        if bytes.is_empty() || bytes.len() >= ALLOWED_DIRS_CAP {
            return false;
        }
        let mut start = 0;
        while start < bytes.len() {
            let end = bytes[start..]
                .iter()
                .position(|byte| *byte == b':')
                .map_or(bytes.len(), |offset| start + offset);
            let item = &bytes[start..end];
            if !item.is_empty() && item.len() < PATH_CAP {
                let candidate = PathBuf::from(OsString::from_vec(item.to_vec()));
                if canonical_directory(&candidate)
                    .is_ok_and(|directory| path_is_inside(&directory, path))
                {
                    return true;
                }
            }
            start = end.saturating_add(1);
        }
        false
    }

    fn granted_by_config(&self, path: &Path) -> bool {
        let Some(home) = self.home.as_ref() else {
            return false;
        };
        let home_bytes = home.as_os_str().as_bytes();
        if home_bytes.first() != Some(&b'/')
            || home_bytes.is_empty()
            || home_bytes
                .len()
                .checked_add(CONFIG_SUFFIX.len() + 1)
                .is_none_or(|n| n > PATH_CAP)
        {
            return false;
        }
        let mut config_bytes = Vec::with_capacity(home_bytes.len() + CONFIG_SUFFIX.len());
        config_bytes.extend_from_slice(home_bytes);
        config_bytes.extend_from_slice(CONFIG_SUFFIX);
        let config = PathBuf::from(OsString::from_vec(config_bytes));
        let Ok(metadata) = fs::symlink_metadata(&config) else {
            return false;
        };
        if !metadata.file_type().is_file()
            || metadata.uid() != self.current_uid
            || metadata.mode() & 0o022 != 0
        {
            return false;
        }
        let Ok(file) = File::open(&config) else {
            return false;
        };
        config_grants(BufReader::with_capacity(PATH_CAP, file), path)
    }

    fn resolve_new_save_path(&self, requested: &OsStr) -> Result<PathBuf, AccessError> {
        let bytes = requested.as_bytes();
        let slash = bytes.iter().rposition(|byte| *byte == b'/');
        let (parent, name_bytes) = match slash {
            None => (Path::new("."), bytes),
            Some(0) => (Path::new("/"), &bytes[1..]),
            Some(index) => (
                Path::new(OsStr::from_bytes(&bytes[..index])),
                &bytes[index + 1..],
            ),
        };
        if name_bytes.is_empty() || name_bytes == b"." || name_bytes == b".." {
            return Err(AccessError::InvalidPath);
        }
        let canonical_parent = fs::canonicalize(parent).map_err(|_| AccessError::NotFound)?;
        let metadata = fs::metadata(&canonical_parent).map_err(|_| AccessError::NotFound)?;
        if !metadata.is_dir() {
            return Err(AccessError::NotDirectory);
        }
        let mut output = canonical_parent;
        output.push(std::ffi::OsStr::from_bytes(name_bytes));
        ensure_output_bound(&output)?;
        if !self.is_granted(&output) {
            return Err(AccessError::NotGranted);
        }
        Ok(output)
    }
}

fn validate_input(path: &OsStr) -> Result<(), AccessError> {
    let bytes = path.as_bytes();
    if bytes.is_empty() || bytes.contains(&0) || bytes.len() >= PATH_CAP {
        Err(AccessError::InvalidPath)
    } else {
        Ok(())
    }
}

fn ensure_output_bound(path: &Path) -> Result<(), AccessError> {
    if path.as_os_str().as_bytes().len() >= PATH_CAP {
        Err(AccessError::InvalidPath)
    } else {
        Ok(())
    }
}

fn canonical_directory(path: &Path) -> Result<PathBuf, AccessError> {
    let canonical = fs::canonicalize(path).map_err(|_| AccessError::NotFound)?;
    ensure_output_bound(&canonical)?;
    if !fs::metadata(&canonical)
        .map_err(|_| AccessError::NotFound)?
        .is_dir()
    {
        return Err(AccessError::NotDirectory);
    }
    Ok(canonical)
}

fn path_is_inside(directory: &Path, path: &Path) -> bool {
    let directory = directory.as_os_str().as_bytes();
    let path = path.as_os_str().as_bytes();
    if !path.starts_with(directory) {
        return false;
    }
    if directory == b"/" {
        return path.first() == Some(&b'/');
    }
    path.get(directory.len()).is_none_or(|byte| *byte == b'/')
}

fn config_grants<R: Read>(mut reader: BufReader<R>, path: &Path) -> bool {
    let mut line = [0_u8; CONFIG_LINE_CAP];
    let mut lines = 0;
    while lines < CONFIG_LINE_LIMIT {
        let mut length = 0;
        let mut complete = false;
        while length < line.len() {
            let mut byte = [0_u8; 1];
            match reader.read(&mut byte) {
                Ok(0) => {
                    complete = true;
                    break;
                }
                Ok(_) => {
                    line[length] = byte[0];
                    length += 1;
                    if byte[0] == b'\n' {
                        complete = true;
                        break;
                    }
                }
                Err(_) => return false,
            }
        }
        if length == 0 {
            break;
        }
        lines += 1;
        if length == CONFIG_LINE_CAP && !complete {
            continue;
        }
        let line_bytes = &line[..length];
        let nul = line_bytes
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(length);
        let mut end = nul;
        while end > 0 && is_ascii_space(line[end - 1]) {
            end -= 1;
        }
        let mut start = 0;
        while start < end && is_ascii_space(line[start]) {
            start += 1;
        }
        if start == end || line[start] == b'#' {
            continue;
        }
        let candidate = PathBuf::from(OsString::from_vec(line[start..end].to_vec()));
        if canonical_directory(&candidate).is_ok_and(|directory| path_is_inside(&directory, path)) {
            return true;
        }
    }
    false
}

fn is_ascii_space(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c)
}
