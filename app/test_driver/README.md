# UI test driver

`driver.py` is a macOS-only, Python standard-library test harness for replaying
bounded JSONL UI events against a placeholder state model. It does not display a
window, read an SVG document, or implement widgets. See `docs/ui-test-driver.md`
for the command schema, input limits, output artifacts, and replay guarantees.

Inputs resolve inside the repository. Outputs must be new directories under
`.cache/`. The semantic tree and PNG artifacts are deterministic for a fixed
script and seed; the virtual frame clock never reads wall time.
