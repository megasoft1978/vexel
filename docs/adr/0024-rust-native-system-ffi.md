# ADR0024: Rust native system services boundary

Accepted2026-10-14 for macOS13 AppleSilicon native services only. No external
crate or first-party C shim. Rust safe wrappers encapsulate tiny audited unsafe
FFI module, crate deny unsafe plus allow only sys and deny unsafe_op_in_unsafe_fn.
ABI types/prototypes checked against local SDK headers and primary docs; foreign
functions borrow buffers/fds only synchronously. No unsupported OS implementation.

Services: Apple Security SecRandomCopyBytes using documented NULL default
provider, bounded<=256bytes, check returned status and clear output on failure;
getuid and getpeereid through System with u32 uid/gid, BorrowedFd and immediate
errno capture. Test actual OS success and pure failure classification without
faking native security certification. Other platform/window/file/graphics/AX
services remain future scopes; AppKit dependency choices not adopted here.

Primary sources fetched2026-10-14:
- https://doc.rust-lang.org/nomicon/ffi.html
- https://developer.apple.com/documentation/security/secrandomcopybytes(_:_:_:)
- https://developer.apple.com/documentation/security/ksecrandomdefault
- https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man3/getpeereid.3.html
Apple Markdown establishes availabilitymacOS10.7 and defaultconstant=NULL;
getentropy localman recommends Security for clients needing random bytes.
API facts only, no external algorithm/editor source copied.
