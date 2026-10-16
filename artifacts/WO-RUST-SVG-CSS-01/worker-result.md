# WO-RUST-SVG-CSS-01 candidate

`CssSheet` now retains the immutable source and its matching XML token slice.
`compute(element, parent)` uses only those retained inputs. `CssPaint` carries
source provenance; passing a paint computed from a different source returns
`Limit` after a pointer/length identity check, with no source clone.

The CSS implementation covers all 39 properties, source spans, cascade and
inheritance behavior, selectors, parser limits, and the original CSS fixture.
Tests now assert exact values for every property and every meaningful value
assertion in `tests/svg/css/test_css.c`, including specificity, inline important,
presentation attributes, dashes, clip, stop, opacity, visibility, display, and
color. The equal-length red/blue source regression rejects cross-source parent
paint values. All 10 prior tests remain; the expanded suite has 13.

Processing style CDATA is an explicitly authorized additive Inkscape
compatibility adaptation; the C creator processes Text tokens only. Its direct
test verifies the unknown declaration source remains unchanged. No dependency
or unsafe code was added. Scene/renderer integration remains downstream. This
is an uncommitted candidate for independent review, not a full-SVG claim.
