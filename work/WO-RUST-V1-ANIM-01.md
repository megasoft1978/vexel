---
id: WO-RUST-V1-ANIM-01
revision: 1
status: READY
milestone: V1-SVG-COMPOSITION
owner: project-owner
executor: gpt-6-luna
base_sha: 8f10553a1a7874c1843ae8161e82c8c2de255351
spec_sha256: adcff821f57e4e36b0c1ebd026574f5c976801e5627d35a58ad0ccc4f7e7880e
parallel_safe: false
network: false
---
# Live animation playback and first-party GIF export

## Goal
Integrate the MCP live-animation candidates, add in-app loop playback via
`vexel-svg-animation` and `vexel-svg-animation-frame`, and add preset GIF export
for the V1 loop (ADR-0029: short keyframed loop, scrub/play, one GIF preset).
No rich animation authoring, onion skin, SMIL breadth or export controls.

## Candidate inputs (read-only reference; byte-audit before use)
`.cache/WO-RUST-MCP-LIVE-ANIMATION-01/candidate-app/`, `candidate-document/`,
`candidate-protocol/` (full-tree copies; compare their `rust/crates/{vexel-app,
vexel-v1-document,vexel-protocol,vexel-mcp-stdio}` against main) and
`integrated-*-target/`. Coordinator seeds `.cache/WO-RUST-V1-ANIM-01/worker/` from
main plus the audited candidate diffs and writes `source-seed.json`
(SHA-256 per file). Worker never edits the original caches or main.

## GIF encoder decision (first-party; no new dependency)
Survey result: Rust has no GIF code. C encodes through ImageIO
(`src/platform/media/platform_animation.m`, `platform_animation_encode_gif`;
limits: <=120 frames, <=60 fps, <=4096 px/dim, <=32M aggregate pixels, <=128 MiB
output; used by `src/mcp/mcp_export_job.c`). The Rust protocol already declares
`export_animation` (gif|frames, fps 1-30). Decision for this order: a new
pure-safe-Rust crate `rust/crates/vexel-gif` (GIF89a, global palette
quantization, variable-code LZW, NETSCAPE2.0 loop, per-frame delay), path deps on
`vexel-base` only. No new external crate; adding any external dependency or
choosing the ImageIO route requires a new ADR cited in this order's revision
(not assumed here). Palette must be deterministic.

## Assumptions
- `vexel-render` (RENDER-01) is integrated for frame rasterization; frames render
  at the 1x offscreen rule. If absent, stop and report.
- Animation timing semantics come from `vexel-svg-animation(-frame)` as accepted;
  anything unsupported by `evaluate_frame` is reported, not approximated.
- GIF quantization is lossy by design; fidelity is judged against a frozen
  reference decode, not source pixels.

## Scope
1. Integrate audited candidate app/document/protocol changes that provide live
   animation tools (`add_animation_fragment`, `animation_control`, preview
   playback state); keep existing tests passing.
2. `vexel-app` playback: play/pause/scrub on a loop clock injected as a trait
   (no wall-clock in tests), frame evaluation through `evaluate_frame`, render via
   `vexel-render`, present via existing path; bounded frame queue.
3. `vexel-gif`: encode N RGBA frames + delays -> bytes with the C limits above
   enforced as typed errors.
4. Export wiring: `export_animation` (format gif) in the app/MCP session writes
   the GIF only inside a granted directory using the existing access crate.

## Non-goals
Frame-sequence export, color-quality tuning, dithering options, audio, video,
timeline editing UI, MCP remote transport, performance targets.

## Files allowed to touch (candidate workspace only)
- `rust/crates/vexel-gif/**` (new)
- `rust/crates/vexel-app/{src,tests,README.md,Cargo.toml}`,
  `rust/crates/vexel-v1-document/{src,tests}`,
  `rust/crates/vexel-protocol/{src,tests}`,
  `rust/crates/vexel-mcp-stdio/{src,tests}`
- `rust/fixtures/v1/anim/**` (new fixtures)
- `artifacts/WO-RUST-V1-ANIM-01/**` (except `source-seed.json`)

## Protected paths
Workspace/lock manifests (coordinator registers `vexel-gif`), all other crates,
C/ObjC sources, `tests/`, goldens, oracles, `docs/**`, `STATE.md`, `LEDGER*.md`,
`ledger.jsonl`, other work orders, the candidate caches, git HEAD/index. No unsafe,
external deps, network, installs, GUI launch, commit, stage, reset, deletion.
Files under 800 lines.

## Acceptance criteria
1. Candidate audit: `source-diff.json` lists every file taken from candidates vs
   main with SHA-256 and reason; all existing tests of touched crates pass.
2. `vexel-gif` structural: output starts `GIF89a`, has logical screen size,
   NETSCAPE2.0 loop block, one graphic-control block per frame with the
   requested delay (centisecond rounding documented), trailer `0x3B`.
3. `vexel-gif` round trip: a first-party test decoder (test-only, in `tests/`)
   decodes the LZW stream; for a <=256-colour frame set decoded indices map back
   to the exact input RGBA; for a many-colour frame max per-channel error is
   within a stated constant. Includes 1x1, single-colour and 255/256/257-colour
   edge cases.
4. Limits: 121 frames, 61 fps, 4097 px, over-aggregate pixels and empty input
   each return the matching typed error before allocation; deterministic bytes
   (two encodes identical); no panic on any input size 0..N property sweep.
5. Playback: with an injected clock, a keyframed position/opacity loop yields
   expected `evaluate_frame` values at t=0, mid, end and wraps at the loop
   boundary; scrub to t and play/pause state are asserted without a window.
6. Export: an animated fixture exports a GIF whose decoded frame count and
   delays equal `duration*fps` per the tool schema, and whose first/last frames
   equal the rendered frames within AC3 tolerance. Path outside the granted
   directory, existing-file-without-overwrite and cancellation are typed errors
   leaving no partial file.
7. Undo/redo still works across animation edits (test: add fragment, undo,
   redo, source bytes equal).
8. `cargo tree` shows no new external package; no `unsafe`.
9. `result.md` (<=400 words): decisions, GIF scope, ImageIO comparison, unexecuted
   items (live playback feel, third-party viewer compatibility).

If an AC cannot be met, stop with the exact blocker; no weakening.

## Verification (frozen; candidate workspace, locked, offline, repo-local cargo dirs)
- `cargo +stable check -p vexel-gif -p vexel-app --locked --offline`
- `cargo +stable test -p vexel-gif --locked --offline`
- `cargo +stable test -p vexel-app -p vexel-v1-document -p vexel-protocol -p vexel-mcp-stdio --locked --offline`
- `cargo +stable fmt --manifest-path Cargo.toml --package vexel-gif --package vexel-app -- --check`
- `python3 scripts/wo_validate.py WO-RUST-V1-ANIM-01 --ready`
- `python3 scripts/ledger.py validate ledger.jsonl`

## Evidence
`artifacts/WO-RUST-V1-ANIM-01/`: `source-seed.json` (coordinator), `result.md`,
`verification.json`, `source-diff.json`, `ac-map.json`, `changed-path-audit.json`.

## Worker and review
Worker: gpt-6-luna High, Standard speed, one writer, existing allowance only; stop
on quota errors. Candidate only until a fresh read-only reviewer (separate
session, Luna or Sol Medium; diff, order and seed only) reruns the frozen
commands, audits paths against the seed and checks every AC. The coordinator then
integrates the exact patch, reruns main-tree checks, opens the exported GIF in a
second independent viewer, and updates STATE.md, LEDGER.md and ledger.jsonl.
