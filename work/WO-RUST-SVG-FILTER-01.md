---
id: WO-RUST-SVG-FILTER-01
revision: 1
status: READY
milestone: V1-SVG-COMPOSITION
owner: project-owner
executor: gpt-6-luna
base_sha: 842d6ce0373b6da3adf9799a6783da01d326efb5
spec_sha256: 93aff8ad0e9858a34a28c6f8203c67a3d3bacb70c5ec575472c10a759a313c3b
parallel_safe: true
network: false
---
# SVG filter primitives and the FinishGroup contract fix
## Goal
Port the legacy C filter pipeline (`feGaussianBlur`, `feOffset`, `feFlood`,
`feBlend`, `feComposite`, `feMerge`/`feMergeNode`, `feColorMatrix`,
`feDropShadow`) as a pure RGBA pixel module in `vexel-render`, and fix the
planner/Decoder `FinishGroup` contract mismatch so a referenced filter decodes
end to end. The C rejects `feTurbulence`, `feDisplacementMap`,
`feConvolveMatrix`, `feImage`, lighting and `feMorphology` with typed errors;
Rust keeps them `Unsupported`. Umbrella: opening documents that contain this feature in `vexel-app` without the
deferral error is NOT part of this order. It is the separate follow-up
`WO-RUST-SVG-WIRE-01` (depends on MASK, PATTERN, FILTER and TEXT).

## Dependencies
Read first: `HANDOFF.md`, `artifacts/WO-RUST-V1-DECODE-SUBSET-01/{result.md,
deferred-map.json}`, `vexel-svg-definitions/src/{filter,planner}.rs`
(`FinishGroup(group)` at planner.rs ~691), `vexel-svg-decode/src/
{definition_finish,plan_validation}.rs` (the filter path rejects `FinishGroup`
with `InvalidArgument`), `vexel-scene/src/decode_contract.rs` (`PostOperation`),
`vexel-svg-engine/src/lib.rs` (Filter/Marker guard in `plan_owned_definition`),
`vexel-render/{src,tests}`, and the legacy C
`git show legacy-c-final:src/raster/shape_filter.c` (+ `.h`),
`...:src/svg/shape/filter.c`.

## Assumptions
- Contract mismatch decision: the order must record which side changes. Default:
  the definitions planner keeps `FinishGroup` and `vexel-svg-decode` accepts it
  for filter owners. Because `vexel-svg-decode` is otherwise protected, only
  `definition_finish.rs` and `plan_validation.rs` may be edited, and only the
  filter arm, after a failing test reproduces `InvalidArgument`. If the fix needs
  `vexel-scene`, stop and report the exact gap.
- Filter regions, `filterUnits`, `primitiveUnits`, `in`/`result` wiring, colour
  space (`color-interpolation-filters`), blur kernel (box passes vs true
  Gaussian), edge handling and rounding are taken from the C code.
- Pure pixel module: `apply_filter_chain(layer: &mut [u8], size, region, chain,
  ..)` operates on one premultiplied/straight RGBA layer exactly as the C does;
  creating layers and compositing the result into the parent is the group
  machinery of WO-RUST-SVG-MASK-01 and is joined in WO-RUST-SVG-WIRE-01. Tests
  here build their layer from `render_shapes` output.
- Only the Filter arm of the engine guard is lifted; Marker stays deferred.

## Scope
1. `vexel-render/src/filter/` (new module, files <= 800 lines each): primitive
   implementations, chain evaluation, bounded scratch memory, cancellation.
2. Engine: remove `DefinitionKind::Filter` from the fail-closed guard and add a
   decode test for a referenced filter (Marker test unchanged).
3. Decode contract fix per Assumptions with a regression test.
4. Fixtures `rust/fixtures/render-filter/` with C goldens: one fixture per
   primitive plus one chained (blur + offset + merge drop shadow) and one with a
   `filterRes`-less explicit region; start from the legacy fixtures under
   `tests/svg/shape/fixtures` where present. Golden generation from the frozen legacy binary, outside the repo, in `$TMPDIR`:
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
5. Parity test `tests/filter_parity.rs` + `tests/common/filter_data.rs`. Port
   the C arithmetic exactly; tolerance: max channel delta 1 and differing
   fraction <= 0.5% for all primitives except `feGaussianBlur`/`feDropShadow`,
   which allow max delta 2 and no pixel over 3. If C is integer math the target
   is exact equality; report it either way.

## Non-goals
`vexel-app` wiring, group layer allocation, mask/opacity, unsupported primitives
listed above, `filter` functions in CSS (`blur()` etc.) unless the C supports
them, animation of filter attributes, performance work, GPU paths.

## Files allowed to touch
- `rust/crates/vexel-render/src/filter/**`, `rust/crates/vexel-render/src/lib.rs`
  (additive lines only), `rust/crates/vexel-render/tests/**` (new files only)
- `rust/crates/vexel-svg-engine/src/lib.rs` (Filter arm of the guard only),
  `rust/crates/vexel-svg-engine/tests/**` (new files)
- `rust/crates/vexel-svg-decode/src/{definition_finish,plan_validation}.rs` and
  `rust/crates/vexel-svg-definitions/src/{filter,planner}.rs` only per the
  contract decision above (listed in the audit with the failing test)
- `rust/fixtures/render-filter/**` (new)
- `artifacts/WO-RUST-SVG-FILTER-01/**`

## Protected paths
Workspace/lock manifests and every `Cargo.toml` (coordinator only),
`vexel-app`, `vexel-protocol`, all crates not listed above, existing goldens and
`tests/golden/**`, `rust/fixtures/legacy/**`, `rust/fixtures/v1/**`, C/ObjC,
`tests/`, oracles, `docs/**`, `STATE.md`, `LEDGER*.md`, `ledger.jsonl`, other
work orders, git HEAD/index. No new dependencies, network (the legacy tag is
local), installs, GUI launch, commit, stage, reset or deletion. Temp worktree
files stay outside the repository. Files under 800 lines. Also protected: `vexel-scene`, the rest of `vexel-svg-decode`,
`vexel-svg-engine` and `vexel-svg-definitions`.

## Acceptance criteria
1. Each fixture passes its parity bound; the report prints max delta, differing
   count and outside-dilation count per fixture and whether equality was exact.
2. Unit tests per primitive: blur radius 0 is identity, blur of a single pixel
   sums to the input energy within C rounding, offset by negative and fractional
   amounts, flood opacity, each `feBlend` mode and `feComposite` operator the C
   supports, `feMerge` order, `feColorMatrix` matrix/saturate/hueRotate/
   luminanceToAlpha, drop shadow, region clipping, unsupported primitive names
   return typed `Unsupported`, oversize region or radius returns `LimitExceeded`,
   cancellation returns `Cancelled`.
3. Contract test: before the fix a referenced filter fails with
   `InvalidArgument`; after, it decodes and the filter owner group publishes.
   Result states which side changed and why.
4. Engine tests: referenced filter decodes without `Unsupported`; a referenced
   marker still returns `Unsupported`; unreferenced filter is preserved. Existing
   engine, decode and definitions tests pass unmodified, counts only increase.
5. All pre-existing `vexel-render` tests pass unmodified; no existing golden
   changes.
6. Fixture `manifest.json` records tag commit, build commands, binary sha256 and
   per-file hashes; a test verifies each `ppm_sha256`.
7. `cargo tree` shows no new package; no `unsafe`.
8. `result.md` (<=300 words) states the contract decision, the C algorithms
   mirrored per primitive, and that app wiring and live use are unexecuted.

If an AC cannot be met, stop with the exact blocker; no tolerance raising, no
test weakening.

## Verification (frozen; repo-local CARGO_HOME/CARGO_TARGET_DIR, locked, offline)
- `cargo +stable check -p vexel-render -p vexel-svg-engine -p vexel-svg-decode --locked --offline`
- `cargo +stable test -p vexel-render --locked --offline`
- `cargo +stable test -p vexel-svg-engine --locked --offline`
- `cargo +stable test -p vexel-svg-decode --locked --offline`
- `cargo +stable test -p vexel-svg-definitions --locked --offline`
- `cargo +stable test -p vexel-app --locked --offline` (callers still compile)
- `python3 scripts/wo_validate.py WO-RUST-SVG-FILTER-01 --ready`
- `python3 scripts/ledger.py validate ledger.jsonl`
- `git diff --check`
`cargo fmt --check` is not frozen for `vexel-render` (pre-existing drift, see
WO-RUST-V1-RELEASE-CI-01); new files must be rustfmt-clean
(`rustfmt --check --edition 2024 <file>`).

## Evidence
`artifacts/WO-RUST-SVG-FILTER-01/`: `result.md`, `verification.json`, `ac-map.json`,
`changed-path-audit.json`, `legacy-build.log` (tail only).

## Worker and review
Worker: gpt-6-luna High, one writer, existing allowance only; stop on quota
errors. Candidate only until a fresh read-only reviewer reruns the frozen
commands and checks each AC. The coordinator integrates the patch (textual
merges in `lib.rs` are the coordinator's), schedules WO-RUST-SVG-WIRE-01, then
updates STATE.md, LEDGER.md and ledger.jsonl.
