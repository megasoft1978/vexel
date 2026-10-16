---
id: WO-RUST-UI-DRAW-01
revision: 1
status: READY
milestone: RUST-MIGRATION
owner: project-owner
executor: codex-gpt-6-luna
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 3f67be21ee959179715b74f5ea1e705f852a98e1ea50a9aab67dc05810dad489
parallel_safe: true
network: false
---
# Goal and frozen scope
Extend accepted UI core with all functions defined ui_draw.c, ui_widgets.c (only its five add wrappers), ui_icons.c, ui_icons_legacy.c, ui_icon_paths.c and their declared contracts. Owned icon definitions/caches matching asset numeric/path data and frozen hashes; bounded strict path parsing, arc/Bezier flattening, masks/backing scales/legacy painter, state colors/clipping/alpha blending, scalar ASCII fallback text (the declared legacy behavior), full tree paint/backgrounds/semantic styles/focus and add-widget wrappers. Validate RGBA buffer dimensions/stride/capacity before writes, preserve original glyph data and pixel/hash cases; port scale/theme/contrast/icon taxonomy/frozen C asset hash results without replacing or weakening oracle. Safe per-owner IconCache rather than unsafe globals if needed; document lifetime/cache adaptation. Preserve all8accepted core tests and original public behavior/limits; cohesive files<800. Pure pixel/unit checks only; no native GUI automation or goldens campaign. Native text shaping/AppKit presentation, interactive ui fields implementations outside named files and actual desktop UX remain later; do not substitute ASCII fallback for native product text rendering.

# Reference paths
src/ui/ui_draw.c/h, ui_widgets.c/h, ui_icons.c/h, ui_icons_legacy.c, ui_icon_paths.c/h, ui_core.h; scoped icon/draw/widget assertions in tests/ui (exclude native GUI/full galleries/sweep)
Legacy reference provenance: original dirty-source hashes in artifacts/WO-RUST-PLAN-01/source-baseline.json; no external algorithms/source fetch.

# Files allowed to touch
- `work/WO-RUST-UI-DRAW-01.md`
- `rust/crates/vexel-ui/src/**`, `rust/crates/vexel-ui/tests/**`
- `artifacts/WO-RUST-UI-DRAW-01/**`
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`, `docs/product/rust-port-goal-and-plan.md` (coordinator only)

# Protected paths and quality
All legacy source/fixtures/oracles, accepted other-crate code, all Cargo/config/lock/toolchain metadata, other worker trees, coordinator records/docs. Existing same-crate accepted behavior/assertions preserved. Worker only own src/tests/artifacts; coordinator alone shared integration. No manifest edit, code outside scope, dependency/download/install, unsupported OS code, Cshim, staging/commit/delete/reset, network/recursive agents, broad UI/E2E/C/performance campaign. Safe owned/lifetime-bound data, bounded inputs/checked arithmetic/fallible significant allocations, no unsafe in new modules; preserve accepted platform sys boundary. Source<800lines. No TODO/unimplemented/production stub for named scope. Map every named C API/helper to Rust symbols, focused tests, source/diagnostic spans, ownership/limits/cleanup and exact safe adaptations in parity-map.json. If dependency or contract cannot meet full scope stop with concrete issue; do not narrow deliverable to convenient subset.

# Execution and verification
Only .worktrees/rust3-ui-draw; Standard subscription LunaHigh, thin scope/context, one writer and focused independent review/repair. Read OWNER-DIRECTIVES/AGENTS and this order; consolidated goal governs full finish line. Preserve accepted underlying interfaces. Root serializes shared metadata/integration.
cargo +stable check -p vexel-ui --locked --offline
cargo +stable test -p vexel-ui --locked --offline
python3 scripts/wo_validate.py WO-RUST-UI-DRAW-01 --ready
python3 scripts/ledger.py validate ledger.jsonl
git diff --check
Exact existing1.99/macOS13, repo-local Cargo/cache/tmp/targets. No rustfmt/clippy installation or unsupported claims. Original fixtures/assertions unchanged; add focused Rust regressions with exact outputs, not only is_err. Run no extra broad suite.

# Output
Leave uncommitted; result<=400words, parity-map.json/verification.txt with actual checks, failures/repairs/adaptations and remaining app scope. Candidate only until independent review, exact scope/main audit and named integrated checks. Existing allowance only; no API billing/credits; stop/checkpoint on rate/allowance errors rather than retryloop.
