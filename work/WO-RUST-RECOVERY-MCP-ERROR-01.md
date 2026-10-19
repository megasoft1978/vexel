---
id: WO-RUST-RECOVERY-MCP-ERROR-01
revision: 1
status: READY
milestone: FULL-RUST-PROJECT
owner: project-coordinator
executor: codex-gpt-6-luna
base_sha: 76898badf27d3b1ee774786b1214c63a42b18a7c
spec_sha256: 7f34b2ca0cc75faa85f4f4d32c3d1f7bb35bf2e783a197b4a409271a48a53b33
parallel_safe: false
network: false
---
# Goal
Keep the MCP file-operation error mapping exhaustive and actionable after adding typed recovery session failures.

# Context
The active recovery session order adds `SessionError::RecoveryConflict`, `RecoveryUnavailable`, `RecoveryCleanup`, and `RecoveryInvalid`. This order owns only their mapping at the MCP boundary; it does not change recovery behavior or public tool schemas.

# Files allowed to touch
- `rust/crates/vexel-app/src/session/mcp_tools.rs`
- `work/WO-RUST-RECOVERY-MCP-ERROR-01.md` and `ledger.jsonl` only by coordinator
- `artifacts/WO-RUST-RECOVERY-MCP-ERROR-01/**`

# Protected paths
Everything else, including all other session and tool implementations, manifests/locks, work orders, interfaces, schemas, fixtures, C references and tests. No dependencies, network, GUI, reset, deletion, or commit to main before review.

# Acceptance criteria
- AC-1: Every `SessionError` variant is mapped to a stable MCP error string; recovery conflict is distinguished from invalid project and generic I/O failure.
- AC-2: `cargo test -p vexel-app --locked` passes, and `git diff --check` passes.

# Verification commands
```text
cargo test -p vexel-app --locked
git diff --check
```

# Non-goals
Changing MCP schemas, transport, file permissions, recovery lifecycle, or any user-facing error alert.
