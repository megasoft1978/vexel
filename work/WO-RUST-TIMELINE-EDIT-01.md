---
id: WO-RUST-TIMELINE-EDIT-01
revision: 1
status: READY
milestone: RUST-MIGRATION
owner: project-owner
executor: codex-gpt-6-luna
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: a6b224d6cd33352803a92610635211fbd850c52ff89bc24e7a87fcccba07c2c3
parallel_safe: true
network: false
---
# Goal and frozen scope
Port EVERY function in project_timeline_edit.c and its property/move/loop include files. Add safe TimelineEdit/limits/failure APIs using accepted TimelineContext/decode and bounded domain JSON; retain all14 decoder tests/public behavior. Copy original timeline source into owned immutable snapshots, source() borrows editor; cloneable Send+Sync source leases pin buffers without copying and survive history trimming/editor drop. Validate each candidate against supplied context before atomic commit; rejected operations preserve source/cursor/undo/redo. Full drawing-track/property-track(+initial key) creation, key set/insert, cel set/remove, item move with baseline/collision rules, loop/onion controls, undo/redo/history trimming and exact limits/errors/offsets. Preserve escaped IDs/names/labels/raw value semantics and unchanged source regions. Defaults16commands/128MiB, max1024commands/512MiB; significant allocation checked/fallible, document unavoidable std Arc allocation behavior without fake OOM injection. Use locale-independent formatting with equivalent values and preserved raw JSON. Port meaningful original edit fixture assertions including rejection/history/source lease lifetimes and references; no schema subset or DOM smaller cap. Session/render/context-scene wiring remains later, no mock renderer here.

# Reference paths
src/project/project_timeline_edit.c/h, project_timeline_property_edit.inc, project_timeline_move_edit.inc, project_timeline_loop_edit.inc, project_timeline_json.h; tests/project/project_timeline_edit_test.c; accepted rust/crates/vexel-timeline
Legacy provenance: original dirty-source hashes in artifacts/WO-RUST-PLAN-01/source-baseline.json. No external source fetch.

# Files allowed to touch
- `work/WO-RUST-TIMELINE-EDIT-01.md` (coordinator freeze only)
- `rust/crates/vexel-timeline/src/**`, `rust/crates/vexel-timeline/tests/**`
- `artifacts/WO-RUST-TIMELINE-EDIT-01/**`
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`, `docs/product/rust-port-goal-and-plan.md` (coordinator only)

# Protected paths and quality
All legacy sources/fixtures/oracles, accepted APIs/behavior/assertions, all other Rust crates, all Cargo/config/lock/toolchain metadata, other worktrees, earlier artifacts/orders and coordinator records. Worker only own src/tests/artifacts; coordinator alone shared interfaces/metadata/integration. No dependencies/download/install/network, unsafe/new sys boundary, C shim, unsupported OS code, staging/commit/delete/reset, recursive agents or broad UI/E2E/fuzz/performance campaign. Mac13+ AppleSilicon only, MPL2, no GPL-editor source or external SVG fetch. Explicit ownership/lifetimes, bounded input/checked arithmetic/fallible significant allocations. Source<800lines; split before exceeding. No TODO/unimplemented/mock production paths for named scope. If needed contract is absent, stop with concrete dependency rather than narrowing scope or editing peer crate. Every named C public API/helper mapped in parity-map.json to symbols/tests/limits/ownership/errors/exact safe adaptations; preserve meaningful assertions, not only is_err checks.

# Execution and verification
Only /Users/megasoft78/Desktop/Freelance/vexel/.worktrees/rust4-timeline-edit; Standard subscription Luna High, one writer. Read OWNER-DIRECTIVES/AGENTS/order; use accepted source contracts.
cargo +stable check -p vexel-timeline --locked --offline
cargo +stable test -p vexel-timeline --locked --offline
python3 scripts/wo_validate.py WO-RUST-TIMELINE-EDIT-01 --ready
python3 scripts/ledger.py validate ledger.jsonl
git diff --check
Exact existing Rust1.99/macOS13, repo-local Cargo/cache/tmp/targets, no fmt/clippy install or claim.

# Output
Uncommitted candidate, result<=400words, parity-map.json/verification.txt with exact checks/adaptations/failures/remaining downstream work. Independent review, preservation audit and named main checks required before acceptance. Existing subscription allowance only; no purchased credits/API billing. Stop/checkpoint on allowance/rate errors, no retry loop.
