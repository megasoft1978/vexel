---
id: WO-RUST-GOAL-05
revision: 1
status: READY
milestone: RUST-MIGRATION
owner: project-owner
executor: codex-coordinator
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: f97c2d5217e599cfe7bb556d8732020aa85d99858fefd3999276afff29731bf9
parallel_safe: false
network: false
---
# Goal
Apply the owner's request to update the goal with the complete Rust product
finish line, retaining every agreed drawing/animation/live-agent/performance/
release/demo requirement. Reconcile the now-active tracker without falsely
claiming its language-neutral objective was edited through unavailable APIs.

# Files allowed to touch
- `work/WO-RUST-GOAL-05.md` (coordinator freeze only)
- `docs/product/rust-port-goal-and-plan.md`
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`
- `artifacts/WO-RUST-GOAL-05/**`
- `artifacts/WO-RUST-NATIVE-BINDINGS-SETUP-01/current-goal-checkpoint.json` (current mutable status only; archive previous copy in own artifacts)

# Protected paths
All product/source/Rust manifests/lock/config, original fixtures/oracles,
previous frozen orders and evidence except exact mutable status path above.
No application code edits/builds/tests, newdependencies/network/installs/agents,
stage/commit/reset/delete or goal completion/status hack. Existing active
tracker objective cannot be changed with available status-only update API;
retain it and publish full canonical wording/authority as repository supplement.

# Requirements
Canonical full goal: all-first-party Rust, AppleSilicon macOS13+, complete vector
editor/cartoon animation, original moderncustom UI informed by modern tools,
owned source-preserving SVG/Inkscape interoperability, versioned native projects/
assets/recovery, measured large-file performance and memory/build/bundle size,
real discoverable authorized visible Codex/Claude MCP with progress/cancel/
undo/save/export, comprehensive final testing and fixes, usable packaged fresh
install, documentation/examples/release evidence, authentic live Codex drawing/
animation recording plus reusable projects/prompts/exports/captions/transcript
for website and README. Maintain all nine outcome areas and seven completion
gates; no scope or quality reduction. Preserve MPL2/minimalreviewedpindeps/noGPL
editorSource/noexternalSVGfetch/legacy603/index. Parallel useful Luna at fitting
effort and Standard subscription-only, named escalations, one writer/module and
coordinator sharedintegration. Focused checks duringimplementation, broadUI/E2E
at final stage. Retain actual460namedtest/27package checkpoint; not wholeappQA.

# Verification
python3 scripts/wo_validate.py WO-RUST-GOAL-05 --ready
python3 scripts/ledger.py validate ledger.jsonl
git diff --check
Validate canonical objective JSON and nine outcome/seven finish-gate coverage
against authoritative plan, actual tracker status active and latest accepted
ledgers. Audit only allowed document/status changes, original603/hashes,
HEAD/index. No application tests/builds. Record tracker objective edit limitation
accurately without repeated known-failing create calls.

# Output
Updated authoritative full goal and current status, canonical machine-readable
objective and completion requirements; active unfinished goal retained.
