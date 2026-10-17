---
id: WO-RUST-UI-ICONS-02
revision: 1
status: READY
milestone: UI-MODERN-2026-10
owner: project-owner
executor: gpt-6-luna
base_sha: 6b404e7e39aa49efcd04316a4fe16c87a648ee44
spec_sha256: 6838406cd6ed19cdeedb0d55dbc01c9cf3fff8c56d3c1fe6298ac532937517b2
parallel_safe: True
network: false
---
# Redraw weak icons from the ICONS-01 review

## Goal
The ICONS-01 contact sheets (`artifacts/WO-RUST-UI-ICONS-01/review/icon-sheet-*-1x-zoom4.png`,
`docs/archive/live/2026-10-17-ui-icons-01-sheet-*.png`) show weak glyphs at 1x. Redraw them
in the existing 16-unit grid (nominal 1.5 stroke, even device widths) with the
reviewer's guidance, keeping ids stable.

## Dependencies
Merged WO-RUST-UI-ICONS-01. Read first: `rust/crates/vexel-ui/src/{icons,icon_paths,icons_draw,tests_icons}.rs`,
the sheets above (view with Read), `rust/crates/vexel-app/tests/ui_icons_sheet.rs`.

## Assumptions
- Ids 1-53 keep their `IconKind` values and names; only artwork changes. PAUSE=56 and
  TIMELINE=66 (ICONS-01 additions) may change artwork too.
- `MAX_PATH`/segment limits are not raised; if a design needs more, simplify the design.
- Icon names (ids): pen 8, node_edit 9, eyedropper 12, gradient 13, group 33,
  actual_size 36, ruler 37, pause 56, timeline 66.

## Scope (artwork, per reviewer)
1. pause: two bars, x about 5-7 and 9-11, clear gap, must not merge at 1x.
2. gradient: diagonal ramp inside a rounded square.
3. timeline: horizontal track with a playhead diamond above it; must not resemble align-left.
4. eyedropper: thin outline body with pointed tip, no heavy filled look.
5. ruler: axis-aligned rectangle with 3 ticks.
6. actual_size: plain corner brackets with a bold 1 (or simplified if the 1 clogs at 1x).
7. group: dashed outer square around two small squares.
8. pen (nib) and node_edit: reduce clutter at 1x (fewer segments, larger gaps).
Record any further icon judged weak in `result.md`; do not redraw it.

## Non-goals
Other icons, new kinds, tokens/colours, layout, rasteriser stroke/cap/join parameters,
snapping rules, the centred helper.

## Files allowed to touch
- `rust/crates/vexel-ui/src/{icons,icon_paths,icons_draw,tests_icons,tests}.rs`
  (`tests.rs`: only the `asset_hash` pins)
- `rust/crates/vexel-app/tests/ui_icons_sheet.rs`
- `rust/fixtures/ui/icons/**`, `artifacts/WO-RUST-UI-ICONS-02/**`

## Protected paths
Workspace/lock manifests and every `Cargo.toml`, all other crates and files, C/ObjC,
`tests/`, `docs/**`, `STATE.md`, `LEDGER*.md`, `ledger.jsonl`, other work orders,
git HEAD/index. No unsafe, new deps, network, installs, GUI launch, commit, stage,
reset, deletion. Files under 800 lines.

## Acceptance criteria (headless)
1. All ICONS-01 AC1-AC7 tests pass unchanged (no weakening, no exempt-list growth
   except documented in the test with a reason).
2. Ids 1-53 unchanged: a test asserts name and id of every kind 1-53 against a literal table.
3. pause at 1x: mask has exactly two 4-connected components of pixels with coverage >= 128,
   each at least 2 px wide, separated by at least 2 px of pixels with coverage < 64 in
   every row they share.
4. timeline at 1x: exactly two components at coverage >= 128 (playhead, track); the
   playhead's bounding box lies entirely above the track's; track is at least 10 px wide
   and at most 3 px tall.
5. ruler at 1x: ink bounds width at least 2x height (or vice versa); a row/column
   crossing at least 3 separate tick runs exists.
6. group at 1x: the outer ring shows at least 4 gaps (dash breaks) along its perimeter and
   the interior holds at least two separate components.
7. gradient, eyedropper, actual_size, pen, node_edit: each has a stated headless
   property in `tests_icons.rs` (e.g. eyedropper covered area at most 30% of the box;
   gradient ink bounds at least 12x12 units; pen and node_edit covered pixel count at 1x
   below the pre-change count recorded in `result.md`; actual_size has at least 4
   corner components plus one centre component).
8. Distinctness: no pair of distinct icon kinds has byte-identical masks at 1x or at 2x,
   and the redrawn timeline mask differs from align_left by at least 12 pixels at 1x.
9. Pins and goldens: `asset_hash` pins in `tests.rs` and the four candidate goldens
   `rust/fixtures/ui/icons/*.rgba` are updated deliberately (`VEXEL_BLESS_GOLDENS=1 cargo +stable test
   -p vexel-app --test ui_icons_sheet`), old and new pin values listed in `result.md`.
   Goldens stay candidates; the worker may not mark them accepted.
10. Review images: the bless run writes PNGs to `artifacts/WO-RUST-UI-ICONS-02/review/`; the
    worker views the 1x-zoom4 dark and light sheets with the Read tool and records in
    `result.md`, per redrawn icon, a one-line verdict (legible / still weak and why).
11. Existing `vexel-ui` and `vexel-app` tests pass (counts only increase); no unsafe,
    no new dependency; `git diff --check` clean.

If an AC cannot be met, stop with the exact blocker; no test weakening.

## Verification (frozen; repo-local CARGO_HOME/CARGO_TARGET_DIR, locked, offline)
- `cargo +stable check -p vexel-ui -p vexel-app --locked --offline`
- `cargo +stable test -p vexel-ui --locked --offline`
- `cargo +stable test -p vexel-app --locked --offline`
- `cargo +stable fmt --manifest-path Cargo.toml --package vexel-ui --package vexel-app -- --check`
- `python3 scripts/wo_validate.py --ready WO-RUST-UI-ICONS-02`
- `python3 scripts/ledger.py validate ledger.jsonl`
- `git diff --check`

## Live check (coordinator, a person at the Mac)
View the review PNGs (1x and 2x, dark and light) before accepting goldens; screenshot the
sheet to `docs/archive/live/YYYY-MM-DD-ui-icons-02-<what>.png` with a README line (what
it shows, verified, unverified). Goldens follow the ICONS-01 golden-image policy
(tolerance 0, first-party, candidates until a human has viewed the PNGs).

## Evidence
`artifacts/WO-RUST-UI-ICONS-02/`: `result.md` (<=400 words), `verification.json`,
`ac-map.json`, `changed-path-audit.json`, `review/`.

## Worker and review
Worker: gpt-6-luna Medium, one writer, existing allowance only; stop on quota errors.
Candidate until a fresh read-only reviewer reruns the frozen commands and checks each AC;
the coordinator integrates, runs the live check, accepts goldens, updates STATE.md,
LEDGER.md and ledger.jsonl.
