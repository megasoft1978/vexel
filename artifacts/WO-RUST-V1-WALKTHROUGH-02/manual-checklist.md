# WO-RUST-V1-WALKTHROUGH-02 manual checklist (native rows)

All rows: NOT RUN BY WORKER. Run on an isolated macOS desktop session with a
release build of `vexel`. Mark Pass/Fail.

| Row | Action | Expected observable result | Pass | Fail | Run by worker |
|---|---|---|---|---|---|
| M1 | Launch `vexel`; click Draw, then Animate | Workspace switches; canvas stays dominant; Animate shows the timeline strip, Draw hides it | [ ] | [ ] | not run by worker |
| M2 | In Draw, draw an ellipse, a rectangle and a path on the canvas with the tool rail | Each shape appears under the pointer with a solid fill and a visible selection | [ ] | [ ] | not run by worker |
| M3 | Select a shape; recolor fill in the inspector | Canvas updates immediately; title shows the dirty marker | [ ] | [ ] | not run by worker |
| M4 | In Animate, add a keyframed loop (or open one); scrub the timeline, then Play and Pause | Scrubbing shows the interpolated frame; play loops; pause holds the current frame | [ ] | [ ] | not run by worker |
| M5 | File > Save As to a `.vexel`, quit, relaunch, File > Open it | Open/Save panels appear; the reopened canvas and animation match the saved state; title is clean | [ ] | [ ] | not run by worker |
| M6 | Export animation as GIF; open the GIF in a second viewer (Preview or a browser) | File plays as a looping animation of the right size; the animated element visibly changes | [ ] | [ ] | not run by worker |
| M7 | Make an edit, press Cmd+Z then Cmd+Shift+Z | Edit disappears, then returns, exactly | [ ] | [ ] | not run by worker |
| M8 | Tab through the toolbar, inspector and timeline with the keyboard | Every control takes visible focus in order; Space/Enter activates it | [ ] | [ ] | not run by worker |
| M9 | Launch `vexel --mcp-preview --grant-dir <tmp>` and connect a real MCP client (Codex or an MCP inspector) over `vexel-mcp-stdio`; call tools/list, element_add, save_as into `<tmp>`, and one path outside `<tmp>` | Listed tools match the routed set (no `render_png`, no grant tool); edits appear in the window; the in-grant save succeeds; the outside path is refused with `path not granted` | [ ] | [ ] | not run by worker |
| M10 | Watch the activity display while M9 runs | Each tool call and its result (operation, element id, success or typed error) is shown visibly as it happens | [ ] | [ ] | not run by worker |
