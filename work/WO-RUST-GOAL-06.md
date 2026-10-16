---
id: WO-RUST-GOAL-06
revision: 1
status: READY
milestone: RUST-MIGRATION
owner: project-owner
executor: codex-coordinator
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 59272368fa0151dbf0137da77eb1e2e507e459d6171c60f15e3c2f29d28745e7
parallel_safe: false
network: false
---
# Goal
Make the owner's final showcase acceptance explicit in the existing full Rust
application goal: real prompts in Codex draw a character, revise that drawing,
animate it, and revise its animation through MCP while Vexel visibly updates
through successive calls. Retain every existing outcome and completion gate.

# Files allowed to touch
- work/WO-RUST-GOAL-06.md (coordinator freeze only)
- docs/product/rust-port-goal-and-plan.md
- STATE.md, LEDGER.md, ledger.jsonl
- artifacts/WO-RUST-GOAL-06/**

# Protected paths
All application sources/manifests/config/tests/oracles, prior work orders and
evidence. No source edits, application builds/tests, network/installs or new
workers in this documentation scope. Other existing active orders remain
independently authorized. No false goal completion or unsupported tracker edit.

# Requirements
Authentic conversation and visible same live app/project throughout the draw,
drawing-revision, animation and animation-revision sequence. Show progressive
changes during MCP iteration, final playback, save/reopen and exported animation.
Real MCP execution must substantiate the demonstration, with retained privacy-
reviewed call evidence and prompts. Production control supports Codex, Claude
Code and other compatible MCP clients; Codex is the required recorded example.
Review footage, reusable native project/prompts/exports, captions or transcript,
preview and working website/README embeds or links. A prerecorded result cannot
replace proof of actual live operation. Goal remains active/incomplete; preserve
nine deliverable areas, seven completion gates and all platform/quality locks.

# Verification
python3 scripts/wo_validate.py WO-RUST-GOAL-06 --ready
python3 scripts/ledger.py validate ledger.jsonl
git diff --check
Validate the updated full-goal JSON retains all nine areas/seven gates, explicit
sequence/live-MCP/reusable/presentation evidence, current active tracker and
unchanged prior-goal evidence. Exact scoped document/source/HEAD/index audit.
No application/UI tests required for this documentation clarification.

# Output
Updated authoritative goal and machine-readable acceptance; final demo remains
required pending actual completed application, final acceptance and recording.
