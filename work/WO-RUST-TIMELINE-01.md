---
id: WO-RUST-TIMELINE-01
revision: 1
status: READY
milestone: RUST-MIGRATION
owner: project-owner
executor: codex-gpt-6-luna
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 4d990d0a1c1bc1f53d58bbcb49acc81dee7cd51333fa52cb67ab2cf85dd37881
parallel_safe: true
network: false
---
# Goal and frozen scope
Port complete project_timeline.c and project_timeline_json.c decode/classify/accessor/iterator/string-copy behavior. Safe lifetime-bound immutable TimelineView source plus Span metadata; owned bounded context of sorted scene IDs and asset references, validated timeline schema/property/drawing tracks/loop/onion parameters, timing/FPS and reference/duplicate rules. Exact error categories/offsets, cancellation during scans, native-v1 classification, UTF8/escapes/surrogates, unknown/duplicate fields, nested-depth and string/source limits,256tracks/1Mtotalitems/100Kpertrack/4096stringbytes. Sequential iterator must keep bounded progress and avoid repeated scans/O(n^2) on large tracks. Port meaningful original decoder/JSON cases, invalid-reference/duplicate/schema, raw escaped string equality, track/item source spans, canceled parse cleanup and malformed input. Accepted general JSON core may have smaller DOM limits: do not silently impose these on timeline; port the domain bounded streaming JSON reader instead. Native edit/session/playback/render are later separate orders, not fabricated here.

# Reference paths
src/project/project_timeline.c, project_timeline.h, project_timeline_json.c, project_timeline_json.h, project_bundle.h; tests/project/project_timeline_test.c and scoped native timeline decoder cases
Legacy reference provenance: original dirty-source hashes in artifacts/WO-RUST-PLAN-01/source-baseline.json; no external algorithms/source fetch.

# Files allowed to touch
- `work/WO-RUST-TIMELINE-01.md`
- `rust/crates/vexel-timeline/src/**`, `rust/crates/vexel-timeline/tests/**`
- `artifacts/WO-RUST-TIMELINE-01/**`
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`, `docs/product/rust-port-goal-and-plan.md` (coordinator only)

# Protected paths and quality
All legacy source/fixtures/oracles, accepted other-crate code, all Cargo/config/lock/toolchain metadata, other worker trees, coordinator records/docs. Existing same-crate accepted behavior/assertions preserved. Worker only own src/tests/artifacts; coordinator alone shared integration. No manifest edit, code outside scope, dependency/download/install, unsupported OS code, Cshim, staging/commit/delete/reset, network/recursive agents, broad UI/E2E/C/performance campaign. Safe owned/lifetime-bound data, bounded inputs/checked arithmetic/fallible significant allocations, no unsafe in new modules; preserve accepted platform sys boundary. Source<800lines. No TODO/unimplemented/production stub for named scope. Map every named C API/helper to Rust symbols, focused tests, source/diagnostic spans, ownership/limits/cleanup and exact safe adaptations in parity-map.json. If dependency or contract cannot meet full scope stop with concrete issue; do not narrow deliverable to convenient subset.

# Execution and verification
Only .worktrees/rust3-timeline; Standard subscription LunaHigh, thin scope/context, one writer and focused independent review/repair. Read OWNER-DIRECTIVES/AGENTS and this order; consolidated goal governs full finish line. Preserve accepted underlying interfaces. Root serializes shared metadata/integration.
cargo +stable check -p vexel-timeline --locked --offline
cargo +stable test -p vexel-timeline --locked --offline
python3 scripts/wo_validate.py WO-RUST-TIMELINE-01 --ready
python3 scripts/ledger.py validate ledger.jsonl
git diff --check
Exact existing1.99/macOS13, repo-local Cargo/cache/tmp/targets. No rustfmt/clippy installation or unsupported claims. Original fixtures/assertions unchanged; add focused Rust regressions with exact outputs, not only is_err. Run no extra broad suite.

# Output
Leave uncommitted; result<=400words, parity-map.json/verification.txt with actual checks, failures/repairs/adaptations and remaining app scope. Candidate only until independent review, exact scope/main audit and named integrated checks. Existing allowance only; no API billing/credits; stop/checkpoint on rate/allowance errors rather than retryloop.
