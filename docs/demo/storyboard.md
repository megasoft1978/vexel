# Preparatory storyboard: Codex edits a Vexel character and blink

**Status: draft; no recording or live interoperability claim.** This sequence
is a rehearsal target for the product goal. It must be exercised with a real
Codex MCP client connected to the same running Vexel app and open native
`.vexel` project before it can be called a demonstrated workflow. Tool names
and argument shapes below come from the Rust source schema at
`rust/crates/vexel-protocol/src/tools.rs`; the connected server's advertised
schema is authoritative during rehearsal. If a required call is absent, record
the gap rather than substitute a fictional tool.

The goal is to show successive actual MCP calls creating a character, revising
the drawing, creating and revising a short blink animation, and the same live
project visibly changing. Include UI undo/redo, save/reopen, and export if the
app's native workflow supports them. This document is a preparation plan, not
proof that those steps currently work end to end.

## Scene 1: Establish the live session

- Show the real Codex conversation and running Vexel window together.
- Confirm the active app-owned `.vexel` project and its document ID using
  advertised `document_list`, `document_info`, and `tree_get` calls.
- State the boundary: a returned tool result is MCP evidence; a visible update
  requires observing the matching Vexel window. Do not imply an edit happened
  live based only on a tool result.
- If the running app or native project cannot be verified, stop; do not record
  this as the required live showcase.

## Scene 2: Draw the character in successive calls

- Prompt Codex to draw the small blue-and-cream character, dark eyes, and red
  scarf on the visible canvas.
- Use `element_add` with schema-supported templates and string attributes.
  Capture the returned element IDs. Use `element_add_fragment` only when
  advertised and necessary. `draw_rect` and `draw_path` are not Rust tool names
  and must not be used unless a later live schema explicitly adds them.
- Pause on successive calls and show each corresponding app update. Use
  `document_info`, `tree_get`, `element_get`, or `render_png` when advertised
  to inspect actual results.
- Caption the action accurately (for example, “Codex added the head” only
  after the call succeeds and the head is visible).

## Scene 3: Revise the drawing

- Change the scarf fill using `attribute_set`; adjust one facial feature and
  add a small detail with further advertised editing calls.
- Keep each call distinct and show the corresponding update in the same
  Vexel project. Inspect after edits. Avoid unsupported `text_set` or
  `element_group` calls unless actually advertised.
- Keep labels/captions readable and do not communicate the change through
  color alone.

## Scene 4: Create a blink on the native timeline

- Confirm the active project is native `.vexel` and the live schema advertises
  the relevant native timeline calls. The listed calls include
  `native_timeline_add_property_track`,
  `native_timeline_set_property_key`, `native_timeline_set_loop`, and
  `native_timeline_get`.
- Add a scalar `opacity` property track targeting the eye element, using an
  interpolation enum accepted by the schema. Add open/closed/open keys at
  suitable project ticks; use the actual tick convention and avoid asserting
  a frame rate until confirmed.
- Set loop mode to `loop` if supported. Show the timeline changing after the
  calls, then preview playback only if an advertised tool and the app support
  it. A timeline data response alone does not prove visible playback.
- If native property targeting, timeline display, or playback does not work,
  capture the gap and stop this scene. Do not replace it with SMIL and describe
  that as native timeline animation.

## Scene 5: Revise the animation and exercise history

- Make a second timeline edit through an actual advertised call, such as
  `native_timeline_set_property_key` or `native_timeline_move_item`.
- Show the changed timeline and playback in the same project, when supported.
- Exercise drawing `undo` then `redo`, and timeline
  `native_timeline_undo` then `native_timeline_redo` if available. Show the
  affected artwork/timeline state after each call. Report exact tool results.

## Scene 6: Save, reopen, export

- Save the editable native project using the app's supported UI workflow.
  Reopen that same project and show both artwork and animation are retained.
  Do not claim MCP `save`/`save_as` implements native project save unless the
  current tool schema and app explicitly confirm it.
- Export a short GIF via `export_animation` only if supported for this project
  and the output directory is explicitly granted. Confirm success from the
  tool result and confirm the artifact exists before naming its path. Use
  `render_png` for a still only when advertised.
- Show the exported animation playing and identify any limitations plainly.

## Captioning and capture requirements

- Keep the Codex conversation, Vexel canvas, timeline, and status/errors
  legible together. Avoid focus changes that obscure either application.
- Provide synchronized captions and a transcript. Captions must describe the
  actual request, call outcome, and visible change without paraphrasing a
  failure as success. Include relevant errors and disclose any cuts or speed
  changes in a later published edit.
- Use a repeatable, dedicated project and record the project file, prompts,
  exported animation/still, app/build identity, MCP configuration, and grant
  setup needed to reproduce the session. Do not include private paths or
  unrelated desktop content.

## Evidence checklist

- [ ] Real Codex client connected to the Rust MCP server; retain sanitized
      request/result evidence or an equivalent trace with timestamps.
- [ ] Same running Vexel instance and same native `.vexel` project visibly
      shown before and after successive drawing calls.
- [ ] Character drawing revision shown through successful actual MCP calls.
- [ ] Native timeline blink creation and subsequent revision shown through
      successful actual MCP calls; timeline and playback visibly update.
- [ ] Drawing and timeline undo/redo outcomes shown and inspected.
- [ ] Native project saved, reopened, and checked for artwork and animation.
- [ ] Export result confirmed; artifact reviewed for appearance and timing.
- [ ] Captions and transcript checked against actual narration and events;
      readability/accessibility reviewed.
- [ ] Project, prompt, export, and capture metadata retained for repeatability.
- [ ] No unsupported calls, unobserved outcomes, or live interoperability
      claims appear in the recording or its description.
