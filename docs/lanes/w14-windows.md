# W14 Windows: UX-09 and UX-10

This lane implements the first-run SVG home screen and multiple SVG documents
using native macOS window tabs. The full work-order text is in the research
report at `/private/tmp/claude-501/-Users-megasoft78-Desktop-Freelance-vexel/73fa8381-5851-4b85-bab5-8911cf2fe579/scratchpad/research2/r2-interaction.md`;
there are no frozen `work/WO-UX-09.md` or `work/WO-UX-10.md` files in this
checkout.

## Changes

- Added a custom-drawn first-run screen with Open SVG, recent files, a local
  editing note, and an SVG drop target. Loading and load errors have clear
  recovery controls. Replacing a document retains its current preview until
  the new document is ready; failure leaves that preview available with Open
  and Retry actions.
- Added local SVG file-drop validation for one regular `.svg` file. Unsupported
  drags report no AppKit drag operation. A drop on an empty window opens in
  that session. A drop on a document first resolves its unsaved edits, then
  opens in a separate document window.
- Added up to eight independent document sessions. Each owns its editor,
  selection, undo history, viewport, task queue, and native `NSWindow`. AppKit
  automatic tabbing uses a shared tabbing identifier and follows the user's
  macOS tab preference. File > New Window and the native Window menu expose
  tab navigation, detach, and merge commands. Closing a tab checks its dirty
  state; Quit checks every open document.
- Added startup file queuing so multiple Finder-opened SVGs reach separate
  sessions instead of overwriting a single startup path. Recent-file entries
  are bounded and SVG-only.
- Added home-state, session-isolation, deterministic layout/render fuzzing, and
  headless light/dark image coverage. Compact error layouts keep both recovery
  actions within a narrow window.

## Verification

- The compact error-layout regression was run red first: `./dev t -k
  viewer-home` failed with `compact windows must retain visible error recovery
  controls`; after the layout fix it passed.
- `./dev t -k viewer-home`: passed, 3/3 CTests.
- `./dev t -k viewer-sessions`: passed, 1/1 CTest. The session test checks
  independent source, dirty state, selection, viewport, undo/redo, activation,
  and close behavior.
- `build/mac-debug/bin/viewer_home_layout_fuzz`: 100,000 layout cases and
  5,000 bounded frame paints passed. Its initial run exposed clipped recent
  rows and loading notices wider than their canvas; both were fixed.
- `./dev build`: passed for Debug arm64/macOS 13.
- `python3 tests/ui/verify_home_goldens.py --binary
  build/mac-debug/bin/viewer_home_render`: 20 goldens match across light/dark,
  1x/2x, and empty/loading/error/document-loading/document-error states.
- `./dev t --all`: 59/61 CTests passed and all 69 Python tests passed. The two
  failures were `index-replay` (offscreen Metal device or queue unavailable)
  and `native-viewer-smoke` (failed; its test details were not included in the
  command's first-failure summary).
- `python3 scripts/ledger.py validate ledger.jsonl`: 406 entries valid.
- `git diff --check`: passed. All changed C and Objective-C files are under
  800 lines.

## Maintainer real-window verification

This lane did not use mouse/keyboard automation or capture the shared real
window. Please verify manually on macOS:

- Open two SVGs from Finder and confirm AppKit groups them as native tabs under
  the current macOS tab preference. Switch tabs and confirm each document
  keeps its source edits, selection, undo/redo, and viewport. Check the native
  Window menu's previous/next, detach, and merge commands.
- Close a dirty tab and test Save, Don't Save, and Cancel. Repeat Quit with
  multiple dirty documents and confirm Cancel leaves the windows open.
- Drop a local SVG onto an empty window and onto a dirty document. Confirm the
  empty window loads it and the occupied window resolves its dirty prompt
  before creating another document. Try a non-SVG, a directory, and a
  non-file URL and confirm AppKit shows the not-allowed cursor.
- Check first-run Open and Recent actions, a failed load followed by Retry,
  retained preview during replacement, and light/dark appearance.

No lifecycle entries were added to `ledger.jsonl` because the referenced work
orders are not frozen in `work/` in this checkout.
