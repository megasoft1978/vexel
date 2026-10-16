# PAINT malformed fill-span parity repair

`classify_inner` previously mapped `XmlSpan::slice(source)` failure to `InvalidArgument`. The original fill URL parser reports `Unsupported` for malformed fill span bounds at `paint.c:553`. The resolver now returns `Unsupported` at the caller-provided fill offset. No parser, cache algorithm, sampler math, limits, or shared API changed.

The new `malformed_fill_spans_are_unsupported_at_fill_offset_in_all_public_flows` test covers offset beyond source, length beyond remaining source, checked `usize` addition overflow, empty span, and short span. It exercises `classify_fill`, `resolve_cached_fill`, and `resolve_fill`; each checks exact kind and offset, unchanged cache entry/slot/storage state, unchanged token visits, every RenderMemoryStats field, and every RenderWorkStats counter. Valid local URL spans succeed through all three flows. Cached and owned resolve paths assert preexisting cancellation still wins and retains the supplied fill offset.

The previous 22 crate tests and 3 graph tests remain. Fresh checks report 23 crate tests and 3 graph tests passing. Exact argv, exit codes, elapsed times, stdout/stderr and per-command logs are in `r3-span-command-evidence.json`; consolidated verification is in `verification.json`.

This is a candidate-only scoped parity repair. Shared production delegate adoption and independent root acceptance/audit remain pending. Mixed renderer, application, release and demo gates are not claimed.
