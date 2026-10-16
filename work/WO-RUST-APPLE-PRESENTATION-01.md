---
id: WO-RUST-APPLE-PRESENTATION-01
revision: 1
status: READY
milestone: V1-APP-SHELL
owner: project-owner
executor: gpt-6-luna
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: a79ef96ec1bad78ed76033428c80e1414913ccf3abe337e7c885e1613babd5f0
parallel_safe: true
network: false
---
# Safe macOS frame presentation boundary
Add the missing platform-owned pixel presentation API required by the V1 Rust
app shell. `vexel-platform-apple::Window` accepts an owned/bounded frame from the
Rust UI/raster code and presents it through its existing NSView/Core Animation
surface. Keep all Objective-C and CoreGraphics operations inside
`vexel-platform-apple`; application/domain crates remain safe Rust and handle
only Vexel-owned pixels and platform event enums.

## Required behavior
1. Define a public safe `RgbaFrame` (or equivalent) with checked dimensions,
   backing scale and owned RGBA8 bytes. Use top-left, tightly packed row-major
   pixels and document the alpha/color-space interpretation. Enforce checked
   row/byte arithmetic, existing axis/capacity limits and valid finite scale.
2. Add `Window::present_rgba(frame)` or equivalent. Reject invalid frames
   deterministically without changing the currently displayed frame. No borrowed
   slice or raw pointer may outlive the synchronous call.
3. Convert the owned pixels to an image using the already pinned Apple
   frameworks. Build and validate the replacement before publishing it. Keep
   the last valid frame visible if allocation, image construction, native call,
   or invalidation fails. The current layer/window owns the published image for
   the full native display lifetime and releases replaced contents safely.
4. Set layer content scale and sizing behavior from the frame dimensions/scale.
   A later rerender submitted after `Resize` or `BackingScaleChanged` must
   replace the prior frame without stale native references or unbounded queues.
5. Public app-facing APIs expose no Objective-C pointers, native object handles,
   or unsafe methods. Every unsafe call is in this crate, documented with the
   concrete ABI/type/lifetime argument, and covered by existing native-call
   exception guards where applicable. No new dependency or platform support.
6. Document this as a frame presentation seam only. Do not claim document,
   editor, SVG renderer, or V1 workflow integration in this order.

## Allowed paths
- `rust/crates/vexel-platform-apple/src/native_window/**`
- `rust/crates/vexel-platform-apple/src/lib.rs`
- `rust/crates/vexel-platform-apple/README.md`
- `rust/crates/vexel-platform-apple/Cargo.toml` only for already pinned,
  already locked existing Apple-framework features if strictly required
- `rust/crates/vexel-platform-apple/tests/**` (new focused pure validation tests)
- `artifacts/WO-RUST-APPLE-PRESENTATION-01/**` except frozen scope/hash reports
- `.cache/**` for isolated build outputs only

## Protected paths
All other workspace crates, shared workspace manifest and lockfile, legacy C
and Objective-C source, frozen fixtures/assertions, application UI/product docs,
work-order files, `STATE.md`, ledgers, HEAD and index. No unsafe code outside the
Apple boundary, new dependency, external SVG fetching, GPL source, unsupported
OS work, app GUI/E2E suite, shared integration, staging, commit, reset, deletion,
or recursive worker spawning. Keep each changed source and test file below 800
lines. One writer owns this Apple platform crate during the order.

## Acceptance and verification
- Pure frame validation tests cover zero/oversized dimensions, checked overflow,
  byte-count mismatch, invalid scale and accepted boundary sizes.
- Focused native presentation tests or an explicit limitation record establish
  atomic replacement and retained image lifetime at the platform API boundary;
  do not claim live app rendering from package unit tests.
- `cargo +stable check -p vexel-platform-apple --locked --offline`
- `cargo +stable test -p vexel-platform-apple --locked --offline`
- `python3 scripts/wo_validate.py WO-RUST-APPLE-PRESENTATION-01 --ready`
- Record exact command results, API ownership/lifetime coverage, affected-file
  hashes and any missing native runtime proof. Do not run broad UI/E2E.

Use Luna High for this narrow unsafe AppKit/CoreGraphics boundary. Use the
existing subscription allowance and Standard speed only; no credits or API
billing. Candidate code needs independent review, protected-scope audit and
fresh main-tree focused checks before acceptance or use by the next app-shell
work order.
