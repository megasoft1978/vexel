---
id: WO-RUST-WAVE4-SETUP-01
revision: 1
status: READY
milestone: RUST-MIGRATION
owner: project-owner
executor: codex-coordinator
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 3aee9749bb3c3355e243e418962cb25d0de9b48c07930762dc32b22bdbc0909a
parallel_safe: false
network: false
---
# Goal
Register/freeze six dependency-ready Rust ports against accepted timeline/UI/native/project contracts. Four std-only crate shells, no implementation or product parity claim. XML parser candidate remains independent.

# Files allowed to touch
- `work/WO-RUST-WAVE4-SETUP-01.md`
- `work/WO-RUST-TIMELINE-EDIT-01.md`
- `work/WO-RUST-UI-WIDGETS-01.md`
- `work/WO-RUST-SVG-COLOR-01.md`
- `work/WO-RUST-MCP-TASK-STORE-01.md`
- `work/WO-RUST-MCP-ACCESS-01.md`
- `work/WO-RUST-PROJECT-IO-01.md`
- `Cargo.toml`
- `Cargo.lock`
- `rust/crates/vexel-svg-color/Cargo.toml`
- `rust/crates/vexel-svg-color/src/lib.rs`
- `rust/crates/vexel-mcp-tasks/Cargo.toml`
- `rust/crates/vexel-mcp-tasks/src/lib.rs`
- `rust/crates/vexel-mcp-access/Cargo.toml`
- `rust/crates/vexel-mcp-access/src/lib.rs`
- `rust/crates/vexel-project-io/Cargo.toml`
- `rust/crates/vexel-project-io/src/lib.rs`
- `docs/product/rust-port-goal-and-plan.md`
- `STATE.md`
- `LEDGER.md`
- `ledger.jsonl`
- `artifacts/WO-RUST-WAVE4-SETUP-01/**`
- `artifacts/WO-RUST-PROTOCOL-01/integrate_candidate.py`

# Protected paths and contracts
All accepted Rust source/manifests except exact new registrations, legacy603sources/fixtures/oracles, active XML worker/candidate, earlier frozen orders and evidence. No new dependency/unsafe/download/install/network/unsupported OS/stage/commit/delete/reset. Domain shells forbidunsafe. Each worker owns one distinct crate; no dependency on unfinished peers. Timeline-edit extends timeline and widgets extends UI without manifests. svg-color(base), mcp-tasks(base,platform-macos), mcp-access(base,platform-macos), project-io(base,project,platform-macos). Coordinator only shared integration. Serialized registration audit/update accepts exact approved manifest hashes for older XML integration helper; candidate cannot overwrite metadata. Six Standard subscription Luna CLI workers: colors Medium, remaining High. Independent focused setup review before launch.

# Verification
cargo +stable generate-lockfile --offline
cargo +stable check --workspace --locked --offline
python3 scripts/wo_validate.py WO-RUST-WAVE4-SETUP-01 --ready
python3 scripts/ledger.py validate ledger.jsonl
git diff --check
Validate all6new frozen orders and exact original603/index/accepted Rust source hashes. Existing1.99/local caches/macOS13. No application/UI tests for setup.

# Output
Accepted registrations only, ready scopes, current-source overlays and actual worker registry; full final product goal unchanged.
