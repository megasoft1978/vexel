# UI test driver

`driver.py` is a macOS-only, Python standard-library test harness. Its default
mode replays bounded JSONL UI events against a placeholder state model. The
`--native-platform-smoke` mode translates a bounded open/resize/key/pointer/quit
scenario into commands for the C app and exercises the AppKit window shim. It
does not read SVG documents or implement widgets. See `docs/ui-test-driver.md`
for the command schema, input limits, output artifacts, and replay guarantees.

Inputs resolve inside the repository. Placeholder replay outputs must be new
directories under `.cache/`. Native smoke reserves a new numbered `run-NNNN`
directory beneath its output root on every invocation so the work-order runner
can repeat the same command without overwriting earlier evidence. The semantic
tree and PNG artifacts are deterministic for a fixed script and seed; the
virtual frame clock never reads wall time.

`--native-metal-smoke` presents a deterministic CPU RGBA tile through Metal,
verifies exact uploaded-texture readback, clears and resizes the window, and
checks a second frame. It requires the Release executable and an active macOS
graphical session.

`--repeat-replay --script ... --repeats 20` runs the bounded placeholder script
in fresh numbered iteration directories, compares normalized command order
and all artifact hashes, and prints the shared replay digest. Re-running the
outer command under the same output root reserves another `run-NNNN` directory.
Frame time is virtual and advances only by explicit `frame.delta_ns` values.

`--native-metal-replay-smoke --repeats 20` submits the same scalar tile through
20 explicit headless Metal blits. Each command has a five-second completion
bound; the implementation checks exact readback bytes and hashes the returned
bytes. Windowed AppKit/Metal presentation remains covered by
`--native-metal-smoke`. The replay needs a Metal-capable macOS session and a
Release executable; it is not an SVG renderer or a performance claim.
