mod create_new;
mod path;
mod replace;

#[cfg(test)]
mod tests;

use std::fmt;
use std::fs::File;
use std::io;
use std::path::Path;

/// Largest accepted path, in bytes excluding the terminating NUL used by C.
pub const MAX_ATOMIC_FILE_PATH_BYTES: usize = 1023;

/// Failure classes returned by atomic replacement of an existing file.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AtomicFileReplaceError {
    Ok,
    Invalid,
    PathLimit,
    NotFound,
    NotRegular,
    Io,
    Durability,
    Unknown,
}

impl fmt::Display for AtomicFileReplaceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Ok => "ok",
            Self::Invalid => "invalid",
            Self::PathLimit => "path_limit",
            Self::NotFound => "not_found",
            Self::NotRegular => "not_regular",
            Self::Io => "io",
            Self::Durability => "durability",
            Self::Unknown => "unknown",
        })
    }
}

impl std::error::Error for AtomicFileReplaceError {}

/// Failure classes returned by atomic no-replace publication.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AtomicFileCreateNewError {
    Ok,
    Invalid,
    PathLimit,
    Exists,
    Io,
    Durability,
}

impl fmt::Display for AtomicFileCreateNewError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Ok => "ok",
            Self::Invalid => "invalid",
            Self::PathLimit => "path_limit",
            Self::Exists => "exists",
            Self::Io => "io",
            Self::Durability => "durability",
        })
    }
}

impl std::error::Error for AtomicFileCreateNewError {}

/// Safe fault and observation hooks for deterministic failure and race tests.
/// Returning success only allows the mandatory native operation to proceed;
/// hooks cannot supply successful writes, syncs, closes, renames, or publishes.
pub trait AtomicFileReplaceHooks {
    /// Return a maximum native write length, or inject an error before writing.
    fn before_write(&mut self, _file: &File, remaining: &[u8]) -> io::Result<usize> {
        Ok(remaining.len())
    }

    fn before_sync_file(&mut self, _file: &File) -> io::Result<()> {
        Ok(())
    }

    fn before_close(&mut self, _file: &File) -> io::Result<()> {
        Ok(())
    }

    fn before_rename(&mut self, _from: &Path, _to: &Path) -> io::Result<()> {
        Ok(())
    }

    fn before_sync_directory(&mut self, _directory: &Path) -> io::Result<()> {
        Ok(())
    }
}

/// Safe fault and observation hooks for deterministic failure and publication-race tests.
/// Successful hooks never replace the mandatory native filesystem operation.
pub trait AtomicFileCreateNewHooks {
    /// Return a maximum native write length, or inject an error before writing.
    fn before_write(&mut self, _file: &File, remaining: &[u8]) -> io::Result<usize> {
        Ok(remaining.len())
    }

    fn before_sync_file(&mut self, _file: &File) -> io::Result<()> {
        Ok(())
    }

    fn before_close(&mut self, _file: &File) -> io::Result<()> {
        Ok(())
    }

    fn before_publish(&mut self, _temporary: &Path, _target: &Path) -> io::Result<()> {
        Ok(())
    }

    fn before_remove_temporary(&mut self, _temporary: &Path) -> io::Result<()> {
        Ok(())
    }

    fn before_sync_directory(&mut self, _directory: &Path) -> io::Result<()> {
        Ok(())
    }
}

/// State reported by an atomic existing-file replacement.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AtomicFileReplaceResult {
    pub error: AtomicFileReplaceError,
    pub file_synced: bool,
    pub renamed: bool,
    pub directory_synced: bool,
}

/// State reported by atomic no-replace new-file publication.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AtomicFileCreateNewResult {
    pub error: AtomicFileCreateNewError,
    pub file_synced: bool,
    pub published: bool,
    pub directory_synced: bool,
}

pub use create_new::atomic_file_create_new;
pub use replace::atomic_file_replace;

pub(super) fn write_all(
    file: &mut File,
    bytes: &[u8],
    mut before_write: impl FnMut(&File, &[u8]) -> io::Result<usize>,
) -> io::Result<()> {
    let mut offset = 0;
    while offset < bytes.len() {
        let remaining = &bytes[offset..];
        match before_write(file, remaining).and_then(|amount| {
            if amount == 0 {
                return Ok(0);
            }
            if amount > remaining.len() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "atomic file write hook exceeded its input",
                ));
            }
            use std::io::Write;
            file.write(&remaining[..amount])
        }) {
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
            Ok(0) => {
                return Err(io::Error::new(
                    io::ErrorKind::WriteZero,
                    "atomic file write returned zero",
                ));
            }
            Ok(amount) if amount > bytes.len() - offset => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "atomic file write exceeded its input",
                ));
            }
            Ok(amount) => offset += amount,
        }
    }
    Ok(())
}

/// Perform the directory sync natively and consume the descriptor once.
/// Darwin's legacy contract ignores close errors after the sync result.
pub(super) fn sync_directory(directory: &Path) -> io::Result<()> {
    let file = File::open(directory)?;
    let sync = file.sync_all();
    let close = crate::sys::close_checked(file.into());
    match sync {
        Err(error) => Err(error),
        Ok(()) => {
            let _ = close;
            Ok(())
        }
    }
}
