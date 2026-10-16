# vexel-svg-paint

This crate currently contains the safe paint sampling and edge-range primitives
from the frozen PAINT partition, plus an owned gradient cache keyed by
definition and both viewport dimensions. Sampling validates the entire stop
range and monotonic stop sequence before interpolation. It preserves the C
linear/radial sampling equations, transform order, spread behavior, and
`lround`-equivalent byte rounding.

The fill resolver is not yet implemented. The accepted `vexel-scene` shared
`PaintCache` does not retain viewport dimensions in `GradientCacheEntry`, so it
cannot satisfy the frozen complete-context cache key through
`ResolveContext`. The local `GradientCache` is independently tested but is not
a substitute for that shared integration contract. Pattern planning, SVG
gradient parsing/inheritance, decode composition, and original mixed-renderer
assertions therefore remain blocked pending a shared API repair and later
coordinator adapters. See `artifacts/WO-RUST-SVG-PAINT-01/result.md`.
