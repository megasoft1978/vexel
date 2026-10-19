# Preparatory Codex prompt: live Vexel character and animation

Status: rehearsal draft only. This prompt has not been run against a live
Codex/Vexel connection. It does not establish client interoperability or prove
that an app-owned `.vexel` project visibly updates. Confirm the connected
server's advertised tools and schemas at rehearsal; use only calls it actually
advertises. The Rust schema source is
`rust/crates/vexel-protocol/src/tools.rs`.

The intended sequence follows [`storyboard.md`](storyboard.md). The complete
profile is needed for the listed drawing and native-timeline calls. The
`--mcp-preview` profile is restricted and is not a substitute for full-profile
coverage; check the current schema before relying on a preview profile.

```text
You are connected to Vexel through MCP. Use only tools the connected Vexel
server advertises. Do not use shell commands, network access, or any other tool
to change files. Work in the currently open app-owned native `.vexel` project.
If the open document is not that project, or a required tool is missing, stop
and report the exact limitation; do not invent a replacement call.

Make a small, friendly blue-and-cream character with dark eyes and a red scarf.
Keep the drawing inside the visible canvas. After each edit call, inspect the
document or render with an advertised read/render tool, and narrate only what
the returned result and Vexel visibly confirm.

1. Inspect using `document_list`, `document_info`, and `tree_get` if advertised.
   Identify the current document ID from the actual result. Confirm the app
   window shows that same project before editing.
2. Draw with successive `element_add` calls using only supported templates
   (`ellipse`, `rect`, `path`, or other templates shown in the schema). Supply
   string-valued SVG attributes. Capture each returned element ID. Use
   `element_add_fragment` only if advertised and needed for a shape that the
   templates cannot express. Render or inspect between calls so the successive
   changes are visible.
3. Revise the drawing through actual successive MCP calls: change scarf fill
   with `attribute_set`, adjust an existing feature, and add a small detail.
   Inspect/render after each call. Do not call `draw_rect`, `draw_path`,
   `text_set`, or `element_group` unless the live schema advertises that exact
   tool and its supplied arguments match that schema.
4. Create a short blink on the native timeline, only if the active app-owned
   project and `native_timeline_*` tools are available. Add a property track
   targeting the eye element, property `opacity`, value type `scalar`, and
   interpolation `linear` or `step` as accepted by the advertised schema.
   Use the project's actual tick/frame convention. Set successive keys for
   open, closed, and open eyes with `native_timeline_set_property_key`, then
   set loop mode with `native_timeline_set_loop`. Inspect using
   `native_timeline_get` and preview playback only with an advertised tool.
   If property targeting, playback, or visible timeline updates are unsupported,
   stop and report that; do not substitute a non-native animation and call it
   timeline animation.
5. Revise the animation through further actual MCP calls: move or add a key
   with `native_timeline_set_property_key` or
   `native_timeline_move_item`, as advertised. Inspect the timeline and play
   the result if supported. State exactly which returned timeline values
   changed.
6. Exercise `undo`/`redo` for a drawing edit and the native timeline undo/redo
   tools for an animation edit, if advertised. Inspect after each call.
7. Save and reopen the native project using the app's supported native-project
   UI workflow. Do not treat `save` or `save_as` as native `.vexel` support
   unless the live schema and app confirm that format. Reopen the same project
   and inspect that drawing and animation survived. Export a short GIF with
   `export_animation` only when the current project, schema, and grant permit
   it; report a path only if the result confirms success. A still may be
   captured with `render_png` if advertised.

Keep one short, accurate caption per action. Clearly distinguish a tool result
from something merely visible in the app. If any action fails, quote its error
and stop that action; do not claim success based on an earlier result.
```

## Rehearsal operator notes

- This is preparatory material, not evidence of a working live connection.
- The live Rust tool schemas are authoritative. In particular, `draw_rect`,
  `draw_path`, and other names not present in those schemas are unsupported.
- Native project save/reopen is an app UI step here; available MCP `save` and
  `save_as` schemas describe the current document path, not an asserted native
  `.vexel` project workflow.
- Use a dedicated test project and an explicitly granted output directory.
- Accessibility: keep the conversation, visible Vexel canvas/timeline, and
  status/error feedback readable; provide synchronized accurate captions and
  a transcript, and do not rely on color alone to communicate edits.
