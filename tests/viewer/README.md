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

`native_smoke.sh` delegates to the bounded `native_smoke.py` runner. It sets
`VEXEL_TEST_OPEN` and invokes `--native-viewer-smoke`, which uses native event
injection for reload, zoom, fit, actual size, and quit. It does not use UI
scripting or present `NSOpenPanel`. The runner has a bounded timeout, reports
process and viewer-log diagnostics on failure, and stops the viewer's process
group during cleanup. It still needs AppKit and a functioning Metal device.

`panel_smoke.sh` is a separate best-effort check of the real AppKit Open sheet
and file selection. Run it with `./dev viewer-panel-smoke` on an interactive
macOS session with Accessibility access enabled. It is not part of the regular
viewer smoke or final gate.
`repeat_native_smoke.sh` runs `./dev viewer-smoke` 20 times by default, logs
each run and a pass/fail summary, and continues after failures so maintainers
can inspect the full repeated-run result.
