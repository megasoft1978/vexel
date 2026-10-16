# WO-RUST-CORPUS-UNSUPPORTED-01 result

Baseline (HEAD 393ae68f) already showed text, filter and clip supported; the
remaining causes were `<image>` (2 files) and `<marker>` (1 file).

Histograms (verdict counts):

| set | before | after |
|---|---|---|
| in-repo (24) | ok 23, unsupported 1 | ok 24, unsupported 0 |
| external (58) | ok 5, unsupported 2, limit 29, xml 2, timeout 1, skipped 19 | ok 6, unsupported 0, limit 29, xml 2, timeout 2, skipped 19 |

No previously `ok` file regressed (per-file diff in `causes.md`).

Constructs fixed (each with a fixture, test and C golden or disclosed oracle):
- Markers: instanced per start/mid/end placement in `vexel-svg-engine/src/marker.rs`
  (the engine planner no longer fails closed). Mid markers keep a fixed
  `orient` angle as legacy C does. Goldens `marker-end`, `marker-mid-end`
  are byte-exact; `marker-start` uses a hand-derived oracle because the legacy C
  draws the start marker twice per subpath.
- Decoder bounds bug (`definition_links.rs`): definitions got `[x, y, w, h]`
  but planners read `[x0, y0, x1, y1]`, so any owner with x0 > width or
  y0 > height failed (clip objectBoundingBox, thin markers). Fixed; test fails
  without it.
- Data-URI PNG images: engine `image.rs`, app PNG + inflate decoders, images
  drawn as pattern paint with a padded shared tile. `image-png-rgba` byte-exact;
  `image-png-types` max delta 1 on 20 channel values (x.5 tie rounding).
- Two existing tests asserting marker fail-closed were superseded in place
  (as the mask order did); `mask_group.rs`/`filter_decode.rs` also got
  rustfmt-only changes so the frozen fmt check passes.

Residuals (all outside this order's scope):
- generated-inkscape_t1: renders (disclosing 34,715 gradient strokes) but takes
  about 160 s, over the 60 s sweep timeout. Cause: scene validation per
  committed shape request is quadratic for shapes not batched at the root
  (4000 shapes 1.5 s, 8000 shapes 6.4 s); same family as cad_t1 (59 s).
  Non-goal: render/decode speed.
- generated-text_t1: `TextCommands` limit at byte 235947 (vexel-scene
  `DOCUMENT_TEXT_COMMAND_LIMIT`, protected crate).

Unmeasured: JPEG and interlaced PNG (typed `Unsupported`); chunk CRCs are not
compared (the fixtures carry wrong IDAT CRCs, Adler-32 still checked);
`preserveAspectRatio` meet/slice with horizontal letterboxing differs from C
on draw-rect border pixels; circle flattening differs from C by up to 32 levels
(pre-existing, not marker related); markers with opacity/mask on the marker
element; hidden images fail closed; T2/T3 files.
