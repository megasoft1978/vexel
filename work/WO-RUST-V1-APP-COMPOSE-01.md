---
id: WO-RUST-V1-APP-COMPOSE-01
revision: 1
status: READY
milestone: V1-SVG-COMPOSITION
owner: project-owner
executor: gpt-6-luna
base_sha: 8f10553a1a7874c1843ae8161e82c8c2de255351
spec_sha256: d1966023f120f4299115a2a8aaf530702f45896e67f8f104ec0fecd24ff3952a
parallel_safe: false
network: false
---
# Compose decode, scene, render and presentation in vexel-app

## Goal
In `rust/crates/vexel-app`, open one SVG file and show it in the native window:
bytes -> `vexel-svg-engine`/`vexel-svg-decode` -> `vexel-scene` ->
`vexel-render` -> RGBA frame -> `vexel-platform-apple` presentation. Covers the
"import the supported SVG example" step of docs/product/vexel-v1-vertical-slice.md
within the ADR-0029 SVG subset only. No project save, animation, GIF, MCP change
or full-SVG claim.

## Dependencies (blocking)
`WO-RUST-V1-DECODE-SUBSET-01` and `WO-RUST-V1-RENDER-01` must be ACCEPTED and
integrated on main (crates `vexel-svg-engine` and `vexel-render` in the
workspace). If either is absent at start, stop and report; do not vendor or
re-implement them.

## Assumptions
- Existing `--mcp-preview` and shell launch behaviour and tests stay unchanged.
- `vexel-render` offers `render_shapes`, `ViewMap` and `offscreen_tile_size`;
  the live path uses the host backing scale from `vexel-platform-apple`.
- "1x backing" means the compose pipeline renders at 1 pixel per point and the
  presenter maps it 1:1 to the window's logical size; Retina scaling is a
  disclosed later item, not silently done.

## Scope
1. New module(s) in `vexel-app` (e.g. `compose.rs`, `open.rs`) with a pure,
   window-free core: `compose_svg(bytes, width, height) -> Result<Frame,
   ComposeError>` returning RGBA8 plus a disclosure list of unsupported items.
2. Open paths: CLI `vexel-app <file.svg>` and a File > Open... menu/open-panel
   action (NSOpenPanel is already enabled in `vexel-platform-apple` features).
   Path handling bounded (size cap 8 MiB, UTF-8/NUL-safe path bytes); no network.
3. Pointer mapping: pure function canvas-pixel <-> document coordinates using
   the same `ViewMap` as render (viewBox, preserveAspectRatio, 1x); window
   pointer events reach it through the existing `Model` layout.
4. Typed errors shown in the status area for unreadable, oversize, malformed or
   `Unsupported` content (never a panic, never silent drop); deferred features
   are listed in the status/disclosure text.
5. Cancellation/stale-frame safety: a new open or resize supersedes in-flight
   composition; no unbounded queue.

## Non-goals
Editing the opened SVG, save/export, zoom/pan, 2x/Retina fidelity, animation,
MCP wiring, SVG features beyond DECODE-SUBSET-01 scope, performance targets,
broad UI/E2E, packaging.

## Files allowed to touch
- `rust/crates/vexel-app/src/**`, `rust/crates/vexel-app/tests/**`,
  `rust/crates/vexel-app/README.md`, `rust/crates/vexel-app/Cargo.toml`
  (add path deps `vexel-svg-engine`, `vexel-svg-decode`, `vexel-scene`,
  `vexel-render`, `vexel-geometry` only)
- `rust/fixtures/v1/compose/**` (new first-party SVG fixtures and expected PPM
  hashes; new files only)
- `artifacts/WO-RUST-V1-APP-COMPOSE-01/**`

## Protected paths
Root and `rust/` `Cargo.toml`/`Cargo.lock` (coordinator edits manifests), every
other crate, C/ObjC sources, `tests/`, goldens, oracles, `docs/**`, `STATE.md`,
`LEDGER*.md`, `ledger.jsonl`, other work orders, git HEAD/index. No unsafe, new
external dependencies, network, installs, commit, stage, reset or deletion. No
launching the GUI; native behaviour is verified by the coordinator separately.
Each source file under 800 lines.

## Acceptance criteria
1. `compose_svg` on a fixture covering rect, circle, ellipse, path, polygon,
   nested transformed group, solid fill/stroke and a viewBox returns a frame
   whose size equals the request and whose bytes equal frozen hashes (or PPM
   fixtures) in `rust/fixtures/v1/compose/`; two runs are byte-identical.
2. Same fixture rendered through the DECODE+RENDER pipeline matches the
   RENDER-01 per-channel tolerance against the C `--render-frame` PPMs for the
   shared first-party fixtures (coordinator supplies PPMs; absent -> stop).
3. Pointer mapping: round-trip canvas->doc->canvas within 1e-9 for meet, slice
   and none; points outside the viewport report `None`; hit test over the
   composed shapes picks the topmost shape at known points.
4. Errors: missing file, directory, >8 MiB, invalid UTF-8/XML, `filter`/`mask`/
   `text` fixture each yield the typed error or disclosure from AC scope 4,
   asserted by test; no panic.
5. CLI arg and menu-open both call one `open_path` function; test covers the
   function with temp files; arg parsing keeps `--mcp-preview` working.
6. Window-free: all new tests run without a display. `cargo test -p vexel-app`
   existing tests still pass (count may only increase).
7. No `unsafe` and `cargo tree -p vexel-app` shows no new external package.
8. `result.md` (<=400 words) lists what is verified, what is unexecuted
   (live window, Retina), and the disclosure behaviour.

If an AC cannot be met, stop with the exact blocker; do not weaken tests.

## Verification (frozen; repo-local CARGO_HOME, CARGO_TARGET_DIR, locked, offline)
- `cargo +stable check -p vexel-app --locked --offline`
- `cargo +stable test -p vexel-app --locked --offline`
- `cargo +stable fmt --manifest-path Cargo.toml --package vexel-app -- --check`
- `python3 scripts/wo_validate.py WO-RUST-V1-APP-COMPOSE-01 --ready`
- `python3 scripts/ledger.py validate ledger.jsonl`
- `git diff --check`
No clippy gate. Record exact commands, exit codes, test counts.

## Evidence
`artifacts/WO-RUST-V1-APP-COMPOSE-01/`: `result.md`, `verification.json`,
`ac-map.json` (AC to test names), `changed-path-audit.json`.

## Worker and review
Worker: gpt-6-luna High, Standard speed, one writer, existing allowance only; stop
on quota errors, no retry loop. Output is a candidate. Acceptance needs a fresh
read-only reviewer (separate session; Luna or Sol Medium; given only diff, order
and fixtures) who reruns the frozen commands and checks each AC against tests.
The coordinator then integrates the exact patch, reruns main-tree checks, and
performs the live-window open check on an isolated desktop session before
updating STATE.md, LEDGER.md and ledger.jsonl.
