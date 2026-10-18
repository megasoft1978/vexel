# Codex drives Vexel: 6-scene storyboard

Target runtime: under 60 seconds total (budget below sums to 56 s).

Tool status key:
- **existing**: listed in `docs/mcp.md` (full profile).
- **preview**: listed in `docs/product/mcp-live-preview.md` (`--mcp-preview` profile: `document_list`, `document_info`, `element_add`, `undo`, `redo` only).
- **planned**: named in the brief but not in any doc. Do not rely on it.

Open issues for this storyboard:
- `draw_rect` and `draw_path` are **planned**. No doc defines them. Scene 2 uses `element_add_fragment` (existing) as the working route.
- `--grant-dir` is **not documented** anywhere in `docs/`. The documented grant is the `VEXEL_MCP_ALLOWED_DIRS` env var (or `~/.config/vexel/mcp-allowed-dirs.txt`).
- `--mcp-preview` exposes only 5 tools. Scenes 3 to 6 need the full profile from `docs/mcp.md`. Full-profile launch flags are not documented. Confirm before recording.
- Native timeline tools (scenes 4 and 6) act only on the active app-owned `.vexel` project.

## Scene 1: Inspect the open document (6 s)

- Calls: `document_list` (preview), `document_info` (preview), `tree_get` (existing).
- Expected visible result: the 512 x 512 artboard is listed and its tree is shown in the Vexel window. Nothing changes on screen.

## Scene 2: Draw a simple character (12 s)

Shapes: blue body, skin head, two dark eyes, red scarf.

- Calls:
  - `draw_rect` for the scarf (**planned**). Working route: `element_add_fragment` with a `<rect>` fragment (existing).
  - `draw_path` for the body outline (**planned**). Working route: `element_add_fragment` with a `<path>` fragment (existing).
  - `element_add` for the head and eyes (ellipses, **preview** and existing).
  - `render_png` (existing) to confirm.
- Expected visible result: four shape groups appear one at a time in the open window. Each add shows in the status line. The PNG shows a character in the artboard.

## Scene 3: Refine and label (8 s)

- Calls: `attribute_set` (existing) to change the scarf fill, `element_group` (existing) to group the head and eyes, `text_set` (existing) for a caption, `element_reorder` (existing) to move the scarf in front of the body.
- Expected visible result: the scarf colour changes, the head and eyes move as one group, and the caption appears under the character.

## Scene 4: Animate the character with the timeline (12 s)

- Calls, in order:
  1. `native_timeline_add_track` (existing, active `.vexel` project only).
  2. `native_timeline_add_drawing_asset` (existing) for the character group.
  3. `native_timeline_add_property_track` (existing) for the head `y` position.
  4. `native_timeline_set_property_key` (existing) at frame 0 and at frame 30.
  5. `native_timeline_set_loop` (existing).
- Expected visible result: the timeline footer shows the track, two diamonds, and a loop. Playback bounces the head between keys. Playback is not verified here.

## Scene 5: Undo, redo, and save (8 s)

- Calls: `undo` and `redo` (existing; also preview), `native_timeline_undo` and `native_timeline_redo` (existing), `save_as` (existing) into the granted directory.
- Expected visible result: the last keyframe is removed and restored. The document title shows the save path. The file is written only inside the granted directory.

## Scene 6: Export the animation (10 s)

- Calls: `export_animation` (existing) as GIF, with 30 FPS and 120 frames or fewer (the documented cap). Then `render_png` (existing) for a still.
- Expected visible result: a GIF file appears in the granted directory. The status line reports the operation. Before the demo, confirm the export result and file exist. Do not claim them in narration until observed.

## Runtime budget

| Scene | Seconds |
|---|---|
| 1 Inspect | 6 |
| 2 Draw | 12 |
| 3 Refine | 8 |
| 4 Animate | 12 |
| 5 Undo/redo/save | 8 |
| 6 Export | 10 |
| **Total** | **56** |
