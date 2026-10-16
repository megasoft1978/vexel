---
id: WO-RUST-JSON-01
revision: 1
status: READY
milestone: RUST-MIGRATION
owner: project-owner
executor: codex-gpt-6-luna
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 7d96668102fa6ed8e11d28e8752a4a6ffeaba6c91f995f3de86a63176ea7455b
parallel_safe: true
network: false
---
# Goal
Port the owned bounded JSON tokenizer/document/accessor/decoded strings/raw tokens/uint parsing and bounded serializer/buffer. Preserve original max bytes/tokens/depth, array/object order, duplicate-key lookup semantics, Unicode/escape/surrogate/embeddedNUL behavior, number grammar and raw ID spans. JsonDocument borrows immutable bytes or owns storage with numeric spans; no C parser/backend or serde dependency. Fallible size-limited buffers and append rollback; real valid/malformed/depth/token/size/overflow/escaping cases. This is JSON core, not full protocol Tasks/bridge parity.

# Reference scope
src/mcp/mcp_json.c, src/mcp/mcp_json.h, parser/buffer cases in tests/mcp/mcp_protocol_test.c
Read OWNER-DIRECTIVES/AGENTS, ADR0022/0023, Rust goal/plan and this order first.

# Files allowed to touch
- `work/WO-RUST-JSON-01.md`
- `rust/crates/vexel-json/src/**`, `rust/crates/vexel-json/tests/**`
- `artifacts/WO-RUST-JSON-01/**`
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`, `docs/product/rust-port-goal-and-plan.md` (coordinator only)

# Protected paths
All legacy C/ObjC/include/Python/shell source, original tests/fixtures/oracles, other Rust crates, Cargo manifests/lock/config/toolchain, docs/records. No main edits, dependencies, unsafe, global tool installs, network, broad app/UI/E2E/C tests, staging/commit/reset/clean, recursive agents. Preserve all dirty baseline files.

# Contracts and evidence
Safe idiomatic Rust2024, bounded input and checked arithmetic/fallible allocation, C public behavior accounted in parity-map.json (C symbol -> Rust symbol -> named focused tests or explicit documented API adaptation). Do not hide unimplemented named behavior behind TODO/unimplemented/stub. Preserve original assertions; port their cases to Rust, do not run old C suite. Source files<800lines, cohesive modules. Root Span/BaseStatus and geometry types remain stable; no shared-interface edits outside your crate. Build standalone package without modifying any workspace manifest. No full subsystem/product/performance claim beyond named scope.

# Execution and verification
Only .worktrees/rust-json; Luna high, one implementation plus one narrowed review repair. Root serializes source/manifest/ledger integration.
cargo +stable check -p vexel-json --locked --offline
cargo +stable test -p vexel-json --locked --offline
python3 scripts/wo_validate.py WO-RUST-JSON-01 --ready
python3 scripts/ledger.py validate ledger.jsonl
git diff --check
Set CARGO_HOME/TMPDIR/CARGO_TARGET_DIR to this worktree .cache/cargo, .cache/tmp and build/rust, MACOSX_DEPLOYMENT_TARGET13. Verify rustc +stable version1.99.0; no downloads. Rustfmt/clippy are not installed; do not install or claim those run. Independent review, exact baseline/allowed path audit, frozen original tests and main integrated named checks required.

# Output
Leave uncommitted; verification.txt + parity-map.json under own artifact with exact checks/public API/adaptations/remaining scope. Final response<=450words. Do not modify coordinator state or claim acceptance yourself.
