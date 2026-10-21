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
+ easing picker, Export Video row present, Export GIF and Export MP4 save
panels (full path typed into the name field, Return, file non-empty with `GIF8`
or `ftyp` header).

Status: written without a live run. Expect to tune the `E2E_L_*` labels on the
first supervised run.

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
