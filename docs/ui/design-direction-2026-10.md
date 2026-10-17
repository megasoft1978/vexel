# Vexel UI design direction, 2026-10

Status: proposal for owner review. Extends, does not replace,
[ADR-0017](../adr/0017-ui-direction-blender-aligned.md) (Blender-aligned
structure: workspaces, one status bar, grouped inspector). This document sets
the visual and interaction layer on top: tokens, icons, motion, layout.
Implementation is split into six work orders, `work/WO-RUST-UI-*.md`.

## 1. Goal and principles

Make the custom-drawn Rust editor look like a 2026 creative tool (Figma, Linear,
Rive, Cavalry class) without giving up speed or density.

1. **Quiet chrome, loud canvas.** The document is the brightest, largest thing.
   Chrome recedes through hierarchy, not decoration (Linear).
2. **Elevation by lightness, not shadow.** Dark surfaces step lighter as they rise.
3. **Fixed, resizable panels. No floating panels.** Figma reversed floating UI3
   panels for slowing heavy users; Penpot dropped them too.
4. **Glass only on chrome edges.** Translucent material on toolbar/sidebar is
   optional macOS Tahoe polish, never behind dense text or the canvas.
5. **Legible first.** Nothing below 12 pt; contrast is computed in tests.
6. **Motion confirms, never delays.** 120-180 ms eased feedback; instant on input.
7. **One icon language.** First-party stroke icons, 16 px grid, 1.5 px stroke.
8. **Modes are explicit.** Draw and Animate are a visible segmented switch;
   Animate adds a footer timeline, as in Rive.

## 2. Research findings and sources

| # | Finding | Consequence for Vexel | Source |
|---|---|---|---|
| 1 | macOS 26 (Liquid Glass): toolbars float on glass over content, sidebars are a glass pane, inspectors use edge-to-edge glass; content may extend under the sidebar; group toolbar controls; use badges; avoid glass for primary content and dense text; rounder window corners. | Optional material for toolbar and tool rail only. Group Undo/Redo, Save/Export. Badges for state (unsaved, MCP). Inspector stays opaque. | [WWDC25 session 310](https://developer.apple.com/videos/play/wwdc2025/310/) |
| 2 | Figma UI3: floating panels reversed for full rollout (slowed heavy users, crowded small screens); panels fixed but resizable; toolbar floats at the bottom; too-small type drew complaints. | Fixed resizable panels; min type 12 pt; tool rail stays docked. Bottom floating toolbar is NOT adopted (see 8, Q3). | [Figma blog](https://www.figma.com/blog/our-approach-to-designing-ui3/), [Penpot forum](https://community.penpot.app/t/penpot-discarded-figmas-ui3-approach-more-than-a-year-ago/5995) |
| 3 | Linear: inverted-L global chrome; LCH theme generated from few base variables (perceptual uniformity); more contrast; hierarchy, density, balance; less noise in sidebar/tabs/headers/panels. | Inverted-L = top toolbar + left tool rail. Neutral ramp generated in LCH from 3 inputs. Fewer borders, more surface steps. | [Linear blog](https://linear.app/blog/how-we-redesigned-the-linear-ui) |
| 4 | Blender 5.0: 300+ theme options removed; toolbar/sidebar regions unified; flatter nodes; breadcrumbs; animation editors and sequencer gained a footer with playback and frame-range controls; tooltip padding. | Keep token count small (matches ADR-0017). Timeline gets a transport footer. Padded tooltips. | [Linuxiac](https://linuxiac.com/blender-5-0-released-with-a-ui-overhaul-smarter-nodes-and-hdr-support/), [5.3 notes](https://developer.blender.org/docs/release_notes/5.3/user_interface/) |
| 5 | Rive: Design and Animate modes; Animate splits into timeline (keyframes, easing, graph) and state-machine panels. | Segmented Draw/Animate. Timeline footer with tracks; graph editor and state machine are later. | [Rive features](https://rive.app/features), [UX Collective](https://uxdesign.cc/how-i-create-animation-for-interfaces-7183b3b6482f) |
| 6 | Dark UI: no pure black (#121212 / #1E1F22 style); elevation via 4-6 progressively lighter surfaces (about 5-12% white overlay), not shadows; desaturate accents; text 87/60/38% white; WCAG AA 4.5:1 body, 3:1 large text and UI components. | Surface ramp, accent chroma cap, three text levels, computed AA tests. | [Uxcel elevation](https://uxcel.com/blog/mastering-elevation-for-dark-ui-a-comprehensive-guide-342), [Uxcel 12 principles](https://uxcel.com/blog/12-principles-of-dark-mode-design-627) |
| 7 | Cavalry: node-based motion design, live-preview feedback. | Preview always live while scrubbing; informs the later graph/state-machine phase. | [Review](https://superrendersfarm.com/article/cavalry-motion-design-review-2026) |
| 8 | Icon craft: fixed canvas grid, fixed stroke width, round joins/caps on open paths, centred strokes, minimum spacing between elements, corner radius tied to element size. | 16 px grid, 1.5 px stroke, round caps/joins, 1 px keyline inset, 1 px element gap. Lucide uses 24 px / 2 px; Vexel scales the same rules to 16 / 1.5. | [Lucide icon design guide](https://lucide.dev/docs/icon-design-guide) |
| 9 | Micro-interactions: most feedback 100-200 ms, instant 50-100 ms, complex up to 300 ms; ease-out for entering, ease-in for leaving; avoid linear; honour reduced motion. (Community guides, not standards; treat as starting values to review live.) | Duration and easing table in 4.5. Reduced-motion switch. | [Animation craft note](https://cdn.jsdelivr.net/npm/@memi-design/cli@2.6.2/notes/animation-craft/animation-craft.md), [timing calibration](https://playbooks.com/skills/dylantarre/animation-principles/timing-calibration) |

Not found: authoritative Phosphor weight rules; none are relied on.

## 3. Critique of the current UI

Evidence: `docs/archive/live/2026-10-16-compose-v1-after-live-fixes.png` (Draw),
`2026-10-17-animate-workspace-after-redraw.png` (Animate); layout in
`vexel-app/src/model.rs::Layout::new`, tokens in `vexel-ui/src/tokens.rs`.

1. **Tool rail is the weakest element.** 54 px column of 42 px buttons, each a
   tiny glyph pinned top-left with an 11 pt label bottom-left. Glyph and label are
   off-centre and unrelated in size; the Pen uses a pencil-like glyph. Reads as
   unfinished. Labels at 11 pt are below the legibility floor.
2. **Toolbar is flat and undifferentiated.** Undo, Redo, Save, Export are identical
   ghost rectangles with text only; enabled and disabled look alike; "No history"
   floats unanchored; wordmark, filename and metadata are untreated text.
3. **Draw/Animate switch is two abutting rectangles** with no shared container.
   The selected fill is a dull navy; the state is clear only by comparing two
   boxes. It is the app's primary mode switch and looks like an afterthought.
4. **No elevation system.** Window, rail, inspector and activity panel are within
   a few lightness points and separated by 1 px lines. Panels do not read as
   layers. The native title bar is a warmer grey than the cool blue-grey chrome.
5. **Inspector is a text dump.** "Inspector" title, an empty-state paragraph,
   Fill/Stroke fields showing a dash in an empty box, Position/Matrix as bare
   inputs. No collapsible sections, no chevrons, no swatches, no units, no
   hierarchy beyond bold-vs-regular at 12-13 pt. Width clamps to 208-252 px
   (`model.rs`), narrower than the `inspector_width: 320` metric.
6. **A permanent 70 px "MCP activity" band** with "Unavailable in this shell" and
   a Connect button sits above a second 28 px status bar. Two bottom bars waste
   about 98 px for a rarely used feature and contradict ADR-0017 (one status bar).
7. **Canvas has no presentation.** The document is a white rectangle butted flush
   against the rail and inspector. No well around it, no artboard shadow, no
   checkerboard for transparency, no zoom label or fit/zoom controls.
8. **Animate workspace is a placeholder.** Timeline panel says "No document to
   animate" over an empty bar even with a document open (tracked by
   WO-RUST-V1-ANIMATE-FIX-01). No ruler, tracks, keyframes, playhead or transport.
9. **Type is small and heavy-handed.** `TypeScale` goes down to 11 pt; section
   headings are bold-only; no tabular numerals for geometry readouts.
10. **Colour tokens are named for the past.** Duplicate fields (`panel_alt`,
    `canvas`, `text`, `muted`, `border`) alias newer ones; selection fill
    (#263C58) is low-contrast against panel; success/warning/error exist but are
    barely used.
11. **No hover/press/focus feedback visible** and no transitions (state changes
    are instantaneous and ungrouped).

Keep: the 8 pt grid already in tokens, computed contrast reports, accent
adaptation (`theme_tokens_accented`), semantic tree and keyboard focus.

## 4. Token proposal

All values below are starting points generated by a script (CIE LCH(ab), D65,
hue 265 for neutrals). WO-RUST-UI-TOKENS-01 regenerates them in Rust and asserts
the contrast numbers; if a value fails, tokens change, not the test.

### 4.1 Surface ramp (6 elevation levels, dark)

Neutral: hue 265, chroma 3 (tinted, never pure grey or black).

| Level | Role | LCH L | sRGB |
|---|---|---|---|
| 0 | window / canvas well (lowest) | 9 | #171A1D |
| 1 | panels (inspector, rail, timeline) | 12.5 | #1E2125 |
| 2 | raised: cards, section headers, toolbar | 16 | #25282C |
| 3 | controls: inputs, buttons at rest | 20 | #2E3135 |
| 4 | hover | 24 | #36393D |
| 5 | pressed, popovers, menus, tooltips | 28 | #3F4247 |

Separator = level 4 (#36393D, 1 px, used sparingly). Control edge = L 34
(#4D5055; 1.83:1 on level 2, so edges are decorative only and every control also
has a filled background). Light theme (as implemented in `vexel-ui` tokens, WO-RUST-UI-TOKENS-01): L 98.5 window,
95, 91.5, 88, 84.5, 81 for the following levels (chroma 2, falling with elevation), so in light
mode the window is the lightest surface. Light elevation uses a 1 px edge plus a soft shadow
because lightness cannot rise further.

### 4.2 Text emphasis (dark; contrast on level 1 / level 2)

| Token | LCH L | sRGB | Approx. white alpha | Contrast |
|---|---|---|---|---|
| text.high | 93 | #E9EBEF | 87% | 13.6 / 12.4 |
| text.medium | 70 | #A8ABB0 | 60% | 7.1 / 6.4 |
| text.low (metadata, placeholders) | 60 | #8E9196 | 45% | 5.1 / 4.7 |
| text.disabled (exempt from AA) | 40 | #5B5F63 | 38% | 2.5 |

Light: text.high L 12 (#1D2024, 15.8), medium L 38 (#575A5E, 6.7), low L 43 (4.65:1 on level 2)
(target at least 4.5 on panel, enforced by test), disabled L 65.
Gate: body text at least 4.5:1, text at 18 px or larger and UI component edges
or glyphs at least 3:1 on the surface they sit on.

### 4.3 Accent and semantic colours (desaturated; chroma capped at 50)

| Token | Dark (LCH, sRGB, on level 1) | Light (LCH, sRGB) |
|---|---|---|
| accent | 66/50/275 #5EA3FA, 6.2:1 | 45/50/275 #006DBE, 4.7:1 (L48 measured 4.27:1, below AA) |
| accent.hover | accent L +6 | accent L -5 |
| accent.subtle (selection fill, selected row) | accent at 16% over surface | 12% |
| success | 75/40/160 #6FCA9E, 8.2:1 | L 50 |
| warning | 80/50/80 #F1BE69, 9.5:1 | L 52 |
| error | 68/55/28 #FF7F7A-class, 6.6:1 | L 46 |
| focus ring | accent L +10, 2 px, 2 px offset | accent |

`theme_tokens_accented` keeps adapting to the macOS system accent, now clamped to
the chroma cap and re-checked against 3:1 on levels 1-3. Semantic colours never
carry meaning alone (icon or word accompanies them).

### 4.4 Spacing, radii, type

- Spacing (px, multiples of 4): 2, 4, 8, 12, 16, 24, 32. Control height 28
  (compact) / 32 (comfortable). Hit target at least 28; icon buttons 32.
- Radii: 4 (chips, fields), 6 (buttons), 8 (cards, popovers), 12 (large panels,
  artboard frame). Window corners come from macOS, not drawn.
- Type (system font via CoreText, SF Pro; never below 12 pt):

| Role | pt / weight | Use |
|---|---|---|
| title | 15 / semibold | document name, empty states 20 |
| heading | 13 / semibold | section headers |
| body | 13 / regular | fields, rows |
| label | 12 / medium | buttons, segmented, tabs |
| meta | 12 / regular, text.low | status bar, hints |
| numeric | 12 / regular, tabular figures (monospaced digits) | geometry, time, zoom |

Line height 1.35. No bold-only headings; weight steps are 400/500/600.

### 4.5 Motion

| Token | ms | Easing | Use |
|---|---|---|---|
| instant | 0-80 | none | pointer-driven feedback (drag, scrub): never animated |
| fast | 120 | ease-out (0.2, 0, 0, 1) | hover, press, focus ring |
| base | 180 | standard (0.3, 0, 0.2, 1) | selection, toggle, segmented thumb slide, section expand |
| slow | 240 | ease-out | panel resize snap, popover in |
| exit | 75% of entry | ease-in (0.4, 0, 1, 1) | popover out, tooltip out |

Never linear for UI. Honour reduced motion (NSWorkspace
`accessibilityDisplayShouldReduceMotion`): durations collapse to 0. A transition
may never delay an input response or a document redraw.

### 4.6 Control states

| State | Treatment |
|---|---|
| rest | level 3 fill (buttons, fields); ghost buttons transparent |
| hover | level 4 fill, 120 ms |
| pressed | level 5 fill, glyph shifts nothing, 80 ms in |
| focus | 2 px accent ring at 2 px offset; keyboard only |
| selected | accent.subtle fill + accent glyph/text (segmented: raised thumb) |
| disabled | text.disabled, no hover, no edge |
| invalid | error colour edge plus message line below field |

## 5. Layout proposal

Inverted-L chrome (toolbar + rail) around fixed, resizable panels.

- **Top toolbar**, 44 px, level 2: left traffic-light inset, document name (title)
  with an unsaved dot badge; centre segmented Draw | Animate (28 px tall, sliding
  thumb); right icon groups [Undo Redo] [Save Export] and a panel-toggle pair.
  Grouped controls share one rounded container (WWDC25).
- **Tool rail**, 44 px wide, level 1: 32 px icon buttons, 16 px icons, no
  labels; tooltip with name and shortcut (padded). Groups divided by a 1 px gap
  line: [Select] [Pen Rect Ellipse Line].
- **Canvas well**, level 0, with the artboard centred: soft shadow
  (0 2 12 rgba(0,0,0,.35)), checkerboard under transparency, 1 px hairline.
  Floating-free overlay bottom-left of the well: zoom - / label / + / Fit.
- **Inspector**, right, default 296 px, resizable 240-420, level 1. Collapsible
  grouped sections (chevron, 13 pt semibold header, state persisted):
  Selection, Fill, Stroke, Transform, Animation. Swatch plus hex in colour
  fields; numeric fields with unit suffix and tabular numerals. Edge-to-edge,
  opaque.
- **Footer timeline (Animate only)**, bottom of the centre column, default
  200 px, resizable 120 px to half height, level 1: transport row (32 px), time
  ruler (24 px), track rows (28 px), playhead line in accent.
- **Status bar**, 24 px, level 0: tool hint left, report centre, readouts right
  (zoom, size, save state), plus an MCP indicator dot that opens a popover. The
  70 px MCP band is removed.
- **Material (optional, owner decision Q2)**: toolbar and tool rail may use
  NSVisualEffectView behind them via `vexel-platform-apple`; fall back to the
  opaque level 2/1 fills when Reduce Transparency is on. Never under the canvas,
  inspector or any text-heavy region.

### 5.1 Wireframe: Draw workspace

<!-- desktop: 80ch -->
```
┌──────────────────────────────────────────────────────────────────────────────┐
│ ●●●  compose-v1.svg ●      ┌──────────┬─────────┐       ⟲ ⟳ │ ⤓ ⇪   ▯ ▯      │
│                            │▌ Draw   ▐│ Animate │                     [1]    │
├────┬───────────────────────┴──────────┴─────────┴──────────────┬─────────────┤
│ ▲  │                                                           │ Selection ▾ │
│ ✎  │          ┌──────────────────────────────┐                 │  Rect 3     │
│ ▭  │          │░ checker / artboard (shadow) │                 │ ─────────── │
│ ○  │          │                              │                 │ Fill      ▾ │
│ ╱  │          │      [ rect ]  (o)  (O)      │                 │  ■ #E53935  │
│    │          │      ◢◣        ◡     ◣       │                 │ Stroke    ▾ │
│    │          │                              │                 │  ■ #1A237E  │
│    │          └──────────────────────────────┘                 │ Transform ▸ │
│    │   [2] ┌─────────────────────┐                             │ Animation ▸ │
│    │       │ − │ 100% │ + │ Fit  │                             │             │
├────┴───────┴─────────────────────┴─────────────────────────────┴─────────────┤
│ Select: click to pick, drag to move        Ready        100% · 160x120 · ●MCP│
└──────────────────────────────────────────────────────────────────────────────┘
 [1] grouped icon buttons   [2] zoom controls over canvas well   ▸ collapsed  ▾ open
```

### 5.2 Wireframe: Animate workspace

<!-- desktop: 80ch -->
```
┌──────────────────────────────────────────────────────────────────────────────┐
│ ●●●  compose-v1.svg ●      ┌──────────┬─────────┐       ⟲ ⟳ │ ⤓ ⇪   ▯ ▯      │
│                            │  Draw   │▌Animate ▐│                            │
├────┬───────────────────────┴──────────┴─────────┴──────────────┬─────────────┤
│ ▲  │                                                           │ Selection ▾ │
│ ✎  │        ┌──────────────────────────────┐                   │  Circle 2   │
│ ▭  │        │ artboard at playhead (live)  │                   │ Animation ▾ │
│ ○  │        │        ◉  ◜───►             │                   │  dur  2.0 s  │
│ ╱  │        └──────────────────────────────┘                   │  ease out   │
│    │                                                           │ Fill      ▸ │
├────┴───────────────────────────────────────────────────────────┴─────────────┤
│ ⏮  ◀  ▶  ⏭   0:00.8 / 2:00   ⟳ loop   1x            [ease ▾]   [+ key]       │
│ track        ├0s────────1s────────2s────────3s──────────4s┤  [3]             │
│ ▾ circle 2   │   ◆─────────◆──────────────◆                                  │
│    cx        │   ◆───────────────◆                                           │
│ ▸ rect 3     │      ◇                                                        │
│              │          ┃ playhead                                           │
├──────────────┴───────────────────────────────────────────────────────────────┤
│ Animate: Space plays, K adds a key             Ready          0.8 s · 24 fps │
└──────────────────────────────────────────────────────────────────────────────┘
 [3] ruler; ◆ keyframe, ◇ selected keyframe; footer height resizable (200 px)
```

## 6. Work orders

Disjoint allowed paths so workers can run in parallel.

| Order | Owns | Needs merged first |
|---|---|---|
| WO-RUST-UI-TOKENS-01 | `vexel-ui` tokens, theme plumbing | none |
| WO-RUST-UI-ICONS-01 | `vexel-ui` icon files | none |
| WO-RUST-UI-MOTION-01 | `vexel-ui` motion/interaction files, loop hook | TOKENS |
| WO-RUST-UI-CANVAS-01 | `vexel-app` canvas, `compose/` overlay | TOKENS |
| WO-RUST-UI-TIMELINE-01 | `vexel-app` timeline files | TOKENS, ICONS, V1-ANIMATE-FIX-01 |
| WO-RUST-UI-CHROME-01 | `vexel-app` model/render/layout/chrome/inspector | TOKENS, ICONS (MOTION optional) |

Every order requires a human-viewed screenshot recorded in `docs/archive/live/`
before goldens are generated, and a fresh read-only reviewer.

## 7. Risks

- First-party goldens are regenerated, never edited to pass; review images are
  stored beside them. Frozen C oracles are untouched.
- Material needs `vexel-platform-apple` changes outside these orders (Q2).
- Raising the minimum font to 12 pt and the inspector to 296 px lowers the
  minimum usable window; `Layout::new` thresholds are re-derived in CHROME.


## Implementation notes (2026-10-17, from the independent review of WO-RUST-UI-TOKENS-01)

Measured against the first proposal: dark `text.high` is L95 `#EEF1F4` (L93 gave 13.54:1, below the 14:1 pin; L95 gives 14.26:1 on level 1); the dark focus ring is accent L+10 (`#ADD9FF`); light success, warning and error are L45; the light accent is L45. All values were independently recomputed (CIE LCH(ab), D65 to sRGB, WCAG relative luminance) and match the code.

Icons (2026-10-17, from the review of WO-RUST-UI-ICONS-01): the stroke is nominally 1.5 units on the 16 px grid but is rendered at an even device width (2 px at 1x, 4 px at 2x, 6 px at 3x) because a true 1.5 px stroke cannot be crisp at 1x and also scale cleanly at 2x/3x (the order's own AC2 and AC4 conflict at 1.5). `snapped_width` is the asserted value. Seven icons are weak and are to be redrawn in a follow-up before the chrome uses them: pause, gradient, timeline, eyedropper, ruler, actual_size, group (pen and node_edit are also noisy at 1x).
