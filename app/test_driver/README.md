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
