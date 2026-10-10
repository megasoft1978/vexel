# W7 animation transform ancestry fix

## Implementation

The decoded shape document now records parent-linked transform frames per
rendered instance. A frame keeps the authored transform separate from motion,
`<use>`/symbol placement, and nested SVG viewport mapping. The raster overlay
applies transforms and motion through that frame chain; numeric geometry
changes are evaluated in local user coordinates and mapped through the current
ancestor transform. Transform replacement, `additive="sum"`, and multiple
transform tracks in document order are covered independently.

Frame-specific clip ownership keeps repeated `<use>` instances distinct.
Animated overlays clone clip chains with a 64 MiB cap, update clip transforms,
and refresh object-box clip and gradient bounds when geometry changes. The
borrowed source and decoded static arrays remain unchanged.

## Limits and unsupported behavior

- Transform ancestry is capped at 128 frames; overlay transform/clip work is
  capped at 50 million units.
- Active path morphing remains unsupported because the renderer stores
  flattened edges. Geometry animation with pattern paint is unsupported.
- Geometry or transform animation in a filtered or masked opacity group
  returns `unsupported`; group-level non-transform property animation also
  returns `unsupported` when it would affect flattened descendants.
- CSS animation, event/wallclock timing, curved motion paths, transform
  accumulation, and animated isolated group opacity remain unsupported.
- Clip overlay copies share repeated roots and are limited to 64 MiB per frame.

## Verification

The Python reference tests generate at least 30 parent/child hierarchy cases,
evaluate expected affine matrices independently, and compare sampled pixels
with `rsvg-convert` rendering of static equivalents. Additional references
cover local geometry, motion, `<use>` host transforms, user-space and
object-bounding-box clips, object-box gradients, multiple transforms, and a
stable unsupported filter diagnostic.

The seeded animation fuzzer generates direct, nested group, `<use>`, `<symbol>`,
and nested viewport trees, exercises attribute/transform/motion/multiple-track
syntax, decodes successful cases, samples rendered frames, and checks that the
source bytes remain unchanged.

Verification completed:

- `./dev build` — passed; Debug build was up to date.
- `ctest --test-dir build/mac-debug -R '^svg-animation' --output-on-failure` —
  3/3 passed, including Python and `rsvg-convert` references.
- `python3 -m unittest tests.anim.test_render_reference tests.anim.test_animation`
  — 17 passed.
- `python3 scripts/ledger.py validate ledger.jsonl` — valid, 406 entries.
- ASan+UBSan and UBSan-only animation targets built; focused
  `svg-animation` CTest passed in both sanitizer builds.
- Both `animation_fuzz 1000000 20261010` runs passed with
  `parsed=1000000`, `tracked=91676`, `decoded=1000000`, and
  `rendered=1000000`. Each run covered animation kinds
  `249998/250543/250195/249264` and hierarchy shapes
  `167158/166137/166506/166583/167123/166493`, with final seed
  `4761710361002158777`.

Full CTest, broad Python discovery, e2e, soak, and native-smoke gates remain
unverified here; the lane brief reserves e2e, soak, and native smokes for the
final gate.
