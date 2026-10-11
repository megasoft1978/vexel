# Animation timeline contract

This document freezes the first source-faithful timeline surface for the
supported SVG SMIL subset. The dock is a preview and inspection surface. The
current implementation does not mutate SVG source or create keys.

## Source and row identity

- A track is identified by its target source span and animation-element source
  span. SVG `id` is display text only; missing or duplicate IDs never merge
  tracks.
- Tracks for one target appear together in source order. Each animation element
  remains a distinct lane, including duplicate animations of the same
  attribute. The lane label includes the target and animated attribute; a
  repeated attribute receives an ordinal marker.
- `<animate>`, `<animateTransform>`, `<animateMotion>`, and `<set>` use the
  parsed track values and timing. Unsupported tracks remain visible, disabled,
  and paired with their parser diagnostic. Path morphs remain read-only when
  they cannot be previewed by the renderer.
- Key diamonds use parsed key times. `discrete` lanes use step marks;
  `linear`, `spline`, and `paced` retain their source interpolation. A spline's
  outgoing interval owns its curve. No generic ease curve is implied.
- Repeats, fill, begin/end, and syncbase timing remain source data. One Shot,
  Loop, Ping-Pong, and speed are preview controls and never rewrite SMIL.
  Indefinite source duration is shown as `∞`; its initial finite preview range
  is 0–5 seconds.

## Time and interaction

- The ruler uses continuous seconds. The document gains no frame-rate field,
  and time is not quantized by default. `[` and `]` keep the existing 1/30
  second step.
- Scrubbing seeks immediately and pauses preview. Clicking or dragging a key
  diamond seeks to its time; its hit target stays a fixed screen-space size as
  the ruler zoom changes.
- Command-scroll zooms around the pointer. Horizontal scroll pans. Fit shows
  the complete preview work range. The visible work range is preview-only and
  does not change source timing.
- Clicking a lane selects its source target in the structure/editor; selecting
  the target in the canvas or structure selects the matching lane. The current
  time and selection remain continuous across these changes.
- Add Key is an explicit authoring command for a selected property at the
  playhead. Static inspector edits never create animation implicitly. No
  auto-key mode is enabled by default. Key mutation is outside this preview
  work order and must use the transactional source-preserving edit/undo path.
- The dock opens on first load when supported SMIL tracks exist. Its open
  height starts at 220 points and is resizable from 184–320 points. The
  collapsed dock is 32 points. The toolbar toggle remembers the user's choice
  while documents change. At constrained window heights, the dock collapses
  before the canvas work area is reduced below its minimum.

## Bounds and accessibility

- The source model remains capped at 128 tracks and 256 keys per track. The UI
  computes only visible rows plus bounded overscan; it never creates retained
  nodes per source track or key. Labels shown in rows stay within the existing
  64-byte widget limit. Full source identity and diagnostics remain available
  to accessibility and the inspector.
- Transport controls have explicit Play/Pause, mode, speed, Fit, and collapse
  names. The time slider reports current and total/preview duration. Disabled
  tracks announce their unsupported diagnostic; the selected row is exposed as
  selected.
- Timeline controls never modify source bytes. Unknown XML/SVG markup and
  namespaces remain untouched. No external SVG resource is fetched.

## Layout sketches

### Wide window, dock open

```text
┌────────────────────────────────────────────────────────────────────────────┐
│ Toolbar: Save  Undo  Redo  …  Layers  Inspector  Timeline                 │
├────────────────────────────────────────────────────────────────────────────┤
│ Tool rail │ Layers │             Canvas             │ Inspector             │
│           │        │                                  │                       │
├────────────────────────────────────────────────────────────────────────────┤
│ Timeline     [Play] [0.00 / 5.00 s] [One Shot] [1×] [Fit]        [Collapse]│
│              0s        1s        2s        3s        4s        5s           │
│ Track name   │    ◆──────────────◆                 │ playhead               │
│ rect#logo/x  │    ◆──────◆                         │                        │
│ group#mark/y │             ◆────────────◆          │                        │
├────────────────────────────────────────────────────────────────────────────┤
│ Status: Ready · Saved                                      No selection   │
└────────────────────────────────────────────────────────────────────────────┘
```

### Wide window, collapsed

```text
┌────────────────────────────────────────────────────────────────────────────┐
│ Timeline                                                        [Expand]   │
├────────────────────────────────────────────────────────────────────────────┤
│ Status: Ready · Saved                                      No selection   │
└────────────────────────────────────────────────────────────────────────────┘
```

### Selected-key inspection state (authoring follow-up)

```text
┌────────────────────────────────────────────────────────────────────────────┐
│ Timeline  [Pause] [1.25 / 5.00 s] [Loop] [0.5×] [Fit]                      │
│ rect#logo / opacity     0s ───────◆────────◆────────◆──────── 5s           │
│                                   ▲ selected key · outgoing interval        │
│ Inspector: Key time 1.25 s · value 0.65 · Linear                           │
└────────────────────────────────────────────────────────────────────────────┘
```

### Narrow window

```text
┌──────────────────────────────────────────────────────┐
│ Toolbar … Timeline                                   │
├──────────────────────────────────────────────────────┤
│ Layers │            Canvas             │ Inspector   │
├──────────────────────────────────────────────────────┤
│ Timeline [Play] [0.0s] [Loop] [1×] [Fit] [Collapse] │
│ rect#logo/x │ 0s      1s      2s      3s             │
│ group/y     │     ◆────────◆                          │
├──────────────────────────────────────────────────────┤
│ Status                                             │
└──────────────────────────────────────────────────────┘
```

The time grid may scroll and zoom, while the name gutter remains fixed. On a
window too short to retain the canvas minimum, the same dock is shown in its
32-point collapsed state.

## Appearance and motion

Use the existing opaque light/dark panel tokens. Routine hover, focus, row
selection, scrubbing, and redraw are immediate. The dock does not autoplay and
does not animate open/closed. System Reduce Motion updates the editor's
appearance state and suppresses decorative chrome motion; explicit Play is
still available and pausable because it previews user-authored SVG motion.
