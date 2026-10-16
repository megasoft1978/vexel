---
id: WO-RUST-SVG-MASK-01
revision: 1
status: READY
milestone: V1-SVG-COMPOSITION
owner: project-owner
executor: gpt-6-luna
base_sha: 842d6ce0373b6da3adf9799a6783da01d326efb5
spec_sha256: be5667a42a070d2cdd672ab8b7da3a67df66ef82635a2ed89c77822b494ccc55
parallel_safe: true
network: false
---
# mask, opacity groups and clip groups in vexel-render
## Goal
Composite scene groups in `vexel-render`: group `opacity`, group `clip-path`
(clip group) and `mask` on groups, with pixel parity against the legacy C
renderer, and unblock mask on groups in the decode path. Today
`UnsupportedKind::{Mask, OpacityGroup, Clip}` is returned and the engine
returns `Unsupported` for a referenced mask (WO-RUST-V1-DECODE-SUBSET-01
`deferred-map.json`: the planner reports offset 0 for group owners; the engine
remaps it to the `<mask>` element). Umbrella: opening documents that contain this feature in `vexel-app` without the
deferral error is NOT part of this order. It is the separate follow-up
`WO-RUST-SVG-WIRE-01` (depends on MASK, PATTERN, FILTER and TEXT).

## Dependencies
Read first: `HANDOFF.md`, `artifacts/WO-RUST-V1-DECODE-SUBSET-01/{result.md,
deferred-map.json}` (commit 8ea3e78b), `vexel-render/{src,tests}`
(`render.rs`, `adapter.rs`, `composite.rs`, `tests/parity.rs`),
`vexel-scene` (`mask_post.rs` `apply_mask_post`/`MaskPostTarget`, `Group`,
`Mask`, `PostOperation::AttachTargetedMask`), `vexel-svg-engine/src/lib.rs`
(`plan_owned_definition`), `vexel-svg-definitions/src/planner.rs` (Mask arm near
line 329, `FinishGroup` near 691), and the legacy C
`git show legacy-c-final:src/raster/shape_render_group.c`,
`...:src/svg/shape/mask.c`, `...:src/svg/clip/clip.c`,
`...:src/svg/shape/shape_clip_apply.c`.

## Assumptions
- Group compositing follows the C two-pass model: render group children into a
  transparent layer, multiply by opacity, clip/mask coverage, then source-over
  into the parent. Luminance vs alpha mask (`mask-type`), `maskUnits`,
  `maskContentUnits` and clip coverage rules are taken from the C code, not from
  the SVG spec, where they differ.
- Existing `render_shapes`, `from_scene_shape` and `RenderShape` stay
  source- and behaviour-compatible: group rendering is additive
  (`render_groups(...)`, new types) and `from_scene_shape` keeps returning
  `Unsupported(Mask|Clip)` for shapes that carry them.
- The engine change is minimal: in `SvgEngineOperations::plan_owned_definition`
  stop failing closed for `DefinitionKind::Mask` and keep the offset-0 remap for
  the remaining unsupported owners. If publishing a mask needs a change outside
  the listed engine/definitions files, or in `vexel-scene`, stop and report the
  exact gap.
- Mask definitions nested more than the C limit (`SHAPE_MASK_MAX_INSTANCES`
  1024) fail with a typed error mirroring C.

## Scope
1. `vexel-render/src/group.rs` and `mask.rs` (new, each <= 800 lines): layer
   buffers with bounded memory (`LimitExceeded`), opacity, clip group, luminance
   and alpha masks, nested groups; cancellation honoured per row.
2. Additive public entry `render_groups` taking an ordered group/shape tree view
   and the scene mask/clip tables; unsupported leftovers (filter, pattern,
   text) still return their typed `Unsupported`.
3. Engine hook for mask on groups in `vexel-svg-engine/src/lib.rs` (Mask arm
   only; the Filter/Marker guard belongs to WO-RUST-SVG-FILTER-01) plus a
   decode-to-scene test proving a referenced mask publishes a group with
   `mask = Some(..)` and source bytes untouched.
4. Fixtures `rust/fixtures/render-mask/` with C goldens. SVGs: group opacity,
   nested opacity, clip group over several shapes, luminance mask with gradient
   content is excluded (gradients not rendered yet): use solid-colour mask
   content, alpha `mask-type`, mask with `maskUnits=userSpaceOnUse`, mask on a
   transformed group. Take the matching legacy fixtures from
   `tests/svg/shape/fixtures` via `git show legacy-c-final:<path>` where they
   exist, plus project-authored files. Golden generation from the frozen legacy binary, outside the repo, in `$TMPDIR`:
`mkdir legacy && git archive legacy-c-final | tar -x -C legacy`; `cd legacy &&
cmake --preset mac-debug && cmake --build --preset mac-debug --target
svgworkbench`; per fixture `build/mac-debug/bin/svgworkbench --render-animation
<fixture.svg> --fps 1 --duration 1 --out <dir>`, take `frame-0000.ppm`, composited
over white exactly as `vexel-render/tests/golden/manifest.json` records. Record
`git rev-parse legacy-c-final`, cmake/compiler/macOS versions and the binary
sha256 in the fixture `manifest.json` (fixture id, source sha256, ppm sha256,
size, exact command). If the legacy build or the headless render fails (a window
opens, a missing feature), stop and report; never hand-edit or Rust-generate a
golden.
5. Parity test `tests/group_parity.rs` + `tests/common/group_data.rs`
   (hand-transcribed shape data in the style of `parity_data.rs`), tolerances copied unchanged from `vexel-render/tests/parity.rs` (max channel
delta 1, no pixel over 2, differing fraction <= 0.5%, exact beyond 1 px of any
edge).

## Non-goals
`vexel-app` wiring, filters, patterns, text, gradients in masks, `use`-instanced
masks beyond the listed fixtures, animation, performance work, GPU paths.

## Files allowed to touch
- `rust/crates/vexel-render/src/**`, `rust/crates/vexel-render/tests/**` (new
  files only, existing tests and goldens unchanged; `lib.rs` additive lines
  only)
- `rust/crates/vexel-svg-engine/src/lib.rs` (Mask arm of `plan_owned_definition`
  only), `rust/crates/vexel-svg-engine/tests/**` (new files)
- `rust/crates/vexel-svg-definitions/src/planner.rs` only if a failing test
  proves the mask owner offset or post operation must change (listed in the
  audit)
- `rust/fixtures/render-mask/**` (new)
- `artifacts/WO-RUST-SVG-MASK-01/**`

## Protected paths
Workspace/lock manifests and every `Cargo.toml` (coordinator only),
`vexel-app`, `vexel-protocol`, all crates not listed above, existing goldens and
`tests/golden/**`, `rust/fixtures/legacy/**`, `rust/fixtures/v1/**`, C/ObjC,
`tests/`, oracles, `docs/**`, `STATE.md`, `LEDGER*.md`, `ledger.jsonl`, other
work orders, git HEAD/index. No new dependencies, network (the legacy tag is
local), installs, GUI launch, commit, stage, reset or deletion. Temp worktree
files stay outside the repository. Files under 800 lines. Also protected: `vexel-scene`, `vexel-svg-decode`, and the rest of
`vexel-svg-engine`/`vexel-svg-definitions`.

## Acceptance criteria
1. Each fixture renders through `render_groups` within the tolerances against
   its C golden; the report prints max delta, differing count and
   outside-dilation count per fixture.
2. Unit tests: opacity 0, 0.5 and 1; nested opacity multiplies; clip group
   excludes outside pixels; luminance vs alpha mask; mask with zero-area region
   renders nothing; 1025 mask instances returns the typed limit error;
   cancellation mid-layer returns `Cancelled`.
3. A group with opacity 1, no clip and no mask renders byte-identically to the
   ungrouped `render_shapes` path (asserted on every existing parity case).
4. Engine test: an SVG with a referenced `<mask>` on a group decodes (no
   `Unsupported`) and the scene group carries the mask; unreferenced masks and
   an invalid mask reference still behave as before (existing engine tests pass
   unmodified; counts only increase).
5. `from_scene_shape` still returns `Unsupported` for a mask/clip-carrying shape
   (existing `errors.rs` assertions unchanged and passing).
6. All pre-existing `vexel-render`, `vexel-svg-engine` and
   `vexel-svg-definitions` tests pass unmodified; no existing golden changes.
7. Fixture `manifest.json` records tag commit, build commands, binary sha256 and
   per-file hashes; a test verifies each `ppm_sha256`.
8. `cargo tree` for both crates shows no new package; no `unsafe`.
9. `result.md` (<=300 words) states engine hunks changed, the C constants
   mirrored, and that app wiring and live use are unexecuted.

If an AC cannot be met, stop with the exact blocker; no tolerance raising, no
test weakening.

## Verification (frozen; repo-local CARGO_HOME/CARGO_TARGET_DIR, locked, offline)
- `cargo +stable check -p vexel-render -p vexel-svg-engine --locked --offline`
- `cargo +stable test -p vexel-render --locked --offline`
- `cargo +stable test -p vexel-svg-engine --locked --offline`
- `cargo +stable test -p vexel-svg-definitions --locked --offline`
- `cargo +stable test -p vexel-app --locked --offline` (callers still compile)
- `python3 scripts/wo_validate.py WO-RUST-SVG-MASK-01 --ready`
- `python3 scripts/ledger.py validate ledger.jsonl`
- `git diff --check`
`cargo fmt --check` is not frozen for `vexel-render` (pre-existing drift, see
WO-RUST-V1-RELEASE-CI-01); new files must be rustfmt-clean
(`rustfmt --check --edition 2024 <file>`).

## Evidence
`artifacts/WO-RUST-SVG-MASK-01/`: `result.md`, `verification.json`, `ac-map.json`,
`changed-path-audit.json`, `legacy-build.log` (tail only).

## Worker and review
Worker: gpt-6-luna High, one writer, existing allowance only; stop on quota
errors. Candidate only until a fresh read-only reviewer reruns the frozen
commands and checks each AC. The coordinator integrates the patch (textual
merges in `lib.rs` are the coordinator's), schedules WO-RUST-SVG-WIRE-01, then
updates STATE.md, LEDGER.md and ledger.jsonl.
