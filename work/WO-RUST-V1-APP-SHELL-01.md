---
id: WO-RUST-V1-APP-SHELL-01
revision: 1
status: READY
milestone: V1-APP-SHELL
owner: project-owner
executor: gpt-6-luna
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: bede89e52f3803b69396ffb64fe161850bc0ff6a8ae4177db275c8b64031b6e7
parallel_safe: false
network: false
---
# Launchable Rust V1 workbench shell

Create the first launchable Rust Vexel application and connect the custom UI to
the accepted macOS window/frame boundary. This is the app-shell phase of the V1
vertical slice. It must show the designed drawing workbench and respond to
window lifecycle, resize, and Draw/Animate mode changes. It does not claim a
complete drawing, project, animation, SVG, or MCP workflow.

## Required behavior
1. Add a `vexel-app` macOS executable crate with no new third-party dependency.
   The coordinator owns the workspace member and lockfile integration. The
   application crate itself contains safe Rust only and depends on existing
   `vexel-ui`, `vexel-raster`, and `vexel-platform-apple` crates.
2. Acquire the platform's verified main-thread capability, start `NativeApp`,
   create and show one resizable Vexel window, poll native events, and perform
   orderly window/app shutdown. Handle open, close, resize, backing-scale and
   appearance changes. Invalid or oversized pixel geometry must fail visibly
   and safely, without overflow, panic, or an unbounded render queue. Redraw
   only when the model or window geometry is dirty; the idle loop must not spin
   at full CPU.
3. Render a responsive workbench using the existing custom `vexel-ui` painter
   and design tokens: top bar with Draw/Animate mode control and undo/redo
   status, slim Select/Pen/Rectangle/Ellipse rail, central canvas well, right
   inspector, bottom status, and visible MCP activity area. Animate mode opens
   a timeline dock. Keep the stage dominant and labels readable at the standard
   1280x820 point window and narrower valid sizes. Do not imitate Blender chrome.
4. Draw/Animate switching works from pointer and keyboard and visibly updates
   the selected mode and timeline layout. Controls for drawing, save/export or
   MCP that are not implemented in this order must be visibly unavailable and
   must not report successful document actions. Empty-state copy must tell the
   user how to proceed without implying a document was created or saved.
5. Supply real macOS system and monospaced text drawing for `vexel-ui` through a
   bounded, safe API owned by `vexel-platform-apple` (CoreText details and native
   handles remain inside that crate). Support the UI's regular/semibold text,
   clipping, backing scale, checked pixel bounds, and RGBA alpha blending. Do
   not use the legacy 5x7 scalar painter for product labels. Any unsafe CoreText
   or CoreGraphics calls must stay in the Apple crate with explicit safety
   arguments and native exception guards where applicable.
6. Convert the final UI buffer to the accepted `RgbaFrame` and present it with
   `Window::present_rgba`. Respect the 4096-axis / 64 MiB limits and supported
   integral backing scales. Release superseded buffers without retaining an
   unbounded frame history.
7. Keep each source/test file below 800 lines. Add focused unit tests for
   deterministic layout breakpoints, mode switching, hit testing and rejected
   geometry. Describe this as a launchable shell, not V1 completion.

## Allowed paths
- `Cargo.toml` and `Cargo.lock` **coordinator only** to add the app member and
  lock the already-existing local dependency graph
- `rust/crates/vexel-app/**` (new app crate, source, focused tests and README)
- `rust/crates/vexel-platform-apple/src/native_window/**`
- `rust/crates/vexel-platform-apple/src/lib.rs`
- `rust/crates/vexel-platform-apple/README.md`
- `artifacts/WO-RUST-V1-APP-SHELL-01/**` except frozen scope/hash reports
- `.cache/**` for isolated worktrees/build outputs only

## Protected paths
All other workspace crates, legacy C/Objective-C, frozen fixtures/assertions,
V1/full-product plans, work orders, `STATE.md`, ledgers, HEAD and index. No new
external dependency, external SVG fetching, GPL source, non-macOS code,
unsupported OS stubs, broad UI/E2E suite, app publication, commit, reset,
delete, or nested workers. The coordinator alone changes the shared workspace
manifest/lockfile and records integration after review. One writer owns the app
crate and Apple platform crate during implementation.

## Acceptance and verification
- Focused app tests prove responsive layout at 1280x820, compact bounds, Draw /
  Animate selection, and deterministic pointer hit testing.
- Focused Apple tests cover text-input limits, checked dimensions, clipping and
  unsupported geometry; package tests do not claim live font/window rendering.
- `cargo +stable check -p vexel-app --locked --offline`
- `cargo +stable test -p vexel-app --locked --offline`
- `cargo +stable check -p vexel-platform-apple --locked --offline`
- `cargo +stable test -p vexel-platform-apple --locked --offline`
- `python3 scripts/wo_validate.py WO-RUST-V1-APP-SHELL-01 --ready`
- Record exact output, file hashes, scope audit, native-runtime limitations and
  a source review. Integrate only after independent review, protected-scope
  audit and fresh main-tree focused checks.
- Do not run the comprehensive UI/E2E campaign or claim visual acceptance here;
  that remains the final integrated V1/full-product gate.

Use Luna Medium for bounded implementation and existing subscription allowance.
Use Sol Medium only for the independent native-boundary final review. Standard
speed; no purchased credits or API billing.
