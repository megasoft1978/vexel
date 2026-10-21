# Changelog

All user-visible changes since `v1.0.0-alpha`. Pre-release; macOS 13+ on Apple
Silicon only. Unreleased changes are listed under `Unreleased`.

## Unreleased

### MCP server (live preview)

- 53 live-preview tools advertised, filtered to routed operations only.
- `batch` tool: ordered operations in one undo step, `atomic` (default true)
  rolls back on the first failure; `atomic: false` continues past failures.
  Mixed scene and timeline ops; `element_add` `id` usable by later ops.
- Native timeline tools: get, add/remove/rename/duplicate/reorder property
  tracks, set/remove/move keys, set loop, set duration
  (`native_timeline_set_duration`), undo and redo. Bulk keys in one edit.
- `element_add` accepts `id`, `opacity`, `transform`, `gradientId`, and any
  group as `parentId`. `element_group` and nested group support.
- `animation_control` drives the native timeline when the document has one.
- Preview document open follows the `document_open` grant rules at launch.
- Per-session request budget raised from 120 to 600 per minute. Batches cost
  `1 + ops/8` of the budget.
- Performance: shared history snapshots, lazy document load, cached scene
  serialization, and a per-edit serialization fix. A scripted-scene benchmark
  and baseline are in `docs/mcp.md`.
- Deterministic MCP driver for the animated walk-home scene.
- Activity popover shows Ready/Active and the last request.
- Live preview scene limit raised to 1024 elements.

### Export

- GIF: native timeline export; loops over 120 frames are stride-sampled.
- Video: `export_animation` `mp4` and `mov` (H.264 or HEVC, BT.709) for native
  timelines, with width/height, quality or bitrate, and loop count.
- HEVC with alpha: `mov` container with the `alpha` option.
- APNG export and PNG frame-sequence export (`apng`, `frames`).
- Export save panel suggests `name.<ext>` and allows only the chosen format.
- Clearer error when the target folder is not writable.
- Render and export failures name the cause.

### SVG

- Path data: `Q`/`q`, `T`/`t`, `S`/`s` commands with correct bounds. Arc
  commands remain unsupported.
- Internal DOCTYPE general entities expanded, with hard limits. External,
  cyclic, and over-limit entities return typed errors.
- Nested `<use>` references; oversized root viewports clamped to 4096 px.
- Gradient `currentColor` stops resolved lazily, removing a quadratic rescan.
- Opacity, transform, dash, and gradient fill controls in the inspector.

### Canvas and timeline UI

- Live rotation preview (ghost and outline) during rotate drag.
- Canvas structure editing, grouping, and rotation gestures; canvas shortcuts.
- Native timeline key editing, including a cubic-bezier easing editor.
- Editable transform pivot X/Y in the inspector.
- Select shows eight resize handles.
- Journal edit groups: begin, end, and rollback for atomic undo.

### App

- New app icon from generated artwork (`scripts/make_icon.py --from-image`).
- Real-window UI e2e script (`scripts/e2e_ui.sh`, not yet run live); see
  `docs/e2e-ui.md`.

### Known limits

- Arcs (`A`/`a`) unsupported.
- Corpus fidelity remains partial (see `STATUS.md`).
- Documents over 32 MiB refused by design.
