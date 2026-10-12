# UI review: aligning the editor chrome with Blender's current interface

Owner-requested review, 2026-10-12. Method: inspect the committed UI fixtures and
renderers, read the chrome/layout code and design tokens, and compare against
Blender's current interface conventions (4.2 LTS → 5.3) from official sources.

Evidence inspected:

- `docs/ui/chrome-dark.png` (real 2784×1888 editor chrome on a live document)
- `tests/ui/command-overlay-golden/palette-dark.png` (editor + command palette)
- `tests/ui/uikit-golden/components-dark-1x.png` (25 components × 7 states)
- `tests/ui/timeline-golden/timeline-dark-1x.png` (docked timeline)
- `tests/ui/native-shell-golden/settings-general-dark.png` (AppKit settings)
- `src/app/viewer_editor_internal.h` layout constants, `docs/ui/editor-interaction.md`
- Blender: [window system](https://docs.blender.org/manual/en/5.3/interface/window_system/introduction.html),
  [workspaces](https://docs.blender.org/manual/en/5.3/interface/window_system/workspaces.html),
  [areas](https://docs.blender.org/manual/en/5.3/interface/window_system/areas.html),
  [regions](https://docs.blender.org/manual/en/5.3/interface/window_system/regions.html),
  [status bar](https://docs.blender.org/manual/en/5.3/interface/window_system/status_bar.html),
  [tool system](https://docs.blender.org/manual/en/5.3/interface/tool_system.html),
  [properties editor](https://docs.blender.org/manual/en/5.3/editors/properties_editor.html),
  [outliner interface](https://docs.blender.org/manual/en/5.3/editors/outliner/interface.html),
  [operators](https://docs.blender.org/manual/en/5.3/interface/operators.html),
  [undo & redo](https://docs.blender.org/manual/en/5.3/interface/undo_redo.html),
  [HIG paradigms](https://developer.blender.org/docs/features/interface/human_interface_guidelines/paradigms/),
  [HIG colour](https://developer.blender.org/docs/features/interface/human_interface_guidelines/color/),
  [HIG tooltips](https://developer.blender.org/docs/features/interface/human_interface_guidelines/tooltips/),
  [HIG dialogs](https://developer.blender.org/docs/features/interface/human_interface_guidelines/dialogs/).

## 1. What the current UI does well

Recorded honestly so the rework does not throw it away:

- **A real design system exists.** `tests/ui/uikit-golden/components-dark-1x.png`
  shows 25 components across rest/hover/pressed/focus/disabled/selected/invalid,
  with computed contrast enforcement
  (`docs/ui/uikit-contrast-report.json`, text ≥ 4.5:1, functional edges ≥ 3:1).
  Blender only formalised this in 5.0; we already have it.
- **The command palette is genuinely good.** `palette-dark.png` shows a title,
  focus-ringed search field, category prefix, shortcut badges (`⌘G`, `⇧⌘G`) and
  a keyboard footer (`↑↓ Navigate · Return Run · Esc Close`). That is Blender's
  `F3` Menu Search, and it is the single most valuable pattern we already have.
- **Quantitative editor state is surfaced.** The zoom HUD reads
  `1069% zoom · 128 × 96 · Sharp 30/70`, and the palette state shows
  `0% zoom · Sharp ready`; the status bar shows `Ready · Saved` and
  `1 layer selected`.
- **Selection is synchronised across surfaces.** The outline row `Rectangle #card`
  is highlighted while the inspector header reads `Rectangle`, so canvas, tree and
  inspector agree — the "one selection, every surface" property.
- **macOS-native where it should be.** Welcome, Settings and file dialogs are
  AppKit; the canvas and chrome are custom-drawn as the ADRs require.

## 2. Defects found

### A. Layout and density

| # | Defect | Evidence |
|---|---|---|
| A1 | Panels consume 600 of 1280 points (left 232 + rail 44 + right 320), leaving the canvas ~53 % of the window at the default size | `EDITOR_LEFT_WIDTH 232`, `EDITOR_RIGHT_WIDTH 320`, `EDITOR_TOOL_RAIL_WIDTH 44` |
| A2 | Row/field density is "comfortable" only: tree rows ≈ 36 pt, inspector fields ≈ 30 pt. Blender runs ≈ 20–24 pt and its 4.3 release raised the max resolution scale to 3.0× because density matters | `chrome-dark.png`, `components-dark-1x.png` |
| A3 | Only 20 outline rows (`EDITOR_OUTLINE_VISIBLE 20`) and 10 attribute rows (`EDITOR_ATTRIBUTE_VISIBLE 10`) are addressable at once | `viewer_editor_internal.h` |
| A4 | There is no compact/comfortable density preference; the only scale control is the global UI scale shared with accessibility | `VexelUIScalePercent` (100–200 %) |

### B. Structure and navigation

| # | Defect | Evidence |
|---|---|---|
| B1 | **There are no workspaces.** One fixed layout serves drawing, node editing, animation and inspection. Blender's topbar tabs (`Layout`, `Modeling`, `Animation`, …) are its primary organising idea | no user-facing workspace concept in `src/app/**`; `ui_rect workspace` in `viewer_chrome.h` is only a rectangle |
| B2 | **There is no pane model.** Panels are always present at fixed widths; no split/join, no per-pane content switcher, no Maximize/Focus mode | `EDITOR_LEFT_WIDTH`/`EDITOR_RIGHT_WIDTH` are constants; no split/join code |
| B3 | The inspector is one long stack (Name, Geometry, Appearance, Additional attributes) rather than tabbed groups. Blender's vertical icon tabs are how a large property set stays navigable | `chrome-dark.png` right pane |
| B4 | The timeline is a single docked panel with no independent show/hide tied to a task; it is not visible at all in the default chrome render | `chrome-dark.png` vs `timeline-dark-1x.png` |

### C. Discoverability

| # | Defect | Evidence |
|---|---|---|
| C1 | The tool rail is 10 icon-only buttons with no labels, no groups, no variant indicator, and no per-tool options surface. Blender shows tool groups with a corner triangle, can widen to icon+text, and exposes **Active Tool** settings in three places | `chrome-dark.png` left rail; `docs/ui/editor-interaction.md` lists `V H Z R O L Y U P N T` |
| C2 | The toolbar is 8 unlabelled icons with no grouping separators and no tooltips visible in any golden; Blender's own icon guidance says labels are the default and every label-less icon needs a tooltip, including the reason a control is disabled | `chrome-dark.png` top row |
| C3 | **The tool hint is duplicated.** A full-width hint row sits under the toolbar ("Selection tool · choose a layer or edit it on the canvas · V") *and* the status bar repeats state ("Ready · Saved", "No selection"). Blender has exactly one status bar with left=hints, centre=progress, right=readouts | `chrome-dark.png`, `palette-dark.png` |
| C4 | The palette cannot show menu paths or accept a user-curated favourites set; Blender's `F3` shows where each operator lives and `Q` gives Quick Favorites | `palette-dark.png` shows category labels only |
| C5 | There is no keyboard reference surface; only the current tool's key is shown | `docs/ui/editor-interaction.md` |

### D. State, feedback and errors

| # | Defect | Evidence |
|---|---|---|
| D1 | With nothing selected the inspector shows a *Document* heading with placeholder-disabled `Width 64` / `Height 64` and two buttons. There is no explicit empty state saying "no selection"; the Document fields appear in the object slot. In the live chrome the same pane shows document width/height, so the pane's subject is ambiguous | `palette-dark.png` right pane vs `chrome-dark.png` |
| D2 | There is **no Adjust Last Operation** surface. Operations are applied and the parameters are gone; Blender's `F9` head-up panel ("act, then tune") is the pattern that makes destructive-feeling operations safe | no such code; `palette-dark.png` has no redo/adjust affordance |
| D3 | There is **no operator/command log**. Undo history exists as a popover, but warnings, errors and executed commands have no persistent home; Blender routes all four message classes through the Info editor | no log pane; `history-dark.png` is undo history only |
| D4 | The status bar has no centre region for progress or transient reports, and no job-cancellation affordance, although long operations exist (open, index, export, render) | `EDITOR_STATUS_HEIGHT 24`, status text is a single string |
| D5 | The window title carries the zoom percentage (`vexel — chrome-showcase.svg — 1069%`) while the zoom HUD shows the same value on the canvas | `chrome-dark.png` |

### E. Consistency

| # | Defect | Evidence |
|---|---|---|
| E1 | **Two visual languages coexist.** The editor is custom-drawn flat dark chrome with its own type scale; Settings/Welcome are stock AppKit (segmented control, checkboxes, 13 pt labels). They do not read as one application | `chrome-dark.png` vs `native-shell-golden/settings-general-dark.png` |
| E2 | The tree row shows `Selected on canvas`, `Hidden`, `Locked` as free text values rather than fixed state columns; the state vocabulary is not visually consistent between the row, the tooltip and the status bar | `src/app/viewer_chrome_structure.c` (row state assembly) |
| E3 | Row state is communicated by icon opacity and colour alone in places, which Blender's HIG explicitly forbids ("avoid using color as the only way of communicating status") | `chrome-dark.png` eye/lock icons |
| E4 | The palette's first result row is visually indistinguishable from a group header | `palette-dark.png` |

### F. Accessibility and input

| # | Defect | Evidence |
|---|---|---|
| F1 | Numeric fields do not support Blender-style drag-to-scrub, `Ctrl` snap, `Shift` precision, or a mixed-value "X" state for multi-selection | numeric fields are plain text fields in the widget kit |
| F2 | There is no `Alt`-apply-to-all-selected for a property change, so editing 20 selected objects means 20 edits | multi-selection editing exists (`Cmd+A`, 128 elements) but not property-wide application |
| F3 | Icon hit targets are small in places: outline visibility is 16 pt wide, row actions 34 pt | `EDITOR_OUTLINE_VISIBILITY_WIDTH_POINTS 16`, `EDITOR_OUTLINE_ACTION_WIDTH_POINTS 34` |

## 3. Blender patterns we should adopt

Ordered by value to a 2-D SVG editor. "P0" = do first.

| Priority | Blender pattern | Vexel change | Effort |
|---|---|---|---|
| **P0** | Workspaces as topbar tabs | `Draw · Nodes · Animate · Inspect` task layouts that switch pane visibility + active inspector tab; persisted; cycle with `⌃⇥` | medium |
| **P0** | One status bar: hints · progress · readouts | delete the duplicated hint row; left = active-tool hint, centre = progress + transient reports + cancel, right = selection/zoom/document readouts | small |
| **P0** | Command palette as the universal entry point | extend the existing palette: menu path as secondary text, favourites group, no result row styled as a header | small |
| **P1** | Properties vertical icon tabs | `Object · Fill · Stroke · Document · Export` icon strip with one active group, `⌃⇥`-style cycling and a Display Filter | medium |
| **P1** | Active Tool / Tool Settings region | a footer strip under the canvas showing the active tool's options (corner radius, polygon sides, pen mode, snap) | medium |
| **P1** | Adjust Last Operation (`F9`) | a dismissible post-action panel for primitives, path ops, transforms and export, re-editable until the next undoable interaction | medium |
| **P1** | Density control | compact/comfortable density token set independent of the accessibility UI scale; default compact | small |
| **P2** | Info editor as operator log | a collapsible Log pane: one line per operation plus warnings/errors, click to repeat, copy as attribute snippet | medium |
| **P2** | Outliner row anatomy + three orthogonal toggles | visible / locked / exported as fixed columns with names, `⇧`-click applies to the subtree, plus an "Unused/Orphans" tree mode with purge | medium |
| **P2** | Asset/reference library | symbol, gradient, pattern and style library with catalogs, thumbnails and `Sort By Name \| Category` | large |
| **P2** | Numeric field behaviour | drag-scrub, `⌃` snap, `⇧` precision, `Tab` traverse, `⌫` reset, mixed-value "X", `⌥` applies to all selected | medium |
| **P3** | Maximize / Focus mode | `⌃⌘F` hides both side panels and the timeline; second press restores | small |
| **P3** | Pane resize with snapping | keep the existing splitters; add `⌃`-snap and double-click-to-reset | small |

## 4. What we will deliberately not copy

From Blender's own anti-patterns (see the HIG citations above):

1. **No keymap editor.** Ship a fixed, documented map plus a preference for the
   few real conflicts (`Space` = pan vs play). Blender needs presets because its
   keymap is enormous and context-dependent.
2. **No editor-local shortcuts.** `⌘`-based commands stay global; only canvas
   gestures are contextual. Blender's manual admits `Ctrl-J` can silently do
   nothing depending on where the mouse is.
3. **No more than six panes.** Canvas, Layers, Properties, Timeline, Nodes/Inspect,
   Preview. Extra views become tabs inside a pane, not new pane types.
4. **No five-flag visibility model.** Three orthogonal toggles only
   (visible / locked / exported).
5. **No right-click-select mode.** On macOS: left-click selects, right-click opens
   the context menu, always.
6. **No hidden modal grammar.** With a visible bounding box and handles, transforms
   need no `G`/`R`/`S` modal alphabet; keep `Esc` cancel, `Return` confirm,
   `⇧` constrain, typed numbers during a gesture.
7. **No pie menus in v1.** Blender caps them at 8 items and forbids nesting; our
   audience comes from Illustrator/Inkscape.
8. **No theme matrix.** Blender 5.0 deleted 300+ theme settings because
   per-editor theming was unmaintainable. We keep ~10 semantic tokens, dark/light,
   following the system appearance.
9. **No confirmation dialogs as the safety net.** Prefer undo, atomic save and
   overwrite-to-trash; the existing atomic-save work already gives us this.

## 5. Programme

The rework is a programme, not one change. Proposed order:

1. **WO-UI-01** — workspace tabs + single status bar (this review's P0 pair).
2. **WO-UI-02** — properties icon tabs + Display Filter + pin.
3. **WO-UI-03** — tool options footer + tool rail groups/labels/tooltips.
4. **WO-UI-04** — Adjust Last Operation panel.
5. **WO-UI-05** — density tokens (compact/comfortable) applied across chrome.
6. **WO-UI-06** — Log pane (operator + warning/error history).
7. **WO-UI-07** — outliner row anatomy, three state columns, Unused/Orphans mode.
8. **WO-UI-08** — numeric field behaviour (scrub, snap, multi-value, apply-to-all).
9. **WO-UI-09** — asset/reference library.
10. **WO-UI-10** — Maximize/Focus mode + resize snapping.

Each step keeps the existing golden discipline: deterministic headless renders in
dark and light at 1× and 2×, semantic-tree assertions for accessibility, and no
change to the frozen oracle/golden files outside its own new fixtures.

## 6. Constraints this review does not change

- macOS-only, C17, custom-drawn UI, no new dependency, MPL-2.0
  (`OWNER-DIRECTIVES.md`, `docs/adr/0001`).
- Source document bytes remain untouched during interaction; every UI action that
  edits the document still goes through `document_edit` with one undo command.
- The palette, rulers, guides, snapping, timeline and accessibility work already
  merged stay; this is a chrome and structure rework, not a rewrite.
