# SVG effects: what the renderer supports

Applies to the CPU scene renderer (`vexel-render`, driven by `vexel-app`
compose). The GPU display route never takes these scenes: any group with a
filter, mask or `mix-blend-mode` falls back to the CPU (`Decision::Cpu("group")`,
test `gpu_route_stays_on_the_cpu_for_blend_and_turbulence_scenes`).
Showcase: `docs/media/effects_showcase.svg`.

## Filters

`filter="url(#id)"` on a group or shape. At most 16 primitives, region and
units as SVG, always `color-interpolation-filters: linearRGB`. Blur
`stdDeviation` is limited to 32 device pixels; total work is capped (50 M
weighted pixel visits), so large canvases need smaller blurs (a full 640x360
canvas takes sigma 9, 1280x720 only about sigma 4).

| Primitive | Notes |
| --- | --- |
| `feGaussianBlur`, `feOffset`, `feFlood`, `feMerge`, `feDropShadow` | as SVG |
| `feBlend` | `normal multiply screen darken lighten overlay color-dodge color-burn hard-light soft-light difference exclusion` |
| `feComposite` | `over in out atop xor lighter arithmetic` |
| `feColorMatrix` | `matrix`, `saturate` (0..1), `hueRotate` (degrees), `luminanceToAlpha` |
| `feTurbulence` | see below |

Unsupported (typed error, never silently skipped): `feDisplacementMap`,
`feConvolveMatrix`, `feImage`, lighting, `feMorphology`, `feComponentTransfer`.

### feTurbulence

`type="fractalNoise|turbulence"`, `baseFrequency="fx [fy]"` (>= 0),
`numOctaves` (integer 0..10), `seed` (rounded half up), `stitchTiles="stitch|noStitch"`.
The SVG 1.1 reference algorithm (spec sample code): the same LCG, lattice
permutation, per-channel unit gradients, `s_curve` interpolation and stitching
rules, in `f64`. Deviations and conventions:

- Pixel `(x, y)` samples the user-space point of its top-left corner (resvg and
  Skia convention, no half-pixel shift).
- Stitch tile = the filter region in user space.
- `primitiveUnits="objectBoundingBox"` does not scale `baseFrequency`.
- Output is generated as straight RGBA and then goes through the linearRGB
  encode like any filter result, so mid grey noise ends up brighter (as in browsers).
- Not cross-checked against a Chrome/resvg render in this lane (no browser);
  correctness rests on the reference code, structural tests and pinned hashes.
- Multi-threaded over row bands; output is identical for any thread count.

Film grain (see the showcase):

```xml
<filter id="grain" x="0" y="0" width="1" height="1">
  <feTurbulence type="fractalNoise" baseFrequency="0.85" numOctaves="2" seed="5"/>
  <feColorMatrix type="matrix" values="1 0 0 0 0  1 0 0 0 0  1 0 0 0 0  0 0 0 0 1"/>
</filter>
<g filter="url(#grain)" style="mix-blend-mode:soft-light" opacity="0.7">
  <rect width="640" height="360" fill="#fff"/>
</g>
```

Clouds: `fractalNoise` at `baseFrequency="0.012 0.03"`, 4 octaves, then
`feColorMatrix` with a steep alpha row (`0 0 0 2.2 -0.95`) tints it white.
Performance: 1024x1024, 3 octaves + `luminanceToAlpha` + `feComposite`, release
build: p50 about 13 ms (budget 60 ms). Bench:
`cargo test -p vexel-render --release --test filter_turbulence -- --ignored --nocapture`.

## mix-blend-mode

Groups and elements: `normal multiply screen darken lighten overlay color-dodge
color-burn hard-light soft-light difference exclusion plus-lighter`.
The group is rendered into a layer, filtered/masked, then composited onto what
is already drawn in its parent with the W3C Compositing formulas in integer
arithmetic on straight RGBA8 (`vexel_render::blend`, results within 1/255 of the
float formulas; `normal` equals the source-over used everywhere else).
Limits: a blend over a transparent backdrop is the source (draw a background
first); `isolation` is parsed but groups always blend against their parent
layer; `background-blend-mode` is not supported.

```xml
<g style="mix-blend-mode:screen" filter="url(#glow)">...</g>   <!-- glow: blur + screen -->
```

## Text

Engine path: CoreText glyph outlines (macOS), published as ordinary shapes.

- Supported: `fill`, `stroke`, `stroke-width`, `stroke-linejoin`, `stroke-linecap`,
  `stroke-miterlimit`, `stroke-dasharray`, `stroke-opacity`, `font-size`,
  `font-weight` (numeric or `bold`), `font-style`, `letter-spacing`,
  `text-anchor`, `x/y/dx/dy`, `<tspan>`.
- `paint-order: stroke` (or `stroke fill`) draws the outline below the fill, so
  an outlined title keeps its full glyph shape. Implemented by publishing two
  shapes (outline, then fill). `paint-order` on non-text shapes is still skipped.
- Not supported: gradient or pattern paint on text, clip paths on text,
  `<textPath>`, CDATA text.
- Fonts: any family installed on the machine, resolved through CoreText by name,
  first match of the `font-family` list. On a stock macOS install that
  includes Helvetica, Helvetica Neue, Arial, Arial Black, Arial Rounded MT Bold,
  Avenir, Avenir Next, Courier, Menlo, Times, Georgia, Impact, Verdana,
  Trebuchet MS, Marker Felt, Brush Script, plus the SF system faces. Bold needs
  a bold face in the family (Impact and Arial Black are single-weight display faces).
  Fonts are not embedded; a document using other families renders differently
  on a machine without them.

## Gradients

Linear and radial gradients with all spread methods are supported on fill and
stroke of shapes. No gaps were found in `feColorMatrix` types while testing
(`hueRotate`, `saturate`, `luminanceToAlpha` are covered by
`rust/crates/vexel-render/tests/filter_turbulence.rs`).
