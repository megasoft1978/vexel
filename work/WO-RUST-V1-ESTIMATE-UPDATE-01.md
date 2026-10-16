---
id: WO-RUST-V1-ESTIMATE-UPDATE-01
revision: 1
status: READY
milestone: V1-SCOPE-REFINEMENT
owner: project-owner
executor: gpt-6-luna
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: a8358f51d9b099848b367cd7994f836497d23b524d3980cde0f1f1eb4f9f6a0b
parallel_safe: false
network: false
---
# Shorten the V1 delivery plan

## Goal
Refine the accepted V1 description after the owner's request for a much
shorter, less deep first delivery. Preserve the one-character drawing-to-
animation workflow and the unchanged full-product goal.

## Required outcome
- Keep V1 to one small character, one limited SVG/Inkscape subset, one short
  animation loop, editable `.vexel` save/reopen, preset GIF export, and visible
  authorized Codex MCP edits to the same project.
- State the deferred capabilities as later full-product work; do not weaken the
  full goal, final acceptance criteria, or final Codex video requirement.
- Provide an understandable canvas-led workspace sketch and a per-feature
  estimate table grounded in the current Rust roadmap. The row minima and maxima
  must sum to the stated total.
- Include one integrated V1 walkthrough; leave the comprehensive UI/E2E pass and
  public website/README video at the full-product stage.
- The prior estimate is 14–26 focused days. This update proposes 10–17 focused
  days based on the current accepted foundation and explicit decoder/composition
  risk; identify the risk and do not present a date promise.

## Allowed paths
- `work/WO-RUST-V1-ESTIMATE-UPDATE-01.md`
- `docs/product/vexel-v1-vertical-slice.md`
- `docs/product/rust-port-goal-and-plan.md` (V1 summary section only)
- `artifacts/WO-RUST-V1-ESTIMATE-UPDATE-01/**`
- `STATE.md`, `LEDGER.md`, `ledger.jsonl` (coordinator records after checks)

## Protected paths
All source, tests, dependencies, ADRs including accepted ADR-0029, canonical
full-goal sections and final gates, unrelated docs, website/README, historical
work orders/evidence and unrelated dirty changes.

## Verification
- Validate this frozen work order and the root ledger.
- Run `git diff --check` on the two documentation targets and direct whitespace
  checks on new work-order/evidence files.
- Audit that the V1 estimate sums to 10–17 focused days and its scope does not
  claim full SVG, performance, accessibility, MCP interoperability, release or
  public-demo acceptance.
- Record exact document hashes and command results.
- No application, GUI, browser, performance or broad UI/E2E tests.

## Output
Leave changes uncommitted. Record evidence and this refinement in STATE.md and
both ledgers while preserving the prior accepted estimate as history.
