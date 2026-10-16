# W15 native macOS shell

## Changes

- Completed the App, File, Edit, View, Window, and Help menus. Menu actions enter
  the same editor/event paths as keyboard commands; save, history, selection,
  sidebar, canvas, and paste availability follow the current state.
- Added an AppKit Welcome window and an eight-path recent-files model shared by
  the Welcome list, Open Recent menu, and Dock menu. The model bounds paths and
  keeps UTF-8 basename/parent slices; the native UI uses Foundation path
  components for Unicode-safe display. New Blank SVG queues a valid untitled
  document. Unavailable and failed recent files can be removed.
- Added the CMake `vexel_app_bundle` target and `app/Info.plist.in`. The SVG
  document declaration uses `public.svg-image`, `.svg`, and `LSHandlerRank`
  `Alternate`, preserving the user's default application. `./dev bundle` now
  signs the CMake-produced app instead of rebuilding its contents separately.
- Added an original two-ribbon Bézier icon as `app/Vexel-Icon.svg` and a clean
  1024px PNG export. The checked generator creates the ICNS and downsampled
  iconset without a new dependency. The app delegate handles LaunchServices
  file opens and supplies Dock commands for New, Open, Recents, and open windows.
- Enabled AppKit system full screen, updated the View menu title on entry/exit,
  and kept sidebar visibility across the system resize. Modifier-scroll zoom
  can be changed in Settings; normal scrolling remains routed to the outline,
  inspector, or canvas under the pointer.
- Added reusable General and Canvas Settings panes at App > Settings… / ⌘,.
  Preferences use a versioned, small `NSUserDefaults` set and include reset and
  recent-list clearing.
- Added native Help and Send Feedback windows. Feedback previews the exact
  version/OS report, copies it offline, and only attaches optional diagnostics
  after the user enables the checkbox. The report excludes document names,
  paths, and SVG contents. No support URL or address existed in the repository,
  so no destination was invented; the user starts the system email composer.

## Verification

- `./dev t --all`: build passed; 59/61 CTests and 69/69 Python tests passed
  (slow Python modules are excluded by this command). The two CTest failures
  are environment-bound: `index-replay` could not obtain an offscreen
  Metal device/queue; `native-viewer-smoke` received LaunchServices and
  hiservices connection failures and could not obtain Metal presentation.
- `python3 -m unittest tests.e2e.test_packaging`: 13 tests passed, including
  alternate SVG registration, the CMake bundle wrapper, ICNS structure, and
  icon source/size checks.
- `cmake --preset mac-release` and
  `cmake --build --preset mac-release --target vexel_app_bundle`: passed.
- `./dev bundle`: passed. The generated `build/Vexel.app` has the expected
  executable, icon, identifier/version, SVG declaration, and a valid ad-hoc
  hardened-runtime signature.
- Focused `welcome-model`, `feedback-diagnostics`, and
  `platform-command-policy` CTests passed. The new `native-shell-goldens` test
  renders Welcome (empty/recent), General and Canvas Settings, Help, Feedback,
  and the app icon offscreen in Aqua and Dark Aqua. All 14 captures match their
  checked-in goldens. The golden test first failed against the pre-fix Welcome,
  Help, and Feedback layouts; it passes after fixing horizontal stack alignment,
  recent-list anchoring, and text wrapping. `git diff --check` passed.
- The existing light/dark editor chrome and style-golden tests pass in the full
  run. No editor canvas or renderer drawing changed. Native captures cover
  content views and the app icon; standard OS titlebar chrome remains part of
  the maintainer review below. Icon package generation is also checked at 16,
  32, 64, 128, 256, 512, and 1024 pixels.

## Maintainer real-window verification

The lane did not automate or capture the shared screen. Before release, check
on macOS 13 and a current macOS release:

- Inspect all menu states with and without a loaded document. Exercise each
  shortcut, SVG and non-SVG clipboard Paste, menu checkmarks, and Cmd+,.
- Review Welcome in light and dark appearance. Open and create a document,
  drag an SVG, reopen/clear recents, remove a missing recent, and launch an SVG
  from Finder. Confirm Open With offers Vexel without changing the default app.
- Inspect Settings pane restoration, reset, and future/current canvas effects.
  Check the app icon in Finder and Dock at small sizes, including system masking
  and light, dark, and tinted contexts.
- Enter/exit full screen with the green button and Control-⌘-F. At 1280×800 and
  a narrow window, check panel restoration, trackpad pan/pinch/modifier-scroll,
  mouse wheel, keyboard fit/zoom, and outline/inspector scrolling.
- Open Help and Feedback from Welcome and the editor. Confirm the composer is
  user-initiated and the preview has no document data; verify Copy Diagnostics
  without an email account and the optional attachment preview.

The requested r5 work-order text is in the external research report; this
checkout has no frozen W15 work orders under `work/`. The ledger validator
requires a frozen work order, so no lifecycle event was added to `ledger.jsonl`.
