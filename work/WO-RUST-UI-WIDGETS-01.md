---
id: WO-RUST-UI-WIDGETS-01
revision: 1
status: READY
milestone: RUST-MIGRATION
owner: project-owner
executor: codex-gpt-6-luna
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 77d0bba193228bf9e1b596ea74ecac11212a2845f5853258ca80b87c7969ab7e
parallel_safe: true
network: false
---
# Goal and frozen scope
Port every function defined in ui_widget_input.c, ui_widget_canvas.c, ui_widget_paint.c and ui_widget_chrome_paint.c, with safe owned pointer/text/number/color/slider/dropdown states and bounded borrowed canvas/draw specs. Retain accepted12 UI tests/API/icons/tree painting unchanged. Full pointer state transitions, UTF8 text selection/cursor editing/commit/cancel, number parse/format/draft/scrub/key rules, RGBA hex parse/format, slider and dropdown key/pointer behavior, all named widget paint/chrome paths and transitions. Preserve 127edit/63label/8unit/16option/8segment bounds, font/style/scale/theme/contrast, buffer stride/capacity/clipping validation and pixel behavior; safe borrowed native text drawing closure replaces C callback and scalar fallback only when callback absent. Define exact existing key/modifier constants/types locally if not yet exported; no upstream crate changes. Split cohesive modules<800lines. Port meaningful input/paint assertions including UTF8 invalid/draft cases, selection/replacement, stepping/modifiers, focus/pointer, no buffer overrun and original semantic colors. Pure focused pixel/unit checks, no GUI/gallery/golden campaign. Native CoreText/AppKit and editor event wiring later.

# Reference paths
src/ui/ui_widget_input.c, ui_widget_canvas.c, ui_widget_paint.c, ui_widget_chrome_paint.c, ui_widget_paint_internal.h, ui_widgets.h, ui_core.h; focused tests/ui widget cases (exclude whole GUI/gallery/fuzz campaign)
Legacy provenance: original dirty-source hashes in artifacts/WO-RUST-PLAN-01/source-baseline.json. No external source fetch.

# Files allowed to touch
- `work/WO-RUST-UI-WIDGETS-01.md` (coordinator freeze only)
- `rust/crates/vexel-ui/src/**`, `rust/crates/vexel-ui/tests/**`
- `artifacts/WO-RUST-UI-WIDGETS-01/**`
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`, `docs/product/rust-port-goal-and-plan.md` (coordinator only)

# Protected paths and quality
All legacy sources/fixtures/oracles, accepted APIs/behavior/assertions, all other Rust crates, all Cargo/config/lock/toolchain metadata, other worktrees, earlier artifacts/orders and coordinator records. Worker only own src/tests/artifacts; coordinator alone shared interfaces/metadata/integration. No dependencies/download/install/network, unsafe/new sys boundary, C shim, unsupported OS code, staging/commit/delete/reset, recursive agents or broad UI/E2E/fuzz/performance campaign. Mac13+ AppleSilicon only, MPL2, no GPL-editor source or external SVG fetch. Explicit ownership/lifetimes, bounded input/checked arithmetic/fallible significant allocations. Source<800lines; split before exceeding. No TODO/unimplemented/mock production paths for named scope. If needed contract is absent, stop with concrete dependency rather than narrowing scope or editing peer crate. Every named C public API/helper mapped in parity-map.json to symbols/tests/limits/ownership/errors/exact safe adaptations; preserve meaningful assertions, not only is_err checks.

# Execution and verification
Only /Users/megasoft78/Desktop/Freelance/vexel/.worktrees/rust4-widgets; Standard subscription Luna High, one writer. Read OWNER-DIRECTIVES/AGENTS/order; use accepted source contracts.
cargo +stable check -p vexel-ui --locked --offline
cargo +stable test -p vexel-ui --locked --offline
python3 scripts/wo_validate.py WO-RUST-UI-WIDGETS-01 --ready
python3 scripts/ledger.py validate ledger.jsonl
git diff --check
Exact existing Rust1.99/macOS13, repo-local Cargo/cache/tmp/targets, no fmt/clippy install or claim.

# Output
Uncommitted candidate, result<=400words, parity-map.json/verification.txt with exact checks/adaptations/failures/remaining downstream work. Independent review, preservation audit and named main checks required before acceptance. Existing subscription allowance only; no purchased credits/API billing. Stop/checkpoint on allowance/rate errors, no retry loop.
