---
id: WO-RUST-V1-POLISH-01
revision: 1
status: READY
milestone: V1-SVG-COMPOSITION
owner: project-owner
executor: gpt-6-luna
base_sha: 1d077a5539d44b64586b5416ccf7dab99f9f58f9
spec_sha256: 4325fcabb49af65b94471d6c34e1dee3f0dd077a2906b20c84de1be1cc6bef8c
parallel_safe: true
network: false
---
# V1 polish: Open filter, Cmd+S double-fire, element_add schema, dead code

## Goal
Close four known V1 gaps listed in HANDOFF.md "Known gaps" without adding
features: File > Open offers `.svg` and `.vexel`; Cmd+S cannot save twice for
one key press; the live-preview `element_add` schema advertises `line` and
`path`; legacy dead code and blanket `allow(dead_code)` in `vexel-app` go.

## Dependencies
Read first: `HANDOFF.md` (lesson 3: when behaviour changes deliberately, fix the
test that pinned the old behaviour, never the guard), `docs/adr/0029`,
`vexel-app/src/file_flow/{mod,menu,tests}.rs`, `vexel-app/src/main.rs`
(`WindowEvent::KeyDown` branch), `vexel-platform-apple/src/native_window/
file_menu.rs` (read only), `vexel-protocol/src/tools.rs` (~line 371),
`vexel-app/src/session/{mcp,tests_mcp,tests_playback}.rs`,
`vexel-app/src/preview_app.rs`.

## Assumptions
- `open_panel(extensions)` takes a slice; `.svg` documents already open through
  the compose path (APP-COMPOSE-01), so the panel only needs to offer the type.
- AppKit tries the menu key equivalent before `keyDown:`; a double fire needs
  both paths to run for one press. The platform crate is protected, so the fix
  lives in `vexel-app`. If analysis shows no double fire is reachable, the order
  still lands the dedup guard and test and says so in `result.md`.
- Other crates call `McpHost::handle` only in the listed tests; verify with a
  symbol search before deleting and list the hits in the audit.

## Scope
1. Open filter: `OPEN_EXTENSIONS` becomes `["svg", "vexel"]` (order fixed).
   Update the two `file_flow` tests that pin `["vexel"]` and add one test that
   an `.svg` selection reaches the open path. Record in `result.md` that this is
   a deliberate behaviour change (HANDOFF lesson 3).
2. Cmd+S dedup: pure, window-free guard in `file_flow` (for example
   `ShortcutDedup`) that suppresses a keyboard-originated file action equal to
   one delivered by the menu within a short window (default 150 ms, a named
   constant, clock injected). `main.rs` routes both paths through it. Different
   actions, or the same action after the window, are never suppressed.
3. Preview `element_add` schema: in the preview branch of `tools.rs` change the
   `template` enum to `["rect","ellipse","line","path"]` and the description to
   name all four. The argument validator already accepts line/path; do not
   change it. Non-preview schema is untouched.
4. Dead code: delete `McpHost::handle` (3-arg) and `standalone` if it becomes
   unused; migrate its callers in `session/tests_mcp.rs` and
   `session/tests_playback.rs` (the INTEGRATE tests) to `handle_with` with an
   explicit `Driver`, preserving every assertion and expected value. Remove the
   dead launcher in `preview_app.rs` (keep only what `main.rs` uses, or delete
   the module and its `lib.rs` line if nothing remains). Replace each blanket
   `#![allow(dead_code...)]` in `vexel-app/src` with nothing, or with
   `#[cfg_attr(not(test), allow(dead_code))]` on a named item with a one-line
   reason, only where the item is genuinely test-only. `tests/common/mod.rs` may
   keep its allow (shared test helper).

## Non-goals
New tools, new menu items, platform crate changes, Save/Open semantics, UI
changes, live-window verification by the worker, formatting unrelated code.

## Files allowed to touch
- `rust/crates/vexel-app/src/file_flow/**`, `rust/crates/vexel-app/src/main.rs`,
  `rust/crates/vexel-app/src/lib.rs`, `rust/crates/vexel-app/src/preview_app.rs`,
  `rust/crates/vexel-app/src/project_session/mod.rs`,
  `rust/crates/vexel-app/src/tools/mod.rs`, `rust/crates/vexel-app/src/preview.rs`
  (only its first-line `allow` and now-unused items),
  `rust/crates/vexel-app/src/session/mcp.rs` (only removing `handle`/
  `standalone`), `rust/crates/vexel-app/src/session/tests_mcp.rs`,
  `rust/crates/vexel-app/src/session/tests_playback.rs`,
  existing files under `rust/crates/vexel-app/tests/` (only if a call site needs
  migration; no new files there)
- `rust/crates/vexel-protocol/src/tools.rs` (only the preview `element_add`
  lines) and `rust/crates/vexel-protocol/tests/mcp_preview_profile.rs`
- `artifacts/WO-RUST-V1-POLISH-01/**`

Shared files: `session/mcp.rs`, `preview.rs` and `tools.rs` are also edited by
WO-RUST-V1-PNG-01 in different hunks. Touch only the hunks named above; the
coordinator applies the two patches sequentially with `git apply --3way`.

## Protected paths
Workspace/lock manifests and every `Cargo.toml` (coordinator only), all other
crates including `vexel-platform-apple`, C/ObjC, `tests/`, goldens, oracles,
`docs/**`, `STATE.md`, `LEDGER*.md`, `ledger.jsonl`, other work orders, git
HEAD/index. No unsafe, new dependencies, network, installs, GUI launch, commit,
stage, reset, or deletion beyond the files named above.
Files under 800 lines.

## Acceptance criteria
1. `OPEN_EXTENSIONS == ["svg","vexel"]`; the two updated tests assert it; a new
   test proves an `.svg` path chosen in the panel opens via the compose path and
   a `.vexel` path via the project path.
2. Dedup test (no display): menu Save then key Save inside the window yields
   exactly one save call; key Save alone yields one; two presses 1 s apart yield
   two; Save then Save As in the window yields two. `main.rs` uses the guard on
   both paths.
3. A preview `tools/list` test asserts the `element_add` enum is exactly
   `rect, ellipse, line, path` and the description mentions line and path; the
   non-preview schema test is unchanged and passes.
4. `grep -rn "McpHost::handle(" rust/` and `.handle(&mut app` on a `McpHost`
   find no 3-arg call; every migrated test keeps its original assertions
   (diff shows only call-shape changes). `handle` is gone from `mcp.rs`.
5. `preview_app.rs` contains no item that nothing references; no
   `#![allow(dead_code` remains in `vexel-app/src`; any retained
   item-level allow is listed with its reason in `result.md`.
6. `cargo test -p vexel-app` and `cargo test -p vexel-protocol` pass; test
   counts may only increase except for the deleted dead-code tests, which are
   listed by name in `result.md` (there should be none).
7. No `unsafe`; `cargo tree -p vexel-app` shows no new package.
8. `result.md` (<=300 words) states whether a double fire was reachable by code
   reading, and lists unexecuted live checks (real Cmd+S, real Open panel).

If an AC cannot be met, stop with the exact blocker; no test weakening.

## Verification (frozen; repo-local CARGO_HOME/CARGO_TARGET_DIR, locked, offline)
- `cargo +stable check -p vexel-app --locked --offline`
- `cargo +stable test -p vexel-app --locked --offline`
- `cargo +stable test -p vexel-protocol --locked --offline`
- `cargo +stable fmt --manifest-path Cargo.toml --package vexel-app -- --check`
- `python3 scripts/wo_validate.py WO-RUST-V1-POLISH-01 --ready`
- `python3 scripts/ledger.py validate ledger.jsonl`
- `git diff --check`
`vexel-protocol` is not fmt-checked here (pre-existing drift; see
WO-RUST-V1-RELEASE-CI-01).

## Evidence
`artifacts/WO-RUST-V1-POLISH-01/`: `result.md`, `verification.json`,
`ac-map.json`, `changed-path-audit.json`.

## Worker and review
Worker: gpt-6-luna Medium, one writer, existing allowance only; stop on quota
errors. Candidate only until a fresh read-only reviewer reruns the frozen
commands and checks each AC. The coordinator integrates the patch, reruns
main-tree checks, performs the live Cmd+S and Open-panel check in an isolated
session, then updates STATE.md, LEDGER.md and ledger.jsonl.
