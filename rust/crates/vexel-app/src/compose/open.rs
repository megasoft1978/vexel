//! The single open path shared by the CLI argument and any menu/open-panel
//! action. Local files only; no network, bounded size, strict path checks.

use super::decode::MAX_SVG_BYTES;
use std::fmt;
use std::fs::File;
use std::io::Read;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};

const MAX_PATH_BYTES: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenError {
    /// Empty, over-long, NUL-containing or non-UTF-8 path.
    InvalidPath,
    /// Missing, permission denied or other I/O failure.
    Unreadable(std::io::ErrorKind),
    /// A directory or other non-regular file.
    NotAFile,
    /// Larger than [`MAX_SVG_BYTES`].
    TooLarge,
}

impl fmt::Display for OpenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPath => write!(f, "invalid file path"),
            Self::Unreadable(kind) => write!(f, "cannot read file ({kind})"),
            Self::NotAFile => write!(f, "not a regular file"),
            Self::TooLarge => write!(f, "file is larger than {} MiB", MAX_SVG_BYTES >> 20),
        }
    }
}

impl std::error::Error for OpenError {}

/// File contents read within the size cap.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenedSvg {
    pub path: PathBuf,
    pub bytes: Vec<u8>,
}

impl OpenedSvg {
    /// Final path component for display, lossy-free because paths are UTF-8.
    pub fn display_name(&self) -> String {
        self.path.file_name().map_or_else(
            || self.path.display().to_string(),
            |n| n.to_string_lossy().into_owned(),
        )
    }
}

/// Reads `path` for composition. Used by both the command line and the
/// File > Open... action so they cannot diverge.
pub fn open_path(path: &Path) -> Result<OpenedSvg, OpenError> {
    let raw = path.as_os_str().as_bytes();
    if raw.is_empty()
        || raw.len() > MAX_PATH_BYTES
        || raw.contains(&0)
        || std::str::from_utf8(raw).is_err()
    {
        return Err(OpenError::InvalidPath);
    }
    let io = |e: std::io::Error| OpenError::Unreadable(e.kind());
    let metadata = std::fs::metadata(path).map_err(io)?;
    if !metadata.is_file() {
        return Err(OpenError::NotAFile);
    }
    if metadata.len() > MAX_SVG_BYTES as u64 {
        return Err(OpenError::TooLarge);
    }
    let mut bytes = Vec::new();
    // One extra byte detects a file that grew past the cap after `metadata`.
    File::open(path)
        .map_err(io)?
        .take(MAX_SVG_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(io)?;
    if bytes.len() > MAX_SVG_BYTES {
        return Err(OpenError::TooLarge);
    }
    Ok(OpenedSvg {
        path: path.to_path_buf(),
        bytes,
    })
}
