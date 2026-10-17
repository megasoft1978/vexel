# WO-RUST-SVG-COMPAT-01 summary

## Fix 1: namespace-prefixed attributes
- `vexel-svg-reference`: new `attributes_supported_with(..., foreign)`. A prefixed common/shape attribute is ignored only when the caller reports its prefix as foreign. Unbound prefixes, SVG-bound prefixes, and `xlink:href` are still rejected.
- `vexel-svg-decode/src/ignored.rs`: resolves each prefix to its nearest in-scope `xmlns:prefix` (element first, then ancestors by depth). Work is charged per token visited. An empty or entity-bearing URI is treated as not foreign, so it fails closed.
- A foreign prefix on a common/shape name is noted once as UnknownAttribute. Source bytes are unchanged.
- Tests: `vexel-svg-decode/tests/svg_compat01.rs` (13 Fix 1 tests), `vexel-svg-reference/tests/svg_compat01.rs` (3 tests).

## Fix 2: separator-free transforms
- `vexel-svg-decode/src/transform.rs`: a zero separator is accepted only when the next byte is an ASCII letter. The function-name match still decides validity.
- Tests: 3 positive, 1 adversarial list (10 inputs), and the 65-function cap.

## Fix 3: symbol
- `vexel-svg-values/src/viewport.rs`: `overflow` added to the symbol allowlist. Values are limited to visible, hidden, scroll, auto. Anything else is rejected.
- No viewBox: identity mapping at the use x/y. Missing use width/height default to 100% of the viewport.
- `vexel-svg-decode/src/traversal.rs`: the symbol viewBox is optional. Without one the inherited viewport is kept (approximation).
- Gap: the engine never clips symbols, before or after this change. `hidden` and default render unclipped, the same as the base default. This is not implemented.
- Tests: 4 positive overflow values, 4 invalid values, non-symbol overflow unchanged, identity mapping, default size, huge width refused.

## Pins
- No existing test was changed, moved, or ignored.

## Sweep (external, 39 files, 128x128, release)
- Base 18 ok, after 19 ok. No base-ok file regressed.
- commons-bulgaria-in-1915-geographic-map-en-svg: unsupported -> ok
- commons-france-osm-mapnik-svg: unsupported -> limit_render (remaining blocker: render safety limit)
- commons-chuaattractormodified-svg: unsupported -> limit_decode
- commons-gaussian-2d-svg: unsupported -> limit_decode
- In-repo corpus: 24/24 ok.

## Verification
- Frozen per-crate test command: all pass.
- decode svg_compat01: 12 failed and 9 passed before the fixes (red). After: 22 passed.
- Workspace: 2006 passed, 5 ignored, 0 FAILED. Base: 1981 passed, 5 ignored.
- check_rust_fmt: pass. git diff --check: clean.
- pipeline_hashes.rs: passes unedited, part of the workspace run.
- Reference-crate tests were written after the fix, not red first.
- cargo fmt ran after the test runs. It was formatting-only and touched the new code, and I reverted its unrelated drift.
