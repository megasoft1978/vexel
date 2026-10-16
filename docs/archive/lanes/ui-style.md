# w4-ui — editor style implementation

## Built

- Added the report's dark and light semantic palette, stronger Increase Contrast
  variants, a separate canvas well, 6 pt control radius, and 8 pt base rhythm.
- Added a CoreText UI adapter using SF Pro Text and SF Mono. It rasterizes glyph
  paths into caller-owned frame storage, caches coverage in a 4 MiB LRU, clips
  visual text to each control, and preserves full UTF-8 strings in semantic
  snapshots up to the editor's 64 KiB per-field limit.
- Added cached 24×24 select, pan, and zoom path masks. The editor chrome uses a
  40 pt toolbar, 32 pt options row, 44 pt tool rail, 232/320 pt side panels,
  virtualized 20-row Structure and 10-row Inspector lists, canvas HUD, and a
  24 pt status row. At narrow widths the layout hides Inspector before
  Structure while preserving the canvas.
- Added focus rings, control edges, hover/pressed state rendering, invalid/error
  status color, and selected-tool styling. Reduce Motion is observed; the new
  chrome uses no animations.
- Added an AppKit accessibility tree with roles, labels, full values, enabled,
  selected and focused flags, screen frames, ordered child elements, status
  announcements, Full Keyboard Access activation, and View menu sidebar toggles.
- Added preference injection, 200,000-case deterministic layout fuzzing, a
  20,000-byte CoreText clipping check, contrast/icon checks, editor semantics
  checks, and new deterministic style screenshots.

## Contrast report

The computed report is in [contrast-report.json](../ui/contrast-report.json).
It uses WCAG relative luminance over the defined sRGB token pairs. The ordinary
light/dark text pairs exceed the research targets; the Increase Contrast text
and control-edge pairs improve over their ordinary variants. Static icon mask
hashes are included in the test output.

## Limits

- `viewer_editor_set_appearance` receives the live AppKit preference callback
  and invalidates the retained tree. The viewer's existing frame loop has no
  editor redraw callback, so a system appearance change is visible at the next
  viewer presentation. The w4-ui allowed paths do not include `src/app/viewer.c`
  or the window event loop needed to mark that frame dirty immediately.
- The canvas HUD currently gives the View menu route for fit and actual size;
  it does not receive live zoom or tile-progress values. Those are owned by the
  viewer composition layer, outside this lane's allowed paths. The tile status
  text describes retained preview behavior, not a live progress percentage.
- Select, pan, and zoom buttons update the active tool indicator and contextual
  hint. Existing app event routing does not consume the active tool, so pan/zoom
  gesture behavior remains on the viewer's existing navigation bindings.
- The retained painter supports hover and pressed flags, but the viewer event
  bridge does not deliver pointer-move/up state to this lane. Native smoke,
  VoiceOver, and system preference changes require an interactive macOS run.
- Values larger than 64 KiB display a bounded fallback and cannot be inspected
  in the inline field. The tree and semantic snapshot each reserve up to 1 MiB
  for full visible strings.
- Existing dark/light fixtures under `tests/ui/chrome-golden/` and
  `tests/editor/chrome-golden/` remain unchanged. New screenshots use the
  deterministic scalar renderer; CoreText is covered by structural pixel
  assertions to tolerate SF revisions.

## Verification

- `./dev build` — passed.
- `python3 tests/ui/verify_style_goldens.py --binary
  build/mac-debug/bin/viewer_chrome_render` — passed; 20 goldens match.
- `ui_tokens_contrast_test`, `viewer_chrome_test`,
  `viewer_chrome_layout_fuzz`, and `platform_text_ui_test` — passed. The layout
  run covered 200,000 cases with seed `0x6d2b79f5`; CoreText accepted a
  20,000-byte label and clipped it to the control.
- `ctest --test-dir build/mac-debug --output-on-failure` — 31/33 passed.
  `native-viewer-smoke` and `index-replay` could not create/use a Metal device
  or queue in this environment.
- `./dev asan` and `./dev ubsan` — builds succeeded; each full CTest run was
  31/33 with the same two Metal-dependent failures. All new UI tests passed in
  both sanitizer builds.
- `python3 -m unittest discover -s tests -t .` — 251 tests, 241 passed, 8
  failed, 2 errored. Seven failures were existing benchmark process-policy or
  output-capture cases. The native 100-cycle and exact-readback checks require
  an unbuilt Release binary; a second replay check also requires Metal.
- `./dev editor-smoke` — not run; it needs an interactive macOS desktop.

The native maintainer smoke is `./dev editor-smoke` on an interactive macOS
desktop.
