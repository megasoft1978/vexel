---
id: WO-RUST-SVG-COMPAT-01
revision: 1
status: READY
milestone: V1-SVG-COMPOSITION
owner: project-owner
executor: gpt-6-luna
base_sha: 115534ebad1b059d2668e66137df9119cc325ba5
spec_sha256: 469bfcad530d3d934679989a4a3c1607f4f19c28b5345275f4041428f49a8952
parallel_safe: false
network: false
---
# SVG compatibility: namespaced attributes, separator-free transforms, symbol overflow and default viewport

## Goal
Remove three fail-closed rejections of browser-accepted SVG that block owner-selected external
corpus files, without weakening any untrusted-input guard. Measured diagnosis (read-only; sites
approximate, the worker confirms each):

1. Namespace-prefixed attributes. Inkscape writes `<circle sodipodi:cx=".." sodipodi:r=".." sodipodi:fill=".."/>`.
   `vexel-svg-reference/src/lib.rs` ~367 (`attributes_supported_inner`): a prefixed attribute
   whose LOCAL name matches `COMMON` or `shape_attribute` returns `false`. Same class in
   `vexel-svg-decode/src/ignored.rs` ~240 (`if a.prefix.length != 0 { return Ok(false) }`).
   Verified experimentally: fixing this turns `commons-bulgaria-in-1915-geographic-map-en-svg` into ok.
2. Transform list without separator. `transform="translate(a b)scale(c d)"` (matplotlib output,
   accepted by browsers) is rejected at `vexel-svg-decode/src/transform.rs` ~143:
   `else if r.index < r.end && !spaced { return Err(fail(span)) }`.
3. `<symbol overflow="visible">` is not in the symbol allowlist (`vexel-svg-values/src/viewport.rs`
   ~416-439, `ALLOWED` has 22 entries, no `overflow`). Also `parse_symbol_mapping` (~369-376)
   requires `viewBox` on the symbol and `width`/`height` on the `<use>`; SVG defaults are
   identity mapping without viewBox and 100% for missing use width/height.
   `commons-france-osm-mapnik-svg` needs fix 3 and possibly others; the worker verifies.

## Out of scope (refuse if needed to pass ACs)
Stroked text; DOCTYPE entities; root size above 4096; edges and text-command budgets; any
new SVG feature beyond the three items; renderer or raster algorithm change; `vexel-app` change.

## Dependencies
- Base is `115534eb` (HEAD). WO-RUST-LIMITS-03 accepted in `ledger.jsonl` (line ~2074). Verify with
  `python3 scripts/ledger.py summary ledger.jsonl`. If not accepted, stop.
- External corpus: `~/Desktop/Freelance/vexel-corpus-real/manifest.json` (read only). If absent, stop and report.
- Base sweep verdict: 18/39 ok (commit `115534eb`, "final corpus numbers (18/39 ok)").
- Read first: `rust/crates/vexel-svg-reference/src/lib.rs` (186-260, 333-420),
  `rust/crates/vexel-svg-decode/src/ignored.rs` (125-262), `transform.rs` (14-160),
  `rust/crates/vexel-svg-values/src/viewport.rs` (340-450), `rust/crates/vexel-svg-decode/src/use_expansion.rs`
  (call sites of `parse_symbol_mapping` / `symbol_attributes_supported`), and the engine's symbol clip path.

## Fix contracts
**Fix 1, namespaced attributes.** Resolve each attribute prefix against the in-scope `xmlns:*`
bindings. A prefix bound to a non-SVG namespace is ignored for rendering: never matched to an SVG
attribute of the same local name, and preserved byte-for-byte in the source via the existing
unknown-source sink (`IgnoredKind`). Keep current behavior (reject) for: prefix bound to the SVG
namespace (including via a redeclared `xmlns:sodipodi="http://www.w3.org/2000/svg"`), unbound
prefixes, `xlink:href` on image/a, `xml:space` on text/tspan, and `xmlns` declarations. Stop and
report if the sink cannot preserve bytes without a format change.

**Fix 2, transform separator.** After a closing paren, accept zero separator when the next byte
starts an ASCII letter (a transform function name). Whitespace and comma variants stay accepted.
Keep every existing negative case, including garbage between functions. The function-name match
(`_ => fail`) still decides validity, so unknown names are still rejected.

**Fix 3, symbol.** (a) Add `overflow` to `ALLOWED`. Clipping per SVG 2: `hidden` and `scroll`
clip; `visible` and `auto` do not; invalid values are rejected. The symbol default (no overflow)
clips. If the engine cannot skip the clip, add the minimal no-clip path in `vexel-svg-engine`
(allowed) or disclose in the artifact that `visible` is clipped. (b) Symbol without `viewBox`:
identity mapping, viewport origin at use x/y. (c) `<use>` without width/height on a symbol:
default 100% of the viewport. Values still pass the existing finite and affine validity checks.

## Failing-first and pins
- Write each new test first, run red against base, record the red output, then fix to green.
- Existing tests that encode an old rejection (grep the crates for `sodipodi`, `transform`,
  `overflow`, symbol without viewBox) move deliberately. Each move is listed in the evidence file
  with old and new expectation. No test is deleted, weakened or `#[ignore]`d.

## Scope
1. Positive tests: prefixed Inkscape-style circle renders as plain circle; `translate(a b)scale(c d)`
   equals the two-step composition; `<symbol overflow="visible">` and `hidden` both render; symbol
   without viewBox maps by identity; use without width/height uses 100%.
2. Adversarial tests (each must pass, numbers recorded):
   - prefix bound to SVG namespace or redeclared `xmlns:sodipodi` to SVG still rejected;
   - unbound prefix with a shape local name still rejected;
   - `translate(1)5`, `translate(1)-`, `translate(1)(`, `translate(1)xyz(`, `translate(1)scale` (unclosed) rejected;
   - chain of 65 functions with no separator refused by the existing 64-operation cap;
   - `overflow="foo"` rejected; `overflow` on non-symbol elements unchanged;
   - huge use width (e.g. 1e308) refused by the existing validity check, not overflowed.
3. Byte-identical renders: `rust/crates/vexel-app/tests/pipeline_hashes.rs` passes with no hash edited.
   In-repo `bench/corpus/manifest.json` 24/24 ok.
4. External sweep, release, 128x128, same flags as WO-RUST-LIMITS-03:
   base from `git archive 115534eb` into `/private/tmp/svgcompat01-base`; after from the main tree.
   Record verdict counts and per-file transitions only.

## Non-goals
Pipeline speed; limit changes; any corpus bytes in the repo; any golden, oracle or hash edit;
any commit, stage, reset or deletion; `ledger.jsonl`, `STATE.md`, `LEDGER*.md` edits.

## Acceptance criteria
1. Red run recorded before each fix; green after. Every new test named in the evidence file.
2. Adversarial cases in Scope 2 pass with recorded inputs and outcomes.
3. No base-ok external file becomes non-ok. `commons-bulgaria-in-1915-geographic-map-en-svg` becomes ok.
   `commons-france-osm-mapnik-svg` becomes ok only if all its blockers clear; otherwise record the
   exact remaining blocker. `chua` and `gaussian` may move to `limit_decode`; record each.
   Final ok count >= 19 of 39.
4. `pipeline_hashes.rs` passes unedited; in-repo corpus 24/24 ok.
5. `cargo test --offline --locked --no-fail-fast --workspace` has no `test result: FAILED`; final
   `cargo test:` summary line recorded; total test count >= base count.
6. `scripts/check_rust_fmt.py` no worse than base; new files clean. `cargo tree -p vexel-app`
   unchanged. No `unsafe` in the diff. `git diff --check` clean.
7. `artifacts/WO-RUST-SVG-COMPAT-01/svg-compat.md` (<= 300 words): per-fix change, tests added,
   pins moved, sweep before/after counts and transitions, remaining blockers per file.

If an AC cannot be met, stop with the exact blocker. No test weakening; no threshold moved.

## Verification (frozen; repo-local CARGO_HOME/CARGO_TARGET_DIR, locked, offline)
- `cargo +stable test -p vexel-svg-reference -p vexel-svg-decode -p vexel-svg-values -p vexel-svg-engine --locked --offline`
- `cargo test --offline --locked --no-fail-fast --workspace > artifacts/WO-RUST-SVG-COMPAT-01/workspace.log 2>&1; grep -n "test result: FAILED" artifacts/WO-RUST-SVG-COMPAT-01/workspace.log; grep "^cargo test:" artifacts/WO-RUST-SVG-COMPAT-01/workspace.log` (grep for FAILED must be empty)
- `python3 scripts/check_rust_fmt.py`
- `cargo +stable run -p vexel-corpus --release --locked --offline -- run --manifest bench/corpus/manifest.json --pins rust/tools/vexel-corpus/pins.json --out artifacts/WO-RUST-SVG-COMPAT-01/in-repo`
- `cargo +stable run -p vexel-corpus --release --locked --offline -- run --manifest ~/Desktop/Freelance/vexel-corpus-real/manifest.json --pins rust/tools/vexel-corpus/pins.json --out /private/tmp/svgcompat01-after`
  (base run from `git archive 115534eb` at `/private/tmp/svgcompat01-base`; if corpus absent, stop and report)
- `python3 scripts/wo_validate.py --ready WO-RUST-SVG-COMPAT-01`
- `python3 scripts/ledger.py validate ledger.jsonl`
- `git diff --check`

## Files allowed to touch
- `rust/crates/vexel-svg-reference/src/**`, `rust/crates/vexel-svg-reference/tests/**` (new files only)
- `rust/crates/vexel-svg-decode/src/ignored.rs`, `rust/crates/vexel-svg-decode/src/transform.rs`,
  `rust/crates/vexel-svg-decode/tests/**` (new files only; existing test modules only for recorded pin moves)
- `rust/crates/vexel-svg-decode/src/use_expansion.rs` only if the symbol default path requires it (record why)
- `rust/crates/vexel-svg-values/src/viewport.rs`, `rust/crates/vexel-svg-values/tests/**` (new files only)
- `rust/crates/vexel-svg-engine/**` only for the symbol no-clip path, if needed (record why)
- `artifacts/WO-RUST-SVG-COMPAT-01/**`

## Protected paths
Everything else: `vexel-app` (src and tests, `pipeline_hashes.rs` read only), `vexel-raster`,
`vexel-render`, `vexel-scene`, `vexel-svg-css`, `vexel-svg-media`, `vexel-svg-index`,
`vexel-scene-query`, workspace and lock manifests, `rust/tools/**`, `rust/fixtures/**`, goldens,
oracles, `docs/**`, `BLOCKERS.md`, `STATE.md`, `LEDGER*.md`, `ledger.jsonl`, other work orders,
git HEAD and index, the external corpus. No unsafe, no new deps, no network. New files under 800 lines.

## Stop rules
- Any base-ok external file becomes non-ok: stop and report the file and offset.
- Fix 1 needs a format change to preserve bytes: stop and report.
- Corpus absent or base sweep not reproducing 18/39: stop and report.

## Worker and review
Worker: gpt-6-luna High, one writer, parallel_safe false. Candidate only until a fresh read-only
reviewer (separate session, diff and this order only) reruns the frozen commands and checks each AC.
The coordinator integrates after the dependencies above, reruns main-tree checks, then updates
`STATE.md`, `LEDGER.md` and `ledger.jsonl`.
