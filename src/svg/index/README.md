# Source-backed SVG index

The index session borrows an immutable complete byte slice until destruction.
`svg_index_step` examines no more than the requested budget, capped at 64 KiB.
Element and name ranges refer directly to that slice; no source text is copied.
Mutable session state belongs to one owner thread. Cancellation and generation
changes are also owner-thread operations. A canceled session cannot be resumed.

The large-input path has independent limits (1 GiB source, depth 256, one
million elements and attributes, 256 MiB record storage). It recognizes XML
lexical boundaries and keeps unsupported element records. The existing
one-shot tokenizer remains the authority for its original <=16 MiB API and
does not change defaults.
