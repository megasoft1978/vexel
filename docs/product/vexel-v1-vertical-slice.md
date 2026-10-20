# Vexel V1: one character, one loop

## Outcome

A person can open Vexel, create or import a small vector character, revise it,
animate one short loop, save and reopen the editable `.vexel` project, and
export an animated GIF. Codex can make the same drawing and animation edits in
the open project over authorized local stdio MCP, while Vexel shows the changes
and their results.

V1 is an early milestone in the unchanged full product goal. It demonstrates
all product pillars in a narrow workflow; it does not claim full Inkscape parity
or final release readiness.

## V1 cutline

| Capability | Included in V1 | Later full-product work |
|---|---|---|
| Drawing | One canvas; select, move, resize and recolor simple paths and shapes; one small text-free character | Advanced path/node tools, layers, precision tools and general asset management |
| SVG/Inkscape | Import and export one disclosed subset: paths, basic shapes, groups, transforms, solid fills and strokes; preserve untouched unknown source where possible and report unsupported content | Broad SVG/CSS/effect fidelity, large corpus acceptance and advanced round-trip editing |
| Animation | One short loop with a small number of position or opacity keys; scrub, play and export GIF using one preset | Rich authoring, onion skin, multiple animation styles and advanced export controls |
| Native project | Save and reopen one editable `.vexel` containing the character and loop | Recovery, migration, compatibility matrix and large-project asset workflows |
| MCP | Authorized local Codex session can inspect, create or revise the character, create the loop, undo/redo, save and export; show activity and operation results | Long-running progress/cancellation, remote transport, broad client interoperability and large render delivery |
| Usability | Canvas-led Draw and Animate views, a few direct tools, compact inspector and timeline, keyboard access to demonstrated actions, visible focus, and actionable errors | Full accessibility, display-scale and task-based acceptance |

V1 uses one small document and one known SVG example. It does not fetch external
resources. Unsupported active content is rejected or reported; source is not
silently discarded. Keep the accepted custom macOS shell and design tokens.
The canvas remains dominant; Draw and Animate are direct choices; inspector,
timeline and MCP activity stay compact and contextual. Use clear sentence-case
actions and visible focus. Blender may inform workspace and timeline concepts,
without copying its chrome.

Current implementation details: Select provides eight resize handles. The
inspector includes fill and stroke swatches plus stroke-width presets. The MCP
popover reports Ready/Active and the last request. Live-preview MCP includes
`native_timeline_get`, `native_timeline_add_property_track`,
`native_timeline_set_property_key`, `native_timeline_set_loop`,
`native_timeline_undo`, and `native_timeline_redo`; the latter two operate on
shared project history and the latest edit. `native_timeline_add_track`,
`native_timeline_add_drawing_asset`, `native_timeline_set_cel`,
`native_timeline_remove_cel`, and `native_timeline_move_item` remain unrouted.
Native GIF export is synchronous and stride-samples loops over 120 frames,
multiplying frame delay by the stride. CI runs the pan-timing gate in release
mode; chrome and footer goldens are informational on hosted runners.

```text
+---------------------------------------------------------------+
| Vexel   Character.vexel       Draw  Animate      Undo  Redo   |
+----------+-------------------------------------+--------------+
| Select   |                                     | Fill / size  |
| Shapes   |         character canvas            |              |
|          |                                     | Codex activity|
+----------+-------------------------------------+--------------+
| Zoom / status              timeline + play     Save   Export |
+---------------------------------------------------------------+
```

## V1 walkthrough

1. Create a project and draw a simple character or import the supported SVG example.
2. Revise a shape, then have Codex revise the same open project; observe the live update and undo/redo one edit.
3. Add a small movement or blink loop through the timeline or Codex; scrub and play it.
4. Save and reopen the project, confirming the drawing and animation remain editable.
5. Export a GIF and verify its frames and timing; export one supported SVG fixture.

Use focused correctness checks as features are added, then one integrated
walkthrough. Verify native window behavior in an isolated environment without
disturbing the user's desktop. Defer the broad automated UI/E2E campaign until
the full feature set is complete. The final public Codex video for the website
and README remains after full-product acceptance.

## Focused effort estimate

These are remaining implementation ranges based on the Rust work already
accepted and the open decoder/composition gate. They assume one engineer-day is
one focused day; they exclude review waiting and broad final acceptance.

| Remaining V1 work | Focused effort |
|---|---:|
| Basic drawing and supported SVG path | 3–5 days |
| Editable native save/reopen integration | 1–2 days |
| Short playback loop and preset GIF export | 2–3 days |
| Live MCP authoring, visible results and undo/redo | 2–4 days |
| UI integration and one end-to-end walkthrough | 2–3 days |

**Total: 10–17 focused engineer-days, about 2–4 focused weeks.** These ranges
include the limited SVG path and one integrated V1 walkthrough, but not the
public video or the full-product release gates. The unaccepted decoder and
composition work is the main estimate risk; revise this range when that gate
closes. This is a planning estimate, not a delivery promise.

## Required after V1

The full goal remains: finish the Rust port; deliver faithful Inkscape import,
export and preservation; advanced vector editing; native project compatibility
and recovery; full animation authoring; measured large-file performance;
detailed MCP progress, cancellation and client interoperability; comprehensive
UI/E2E, accessibility, security and packaged-release acceptance; then record
and publish the authentic Codex character-and-animation video for the website
and README. V1 deferrals remain required full-product work.
