---
id: WO-RUST-UI-CORE-01
revision: 1
status: READY
milestone: RUST-MIGRATION
owner: project-owner
executor: codex-gpt-6-luna
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: abab3028bc21039ca777a39616c512e51d02cd3165ad93866f853a030a4d1c65
parallel_safe: true
network: false
---
# Goal and frozen scope
Port theme/accent/contrast tokens/type scale/design metrics, bounded tree init/add/layout, basicrectangle tree_render/focusrings, keyboardfocus/commands and owned semantics snapshots/text. Only functions actually defined ui_core.c; header functions defined ui_draw/widgets/icons are explicitly future orders, no stubs. Rust owns text/node data, preserves128nodes/24depth/64commands/1MiBtext caps and C layout/role/state defaults. Preserve failure atomicity and partialinvalidinput handling. Safe RGBA slices and RasterTile used for basic rendering; no nativeGUI harness. Focused pure unit tests translate existing assertions including longunicode labels, overflow/depth/duplicateids/commands/focus-disabled nodes, failure nonmutation. No desktopUI/visual quality claim.

# Reference paths
All functions defined src/ui/ui_core.c and relevant declarations ui_core.h; original core state/layout/focus/commands/semantic cases; accepted Rust Tile

# Files allowed to touch
- `work/WO-RUST-UI-CORE-01.md`
- `rust/crates/vexel-ui/src/**`, `rust/crates/vexel-ui/tests/**`
- `artifacts/WO-RUST-UI-CORE-01/**`
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`, `docs/product/rust-port-goal-and-plan.md` (coordinator only)

# Protected paths
All old C/ObjC/include/Python/shell, original tests/fixtures/oracles, other Rust crates, all Cargo manifests/lock/config/toolchain, coordinator docs/records. Preserve accepted same-crate behavior/assertions while extending it. No main edits, deps/downloads/system tools/formatters, unsupported OS code, C runtime shim, staging/commits/reset/clean, recursive agents/network, broad UI/E2E/C tests/corpus/perf campaign. Domain unsafe forbidden; only native-system order permits documented sys.rs boundary under ADR0024.

# Evidence and quality
Read OWNER-DIRECTIVES/AGENTS/ADR0022-0024/Rust goal then this order. Safe owned or lifetime-bound data, checked math and fallible bounded allocation. Map named Csymbols to Rustsymbols/focused tests/API adaptations in parity-map.json. No TODO/unimplemented/stub for namedscope; no whole-subsystem completion claim. Source<800lines, cohesive modules. If API changes outside scope needed stop with concrete dependency conflict, no manifest edit. Preserve original assertions and port meaningful cases, not merely is_err for diagnostic positions. Root reviews source and integrated checks before acceptance.

# Execution and verification
Only .worktrees/rust2-ui; Luna high, one implementation plus one narrowed review repair, thin context. Root serializes integration/shared metadata.
cargo +stable check -p vexel-ui --locked --offline
cargo +stable test -p vexel-ui --locked --offline
python3 scripts/wo_validate.py WO-RUST-UI-CORE-01 --ready
python3 scripts/ledger.py validate ledger.jsonl
git diff --check
Exact compiler1.99.0 via installed stable, repo-local CARGO_HOME/TMPDIR/target, MACOSX_DEPLOYMENT_TARGET13. No rustfmt/clippy installed; do not claim/install them. Workers only own src/tests/artifact; never root records.

# Output
Leave uncommitted. verification.txt/parity-map.json with exact behavior/checks/errors/adaptations/remaining scopes. Return<=450words. No additional billing/credit purchase; stop/checkpoint on rate/allowance errors instead of retryloop.
