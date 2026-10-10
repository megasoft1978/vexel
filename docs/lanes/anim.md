# Animation v1 core, headless, and MCP

## Current implementation

The decoder recognizes direct-child `<animate>`, `<animateTransform>`, `<set>`,
and `<animateMotion>` elements and excludes them from static shape traversal.
`svg_shape_document.animation` owns up to 128 tracks and up to 256 keyframes per
track. Track IDs, target element spans, animation element spans, and keyframe
value spans borrow the unchanged source buffer. Up to 128 unsupported-feature
diagnostics are retained as stable kind/source-offset pairs.

The supported timing profile includes bounded second/millisecond offsets,
`indefinite` begin/duration/repeat forms, track-ID `.begin`/`.end` syncbases,
finite repeat count/duration, explicit end times, and `remove`/`freeze` fill.
Interpolation includes linear, cubic spline, discrete, and paced numeric/color
values with key times. Numeric additive/accumulate modes are evaluated. The
profile accepts the numeric geometry and opacity properties listed in
`src/svg/animation/animation.h`, literal solid fill/stroke colors, and
`translate`, `scale`, `rotate`, `skewX`, and `skewY` transforms. Motion paths
accept local path data or local `<mpath>` references made from `M`, `L`, `H`,
`V`, and `Z` commands.

CSS keyframes/animation/transition rules and event/wallclock/accessKey timing
are reported as unsupported diagnostics. The path morph parser checks matching
commands and numeric arity, but `raster_shape_render_at` returns an unsupported
diagnostic for active `d` tracks because changing flattened edge topology is
not implemented. Curved and arc motion path segments are unsupported.

The synchronous CLI renders one explicit sample with `--render-frame ...
--time T`, or writes a PPM image sequence with `--render-animation ... --fps N
--duration D --out <existing-dir>`. Sequence export caps at 300 frames and
1..60 fps, with 32 million total frame pixels. The ImageIO platform helper
encodes looping GIFs from bounded RGBA frames. MCP exposes `animation_list`, source-preserving `animation_add`,
`animation_set`, and `animation_remove`; `render_png` accepts a time; and
`export_animation` writes PNG sequences or GIFs inside granted paths. MCP export
caps at 120 frames, 30 fps, 512 pixels per axis, and 16 million cumulative
frame pixels. Existing export files are not replaced.

## C API for a timeline client

1. Decode the borrowed source with `svg_shape_decode` and retain the source
   buffer for the life of the `svg_shape_document`.
2. Enumerate `document.animation.tracks[0..track_count)`. Each track exposes
   `attribute`, `kind`, `target_offset/target_length`, and
   `element_offset/element_length` for source-preserving edits.
3. Enumerate `track.keys[0..key_count)`. Each key exposes normalized `time`, a
   typed `value`, and its original `source_offset/source_length`.
4. Use `svg_anim_eval(&document.animation, seconds, &overlay)` for an owned
   overlay containing each track's active value and repeat iteration. Destroy
   it with `svg_anim_overlay_destroy`.
5. Render deterministically with
   `raster_shape_render_at(&document, seconds, &tile, &diagnostic)`. Repeated
   renders at the same decoded revision/time are deterministic. The API does
   not write into source or decoded arrays.

Track editing should replace only the matching `element_offset/element_length`
span via the document edit engine. After an edit, discard the decoded document
and re-decode the new revision so offsets and target handles cannot become
stale. No timeline UI or frame cache is part of this lane.

## Verification and remaining limits

The test set includes C coverage for numeric geometry, solid color, opacity,
transforms, line motion and local `mpath`, syncbase timing, repeat/fill behavior,
unsupported timing/CSS diagnostics, and unchanged source bytes. An independent
Python evaluator compares linear, spline, discrete, and syncbase behavior at
sampled times. A librsvg cross-check renders equivalent static SVG values and
compares interior pixels at four times. Fifteen authored fixtures cover
spinners, loaders, color/opacity changes, transforms, motion, sets, morphs, and
unsupported event timing. CLI frame and sequence exports, MCP editing, timed
PNG rendering, PNG frame export, and GIF export have automated checks.

The million-case seeded animation fuzzer passed with seed `20261010` in both
the combined ASan/UBSan build and the separate UBSan build, with final seed
state `1135527464435917069`. Debug, ASan, and UBSan animation CTests pass all
three lane checks. Full CTest runs pass 43/45, with the MCP socket test skipped
because the sandbox denies UDS bind and two existing native checks failing
because LaunchServices and offscreen Metal are unavailable. Full Python
discovery runs 277 tests: 266 pass, one skips, and ten fail in existing
benchmark process-policy and native Metal sandbox checks. The interactive viewer has no timeline or playback controls and does not cache
frames. CSS animation and event-based timing remain unsupported. Morph tracks
are listed but cannot be rasterized. Animated group opacity is not reconstructed
by the flattened scene, and flattened transforms can differ for nested
viewports and `<use>` instances. MCP animation listing/editing and export use a
16 MiB source limit; export output is bounded to 120 frames at 512 pixels per
axis and 16 million cumulative frame pixels, and existing files are left
untouched.
