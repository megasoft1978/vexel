# Vexel MCP server

The Vexel app serves the Model Context Protocol over stdin/stdout when launched
with `--mcp-preview`. The app itself is the MCP server and opens its live
window; there is no separate `vexel-mcp` sidecar binary or broker socket in the
current Rust build. The only app binary is `vexel`, built at
`build/rust/aarch64-apple-darwin/release/vexel` by
`cargo build --release --locked -p vexel-app`.

The server accepts JSON-RPC over stdio. The window remains live while MCP
requests operate on the app-owned document. The app starts in the shell's
live-preview mode and uses the app's UI thread for document changes.

## Start and grant access

Pass `--mcp-preview` and one or more `--grant-dir <existing directory>`
arguments to the `vexel` executable. Each explicit grant is checked and
canonicalized at launch. The filesystem access policy also reads
`VEXEL_MCP_ALLOWED_DIRS` as a colon-separated list and
`~/.config/vexel/mcp-allowed-dirs.txt` as a newline-separated list. Explicit
directories, the environment value, and the config file form the process's
fixed access policy. Directories must exist; `/` and the home directory cannot
be granted directly.

For example:

```sh
cargo build --release --locked -p vexel-app
"$PWD/build/rust/aarch64-apple-darwin/release/vexel" \
  --mcp-preview \
  --grant-dir /Users/me/Artwork
```

The MCP client must launch this command as a stdio server. The live Vexel
window appears while the client communicates over the process's stdin and
stdout.

## Register with MCP clients

Replace the executable path and grant directory with local absolute paths.
These examples run the app directly as the MCP stdio server.

### Codex

Add this to `~/.codex/config.toml`:

```toml
[mcp_servers.vexel]
command = "/absolute/path/to/vexel/build/rust/aarch64-apple-darwin/release/vexel"
args = ["--mcp-preview", "--grant-dir", "/Users/me/Artwork"]
startup_timeout_sec = 10
tool_timeout_sec = 60
```

For a non-interactive `codex exec`, allow tool approval explicitly, for example:

```sh
codex exec \
  -c 'mcp_servers.vexel.command="/absolute/path/to/vexel/build/rust/aarch64-apple-darwin/release/vexel"' \
  -c 'mcp_servers.vexel.args=["--mcp-preview","--grant-dir","/Users/me/Artwork"]' \
  -c 'mcp_servers.vexel.default_tools_approval_mode="approve"' \
  "Inspect the open Vexel document"
```

### Claude Code

Register the command at user scope:

```sh
claude mcp add vexel --scope user -- \
  /absolute/path/to/vexel/build/rust/aarch64-apple-darwin/release/vexel \
  --mcp-preview --grant-dir /Users/me/Artwork
```

Or add a project `.mcp.json` entry:

```json
{
  "mcpServers": {
    "vexel": {
      "type": "stdio",
      "command": "/absolute/path/to/vexel/build/rust/aarch64-apple-darwin/release/vexel",
      "args": ["--mcp-preview", "--grant-dir", "/Users/me/Artwork"]
    }
  }
}
```

### Claude Desktop

Add the server under `mcpServers` in `claude_desktop_config.json`, then restart
Claude Desktop:

```json
{
  "mcpServers": {
    "vexel": {
      "command": "/absolute/path/to/vexel/build/rust/aarch64-apple-darwin/release/vexel",
      "args": ["--mcp-preview", "--grant-dir", "/Users/me/Artwork"]
    }
  }
}
```

## Tools

Live preview advertises document open/close/list/info, tree and element
inspection/query, selection, shape and fragment editing, grouping,
duplication/reordering, attributes, text, transactions, undo/redo, SVG save and
export, PNG rendering, gradient and clip editing, and animation/timeline
operations. The native timeline surface includes `native_timeline_get`,
`native_timeline_add_property_track`, `native_timeline_set_property_key`,
`native_timeline_set_loop`, `native_timeline_undo`,
`native_timeline_redo`, `native_timeline_remove_key`,
`native_timeline_move_key`, and track remove/rename/duplicate/interpolation/
reorder operations, alongside drawing asset/cel tools. The timeline defaults
to 48 ticks at 24 ticks per second (ticks 0-47); `native_timeline_set_duration`
(`durationTicks`, optional `ticksPerSecond`) lengthens or shortens it, capped at
3600 seconds and never below the last key/cel tick plus one. Exact arguments are
published by MCP tool schemas. Timeline edits use shared project history.
Native GIF export is synchronous; loops over 120 frames are stride-sampled,
with frame delay multiplied by the stride.

### Building an animated scene

Use `element_add` to create the scene's rects, ellipses, paths, and text.
`parentId` is `"root"` or the id of an existing group (from `element_group`), so
nested character hierarchies can be built; native tracks, including TRS pivots,
can target elements at any depth.
`gradient_add` returns a `gradientId` and stop IDs; apply the gradient with
`element_set_paint`. Create a rect or ellipse clipping path with `clip_add`,
then apply it to a shape or group with `element_set_clip`. Use
`element_add` also takes optional `id` (explicit id, `[A-Za-z0-9_.-]`, max 128;
a clash with an existing id is rejected), `opacity` (0-1), `transform`
(`translateX`, `translateY`, `rotate`, `pivotX`, `pivotY`, `scaleX`, `scaleY`,
`skewX`) and `gradientId` (+ `paint`: `fill` default or `stroke`), all applied in
the same undoable edit. `element_group` accepts `elementIds[]` (1-64 siblings of
one parent) to wrap several elements in one group, and `element_reparent`
(`elementId`, `parentId`, optional `index`) moves an element into another group
or back to `"root"` in one edit (ancestor transforms are not compensated). Use
`element_group` to collect character parts and `element_duplicate` plus
`element_reorder` to place repeated/background elements.

Create motion with `native_timeline_add_property_track`, passing the target's
element ID, property, `valueType`, interpolation, and initial `value`. Use
`transform` with `valueType: "trs"` for pivot rotations/translations, and
`opacity` with `valueType: "scalar"` for blinking. Add later values with
`native_timeline_set_property_key` (optional `ease`), or pass an optional
`keys: [{tick, value, interp?}]` (at most 256, same value forms; `interp` takes
the `ease` names) to `native_timeline_add_property_track` to create the track and
all its keys in one undoable edit. A `trs` value is either
an array `[tx,ty,rotationDeg,sx,sy,pivotX,pivotY(,skewXDeg)]` or an object with
optional `translate` (`[x,y]` or `{x,y}`; or `tx`/`ty`), `rotate` (degrees; also
`rotation`), `scale` (number, `[x,y]` or `{x,y}`; or `sx`/`sy`), `pivot`
(`[x,y]` or `{x,y}`; or `pivotX`/`pivotY`) and `skewX` (degrees); omitted parts
are identity. Unknown or mistyped members are rejected naming the field
(`value.rotate`). Then refine keys and
tracks with the corresponding `native_timeline_move_key`, `remove_key`,
`duplicate_track`, `rename_track`, `set_interpolation`, and `reorder_track`
tools. Inspect with `element_get`, `element_query`, `tree_get`, and
`native_timeline_get`; use `render_png` at selected times and `export_animation`
for GIF output. `render_png` `time` is in seconds (`tick / ticksPerSecond`), not
ticks, and a time past the end of the timeline is rejected; the reply reports
the rendered `tick`. Save a copy with `save_as` before sharing it.

`export_animation` also writes video. `format` `mp4` or `mov` encodes the whole
native timeline loop (one frame per tick; ping-pong included) as H.264 (default)
or HEVC (`codec`: `h264` | `hevc`), BT.709. It needs a native timeline, writes
synchronously like `render_png`, and the path must be inside a granted
directory with an extension matching the format. Optional arguments: `fps`
(1-120, default the timeline tick rate; every `round(ticksPerSecond / fps)`-th
tick, never above the tick rate), `quality` (0-1) or `bitrate` (kbps, 50-200000;
not both), `width` and `height` (1-4096, given together, default the document
size; odd sizes round up to even), `loops` (1-100 repeats of the loop) and
`background` (`#rgb` or `#rrggbb`, default white) behind transparency. `fps` and
`duration` are not required for video, and `duration` is rejected. Loops longer
than 1440 frames keep every n-th tick so the video stays at or under 1440
frames. The reply carries `path`, `format`, `codec`, `width`, `height`,
`frames`, `fps`, `durationSeconds` and `bytes`. The app's Export popover has the
same export as "Export Video..." (H.264 MP4 at the document size, enabled when
the document has a native timeline).

Resources expose current document information and rendering data supported by
the active session. Tool results include structured JSON and a text fallback;
rendering can also return an MCP image content block.

## Performance

`cargo test -p vexel-app --release --test mcp_bench -- --ignored --nocapture`
builds 120 elements (the live preview scene cap is 128) and 50 opacity tracks x
8 keys through the headless rig and prints per-tool latency. The rig measures
handler cost only; it excludes the shell loop's wake/sleep latency.

Baseline (before optimization), p50 / p95 ms:

| tool | n | p50 | p95 |
| --- | --- | --- | --- |
| element_add | 120 | 0.26 | 0.38 |
| native_timeline_add_property_track | 50 | 1.21 | 2.26 |
| native_timeline_set_property_key | 400 | 1.71 | 3.20 |

Total wall 0.80 s (idle machine). The scene cap is now 1024 elements; the
benchmark keeps 120 elements to stay inside the 600 requests/min session budget.

After bulk keys, shared history snapshots and per-edit serialization removal
(machine under load, A/B run back to back): `element_add` 0.31-0.40 ms p50,
`native_timeline_add_property_track` 1.5-1.7 ms, `native_timeline_set_property_key`
2.0-2.2 ms (unchanged within noise at this scene size, where handler time is
dominated by timeline encoding). A track created with 8 `keys` in one call takes
about 11 ms p50 against about 14.9 ms for 9 separate calls, and costs one request
instead of nine.

Main-loop latency: an incoming request now wakes the idle shell loop at once
(it used to wait out a 16 ms sleep), the loop serves up to 16 queued requests
(8 ms budget) per pass, and the window redraws only when the document, playback
or MCP status changed.

## Historical/unsupported transport notes

Older versions of this page described a `vexel-mcp` sidecar, a per-user Unix
socket, socket relay behavior, and sidecar-specific relay fixtures. Those
descriptions do not apply to the current Rust app and are retained here only
as a record that the old launch flow is unsupported. The current flow is the
app's stdio server launched with `--mcp-preview` as shown above.
