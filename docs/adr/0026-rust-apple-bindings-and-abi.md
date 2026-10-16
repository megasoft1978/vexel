# ADR0026: pinned Apple bindings and contained ABI failures

Status: Accepted after independent Sol6.1Medium review; adoption/checks still required.
Date:2026-10-14. Work order:WO-RUST-NATIVE-BINDINGS-SETUP-01.

## Decision

Use the exact16package closure and default-disabled native feature selection in
`docs/dependencies/objc2-macos.lock.json`, derived from the accepted native audit.
Add Foundation NSException for genuine exception-boundary probes/diagnostics;
no package is added. Application/platform implementations remain first-party
Rust. Generated Apple bindings and the small upstream Objective-C exception
helper are reviewed third-party FFI dependencies, not a first-party backend shim.
The owned SVG/XML/raster and custom UI locks remain unchanged. No dispatch,
GNUstep, static-class/selector macros or unsupported-OS application work.

Keep AppKit-facing owners !Send/!Sync with an acquired main-thread capability.
Native pointers remain private to platform implementation. Callbacks enqueue
bounded owned data without model mutation/reentrancy or retained borrowed inputs.

The enforceable ABI contract is: no Rust panic or Objective-C exception escapes
a platform public operation or an inbound Objective-C/C/block callback. objc2
catch-all uses its pinned helper's explicit C-unwind trampoline and native catch
fence, then converts message exceptions to Rust panics. This audited dependency
mechanism is allowed inside the platform boundary; the prior proposal's absolute
zero-foreign-unwind-at-any-Rust-frame wording is superseded. Use Rust guards at
public platform operations and inside each inbound callback; latch owned bounded
faults and return ABI-valid fallback. No application-model/RefCell borrow may
span a native call that can reenter callbacks. Runtime source inspection, real
exception/panic probes and subsequent per-callback reviews remain necessary.

Raw C/CF APIs, retain/release/Drop and foreign exception cleanup are not all
fenced by catch-all. Rely only on their published validated ownership/argument
contracts; do not advertise recovery from corrupted objects or unsupported
cleanup exceptions. Panic=abort/allocator abort cannot recover; supported guards
use unwind. No manual objc_msgSend/transmute, fabricated main-thread marker,
foreign-unwind bypass or C shim is allowed.

## Provenance and licensing

Published registry checksums/manifests/selected APIs and exact VCS license
overviews are retained. Canonical MIT/Apache2/Zlib grants linked by those
overviews are retained, not invented copyright attribution. Select MIT where
the published package permits it; preserve all upstream notices and declared
authors with exact-version links in release attribution. Published build tools
include their own MIT/Apache notices. Apple SDK derivation and installed SDK27
provenance are recorded; this decision is no legal-clearance claim, no SDK/source
redistribution approval, and no acceptance of another agreement on the owner's
behalf. Release notice assembly and distribution review remain release gates.
Local use of the existing SDK is unchanged; no system packages are installed.

## Evidence required

Independent focused review before adoption. Exact-lock16package graph and
feature/host-target resolution, compiler1.99/mac13 deployment, offline build,
real Foundation exception handling, Rust panic/success, mainthread denial and
opaque/thread-confined API checks. Newer APIs require explicit availability
checks; current-host success alone cannot establish macOS13 runtime support.
Native shell/IME/text/media/GPU/menu/AX/teardown and final application acceptance
are separate complete work orders. Dependencies neither replace owned vector
logic nor prove performance improvements.

## Cargo lock records versus selected build units

R2 setup distinguishes16selected native build packages from18Cargo lock records.
Dispatch2=0.3.1 and objc2-io-surface=0.3.2 are inactive optional resolution records
and cached archives; selected normal/build tree and compiler outputs exclude
them. Official checksums, manifest license and versions are recorded; they are
not direct dependencies or native functionality. They may not be activated
without a reviewed decision. R1's overly narrow download scope and actual
observation remain preserved; no source/feature expansion was accepted.
Cargo's lock resolution and selected compilation are separate stages; see the
[Cargo resolver reference](https://doc.rust-lang.org/cargo/reference/resolver.html#features).
