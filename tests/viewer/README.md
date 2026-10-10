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

`native_smoke.sh` uses System Events to exercise the AppKit Open panel, reload,
zoom, display toggle, file selection, actual size, and quit. It is intended for
a maintainer's interactive macOS session and needs Accessibility permission
and a functioning Metal device.
