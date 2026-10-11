# w6-editor2 — finish the editor MVP

## Delivered

- Connected Save As, dirty open/close/quit choices, save errors, and external
  file conflicts to native AppKit dialogs.
- Added Shift-click, marquee, and Cmd+A multi-selection with group movement,
  transforms, nudging, deletion, duplication, and keyboard z-order edits.
- Added polygon, polyline, pen, and text tools; retained path node/handle
  editing; snapping guides; SVG fragment clipboard; PNG file export and Copy as
  PNG; and composed transform matrices for successive move/resize edits.
- Added document edit APIs for validated bounded XML-region replacement,
  byte-local source-span replacement, and escaped one-command text insertion.
- Added focused editor, document, dialog, PNG codec, and PNG export tests. The
  editor fuzz target issues 200,000 seeded events and checks source validity,
  selection spans, and undo-all restoration.
- Extended the native editor smoke with polygon creation and undo.

## Limits

- Multi-selection is capped at 128 rendered elements. Group actions require
  sibling elements under one parent and a changed source region no larger than
  64 KiB.
- Node editing supports retained `M`, `L`, `H`, `V`, `C`, `S`, `Q`, `T`, and `A`
  path commands. Double-click insertion is limited to line segments; Delete
  cannot remove the initial move-to node.
- New text defaults to 16 units and `#172033`. Its `font-size` and `fill` are
  editable through the existing attribute inspector.
- Clipboard paste accepts one validated root fragment up to 64 KiB. Multi-item
  copy wraps siblings in a group.
- PNG output is limited to 4096 pixels per axis, 16 million pixels, and 64 MiB
  encoded bytes. It uses the current SVG decoder/rasterizer feature set.

## Verification

- `./dev build` passed.
- Focused Debug CTest for the editor, edit engine, platform dialogs, PNG codec,
  PNG export, and selection goldens passed 9/9.
- Full Debug CTest: 42 passed, 2 failed, and 1 skipped. `native-viewer-smoke`
  and `index-replay` could not create a Metal device/queue; `mcp-stdio-e2e` was
  skipped by the harness.
- `./dev asan` and `./dev ubsan` completed the same 45-test suite. All editor,
  edit-engine, PNG, and fuzz tests passed; each run had 42 passed, the same two
  Metal failures, and one harness skip. No sanitizer report was emitted by the
  editor or export tests.
- `editor-canvas-fuzz` passed 200,000 seeded events, checked source/selection
  invariants, and undid 50 edits to the exact original bytes in Debug, ASan, and
  UBSan builds.
- After adding PNG axis/pixel-limit cases, the `platform-png` test passed 1/1 in
  both ASan and UBSan builds.
- `python3 -m unittest discover -s tests -t .` and `./dev test-fast` each ran
  269 tests: 258 passed, 8 failed, 2 errored, and 1 skipped. Seven failures are
  in benchmark sandbox/policy and subprocess-capture tests; one native driver
  test needs the missing Release executable. The two errors are native Metal
  driver tests (missing Release executable and unavailable Metal queue).
- `./dev editor-smoke` could not complete: AppleScript returned error `-10827`
  before the scripted editor interaction. The full CTest native viewer smoke
  also failed before presenting a frame because Metal was unavailable.
- `git diff --check` and `sh -n tests/editor/native_smoke.sh` passed. Existing
  chrome/style and selection goldens were not changed.
- `python3 scripts/ledger.py validate ledger.jsonl` passed with 406 entries.

## Unverified

The platform dialog tests inject choices and do not open native panels. The
PNG export test injects the save panel response; interaction with physical
AppKit panels and external clipboard applications still needs an interactive
desktop session. Native presentation and mouse automation remain unverified in
this environment.
