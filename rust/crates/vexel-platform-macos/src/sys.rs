//! Narrow ABI boundary for macOS system services.
//!
//! The declarations mirror the local macOS SDK: `SecRandomRef` is an opaque
//! pointer, Darwin `uid_t`/`gid_t` are `u32`, and `getpeereid` accepts `int`.

use std::ffi::{c_int, c_void};
use std::io;
use std::os::fd::{AsRawFd, BorrowedFd, IntoRawFd, OwnedFd};

#[link(name = "Security", kind = "framework")]
unsafe extern "C" {
    fn SecRandomCopyBytes(random: *const c_void, count: usize, bytes: *mut c_void) -> c_int;
}

unsafe extern "C" {
    fn getuid() -> u32;
    fn getpeereid(socket: c_int, uid: *mut u32, gid: *mut u32) -> c_int;
    fn __error() -> *mut c_int;
    fn close(descriptor: c_int) -> c_int;
}

/// Call Security synchronously with a bounded, exclusively borrowed output.
pub(super) fn fill_random(output: &mut [u8]) -> i32 {
    debug_assert!(!output.is_empty());
    debug_assert!(output.len() <= super::MAX_RANDOM_BYTES);

    // SAFETY: NULL is Apple's documented default SecRandomRef. The slice
    // pointer is writable for exactly `len()` bytes, `len()` is bounded to 256,
    // and the synchronous API does not retain the pointer, dispatch callbacks,
    // or access it on another thread.
    unsafe {
        SecRandomCopyBytes(
            std::ptr::null(),
            output.len(),
            output.as_mut_ptr().cast::<c_void>(),
        )
    }
}

/// Read the process real user ID using Darwin's `getuid` ABI.
pub(super) fn current_uid() -> u32 {
    // SAFETY: `getuid` takes no pointers and has no preconditions.
    unsafe { getuid() }
}

/// Query the credentials tied to the borrowed descriptor and capture errno.
pub(super) fn peer_credentials(socket: BorrowedFd<'_>) -> Result<(u32, u32), i32> {
    let mut uid = 0_u32;
    let mut gid = 0_u32;
    // SAFETY: `socket` keeps a live descriptor borrowed for this call. The
    // output pointers reference initialized local `u32`s matching Darwin's
    // uid_t/gid_t ABI. getpeereid is synchronous, uses the calling thread, and
    // retains neither pointers nor descriptor after returning.
    let result = unsafe { getpeereid(socket.as_raw_fd(), &mut uid, &mut gid) };
    if result == 0 {
        return Ok((uid, gid));
    }

    // SAFETY: Darwin's `__error` returns this thread's writable errno slot,
    // which is valid to read immediately after a failed system call.
    let errno = unsafe { *__error() };
    Err(errno)
}

/// Consume an owned descriptor with Darwin's checked, single-call close.
pub(super) fn close_checked(descriptor: OwnedFd) -> io::Result<()> {
    let raw = descriptor.into_raw_fd();
    // SAFETY: `into_raw_fd` transfers this live descriptor to this function.
    // Darwin close(int) consumes it exactly once, synchronously, and retains no
    // pointer or descriptor. Ownership is never reconstructed or retried.
    let result = unsafe { close(raw) };
    if result == 0 {
        return Ok(());
    }

    // SAFETY: Darwin __error returns this thread's errno slot. It is read
    // immediately after the failed close call, before any other native call.
    let errno = unsafe { *__error() };
    Err(io::Error::from_raw_os_error(errno))
}
