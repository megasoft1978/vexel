# WO-RUST-SVG-DEFINITIONS-01 candidate result

Status: **source-only candidate; root full re-review, protected audit, and fresh-main checks remain required**.

The full frozen 79-symbol, 10-suite scope remains represented. All 36 prior meaningful assertions and fixtures remain intact; four focused marker dimension regressions were added. Nonpositive `markerWidth` or `markerHeight` now returns `Limit(Shapes)` at the marker name offset, matching the original C. Malformed numeric dimensions remain `Number` at the value offset; overlimit, positive, and default behavior is guarded.

The accepted production channel tests remain unchanged. Fill `.25`, Stroke `.75`, and omitted opacity `1` continue through real URL resolution, planning, mapping, ticket completion, and `PatternId` publication.

Locked offline crate check passed. The full suite passed with 25 unit and 15 integration tests (40 total, 0 failures); WO validation passed. All owned Rust source/test files remain under 800 lines. Full original renderer/oracle runs, combined decoder/provider integration, and independent root acceptance gates remain outstanding.
