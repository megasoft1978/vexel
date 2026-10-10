# SVG affine transforms

`transform.c` parses the supported original-source transform attribute values
and composes them with the traversal's inherited matrix. It borrows spans from
the single decoder token array and allocates no scratch storage. Supported
operations are `matrix`, `translate`, `scale`, and `rotate`, with at most 64
operations in one attribute and 4,096 accumulated operations per decode.
Malformed syntax returns `unsupported`; non-finite or out-of-range composition
returns `number`. Diagnostics use the original source offsets.
