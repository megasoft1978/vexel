# Source-backed SVG index

The index session borrows an immutable complete byte slice until destruction.
`svg_index_step` examines no more than the requested budget, capped at 64 KiB.
Element and name ranges refer directly to that slice; no source text is copied.
Mutable session state belongs to one owner thread. Cancellation and generation
changes are also owner-thread operations. A canceled session cannot be resumed.

The large-input path has independent limits (1 GiB source, depth 256, one
million elements and attributes, and 256 MiB combined element-record and
token-vector capacity (each vector is capped at half that budget). Index
records are owner-thread-only. A bounded task validates the complete source
with the same XML grammar and exposes its token spans after completion; worker
allocations are capped before retrying token output. The existing one-shot
tokenizer remains capped at 16 MiB and its defaults do not change.

Once a supported depth-one shape has a complete source range,
`svg_index_make_preview_source` returns an owned minimal SVG consisting of the
original root opener and shape span. Callers schedule decode/raster work and
must retain that owned byte buffer through task completion.
