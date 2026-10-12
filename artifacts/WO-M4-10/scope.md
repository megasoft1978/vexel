# WO-M4-10 scope of implementation commit 071e0ec

Commit `071e0ec` (parent `c1ddee9`; WO-M4-10 base is `5894cb4`) changed 26 paths: 3 inside and 23 outside the WO-M4-10 allowlist.

## Inside the WO-M4-10 allowlist

- M `CMakeLists.txt`
- M `src/svg/shape/decode.c`
- M `src/svg/shape/traverse_internal.h`

## Outside the WO-M4-10 allowlist

- M `README.md`
- A `artifacts/hygiene-worktrees-after.json`
- A `artifacts/hygiene-worktrees-before.json`
- A `artifacts/release/install-smoke.json`
- A `artifacts/release/sbom.cyclonedx.json`
- M `docs/ui/blender-alignment-review.md`
- A `scripts/install_smoke.py`
- A `scripts/make_sbom.py`
- M `src/app/viewer.c`
- M `src/app/viewer_chrome.c`
- M `src/app/viewer_chrome.h`
- M `src/app/viewer_chrome_editor.c`
- M `src/app/viewer_chrome_inspector.c`
- M `src/app/viewer_chrome_structure.c`
- M `src/app/viewer_editor.c`
- M `src/app/viewer_editor.h`
- M `src/app/viewer_editor_canvas_pointer.c`
- M `src/app/viewer_editor_input.c`
- M `src/app/viewer_editor_internal.h`
- M `src/app/viewer_editor_transform.c`
- M `src/app/viewer_editor_view.c`
- M `src/platform/macos/platform_settings.m`
- M `src/platform/platform_settings.h`

The out-of-allowlist paths are governed by WO-RETRO-01 (`work/WO-RETRO-01.md`), not by WO-M4-10. WO-M4-10 claims only the inside-allowlist paths above; it makes no claim about, and its acceptance does not cover, the others. AC-6 of WO-M4-10 (no file outside the allowlist) is therefore not satisfied by `071e0ec` and is not asserted; this order's AC-4 (its own commit lists only allowlisted paths) is the applicable check.
