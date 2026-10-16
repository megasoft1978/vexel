---
id: WO-RUST-V1-WALKTHROUGH-01
revision: 1
status: READY
milestone: V1-SVG-COMPOSITION
owner: project-owner
executor: gpt-6-luna
base_sha: 8f10553a1a7874c1843ae8161e82c8c2de255351
spec_sha256: d7d9e43da87f62f12a366b9a86eabe5f103e5935d35571c354c6e96462227620
parallel_safe: false
network: false
---
# Scripted Codex-over-MCP V1 walkthrough

## Goal
Prove the ADR-0029 V1 workflow end to end with an automated test that drives
the real `vexel-mcp-stdio` protocol against the integrated app core, plus a
manual checklist for the native window. Acceptance = the ADR-0029 cutline items
(below). It is not the broad UI/E2E campaign, not the public Codex video, and
makes no full-product claim.

## Dependencies (blocking)
`WO-RUST-V1-DECODE-SUBSET-01`, `WO-RUST-V1-RENDER-01`, `WO-RUST-V1-APP-COMPOSE-01`,
`WO-RUST-V1-SAVE-01` and `WO-RUST-V1-ANIM-01` all ACCEPTED and integrated. If any
is missing or its MCP tool (save/open/export/undo/redo/animation) is absent,
stop and report the exact gap; do not add features here beyond thin glue.

## Assumptions
- The automated test runs a headless session core (same `V1Document`, render,
  GIF and project code as the app, no NSWindow) speaking JSON-RPC over
  stdin/stdout pipes via `vexel-mcp-stdio`, so it runs without a display. Native
  frame acceptance is covered only by the manual checklist.
- Codex is simulated by a deterministic scripted client (fixed JSON requests);
  no network, no LLM, no API spend.
- Tool names are those in `vexel-protocol` (`element_add`, `animation_add`,
  `export_animation`, `export_svg`, undo/redo, save/open as integrated).

## Scope
1. `rust/crates/vexel-app/tests/walkthrough.rs` (+ helpers, fixtures): scripted
   session, steps in order:
   a. initialize, list tools, grant temp directory;
   b. draw a text-free character (>=5 shapes: body, head, two eyes, mouth) with
      distinct ids and solid fills/strokes;
   c. revise one shape (recolor/resize/move) and assert the document source
      changed only for that element;
   d. create a 2-4 key position or opacity loop (blink) on one element;
   e. undo then redo one edit; assert source bytes restore exactly each time;
   f. save the project to the granted directory, close the session, reopen in a
      new session, assert source bytes and animation intact and still editable
      (one more edit succeeds);
   g. export GIF (preset fps/duration) and one SVG;
   h. assert each step's MCP result reports activity (operation name, element
      ids, outcome) so the UI can show it.
2. GIF verification: parse the exported file with the ANIM-01 test decoder:
   frame count = fps*duration, constant delay, loop block, canvas size, first
   frame differs from the blink frame at the edited element's pixels.
3. SVG verification: exported SVG re-decodes through the DECODE-SUBSET pipeline
   and re-renders equal (within RENDER-01 tolerance) to the saved document.
4. Negative paths: an unauthorized path, malformed request and out-of-order call
   return typed protocol errors without altering the document.
5. `docs`-free manual checklist at `artifacts/WO-RUST-V1-WALKTHROUGH-01/manual-checklist.md`:
   native window steps (Draw/Animate switch, canvas draw, inspector recolor,
   scrub/play, Save/Open panels, GIF in a second viewer, undo/redo shortcuts,
   keyboard focus visible, Codex activity panel updates), each with pass/fail
   boxes and "not run by worker".

## ADR-0029 cutline mapping (each must have a named test or checklist row)
one document; one text-free character; a few edit actions; short keyframed loop;
editable save/reopen; basic SVG subset (import/export, unsupported disclosed);
GIF export; local authorized MCP; visible activity/results; undo/redo of
demonstrated edits. `cutline-map.json` maps each to test or checklist row.

## Non-goals
Pixel-golden UI, accessibility audit, performance, remote MCP, multi-client,
recovery, packaging, recording the Codex video, new product features.

## Files allowed to touch
- `rust/crates/vexel-app/tests/**` (new files), `rust/crates/vexel-app/src/**`
  only for thin glue proven necessary by a failing walkthrough step (each
  change listed in the audit with the failing step), `rust/crates/vexel-app/Cargo.toml`
  (dev-dependency path crates only)
- `rust/fixtures/v1/walkthrough/**` (new)
- `artifacts/WO-RUST-V1-WALKTHROUGH-01/**`

## Protected paths
Workspace/lock manifests, all other crates, C/ObjC, `tests/`, goldens, oracles,
`docs/**`, `STATE.md`, `LEDGER*.md`, `ledger.jsonl`, other work orders, git
HEAD/index. No unsafe, external deps, network, installs, GUI launch, real Codex
or API calls, commit, stage, reset, deletion. Files under 800 lines. Never
weaken or delete another order's tests.

## Acceptance criteria
1. Steps a-h pass in one `cargo test` run, deterministically (3 consecutive runs
   byte-identical on saved project, GIF and SVG files).
2. Each undo/redo assertion compares full source bytes before/after.
3. Reopened project equals the saved one and accepts a further edit.
4. GIF and SVG verification (scope 2, 3) pass as stated.
5. Negative paths (scope 4) pass with no document mutation.
6. `cutline-map.json` covers all ten cutline items, each pointing at an existing
   test name or checklist row; unmet items are named as gaps (never omitted).
7. Manual checklist exists, is runnable by the owner, and every row states
   expected observable result.
8. Existing tests of every touched crate pass; no new external package.
9. `result.md` (<=400 words) lists passed, failed, glue added and unexecuted
   (all native-window rows).

If an upstream gap blocks an AC, stop with the exact blocker; do not fake.

## Verification (frozen; repo-local cargo dirs, locked, offline)
- `cargo +stable test -p vexel-app --test walkthrough --locked --offline`
- `cargo +stable test -p vexel-app --locked --offline`
- `cargo +stable test -p vexel-mcp-stdio -p vexel-protocol --locked --offline`
- `cargo +stable fmt --manifest-path Cargo.toml --package vexel-app -- --check`
- `python3 scripts/wo_validate.py WO-RUST-V1-WALKTHROUGH-01 --ready`
- `python3 scripts/ledger.py validate ledger.jsonl`
- `git diff --check`

## Evidence
`artifacts/WO-RUST-V1-WALKTHROUGH-01/`: `result.md`, `verification.json`,
`cutline-map.json`, `manual-checklist.md`, `changed-path-audit.json`,
three-run hash table.

## Worker and review
Worker: gpt-6-luna High, Standard speed, one writer, existing allowance only; stop
on quota errors. Candidate until a fresh read-only reviewer (separate session,
Luna or Sol Medium; diff, order, checklist only) reruns frozen commands and maps
every cutline row. The coordinator integrates, reruns main checks, executes the
manual checklist on an isolated desktop session, then updates STATE.md,
LEDGER.md and ledger.jsonl. V1 is declared only after the checklist passes.
