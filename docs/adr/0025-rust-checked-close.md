# ADR0025: consuming Darwin checked close for atomic saves

Status: accepted2026-10-14. Scope: Apple Silicon macOS13+, platform boundary only.
This resolves WO-RUST-ATOMIC-FILES-01 r1 std-only blocker; it does not approve a
new dependency, Cshim, OS target or weakened publication/durability behavior.

Rust File Drop ignores close errors. Owned C atomic operations check close before
rename/link commit. Preserve that observable error contract with one tiny private
Darwin binding, alongside existing Security/System bindings in sys.rs:
close(int)->int, SDK unistd.h declaration line449 inspected locally.

Safe public signature: close_checked(OwnedFd)->std::io::Result<()>. Convert File
with its safe owned conversion. sys consumes the descriptor using IntoRawFd,
performs exactly one synchronous close, checks result, captures errno immediately
on failure and returns an owned IO error. No public raw pointer/descriptor API,
no double drop, blind close retry or resurrection of consumed ownership. This
matches the existing C close_checked policy, which clears its descriptor before
calling close. No claim of platform-independent EINTR semantics is introduced.

All unsafe stays in tiny private sys.rs with individual SAFETY ABI/ownership/
no-retention comments; crate deny unsafe and unsafe_op_in_unsafe_fn remain. Keep
accepted entropy/getuid/getpeereid bodies/tests intact. Atomic file operations
and typed safe hooks stay outside the unsafe module. Close errors before commit
must prevent rename/publication; postcommit directory durability errors report
published state accurately. Tests cover actual consuming native success and
safe error classification/precommit close failure seam without forged invalid
OwnedFd, secrets or synthetic native failure claims. Cleanup closes temporary
handles exactly once and never publishes after a precommit failure.

Primary references checked2026-10-14 (facts only, no source copied):
- [Rust File close/Drop behavior](https://doc.rust-lang.org/std/fs/struct.File.html)
- [Rust OwnedFd ownership/conversions](https://doc.rust-lang.org/std/os/fd/struct.OwnedFd.html)
- [Apple close return/error contract](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/close.2.html)
- Installed Apple SDK usr/include/unistd.h close(int) declaration.
