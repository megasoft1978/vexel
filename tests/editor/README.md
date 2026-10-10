# Editor tests

`editor_state_test.c` drives the editor through its headless key, pointer, and
scroll event API. It checks bounded semantics, selection, attribute creation
and removal, text/structural edits, save, undo/redo, source well-formedness, and
that undo-all restores the original bytes after a deterministic 200,000-event
sequence.

`verify_chrome.py` runs `editor_chrome_render` and compares deterministic
1280x800 dark and light PNGs against the new fixtures in `chrome-golden/`.
Review both images before using `--update` to replace them.

`native_smoke.sh` opens a temporary SVG in the native viewer, exercises canvas
selection and transforms, draws and undoes a rectangle and polygon, saves, and
checks exact source preservation outside the expected transform. Run it from a
maintainer's interactive macOS desktop with `./dev editor-smoke`; the headless
environment does not run this AppKit automation.

`viewer_chrome_test.c` checks 64+ byte attribute semantics, injected appearance
changes, error state, panel visibility, inspector-first narrow collapse, and 2×
backing-scale geometry. `viewer_chrome_layout_fuzz` exercises 200,000 seeded
window/scale combinations in the UI test suite.

`editor_canvas_test.c` covers source-offset selection, immutable previews,
single and multi-element move/resize/rotate/nudge/delete/duplicate/reorder,
snapping, SVG clipboard fragments, text, path drawing and node editing,
rectangle/ellipse/line creation, clipped overlays, and exact undo.
`editor_canvas_fuzz_test.c` drives 200,000 seeded pointer/key events, checks
well-formed source and valid selection after every event, then undoes all 50
commits to the exact input bytes. `verify_selection_goldens.py` checks new
1280×800 and 2560×1600 selection overlay screenshots in dark and light themes;
these are separate from the frozen chrome fixtures.

`native_smoke.sh` runs the maintainer's interactive click-select, drag-move,
resize, rectangle draw/undo, polygon draw/undo, and save flow, then checks that
saved bytes differ only by the expected transform attribute. Run it with
`./dev editor-smoke` on an interactive macOS desktop with Accessibility access
enabled.
