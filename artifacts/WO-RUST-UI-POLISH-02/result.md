# WO-RUST-UI-POLISH-02 result

Scope: visual polish of four review findings (Export popover, footer scroll thumb,
Draw strip elapsed fill, three weak icons). Branch worktree-agent-a09eec6a10ac3970e.

## Changed pins and goldens

| Item | Before | After |
|---|---|---|
| `vexel-ui/src/tests.rs` dark_1 / light_1 asset hash | 0x1187_2e29_62e7_ae15 | 0x97c1_e090_324b_8bd2 |
| dark_2 / light_2 asset hash | 0xf55c_7d47_474e_2645 | 0x432c_829f_3183_a806 |
| dark_3 / light_3 asset hash | 0xdd56_a472_0e58_2873 | 0x478e_1385_58cc_7c7f |
| `fixtures/ui/icons/icon-sheet-{dark,light}-{1,2}x.rgba` | ICONS-02 candidates | re-blessed (VEXEL_BLESS_GOLDENS=1, ui_icons_sheet) |
| `fixtures/ui/timeline/footer-long-scrolled-{dark,light}-{1,2}x.rle` | pre-POLISH-02 | re-blessed (ui_timeline_golden; only the scroll thumb differs) |

Test expectations changed for the requested design change (not weakened):
- `ui_timeline2_footer.rs`: thumb colour now `text_tertiary` (was `control_edge`); test renamed.
- `ui_timeline2_strip.rs`: accent is now the elapsed fill plus thumb, so the old
  "widest horizontal accent run <= 4 px" check was replaced by "no column carries
  more accent than the thumb height".

Review PNGs: bless writes into the older `artifacts/WO-RUST-UI-ICONS-02/review` and
`artifacts/WO-RUST-UI-TIMELINE-01/review` folders. Those were restored to HEAD; the new
icon sheets are in `artifacts/WO-RUST-UI-POLISH-02/review/`.

## Changes

1. Export popover (`layout/menu.rs`, `motion/mod.rs`): 4 px below the Export button,
   right-aligned. With the inspector open it starts below the inspector header row
   (y = 80), so it no longer covers the header text. This is a deliberate trade-off: the
   gap to the button is larger in that state. Fill surface[5], 1 px `text_tertiary` edge,
   two-layer soft shadow. Row hover uses `selection_fill`.
2. Footer scroll thumb (`timeline/layout.rs`, `timeline/paint.rs`): 6 px wide, `text_tertiary`
   at rest, `text_primary` on hover (new `Hit::Thumb`, hover only). No drag: the thumb was
   never draggable, and adding drag was out of scope.
3. Strip (`animation/strip.rs`): elapsed segment in accent, 3 px accent thumb.
4. Icons (`vexel-ui/src/icon_paths.rs`): eyedropper (near-vertical pipette, tip bottom-left),
   group (dashed rounded outer ring, two 4x4 inner squares), node_edit (anchor square,
   diagonal, hollow circle). IDs and names unchanged.

## Verification

- `cargo test --offline --locked -p vexel-ui -p vexel-app --no-fail-fast`: 457 passed, 1 ignored.
- `python3 scripts/check_rust_fmt.py`: pass after `cargo fmt -p vexel-app`.
- Live screenshot: `docs/archive/live/2026-10-17-polish02-draw-after.png` (Draw mode, loop.svg).
  The elapsed fill is not visible at t=0. No live Export-popover or scroll-thumb screenshot
  was captured.
- Before images: `docs/archive/live/2026-10-17-icon-sheet-before-dark-1x-zoom4.png` and
  `docs/archive/live/2026-10-17-footer-scroll-before-dark-1x.png`, taken from HEAD.
- Not verified visually: the Export popover, the footer thumb state, and the strip fill in motion.
