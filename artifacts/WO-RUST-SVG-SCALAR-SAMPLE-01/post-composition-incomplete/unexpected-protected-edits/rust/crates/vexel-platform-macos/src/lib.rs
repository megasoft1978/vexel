#![deny(unsafe_code)]
#![deny(unsafe_op_in_unsafe_fn)]
//! Small, synchronous wrappers for macOS system identity and entropy services.
//!
//! This crate is scoped to the Apple Silicon macOS 13+ target selected by the
//! workspace. It does not provide implementations for other operating systems.

use std::fmt;
use std::os::fd::BorrowedFd;

#[allow(unsafe_code)]
mod sys;

mod atomic_files;

pub use atomic_files::{
    AtomicFileCreateNewError, AtomicFileCreateNewHooks, AtomicFileCreateNewResult,
    AtomicFileReplaceError, AtomicFileReplaceHooks, AtomicFileReplaceResult,
    MAX_ATOMIC_FILE_PATH_BYTES, atomic_file_create_new, atomic_file_replace,
};

/// Largest request accepted by [`fill_random`].
pub const MAX_RANDOM_BYTES: usize = 256;

/// Failure reported by a native system service.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SystemError {
    /// The requested random buffer exceeds the wrapper's fixed bound.
    RandomRequestTooLarge { requested: usize, maximum: usize },
    /// `SecRandomCopyBytes` returned a nonzero `OSStatus`.
    RandomStatus(i32),
    /// `getpeereid` failed; this is the `errno` value captured immediately.
    PeerCredentialsErrno(i32),
}

impl fmt::Display for SystemError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RandomRequestTooLarge { requested, maximum } => {
                write!(f, "random request of {requested} bytes exceeds {maximum}")
            }
            Self::RandomStatus(status) => {
                write!(f, "SecRandomCopyBytes failed with OSStatus {status}")
            }
            Self::PeerCredentialsErrno(errno) => {
                write!(f, "getpeereid failed with errno {errno}")
            }
        }
    }
}

impl std::error::Error for SystemError {}

/// Fill `output` with bytes from Apple's default cryptographic random source.
///
/// Empty output succeeds without calling the native function. Requests above
/// [`MAX_RANDOM_BYTES`] are rejected before any native call. If the native
/// call fails, every byte in `output` is cleared before its status is returned.
pub fn fill_random(output: &mut [u8]) -> Result<(), SystemError> {
    if output.len() > MAX_RANDOM_BYTES {
        return Err(SystemError::RandomRequestTooLarge {
            requested: output.len(),
            maximum: MAX_RANDOM_BYTES,
        });
    }
    if output.is_empty() {
        return Ok(());
    }

    let status = sys::fill_random(output);
    random_result(status, output)
}

fn random_result(status: i32, output: &mut [u8]) -> Result<(), SystemError> {
    if status == 0 {
        Ok(())
    } else {
        output.fill(0);
        Err(SystemError::RandomStatus(status))
    }
}

/// Return the real user ID of the calling process, as reported by `getuid`.
pub fn current_uid() -> u32 {
    sys::current_uid()
}

/// Return the user and group IDs associated with a connected peer socket.
///
/// The descriptor is borrowed only for the duration of this call. Credentials
/// are returned as Darwin's 32-bit `uid_t` and `gid_t` values.
pub fn peer_credentials(socket: BorrowedFd<'_>) -> Result<(u32, u32), SystemError> {
    sys::peer_credentials(socket).map_err(SystemError::PeerCredentialsErrno)
}

#[cfg(test)]
mod tests {
    use super::{SystemError, random_result};

    #[test]
    fn failed_random_status_clears_the_entire_output() {
        let mut output = [0xA5; 8];

        assert_eq!(
            random_result(-42, &mut output),
            Err(SystemError::RandomStatus(-42))
        );
        assert_eq!(output, [0; 8]);
    }

    #[test]
    fn successful_random_status_preserves_native_output() {
        let mut output = [0x3C; 4];

        assert_eq!(random_result(0, &mut output), Ok(()));
        assert_eq!(output, [0x3C; 4]);
    }
}
