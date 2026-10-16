---
id: WO-RUST-NATIVE-BOUNDARY-01
revision: 1
status: READY
milestone: RUST-MIGRATION
owner: project-owner
executor: codex-gpt-6-luna
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: f86de04534fab0d39f6fa9f390d0860149f6b1c859b9960be9162de0a692d651
parallel_safe: true
network: false
---
# Goal and complete new API
Implement all shared native-boundary behavior for `vexel-platform-apple`, backed
by accepted exact Apple bindings. This is actual mainthread/exception handling,
not an application/window stub. Expose safe opaque `MainThread` acquired only via
objc2 MainThreadMarker::new (Apple pthread_main_np), !Send/!Sync by construction.
Provide crate-private marker access for later platform modules, no public native
pointer or new_unchecked. A named guarded native-call helper catches Rust unwinds
and objc2 catch-all converted message exceptions, returning a small owned typed
fault with no retained arbitrary panic/native payload or unbounded formatting.
Preserve success return value exactly; drop fault payload safely and document
nonrecoverable foreign cleanup/retained-object/allocator/abort cases. Do not claim
raw-C/native-release exceptions are fenced by catch_unwind. No application/model
borrows or reentrant callback mutation inside native boundary design. Every
future inbound ObjC/C/block callback must fence before ABI return, return
ABI-valid fallback and latch bounded failure; this module supplies the reusable
Rust guard, not fictitious callbacks. Thirdparty exception fence is ADR0026's
explicit audited C-unwind exception. All new unsafe, if any, confined to private
platform implementation or exact native regressionprobe, with safety contracts.

# Required focused evidence
Real Foundation-only native exception regression using NSString/NSException
raise routed through objc2 catch-all and guard; no fake exception/mockbackend.
Catch ordinary Rust panic and retain exact success result; demonstrate thread
capability acquisition denial on spawned thread, no cross-threadmarker/pointer
exposure (compilefail docs if appropriate). No NSApplication/window startup or
UIautomation. No tests skipped to dodge native exception/linking. Report complete
limits/thread/lifetime/ABI/fatal-case adaptation and symbols in parity-map.
Split source/tests<800, current macros primary published semantics; no unsafe
transmute/manual objc_msgSend/rawclasspointer shortcut or firstparty C shim.

# Files allowed to touch
- `work/WO-RUST-NATIVE-BOUNDARY-01.md` (coordinator freeze only)
- `rust/crates/vexel-platform-apple/src/**`, `rust/crates/vexel-platform-apple/tests/**`
- `artifacts/WO-RUST-NATIVE-BOUNDARY-01/**`
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`, `docs/product/rust-port-goal-and-plan.md` (coordinator only)

# Protected and execution
All peer/legacy/fixtures/manifests/locks/config/docs/orders/earlier evidence/index.
One isolated LunaHigh writer in .worktrees/rust-native-boundary, own src/tests/
artifacts only. Accepted bindings/provenance/ABI decisions consumed unchanged.
No network/fetch/install/dependencies/unsupportedOS/recursiveagents/stage/commit/
reset/delete or broadUI/E2E. Exact installedRust1.99+stable/mac13/localcache,
Standard subscription/no extra credits/APIbilling; stop allowanceerrors.

# Verification
cargo +stable check -p vexel-platform-apple --locked --offline
cargo +stable test -p vexel-platform-apple --locked --offline
python3 scripts/wo_validate.py WO-RUST-NATIVE-BOUNDARY-01 --ready
python3 scripts/ledger.py validate ledger.jsonl
git diff --check

# Output
Complete uncommitted native boundary candidate, concise result<=400words,
parity-map/verification exact real outcomes. IndependentHighreview, exact protected
source/root audit and fresh namedmainchecks before acceptance. No full AppKit/
macOS13runtime/desktop/renderer/product claim. Missing true dependency must be
reported, not replaced by stub/mocks/skips or smaller scope.
