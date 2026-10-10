# SVG SMIL animation model

`smil.c` reads bounded animation values and simple timing controls;
`smil_tracks.c` associates supported animation elements with their target spans;
`motion_path.c` resolves local `<mpath>` references and flattens the line-only
motion-path subset; `evaluate.c` samples timing and interpolation without
changing source bytes. `svg_shape_document.animation` owns the track/keyframe
arrays and borrows every value and source span from `document.source`.

The model accepts up to 128 tracks, 256 key values per track, 128 stable
diagnostics, and a one-hour document timeline. It supports numeric geometry and
opacity properties, literal solid `fill`/`stroke` colors, `<set>`, the
`translate`/`scale`/`rotate`/`skewX`/`skewY` transform types, simple offset and
syncbase begin/end timing, repeat count/duration, `remove`/`freeze`, key times,
linear/spline/discrete/paced interpolation, and additive/accumulate for numeric
and color values. Motion paths accept local `path` or `mpath` references made
from one `M` plus `L`, `H`, `V`, and `Z` segments, with numeric, `auto`, and
`auto-reverse` rotation.

Event timing, CSS animation/transition rules, external motion references,
curved/arc motion paths, and malformed or over-limit tracks produce stable
diagnostic kinds and byte offsets. `<animate attributeName="d">` is parsed only
when each key has the same path command/number structure; the current raster
overlay reports it unsupported because changing flattened curve topology is
not yet implemented. The renderer applies numeric and color overlays to leaf
shapes and composes transform/motion overlays through a bounded per-render-
instance ancestry chain. This includes groups, `<use>`/symbol placement, and
nested viewport mappings; animated transform replacement and additive sum are
kept distinct, and multiple transform tracks compose in document order.
Object-box gradients and clips update with animated bounds. Filtered/masked
opacity groups, active path morphing, geometry changes under patterns, and
group opacity animation still return stable unsupported results when their
flattened scene data cannot represent the update.
