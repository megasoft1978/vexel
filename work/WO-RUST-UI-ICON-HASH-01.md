---
id: WO-RUST-UI-ICON-HASH-01
revision: 1
status: READY
milestone: RUST-MIGRATION
owner: project-owner
executor: codex-gpt-6-luna
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: d438a358e108be8b6bb33303c2ffe1cd8bc09d6b14ae3f1f4f9b2e7767d2fbf1
parallel_safe: true
network: false
---
# Goal
Strengthen the accepted `vexel-ui` icon asset tests with exact frozen C hash expectations for scales 1x, 2x and 3x, for both light and dark themes. Add provenance to the parity map for the unchanged C probe and include the complete 12-test focused inventory. Do not change production code or C oracle/fixture files.

# Context
Prior accepted order: `WO-RUST-UI-DRAW-01`. Independent review found asset-hash tests checked only relative properties and the parity inventory omitted a focused pan-mask case. Frozen C-derived expected hashes reported by an unchanged-source oracle probe: `dd0cedd4ec723d06` (1x), `2d4e975f7b5775c5` (2x), `be120e3a5aa41d42` (3x); theme is expected to be hash-neutral.

# Files allowed
- `rust/crates/vexel-ui/src/tests.rs`
- `artifacts/WO-RUST-UI-ICON-HASH-01/**`

# Protected
All UI production source/public APIs, C sources/tests/oracles, manifests/locks, other worktrees and coordinator files. No dependency, unsafe code, GUI/E2E/golden campaign, or weakened/removed existing assertion. Keep source files below 800 lines.

# Acceptance
- Exact Rust hash assertions cover all six theme/scale combinations and match the C-derived constants.
- The focused test inventory maps every current relevant test, including the pan-mask scales test.
- C oracle files and values remain unchanged; parity map identifies source/tool and exact hashes.
- Independent review, exact allowed-path/hash audit and fresh main-tree checks pass.

# Verification
`cargo +stable check -p vexel-ui --locked --offline`
`cargo +stable test -p vexel-ui --locked --offline`
`python3 scripts/wo_validate.py WO-RUST-UI-ICON-HASH-01 --ready`
`python3 scripts/ledger.py validate ledger.jsonl`
`git diff --check`

# Candidate
Coordinator may reuse `.worktrees/rust3-ui-draw` only after recording the main test seed. Candidate output is `parity-map.json`, `verification.txt`, and `result.md` under this order's artifact directory. Coordinator alone integrates and updates ledgers/STATE.
