# Incremental index tests

`test_index.c` covers bounded one-byte stepping, source-backed hierarchy,
unsupported elements, preview readiness, generation invalidation, cooperative
cancellation, and oversized slices rejected without dereferencing their
pointer. Python-level performance and scripted progress tests live in
`tests/test_svg_index.py`.
