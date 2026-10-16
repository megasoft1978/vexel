---
id: WO-RUST-NATIVE-PLAN-01
revision: 1
status: READY
milestone: RUST-MIGRATION
owner: project-owner
executor: codex-gpt-6.1-sol
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 45e1053a56d2371d101260f39e56cc3b1c8236db62cf4e63cecfe6f93d40605f
parallel_safe: true
network: false
---
# Goal
Read-only narrow native Rust/AppKit integration contract plan for firstusable desktop, consuming accepted customUI/ownedsource target while preserving entireproduct goal. This is a plan, not dependency adoption/implementation acceptance.

# Allowed output
- `artifacts/WO-RUST-NATIVE-PLAN-01/**` (workerreports only)
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`, `docs/product/rust-port-goal-and-plan.md` (coordinatoronly)

# Named reads / questions
OWNER/AGENTS; platform_window.h, platform_text_ui.h, platform_text.h, platform_metal.h, macos platform_window_internal.h/native_shell.m/window.m/geometry.m/events andtextheaders; accepted RustUI/theme/canvas callback contracts; artifacts/WO-RUST-PLAN-01/research/platform_ui.md andtoolchain.md; docowned readyorder asproposedAPI, notaccepted source. SDK declarations read-only ifneeded for mac13availability; no externalcodefetch. Define main-thread/ownership/callback-unwind/lifetime/eventqueue/drawing/menu/AX/resource teardown interfaces, owned source and job/eventhand-off, public platformboundary typedAPI sketch and earliest complete runnable shell dependency sequence. Identify minimalApplebinding dependency decision/ADR/provenance needed fromexisting proposal, notnew unverifiedversion claims. Futurelibrarymetadata/currentAPI verification separatebeforeadoption; do notinstall crates or change locks. Account for C/ObjCmoduleinventory andMac13availability gaps; no portable backend/no stub nativecalls or GUIframework replacement. Originalsource preserved, GPLeditorsource unread.

# Execution / output
Actual Sol6.1Medium Standard subscription, read-only/no network/build/tests/edits except own finalreport and evidence files byCLIoutput. No agents/extra billing/retryloops. Report<=700words with concrete blockers/APIcontracts/order sequence, include machine-readable inventory ifuseful; full nativeUI/editor acceptance remains required. Coordinatorready/ledger/diff/source checks, no applicationtests.
