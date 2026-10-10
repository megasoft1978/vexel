# Viewer quality tests

`viewer_quality_test.c` is a pure-C test of sharp-tile planning, clipping,
cache ownership and LRU limits, generation cancellation, extreme navigation,
work budgeting, and two seeded 200,000-event sequences. It does not create a
window or require Metal.

`tests/test_viewer_frame.py` drives the headless `--render-frame` entry point
against small SVG fixtures. It checks deterministic frames, malformed and
unsupported input, zoom boundaries, pan parsing, bounded source regions, and
out-of-range region errors. CTest registers both suites as `viewer-quality`
and `viewer-frame`.

`native_smoke.sh` delegates to the bounded `native_smoke.py` runner. System
Events exercises the AppKit Open sheet, reload, zoom, display toggle, file
selection, actual size, and quit. The runner bounds startup, automation, and
shutdown, reports process and viewer-log diagnostics on failure, and kills
both process groups during cleanup. It is intended for an interactive macOS
session and needs Accessibility permission and a functioning Metal device.
`repeat_native_smoke.sh` runs `./dev viewer-smoke` 20 times by default, logs
each run and a pass/fail summary, and continues after failures so maintainers
can inspect the full repeated-run result.
