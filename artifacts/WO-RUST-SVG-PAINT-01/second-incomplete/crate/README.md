# vexel-svg-paint

This crate currently contains safe paint sampling and edge-range primitives
from the frozen PAINT partition. Sampling validates the entire stop range and
monotonic stop sequence before interpolation. It preserves the C
linear/radial sampling equations, transform order, spread behavior, and
`lround`-equivalent byte rounding.

The fill resolver remains blocked on scene stop publication. `ResolvedPaint`
returns owned stops but no published graph range or template identity;
`ResolveContext` has no builder, scene, or draft token. The shared cache range
indexes its own template stop vector, and validation requires that range to
match `paint.stops`. It therefore cannot also carry the range returned by
`SceneBuilder::append_stops`. A consumer cannot safely append each paint's
stops without duplicating template stops, nor reuse a published range across
draft rollback. No substitute publication semantics are implemented. See
`artifacts/WO-RUST-SVG-PAINT-01/result.md`.
