---
id: WO-RUST-UI-TOKENS-01
revision: 1
status: READY
milestone: UI-MODERN-2026-10
owner: project-owner
executor: gpt-6-luna
base_sha: 76dc7bef0762fac199d292cb96d2e51ba33dbcc3
spec_sha256: 43e9bb48a3f807fbd550586b0ee8a2c5949afc08d395a36c24c745f33d5188ce
parallel_safe: True
network: false
---
# Modern design tokens and theme plumbing in vexel-ui

## Goal
Replace the dark/light colour, type, spacing, radius and motion values in
`vexel-ui` with the token system in section 4 of `docs/ui/design-direction-2026-10.md`: an LCH-derived neutral
ramp with 6 elevation levels, desaturated accent and semantic colours, three text
emphasis levels, 4/8 px spacing, radii, a type scale with a 12 pt floor, and
motion constants. Colours, typography and spacing constants only: no layout,
widget or behaviour change. Evidence of the problem: critique items 4, 9, 10 in
the design doc and `docs/archive/live/2026-10-16-compose-v1-after-live-fixes.png`.

## Dependencies
None (parallel with ICONS). Read first: `docs/ui/design-direction-2026-10.md` section 4;
`rust/crates/vexel-ui/src/tokens.rs` (`Tokens`, `theme_tokens`,
`theme_tokens_accented`, `type_scale`, `design_metrics`, `legacy_tokens`),
`widget_canvas.rs` (token consumers), `vexel-app/src/render.rs`.

## Assumptions
- The doc's sRGB values are script-generated starting points. The order must
  re-derive them from the same LCH parameters in Rust (a `const fn`-free
  deterministic function in a new `theme.rs`, no float dependence beyond
  `std`) and fail loudly if a contrast gate does not hold; change the token,
  never the gate.
- Field names in `Tokens` stay (additive change only; legacy aliases `panel_alt`,
  `canvas`, `text`, `muted`, `border` keep resolving) so `vexel-app` still
  compiles unchanged. New fields: `surface: [u32; 6]`, `text_low`,
  `accent_hover`, `accent_subtle`, `focus_ring`, `motion` constants.
- `design_metrics()` toolbar/rail/inspector numbers change only where the doc
  states them (toolbar 44, rail 44, inspector 296, status 24, section header 28,
  field 28); `vexel-app` layout keeps its own constants until CHROME lands.

## Scope
1. `theme.rs`: LCH(ab, D65) to sRGB generator, `Tokens` population for Dark and
   Light, high-contrast variants (text.low and edges raised until 7:1 / 4.5:1).
2. 6 surface levels per theme, separators and control edges, text emphasis
   tokens, accent/semantic tokens with the chroma cap (50), `accent_subtle`.
3. `theme_tokens_accented`: clamp the system accent to the chroma cap, then
   re-run the existing contrast search against levels 1 to 3 (3:1; 4.5:1 in
   high contrast).
4. `type_scale()`: roles title 15, heading 13, body 13, label 12, meta 12,
   numeric 12 (tabular figures flag); no role under 12; weights 400/500/600.
5. Spacing 2/4/8/12/16/24/32, radii 4/6/8/12, motion constants (instant 80,
   fast 120, base 180, slow 240 ms; easing control points as `[f32; 4]`).
6. Computed contrast report: a test that writes
   `artifacts/WO-RUST-UI-TOKENS-01/contrast-report.json` (every pair checked,
   ratio, threshold, pass).

## Non-goals
Layout, widgets, icons, hover/press logic, AppKit settings window colours,
vibrancy/material, removing legacy fields, C oracles.

## Files allowed to touch
- `rust/crates/vexel-ui/src/{tokens,theme,tests_tokens}.rs` (`theme.rs`,
  `tests_tokens.rs` new), `rust/crates/vexel-ui/src/tests.rs` only to update
  assertions that pin old colour values (list each in the audit)
- `rust/crates/vexel-app/tests/ui_tokens_*.rs` (new)
- `rust/fixtures/ui/tokens/**` (new), `artifacts/WO-RUST-UI-TOKENS-01/**`
- `rust/crates/vexel-ui/src/lib.rs` (`mod`/`pub use` registration lines only)

## Protected paths
Workspace/lock manifests and every `Cargo.toml` (coordinator only), all other
crates, C/ObjC, `tests/`, existing goldens and oracles (new first-party goldens
go only under this order's fixture directory), `docs/**` (the coordinator
records screenshots), `STATE.md`, `LEDGER*.md`, `ledger.jsonl`, other work
orders, git HEAD/index. No unsafe, new external deps, network, installs, GUI
launch by the worker, commit, stage, reset, deletion. Files under 800 lines.

## Acceptance criteria (all headless, no display)
1. Ramp: for Dark and Light, the 6 surface lightness values are strictly
   monotonic in the elevation direction (dark rises, light falls toward the
   edge-plus-shadow rule), each step differs by at least 3 L; none is pure
   black or pure white (RGB 0,0,0 / 255,255,255 rejected for surfaces and text).
2. Contrast, computed with the repo's WCAG formula: text.high and text.medium at
   least 4.5:1 and text.low at least 4.5:1 on surface levels 1 and 2; text at
   18 px or larger, focus ring, accent glyph and control edges at least 3:1 on
   the surface they sit on (control edges only where the doc marks them as
   required indicators; decorative separators are exempt and listed); accent,
   success, warning and error at least 4.5:1 on level 1. Disabled text is exempt
   and listed. Same for high contrast at 7:1 for text.high/medium.
3. Doc parity: generated sRGB for the values printed in section 4 of the design
   doc match within 1 per channel, or the doc table is reported as stale in
   `result.md` (the worker may not edit the doc).
4. Accent adaptation: for 12 sample system accents (including pure red, pure
   yellow, near-white, near-black) `theme_tokens_accented` returns accent with
   chroma at most 50 and passing AC2 on levels 1 to 3 in both themes; a zero
   or alpha-0 accent returns the default token set unchanged.
5. Type and spacing: no type role below 12 pt; every spacing value is a multiple
   of 2 and the scale is strictly increasing; radii strictly increasing.
6. Compatibility: `legacy_tokens`, all legacy `Tokens` fields and every existing
   `vexel-ui`/`vexel-app` test compile and pass, apart from the colour-pin
   updates listed in the audit (each justified in `result.md`).
7. Determinism: two calls return identical values; running the generator in
   debug and release test profiles yields identical `u32`s (assert via a frozen
   hash of all token values for each theme, stored in the test).
8. Golden: a token swatch sheet (6 surfaces, 3 text levels on each, accent,
   semantics, focus ring) rendered with `WidgetCanvas` at 1x and 2x for Dark,
   Light and Dark high-contrast matches first-party goldens under the policy
   below (candidates until reviewed).
9. `cargo tree -p vexel-ui` and `-p vexel-app` show no new external package; no
   `unsafe`. `result.md` lists every changed token with old and new value.

If an AC cannot be met, stop with the exact blocker; no test weakening.

## Golden-image policy (applies to every golden in this order)
- Goldens are first-party: rendered by the Rust renderer (`WidgetCanvas` /
  `vexel-raster` software path), never by C oracles. Test code compares RGBA.
- Compare tolerance, documented in the test file header and `result.md`: software
  rasterised shapes and icon masks match exactly (tolerance 0). Anything that
  draws CoreText glyphs may differ by OS version: per-channel delta at most 4 on
  at most 0.5% of pixels, no pixel above 24; otherwise assert on geometry
  (rects, positions) rather than pixels.
- Blessing is a separate manual step: a `VEXEL_BLESS_GOLDENS=1` run writes
  candidate goldens plus review PNGs (via `vexel-png`, already a `vexel-app`
  dependency) to `artifacts/<ID>/review/`. Candidates are NOT accepted until a
  human has viewed the PNGs (screenshots must be viewed) and the coordinator
  has recorded the live-window screenshot in `docs/archive/live/` (see Live
  check). A worker may not mark goldens accepted. Normal test runs never write.
- Fixtures: `rust/fixtures/ui/<area>/` (new files only), 1x and 2x, dark and
  light where the order says so.

## Verification (frozen; repo-local CARGO_HOME/CARGO_TARGET_DIR, locked, offline)
- `cargo +stable check -p vexel-ui -p vexel-app --locked --offline`
- `cargo +stable test -p vexel-ui --locked --offline`
- `cargo +stable test -p vexel-app --locked --offline`
- `cargo +stable fmt --manifest-path Cargo.toml --package vexel-ui --package vexel-app -- --check`
- `python3 scripts/wo_validate.py WO-RUST-UI-TOKENS-01 --ready`
- `python3 scripts/ledger.py validate ledger.jsonl`
- `git diff --check`

## Live check (coordinator, a person at the Mac; not the worker)
1. Run the built `vexel` with `rust/fixtures/v1/compose/compose-v1.svg`, dark appearance, then light (System Settings > Appearance). Screenshot both.
2. Verify surfaces read as distinct layers (window darker than panels), text legible, accent not neon, no layout shift versus `docs/archive/live/2026-10-16-compose-v1-after-live-fixes.png`.
3. Open the swatch golden review PNGs beside the live window.
Record each viewed screenshot in `docs/archive/live/` as
`YYYY-MM-DD-ui-tokens-01-<what>.png` with a line in
`docs/archive/live/README.md` (what it shows, what was verified, what remains
unverified). Launch from a Finder/Terminal session in the foreground, not from
a harness; find the window id with Quartz `CGWindowListCopyWindowInfo`, then
`screencapture -x -l <id> out.png`, and view the PNG before accepting goldens.

## Evidence
`artifacts/WO-RUST-UI-TOKENS-01/`: `result.md` (<=400 words), `verification.json`,
`ac-map.json`, `changed-path-audit.json`, `review/` (candidate golden PNGs).

## Worker and review
Worker: gpt-6-luna Medium, Standard speed, one writer, existing allowance
only; stop on quota errors. Candidate only until a fresh read-only reviewer
(separate session, Luna or Sol Medium, diff + order only) reruns the frozen
commands and checks each AC. The coordinator integrates the exact
patch, reruns main-tree checks, does the live check, accepts goldens, then
updates STATE.md, LEDGER.md and ledger.jsonl.
