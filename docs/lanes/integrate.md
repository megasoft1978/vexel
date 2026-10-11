# W16 lane integration

Branch: `integrate/w16`, based on `main`. The base already contains
`lane/w14-windows` (`7735f51`). The nine requested lanes were merged one at a
time in this order; each merge was followed by `./dev t --all` and a merge
commit with the requested message.

## Merge log

| # | Lane | Merge commit |
|---:|---|---|
| 1 | `lane/w14-rulers` | `0ca89f0` |
| 2 | `lane/w14-palette` | `b9f8386` |
| 3 | `lane/w14-select` | `ec98f42` |
| 4 | `lane/w14-nodes` | `7f4d935` |
| 5 | `lane/w14-timeline` | `7ac6387` |
| 6 | `lane/w14-keyframes` | `5cec452` |
| 7 | `lane/w15-visual` | `668ccb7` |
| 8 | `lane/w15-native` | `253e6c3` |
| 9 | `lane/w15-a11y` | `18137a6` |

## Conflict resolutions and semantic integration

- `CMakeLists.txt` keeps all lane source and test registrations. The lane
  sections were recombined with balanced conditionals. Debug and Release
  configurations build the integrated target set.
- Platform action enums and menu dispatch retain the W14 editing, canvas,
  ruler, guide, timeline, and window actions alongside the W15 native shell
  and accessibility actions. `viewer_events.c` dispatches both platform
  actions and editor commands; the document load path remains in the shared
  viewer runtime.
- `viewer_runtime.c` remains the only document/session owner. W15 native
  Welcome, recent-file, Dock-menu, and native-window entry points use that
  runtime and the shared recents model. The launch flow enters the native
  Welcome window and returns to the W14 home/empty-document state when Welcome
  closes; there is no second independent document or recents store.
- Timeline and keyframe editing now use one docked timeline. The keyframe
  lane's source-preserving edit operations are reachable from the timeline,
  including undo/redo and easing changes; the integration keeps the timeline's
  selection and edit state in the existing editor session.
- Visual scale and accessibility text size share one `VexelUIScalePercent`
  preference, editor value, and View menu. The supported choices are 100, 110,
  120, 125, 150, 175, and 200 percent. Text and layout scale together; canvas
  zoom remains separate. This removes duplicate scale persistence.
- Native menu strings are localized from `resources/en.lproj/Localizable.strings`.
  The packaging helper copies that resource into the signed app bundle, and the
  CMake app-bundle target installs the same table. The test map retains all lane
  test registrations. `STATE.md` and `LEDGER.md` retain prior entries;
  `ledger.jsonl` validates with 412 entries. No lifecycle entry was added
  because there is no frozen W16 work order under `work/`.
- The command palette has one active `Search Commands…` item in Help with
  `⌘K`; the shortcut-discovery submenu omits its duplicate disabled listing.
  Cycle Background now has both its existing `B` key and advertised `⌘B` menu
  binding. Pixel Grid remains plain `G`, separate from Group's `⌘G`.
- The frozen Align Center binding is `⌥⌘H`. The native macOS “Hide Others”
  action moved to `⇧⌥⌘H` to remove the collision while retaining both actions.
- The merged tooltip painter exposed a stack-use-after-scope under ASan: its
  draw spec retained a pointer to loop-local tooltip storage. The buffer now
  lives through the final paint call. The packaging helper and integrated
  chrome goldens were also corrected during final verification.
- Overlay ownership remains split by UI layer: canvas zoom/ruler/guide
  overlays use the viewer overlay pass; command/history popovers use the editor
  overlay pass; docked timeline and inspector content use the editor chrome
  pass. Combined chrome and accessibility goldens pass with the integrated
  layout.

## Shortcut table

No chord is assigned to two unrelated actions in the same focus context.
Multiple chords in a cell are aliases for the same action. When a text field
is active, field editing gets first chance to handle keys before canvas
shortcuts. Modifier
distinctions are intentional: for example,
`G` toggles Pixel Grid while `⌘G` groups a selection, and `T` selects the Text
tool while `⌘T` toggles the checkerboard.

| Shortcut | Action / context |
|---|---|
| `V` | Select tool |
| `H`; hold `Space` | Pan tool; temporary canvas pan |
| `Z` | Zoom tool |
| `R` | Rectangle tool |
| `O` | Ellipse tool |
| `L` | Line tool |
| `Y` | Polygon tool |
| `U` | Polyline tool |
| `P` | Pen tool |
| `N` | Node-edit tool |
| `T` | Text tool |
| `⌘Z` | Undo |
| `⇧⌘Z` | Redo |
| `⌘A` | Select All; selects all text when a text field is active |
| `⌘C` | Copy SVG selection |
| `⌘X` | Cut SVG selection |
| `⌘V` | Paste SVG |
| `⇧⌘V` | Paste in Place |
| `⌥⌘C` | Copy PNG |
| `⌘D` | Duplicate selection |
| `Delete`; `Forward Delete` | Delete selection/node; in a text field, delete before/after the caret |
| `⌘G` | Group |
| `⇧⌘G` | Ungroup |
| `⌘]` | Bring Forward |
| `⌘[` | Send Backward |
| `⇧⌘]` | Bring to Front |
| `⇧⌘[` | Send to Back |
| `⌥⌘←` | Align Left |
| `⌥⌘H` | Align Center |
| `⌥⌘→` | Align Right |
| `⌥⌘↑` | Align Top |
| `⌥⌘V` | Align Middle |
| `⌥⌘↓` | Align Bottom |
| `W` | Fit Width |
| `⌘0`; `0` | Fit in Window |
| `⌘1`; `1` | Actual Size |
| `⌘+`; `⌘=`; `+`; `=` | Zoom In |
| `⌘−`; `−` | Zoom Out |
| `⌘'` | Toggle Snapping |
| `⌘T` | Toggle Checkerboard |
| `B`; `⌘B` | Cycle Background |
| `G` | Toggle Pixel Grid |
| `⌘O` | Open SVG |
| `⌘S` | Save |
| `⇧⌘S` | Save As |
| `⌘R` | Reload |
| `⇧⌘E` | Export PNG |
| `⌘K` | Search Commands / open command palette |
| `⌘N` | New SVG document |
| `⇧⌘N` | New Window |
| `⌘W` | Close document/window |
| `⌘,` | Settings |
| `⌘Q` | Quit Vexel |
| `⌘H` | Hide Vexel |
| `⇧⌥⌘H` | Hide Others |
| `⇧⌘?` | Open Vexel Help |
| `⌃⌘F` | Enter or exit Full Screen |
| `⌃⌘[` | Show Previous Tab |
| `⌃⌘]` | Show Next Tab |
| `⌘M` | Minimize Window |
| `Tab`; `⇧Tab` | Move focus forward/backward in editor controls |
| `Return`; `Enter` | Activate focused control or commit a text field |
| `Escape` | Cancel an edit or close the active popover |
| Arrow keys | Navigate focused outline, palette, and timeline controls; otherwise navigate canvas |
| `Shift` + Arrow | Move selected geometry or path node by 10 units; plain arrows move by 1 |
| `Option` + Left/Right | Move by word in an active text field |

## Final test results

| Command | Result |
|---|---|
| `./dev t --all` | PASS: build; 79/79 CTests; 69/69 Python tests |
| `./dev t --full` | PASS: 79/79 CTests; 66/66 Python tests |
| `./dev verify` | Debug CTest PASS 79/79 and Debug/Release builds succeed. Command exits 1 in the full Python phase: 291/302 pass, 8 fail, 2 error, 1 skip. |
| `./dev asan` | PASS: 79/79 CTests, no sanitizer reports; 3 host-gated CTests skipped |
| `./dev ubsan` | PASS: 79/79 CTests, no sanitizer reports; 3 host-gated CTests skipped |
| `python3 scripts/ledger.py validate ledger.jsonl` | PASS: 412 entries |
| `git diff --check` | PASS |
| `python3 -m json.tool tests/test_map.json` | PASS |
| Source line-count audit | PASS: all C, Objective-C, and header files are below 800 lines |

The `./dev verify` Python failures are environmental: seven benchmark harness
tests cannot start `/usr/bin/sandbox-exec` because the outer execution sandbox
returns `sandbox_apply: Operation not permitted`; the native smoke test cannot
use LaunchServices/HIServices; and the two Metal tests cannot obtain an offscreen
Metal device or queue. The frozen tests were not changed. CTest's three skipped
checks are `mcp-stdio-e2e`, `native-viewer-smoke`, and `index-replay`.

## Maintainer real-window verification

No real application window was driven or captured during this integration.
Please verify these interactions on the target macOS desktop:

- Launch Welcome, open and close documents, return to the home/empty state, and
  confirm tabs, multiple windows, recent files, and the Dock recent menu share
  one session/history flow.
- Check native menu labels, enablement, the `⌘K`, `⌘B`, and
  `⇧⌥⌘H` bindings, Full Screen, and the selected interface-scale menu item.
- Traverse editor controls with Tab/Shift+Tab and VoiceOver. Check layer
  actions, localized status/dialog strings, and text/layout scales at 100–200%.
- Edit keyframes and easing from the docked timeline; confirm edits, selection,
  undo/redo, and the canvas preview stay synchronized.
- Review the canvas HUD, rulers/guides, command/history popovers, timeline, and
  inspector at common window sizes and scale settings for readable text and
  unobstructed controls.
