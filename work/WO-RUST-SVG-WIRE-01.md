---
id: WO-RUST-SVG-WIRE-01
revision: 1
status: READY
milestone: V1-SVG-COMPOSITION
owner: project-owner
executor: gpt-6-luna
base_sha: 842d6ce0373b6da3adf9799a6783da01d326efb5
spec_sha256: 0fa74a65de6e611629b1bb19adc07bf8cc4837424cd7b66bddcb644eab98360a
parallel_safe: false
network: false
---
# wire mask, pattern, filter and text into vexel-app
## Goal
Open and render documents containing mask, group opacity/clip groups, pattern
paint, filters and text in `vexel-app` without the deferral error, by routing
compose through the new `vexel-render` entry points and updating the unsupported
fixtures' expectations deliberately. Final integration of the four renderer
orders; no new rendering algorithm here.

## Dependencies (blocking)
`WO-RUST-SVG-MASK-01`, `WO-RUST-SVG-PATTERN-01`, `WO-RUST-SVG-FILTER-01` and
`WO-RUST-SVG-TEXT-01` must be ACCEPTED and integrated on main. If any is absent
at start, stop and report; do not vendor or re-implement it. If one of them is
absent by decision, this order may be re-issued with that feature listed as still
deferred. Read first: `rust/crates/vexel-app/src/compose/{decode,error,shapes,
mod}.rs`, `.../src/animation/frame.rs`, `rust/fixtures/v1/compose/` (the
`unsupported-{filter,mask,text}.svg` and `deferred-paint.svg` fixtures) and the
tests that reference them, plus the four orders' `result.md`.

## Assumptions
- The compose-level rejection of scene-level features (the `Unsupported` mapping
  in `compose/error.rs`/`shapes.rs` via `UnsupportedKind`) is lifted per feature
  only where the matching renderer exists; remaining deferred kinds (gradients,
  images, dashes unless WO-RUST-V1-RENDER-DASH-01 is integrated, `textPath`,
  unsupported filter primitives, markers) still produce the typed disclosure.
- Pattern: compose completes `NeedPattern` suspensions by rendering the tile via
  the PATTERN entry point and resuming the decoder. Filter: compose allocates the
  group layer with the MASK group machinery and applies the FILTER chain before
  compositing. Text: compose injects the `vexel-platform-apple` layout provider
  into the engine (macOS only; on other targets text stays `Unsupported`).
- Expectations in unsupported-* fixtures are changed deliberately: each changed
  assertion is listed in the audit with before/after and the reason; fixtures
  that must still fail (unsupported filter primitive, `textPath`) are added as new
  fixtures rather than weakening old ones.

## Scope
1. `vexel-app` compose and animation frame paths call the new render entry
   points; the window-free `compose_svg` core stays pure and testable without a
   display.
2. New fixtures under `rust/fixtures/v1/wire/` (new files only): one document per
   feature plus one combining mask+filter+pattern+text, each with a C-made golden
   generated as in the four orders (tag `legacy-c-final`, temp worktree,
   `svgworkbench --render-animation`, composited over white; record versions and
   hashes in a manifest; stop if the legacy build fails).
3. Disclosure list and status text updated: rendered features no longer appear as
   deferred; still-deferred features still do.
4. Source preservation: opening and saving a feature-bearing document through the
   existing project path keeps the source bytes identical.

## Non-goals
New render algorithms, editing these features, selection/hit-testing changes,
Retina fidelity, performance targets, live-window verification by the worker,
MCP tool changes, packaging.

## Files allowed to touch
- `rust/crates/vexel-app/src/**`, `rust/crates/vexel-app/tests/**`,
  `rust/crates/vexel-app/README.md`
- `rust/fixtures/v1/compose/**` (expectation updates only, each listed) and
  `rust/fixtures/v1/wire/**` (new)
- `artifacts/WO-RUST-SVG-WIRE-01/**`

## Protected paths
Workspace/lock manifests and every `Cargo.toml` (coordinator pre-adds any path
dependency, e.g. `vexel-platform-apple` features, before start), every crate
except `vexel-app`, `vexel-protocol` and all other crates' fixtures, existing
goldens, C/ObjC, `tests/`, oracles, `docs/**`, `STATE.md`, `LEDGER*.md`,
`ledger.jsonl`, other work orders, git HEAD/index. No new dependencies, network,
installs, GUI launch, commit, stage, reset or deletion. Files under 800 lines.

## Acceptance criteria
1. Each wire fixture composes without `Unsupported` and matches its C golden at
   the tolerances of the corresponding renderer order; per-fixture max delta,
   differing count and outside-dilation count are reported.
2. `unsupported-mask.svg`, `unsupported-filter.svg`, `unsupported-text.svg` and
   `deferred-paint.svg` (pattern part) now compose; their test expectations are
   updated one by one with the audit before/after list; no other assertion is
   weakened.
3. Still-deferred features (an unsupported filter primitive, `textPath`, gradient
   paint, image) still return the typed disclosure; each has a test.
4. Pure core: all new tests run without a display; existing `vexel-app` tests
   not named in AC2 pass unmodified, counts only increase.
5. Cancellation and stale-frame behaviour of compose is preserved (existing
   stale-frame tests pass) and layer/tile memory stays bounded by the renderers'
   limits (a test with an oversize filter region returns the typed error).
6. Source preservation: open then save of a mask/filter/text document keeps the
   source bytes identical (byte compare).
7. `cargo tree -p vexel-app` shows no new external package; no `unsafe`.
8. `result.md` (<=400 words) lists each lifted deferral, each still-deferred
   feature, the expectation changes, and what is unexecuted (live window,
   Retina, MCP).

If an AC cannot be met, stop with the exact blocker; no tolerance raising, no
test weakening.

## Verification (frozen; repo-local CARGO_HOME/CARGO_TARGET_DIR, locked, offline)
- `cargo +stable check -p vexel-app --locked --offline`
- `cargo +stable test -p vexel-app --locked --offline`
- `cargo +stable test -p vexel-render --locked --offline`
- `cargo +stable fmt --manifest-path Cargo.toml --package vexel-app -- --check`
- `python3 scripts/wo_validate.py WO-RUST-SVG-WIRE-01 --ready`
- `python3 scripts/ledger.py validate ledger.jsonl`
- `git diff --check`

## Evidence
`artifacts/WO-RUST-SVG-WIRE-01/`: `result.md`, `verification.json`,
`ac-map.json`, `changed-path-audit.json` (with the fixture-expectation table),
`legacy-build.log` (tail only).

## Worker and review
Worker: gpt-6-luna High, one writer, existing allowance only; stop on quota
errors. Candidate only until a fresh read-only reviewer reruns the frozen
commands and checks each AC. The coordinator integrates the exact patch, reruns
main-tree checks, does the live open of each feature fixture in an isolated
session, then updates STATE.md, LEDGER.md and ledger.jsonl.
