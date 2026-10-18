# Codex prompt: Codex drives Vexel demo

Paste this into a Codex or Claude Code session with the `vexel` MCP server connected. Scene order follows `docs/demo/storyboard.md`.

```text
You are connected to the Vexel MCP server. Use only the Vexel MCP tools. Do not use shell commands, network access, or any other tool to edit files.

Work through these steps in order. After each step, inspect the result with the tool that reports state before you move on. Report only what the tool results show.

1. Call document_list and document_info. Describe the open document's size and tree.
2. Draw a simple friendly character in the open 512x512 document. Use only rectangles, ellipses, and path fragments. Add a blue body, a skin-coloured head, two dark eyes, and a red scarf. Keep every shape inside the canvas and use six-digit RGB colours. Add the shapes one at a time. Then call render_png.
3. Change the scarf to green. Group the head and eyes together. Add the caption "Vexel demo" under the character with text_set.
4. Add a timeline track for the character. Add a property track for the head's vertical position. Set keys at frame 0 and frame 30, and set the loop. Describe what the timeline shows.
5. Call undo once, then redo once, and report the document state after each. Save the document with save_as into the granted directory only.
6. Export the animation as a GIF with at most 30 FPS and at most 120 frames. Call render_png for a still. Report the output path only if the export result confirms it.

Rules:
- Do not claim a result a tool did not return. If a call fails, quote the error and stop that step.
- Do not invent tool names. If a tool you expected is missing, say so and stop that step.
- Keep the narration to one short line per step.
```

## Notes for the operator

- `--mcp-preview` exposes only `document_list`, `document_info`, `element_add`, `undo`, and `redo`. Steps 3 to 6 need the full MCP profile.
- The grant directory is set with `--grant-dir <directory>` on the `--mcp-preview` command line (supported by `rust/crates/vexel-app/src/compose/launch.rs`).
