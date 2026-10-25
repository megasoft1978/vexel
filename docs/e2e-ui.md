# Real-window UI check

`scripts/e2e_ui.sh` launches `target/debug/vexel` on a temp copy of
`rust/fixtures/v1/anim/loop.svg` and drives the real window through System
Events (`scripts/e2e_ui.applescript`) and `screencapture`. No extra tools.

## Run

```sh
cargo build -p vexel-app            # produces target/debug/vexel
VEXEL_E2E_OK=1 scripts/e2e_ui.sh    # ~5 min, do not touch mouse/keyboard
```

Refuses to run without `VEXEL_E2E_OK=1`. Everything it writes lives in one
`mktemp -d` directory, removed on exit (set `VEXEL_E2E_KEEP=1` to keep the
screenshots and exported files). The app is always killed by a trap. Output is a
PASS/FAIL line per step plus `RESULT: PASS|FAIL`; exit 1 on any failure, 2 when
refused or misconfigured.

## Prerequisites

- Apple Silicon macOS 13+, logged-in desktop, display awake, no other app
  stealing focus.
- Accessibility permission for the terminal running the script (System
  Settings > Privacy & Security > Accessibility). Screen Recording permission
  for screenshots.
- Controls are found by AX name/title/description substring. If a label
  changes, override with `E2E_L_*` env vars (see the script header). A control
  that is not found is a FAIL, not a skip.

## Steps

launch, screenshot, inspector edit (canvas pixels change), rotate via the
inspector rotation field, Cmd+G / Cmd+Shift+G, timeline Animate mode + add key
(the K shortcut of the transport "Add key" button; the button is painted, not
an AX element, so set `E2E_L_ADD_KEY` only if it gains an AX label) + easing
picker, Export Video row present, Export GIF and Export MP4 save
panels (full path typed into the name field, Return, file non-empty with `GIF8`
or `ftyp` header).

Status: written without a live run. Expect to tune the `E2E_L_*` labels on the
first supervised run.

## Coverage list for the new features (planned steps, not yet in the script)

Each item is one PASS/FAIL step to add to `scripts/e2e_ui.sh`; none is run yet.
Headless equivalents live in `tests/ui_*` and `src/tools/tests_*`.

- Add key button: in Animate with a shape selected, click the transport "Add key"
  button; a diamond appears at the playhead on that shape's track; Cmd+Z removes
  it and Cmd+Shift+Z restores it.
- K shortcut: with focus outside text fields, press K at a second playhead
  position; one more key, same track; pressing K again on the same tick updates
  the key instead of adding a duplicate.
- Nudge: select a shape, press Right/Down arrows (1 unit) and Shift+arrows if
  bound; the canvas pixels shift, one undo entry per press, no error toast.
  Repeat on a rotated shape (matrix transform): it moves, never errors.
- Group: Cmd+G wraps the selection (selection becomes the group), Cmd+Shift+G
  ungroups; moving the group moves every child; undo/redo round-trips.
- Rotate: drag outside a corner handle to rotate (Shift snaps to 15 degrees);
  rotate a second time (angles add, centre stays); then drag the body to move
  the rotated shape (no error); resize handles on a rotated shape do not error
  (they fall back to moving).
- Timeline scroll: open a document with more tracks than fit (walk_home, 136
  tracks); two-finger scroll over the track list moves whole rows, the thumb at
  the right edge follows, scrolling stops at both ends; selecting a shape on the
  canvas scrolls its track into view; clicking a row after scrolling selects the
  row under the pointer.

## Manual checklist (not automatable)

- [ ] NSSavePanel: Cancel leaves no file; overwrite prompt appears for an
  existing name; default name and extension are right; tags/format popup sane.
- [ ] NSSavePanel: sandbox/permission prompts, remembered last directory.
- [ ] VoiceOver: inspector fields, timeline keys and toolbar are announced with
  sensible names and roles; focus order follows reading order.
- [ ] IME: compose Japanese/Chinese text in a text field; marked text, candidate
  window position and commit behave.
- [ ] New app icon: shown in Dock, Cmd+Tab, Finder bundle, About, at 1x/2x.
- [ ] Rotate preview feel: dragging the rotate handle is smooth, snaps with
  Shift, no lag or flicker; live preview matches the committed result.
- [ ] Dark/light mode and reduced-motion switch.
