---
id: WO-RUST-MCP-LIVE-ANIMATION-01
revision: 1
status: READY
milestone: MCP-LIVE-ANIMATION
owner: project-owner
executor: gpt-6-luna
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: c7353f87c209ec51434f027464315c7d6231c3289f6af76c8260c13a6eaea15c
parallel_safe: true
network: false
---
# Live MCP animation preview

Extend the bounded live preview so Codex can add an SVG element through MCP,
author bounded SMIL tracks against its stable ID, and watch the animated canvas
update in Vexel while playback controls remain responsive.

## Intent and boundaries

- Preserve the full product goal and existing static preview behavior.
- Use the existing `vexel-svg-animation` parser/evaluator over the exact SVG
  source held by `V1Document`. Do not create a second keyframe evaluator or use
  canned motion.
- This work remains a clearly labeled primitive preview. It is not the
  production arbitrary-SVG decoder/renderer, general animation authoring UI,
  native project timeline, export, or final README/website recording.
- Implement the Pip art direction in `docs/product/mcp-live-preview.md`: paper
  cut silhouette, ink/cobalt/apricot/mint/coral/gold, gentle 3.2-second greeting
  loop (bob, two waves, scarf lag, one blink, staggered rising glints).

## Parallel ownership

Use three isolated Luna candidates, one writer per scope:

1. Protocol: `rust/crates/vexel-protocol/src/**` and
   `rust/crates/vexel-protocol/tests/mcp_preview_profile.rs`. In the preview
   profile expose `animation_add` (existing source-fragment tool narrowed to
   bounded preview SMIL) and a preview-only `animation_control` tool with
   `play`, `pause`, and `seek`. Also expose `animation_list` if its response is
   implemented. Preserve the default/full profile's schemas and behavior.
2. V1 document: `rust/crates/vexel-v1-document/src/**`. Add a bounded method to
   insert one animation fragment under an existing stable-ID shape, using the
   existing edit engine so undo/redo and source bytes remain authoritative.
3. App: `rust/crates/vexel-app/src/**`. Extend preview operations, source
   snapshots/history, track parsing/evaluation, canvas raster, timer, and MCP/UI
   publication ordering. Assume coordinator adds existing local Cargo
   dependencies on `vexel-xml` and `vexel-svg-animation`; do not edit manifests.

The coordinator alone edits workspace/package manifests, lockfiles, this work
order, source seeds/scopes, `STATE.md`, ledgers, and root integration. Workers
must use isolated current-source candidates and return finite diffs/evidence.
Do not spawn nested workers.

## Required behavior

1. Every rectangle/ellipse add has a stable source `id` and returns it in the
   MCP response. IDs remain stable across undo/redo and later edits.
2. `animation_add` inserts one `<animate>` fragment under the named existing
   shape and commits it as one undoable source edit. Preview support is limited
   to numeric `x`, `y`, `cx`, `cy`, and `opacity`; linear/spline interpolation;
   2–16 finite values; 100–10,000 ms duration; no external references; at most
   16 tracks. Reject unsupported or out-of-bound tracks atomically.
3. Playback uses `parse_owned` and `evaluate_view` from the SVG animation crate
   against current `V1Document` source. Evaluated overlays are matched to real
   target IDs and applied to the existing bounded primitive raster. Opacity is
   composited over the preview background. No hardcoded Pip animation data is
   embedded in the app.
4. `animation_control` play/pause/seek updates playback on the UI event loop at
   a bounded cadence (target 30 fps), keeps native/MCP event polling responsive,
   and presents the resulting frame before acknowledging the call. Status shows
   playback state and time. Loop duration is bounded to 10 seconds.
5. Animation insertion, undo, and redo restore source and rendered output
   atomically. Failed tool calls do not change source, history, playback, or
   pixels. Bounds include element/track counts, IDs, animation XML bytes,
   numeric values, and finite time.
6. Deterministic headless tests cover source IDs/track insertion, real parser
   and evaluator use at distinct times, value and target bounds, opacity
   compositing, playback state/time, undo/redo, and the guarantee that each
   successful mutation/control call waits for frame presentation. Add protocol
   tests for preview exposure, validation, and unchanged default profile.
7. Update the live-preview guide with the actual tool flow and a copyable Pip
   drawing/animation prompt. Explicitly label native GUI runtime visual review
   as pending unless performed on an isolated display. Do not run broad UI/E2E
   suites or take over the user's desktop.

## Allowed paths

- `rust/crates/vexel-protocol/src/**`
- `rust/crates/vexel-protocol/tests/mcp_preview_profile.rs`
- `rust/crates/vexel-v1-document/src/**`
- `rust/crates/vexel-app/src/**`
- `rust/crates/vexel-app/README.md`
- `rust/crates/vexel-app/Cargo.toml` (coordinator only)
- `Cargo.toml` and `Cargo.lock` (coordinator only)
- `docs/product/mcp-live-preview.md`
- `STATE.md`
- `artifacts/WO-RUST-MCP-LIVE-ANIMATION-01/**`
- `.cache/WO-RUST-MCP-LIVE-ANIMATION-01/**` for isolated candidates and build output
- `work/WO-RUST-MCP-LIVE-ANIMATION-01.md` (coordinator only)

## Protected paths and limits

Protect all other app/protocol/document crates, accepted source oracles,
fixtures, decisions, full product goal, frozen work orders, ledgers, HEAD, and
index. No dependencies, network, unsupported OS code, broad UI/E2E run, visible
app launch, commit, staging, deletion, reset, or recursive agents. Keep changed
source files below 800 lines. Preserve every candidate and exact verification
evidence.

## Verification commands

```text
CARGO_HOME="$PWD/.cache/cargo" cargo +stable check -p vexel-protocol -p vexel-v1-document -p vexel-app --locked --offline
CARGO_HOME="$PWD/.cache/cargo" cargo +stable test -p vexel-protocol -p vexel-v1-document -p vexel-app --locked --offline
cargo +stable fmt -p vexel-protocol -p vexel-v1-document -p vexel-app -- --check
python3 scripts/wo_validate.py WO-RUST-MCP-LIVE-ANIMATION-01 --ready
python3 scripts/ledger.py validate ledger.jsonl
git diff --check
```

Run relevant focused checks after each change/failure. Record exact outputs and
candidate hashes under `artifacts/WO-RUST-MCP-LIVE-ANIMATION-01/`. Coordinator
integration requires independent review and fresh main-tree offline checks.
No broad UI/E2E is part of this milestone. Keep the full product goal active.
