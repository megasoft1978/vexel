---
id: WO-RUST-PROJECT-IO-BOUNDARY-01
revision: 1
status: READY
milestone: RUST-MIGRATION
owner: project-owner
executor: codex-gpt-6-luna
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: d16fdc134ee79c9acd42229c23c7a0c8d29b4e8ff32e0e2357d66af1b7b15387
parallel_safe: true
network: false
---
# Goal
Add focused regression assertions to the already accepted `vexel-project-io` crate for recovery decode error offset and identity-check precedence, plus bounded total-size declaration arithmetic. Correct the parity map to describe that `RecoveryView` omits C's redundant `encoded_size`, which callers infer from the validated input length. Do not change production code or weaken prior assertions.

# Context
The prior accepted WO is `WO-RUST-PROJECT-IO-01`. A review found gaps in its acceptance evidence; see the current candidate's repair and `artifacts/WO-RUST-PROJECT-IO-01/parity-map.json`. The Rust view is borrowed from input bytes, and maximum valid capsule length is derived from fixed header, bounded path and bundle maxima.

# Files allowed
- `rust/crates/vexel-project-io/src/tests.rs`
- `artifacts/WO-RUST-PROJECT-IO-BOUNDARY-01/**`

# Protected
All production source, public API, C sources/tests/oracles, manifests/locks, other worktrees and coordinator files. No dependencies, unsafe, filesystem cleanup outside test-owned temp roots, broad suites, UI/E2E, performance or changes to previous orders. Keep test code below 800 lines.

# Acceptance
- A checksum-repaired invalid inner bundle returns `InvalidProject` with the precise inner error/offset; a mismatched expected source path preserves identity-check precedence and offset.
- Recovery boundary assertions cover the documented max formula and exact/over-limit declarations without large allocation; describe these as declaration/arithmetic checks, not a fully materialized max-size decode.
- Existing project-I/O assertions remain unchanged; no production behavior changes.
- Parity mapping accurately describes `encoded_size` adaptation and maps all new tests.
- Independent review, exact allowed-path/hash audit and fresh main-tree checks pass.

# Verification
`cargo +stable check -p vexel-project-io --locked --offline`
`cargo +stable test -p vexel-project-io --locked --offline`
`python3 scripts/wo_validate.py WO-RUST-PROJECT-IO-BOUNDARY-01 --ready`
`python3 scripts/ledger.py validate ledger.jsonl`
`git diff --check`

# Candidate
Coordinator may reuse `.worktrees/rust4-project-io` only after recording the main test seed. Candidate output is `parity-map.json`, `verification.txt`, and `result.md` under this order's artifact directory. Coordinator alone integrates and updates ledgers/STATE.
