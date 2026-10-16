---
id: WO-RUST-SVG-TEXT-01
revision: 1
status: READY
milestone: V1-SVG-COMPOSITION
owner: project-owner
executor: gpt-6-luna
base_sha: 842d6ce0373b6da3adf9799a6783da01d326efb5
spec_sha256: 14699238282a07968381a27ff94a11b9ed5ea9ee9bb5b0aa9450833e1757fb41
parallel_safe: true
network: false
---
# text layout via CoreText and text rendering
## Goal
Render SVG `<text>`/`<tspan>` by obtaining glyph outlines from CoreText inside
`vexel-platform-apple` (macOS only, behind the platform boundary) and filling
them through `vexel-render`, with parity against the legacy C text output. Today
the engine returns `Unsupported` for text (`decode_text`) and `vexel-render`
reports `UnsupportedKind::Text`. Umbrella: opening documents that contain this feature in `vexel-app` without the
deferral error is NOT part of this order. It is the separate follow-up
`WO-RUST-SVG-WIRE-01` (depends on MASK, PATTERN, FILTER and TEXT). Text selection and editing are out
of scope.

## Dependencies
Read first: `HANDOFF.md`, `vexel-scene/src/media.rs` (`TextRequest`,
`TextLayout`, `TextGlyph`, `TextCommand`, `TextState`),
`vexel-svg-engine/src/lib.rs` (`decode_text`), `vexel-platform-apple/src/
{lib,native_boundary}.rs` and `native_window/text.rs` (existing CoreText/IME
code is for editing UI, not for outlines; reuse its safe-boundary conventions),
`vexel-render/{src,tests}`, and the legacy C
`git show legacy-c-final:src/platform/media/platform_text.m` (+ `.h`; the
CoreText provider), `...:src/app/media_providers.c` (`app_text_layout`),
`...:src/svg/shape/text.c` and `text_cache.c` (positioning, anchors, `x/y/dx/dy`
lists, whitespace, spacing, `outline_context`).

## Assumptions
- Layering: `vexel-platform-apple` exposes a safe `text_layout(&TextRequest)
  -> Result<TextLayout, TextLayoutError>` (outlines as `TextCommand` Move/Line/
  Cubic/Close in font units with per-glyph advance and metrics, as the C
  provider). The engine receives it through a provider trait object defined in
  `vexel-svg-engine` (default `None` keeps today's `Unsupported`), so the engine
  never depends on the platform crate; injecting the provider is
  WO-RUST-SVG-WIRE-01. Quadratic CoreText segments are elevated to cubics
  exactly as the C does.
- Glyph placement (anchor start/middle/end, `x/y/dx/dy` per-glyph lists,
  `letter-spacing`, `word-spacing`, `xml:space`, unit scale) is ported from
  `text.c`; limits (`SVG_SHAPE_MEDIA_MAX_TEXT_BYTES`,
  `..._MAX_DOCUMENT_TEXT_BYTES`, `..._MAX_DOCUMENT_GLYPHS`, `..._MAX_FAMILIES`)
  are mirrored with typed `Limit` errors.
- Source preservation: decoding text never rewrites source bytes; offsets of the
  `<text>` element stay stable and a text-bearing document saved through the
  existing path is byte-identical. Layout results are derived caches only.
- Goldens depend on installed fonts and the macOS version. Fixtures use only
  fonts present on every supported macOS (`Helvetica`, `Times`, `Menlo`) and the
  manifest records macOS and font versions. The test re-derives nothing: it
  compares to the recorded C PPMs, and a font-availability precondition failure
  is reported as an explicit skip with the reason, never a pass.
- `unsafe` may be used only inside `vexel-platform-apple` text code in the
  existing audited style, one `// SAFETY:` comment per block, all listed in the
  audit; `vexel-render`, `vexel-scene` and `vexel-svg-engine` stay `unsafe`-free.

## Scope
1. `vexel-platform-apple/src/text_layout.rs` (new, <= 800 lines): font
   descriptor from family list/weight/style, `CTLine`/`CTRun` glyph walk, glyph
   outlines via `CGPath` apply, guarded by `guard_native_call`, `cfg(target_os =
   "macos")`; main-thread requirement documented. Existing crate features
   (`objc2-core-text`, `objc2-core-graphics` `CGPath`) are already enabled; a new
   feature flag needs coordinator action (stop and report).
2. `vexel-svg-engine`: `TextLayoutProvider` trait and `decode_text` that calls it
   when present, else `Unsupported` as now; scene text shape publication as the C
   (`text_geometry`) using existing `vexel-scene` APIs.
3. `vexel-render/src/text.rs` (new): convert published text outlines into
   `RenderShape` edges (cubic flattening as the C) and fill nonzero; fill colour
   and opacity from the shape; `Unsupported(Text)` remains for stroke on text and
   `textPath`.
4. Fixtures `rust/fixtures/render-text/` with C goldens: single-line Helvetica,
   anchor middle/end, per-glyph `x`/`dx`, `letter-spacing`, `tspan` colour
   change, bold and italic, one with `transform`, plus the project's existing
   text SVG samples if any are solid-fill. Golden generation from the frozen legacy binary, outside the repo, in `$TMPDIR`:
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
5. Tests `tests/text_parity.rs` (macOS only, `cfg(target_os = "macos")`) with
   tolerances copied unchanged from `vexel-render/tests/parity.rs` (max channel
delta 1, no pixel over 2, differing fraction <= 0.5%, exact beyond 1 px of any
edge); layout unit tests in the platform crate compare glyph counts, advances
   (within 1e-6 of the values recorded from C) and command streams for fixed
   strings.

## Non-goals
Selection, caret, IME, editing, `textPath`, stroke on text, vertical writing,
bidi beyond what C does, font fallback beyond the C provider, web fonts,
`vexel-app` wiring, non-macOS platforms, performance work.

## Files allowed to touch
- `rust/crates/vexel-platform-apple/src/text_layout.rs` (new), `.../src/lib.rs`
  (additive `mod`/`pub use` only), `.../tests/**` (new files)
- `rust/crates/vexel-svg-engine/src/lib.rs` (`decode_text` and the provider
  trait only), `.../tests/**` (new files)
- `rust/crates/vexel-render/src/text.rs` (new), `.../src/lib.rs` (additive
  lines only), `.../tests/**` (new files)
- `rust/fixtures/render-text/**` (new)
- `artifacts/WO-RUST-SVG-TEXT-01/**`

## Protected paths
Workspace/lock manifests and every `Cargo.toml` (coordinator only),
`vexel-app`, `vexel-protocol`, all crates not listed above, existing goldens and
`tests/golden/**`, `rust/fixtures/legacy/**`, `rust/fixtures/v1/**`, C/ObjC,
`tests/`, oracles, `docs/**`, `STATE.md`, `LEDGER*.md`, `ledger.jsonl`, other
work orders, git HEAD/index. No new dependencies, network (the legacy tag is
local), installs, GUI launch, commit, stage, reset or deletion. Temp worktree
files stay outside the repository. Files under 800 lines. Also protected: the rest of `vexel-platform-apple` (windowing, IME,
panels), `vexel-scene`, `vexel-svg-decode`, the rest of `vexel-svg-engine`.

## Acceptance criteria
1. Each fixture renders within the tolerances against its C golden; the report
   prints max delta, differing count and outside-dilation count per fixture.
2. Layout unit tests: empty string, single glyph, ligature-free ASCII, combining
   mark, missing font family falls back as the C does, size 0 and negative size
   return typed errors, over-limit text and glyph counts return the typed limit
   error, no panic on invalid UTF-8 (typed error).
3. Engine tests: with a stub provider, `<text>` decodes, per-glyph `x/dx` lists
   and anchors place glyphs as the ported C math expects, and source bytes
   around the element are byte-identical; with no provider the result is the
   current typed `Unsupported` (existing tests unchanged).
4. Source preservation: a fixture with `<text>`, a foreign-namespace attribute
   and editor metadata decodes with element offsets unchanged (asserted by
   byte-range compare).
5. All native calls go through `guard_native_call`; an injected panic in the
   provider surfaces as a typed error, not an unwind (test).
6. All pre-existing tests of the three crates pass unmodified, counts only
   increase; no existing golden changes.
7. Fixture `manifest.json` records tag commit, build commands, binary sha256,
   macOS and font versions and per-file hashes; a test verifies each
   `ppm_sha256`.
8. `cargo tree` shows no new package; `unsafe` limited to the platform text
   file with `// SAFETY:` comments, counted in the audit.
9. `result.md` (<=300 words) states provider layering, mirrored C constants, any
   font-dependence caveat, and that app wiring and live use are unexecuted.

If an AC cannot be met, stop with the exact blocker; no tolerance raising, no
test weakening.

## Verification (frozen; repo-local CARGO_HOME/CARGO_TARGET_DIR, locked, offline)
- `cargo +stable check -p vexel-platform-apple -p vexel-svg-engine -p vexel-render --locked --offline`
- `cargo +stable test -p vexel-platform-apple --locked --offline`
- `cargo +stable test -p vexel-svg-engine --locked --offline`
- `cargo +stable test -p vexel-render --locked --offline`
- `cargo +stable test -p vexel-app --locked --offline` (callers still compile)
- `python3 scripts/wo_validate.py WO-RUST-SVG-TEXT-01 --ready`
- `python3 scripts/ledger.py validate ledger.jsonl`
- `git diff --check`
`cargo fmt --check` is not frozen for `vexel-render` (pre-existing drift, see
WO-RUST-V1-RELEASE-CI-01); new files must be rustfmt-clean
(`rustfmt --check --edition 2024 <file>`).

## Evidence
`artifacts/WO-RUST-SVG-TEXT-01/`: `result.md`, `verification.json`, `ac-map.json`,
`changed-path-audit.json`, `legacy-build.log` (tail only).

## Worker and review
Worker: gpt-6-luna High, one writer, existing allowance only; stop on quota
errors. Candidate only until a fresh read-only reviewer reruns the frozen
commands and checks each AC. The coordinator integrates the patch (textual
merges in `lib.rs` are the coordinator's), schedules WO-RUST-SVG-WIRE-01, then
updates STATE.md, LEDGER.md and ledger.jsonl.
