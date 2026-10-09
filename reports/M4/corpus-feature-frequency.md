# Real-corpus renderer priority inventory

Input: `/Users/megasoft78/Desktop/Freelance/vexel-corpus-real`; manifest SHA-256 `9fbf9a2f384d70a41da72e99ac259d2c5d137ffd3c8a01ce4be4741c1c29bc30`.
The inventory streamed all 58 files (4,429,240,515 declared bytes) with Expat; it stored counts only. Parse failures: 0.

Counts below are occurrences across all documents; file counts measure distinct files that contain the feature. They identify opportunities, not guarantees that an individual file becomes fully renderable.

## Highest coverage and frequency

| Feature | Occurrences | Files |
|---|---:|---:|
| `<path>` | 9,686,313 | 56 |
| `<use>` | 19,885,734 | 14 |
| `<circle>` | 9,092,377 | 42 |
| `<rect>` | 2,952,249 | 25 |
| `<line>` | 1,076,244 | 8 |
| `<image>` | 616,491 | 9 |
| `<g>` | 127,862 | 54 |
| `<clipPath>` | 60,161 | 15 |
| `<linearGradient>` | 997 | 38 |
| `<radialGradient>` | 16,654 | 2 |
| `<style>` | 2,890 | 45 |
| `<filter>` | 2,685 | 35 |
| `<text>` | 1,681,979 | 22 |
| `<ellipse>` | 388 | 5 |
| `<polygon>` | 176 | 3 |
| `<polyline>` | 39 | 1 |
| `<defs>` | 234 | 51 |
| `<symbol>` | 102 | 34 |
| `@transform` | 30,592 | 23 |
| `@style` | 506,200 | 17 |
| `@class` | 409,143 | 16 |
| `@fill` | 26,947 | 44 |
| `@stroke` | 11,170 | 25 |
| `@stroke-width` | 6,240 | 22 |
| `@clip-path` | 242,339 | 14 |
| `@href` | 20,519,286 | 20 |
| `@fill-rule` | 68,126 | 6 |
| `@x` | 25,140,463 | 41 |
| `@y` | 25,140,413 | 41 |
| `@width` | 23,457,168 | 58 |
| `@height` | 23,457,168 | 58 |

## Recommended implementation order

1. **M4-01 geometry primitives and affine groups:** support `circle`, `ellipse`, `line`, `polyline`, and `polygon`, and establish checked affine transforms on nested `<g>`/shape geometry. Groups occur in 54 files; circles in 42; lines are common in generated technical drawings; rects occur in 25. Existing M2 rect/path behavior remains the base.
2. **M4-02 path grammar and curve coverage:** paths occur in 56/58 files (9.69M path elements). Extend only where corpus feature evidence and differential fixtures show gaps; preserve source offsets and bounded limits.
3. **M4-03 fill rules and antialiasing:** fill is styled across 50 files; `fill-rule` occurs in 12 CSS-bearing files and appears as an attribute in corpus input. Keep scalar output deterministic and reference-tested.
4. **M4-04 stroke geometry:** CSS stroke is present in 50 files and stroke width in many; add width, caps, joins, dashes, and vector-effect in bounded steps, reporting which subset works.
5. **M4-05 CSS and class cascade:** 45 files have `<style>` elements, 16 have class attributes, and common CSS fill/stroke properties occur in 50. Implement a deliberately small cascade with unsupported declarations preserved and diagnosed.
6. **M4-06 reference and paint servers:** `<defs>` occurs in 51 files, `<use>` in 14 files (19.9M instances), `<symbol>` in 34 and gradients in 38. Add local-reference cycle/expansion limits, then gradients and clipping. Frequency by element count is skewed by repeated instances in generated files; prioritize distinct-file coverage first.
7. **Text and images later:** text appears in 22 files and images in a smaller set; font shaping, image decoding and platform/resource policy are larger and should follow the geometry/style wedge.

## Caveats

- A document can contain multiple unsupported features. Presence counts cannot be summed into predicted renderable-file counts.
- Metadata and editor-specific attributes should be preserved/ignored where safe; they do not by themselves imply render failure.
- This is a feature inventory, not renderer coverage or fidelity evidence. M4 work orders must update the runnable corpus report and compare pixels with an independent reference.
- Corpus files remain at the external, read-only path. This repository stores the manifest digest and aggregate counts only.
