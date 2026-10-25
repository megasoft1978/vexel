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

To start on an existing document, pass its path after the flags:

```sh
"$PWD/build/rust/aarch64-apple-darwin/release/vexel" \
  --mcp-preview \
  --grant-dir /Users/me/Artwork \
  /Users/me/Artwork/logo.svg
```

The file opens through the same rules as the `document_open` tool: it must be
a `.vexel` or `.svg` file inside a granted directory. The path itself grants
nothing. A path outside every grant is refused: the app prints
`Vexel: cannot open <path>: path not granted` on stderr and starts with the
blank document. An opened `.svg` is untitled, so writing it back needs
`save_as` to a `.vexel` path inside a grant; `save` on an opened `.vexel`
writes in place and needs that file's directory to be granted.

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

### Audio

`audio_add_track`, `audio_set` and `audio_list` keep up to 64 audio tracks in the
native timeline (root `audio` array of the `.vexel` timeline JSON), so they save,
reopen, undo and redo with it. Each track has `trackId`, `kind` (`voice`, `sfx`,
`music`), `path`, `startTick`, `trimInMs`/`trimOutMs`, `gainDb` (-120 to 24),
`fadeInTicks`/`fadeOutTicks` (linear), `loop` and an optional
`duck: {under, amountDb}` (`under` is a track id or a kind; -60 to 0 dB while it
is audible). `audio_set` changes any of these (`trimOutMs: 0` and
`clearDuck: true` clear), moves a clip with `startTick`, or deletes it with
`remove: true`.

Sources are WAV, AIFF/AIFC or CAF PCM (16/24/32-bit integer or 32-bit float, mono
or stereo, 8-192 kHz), at most 10 minutes and 256 MiB. The document must be saved
first: `path` is relative to the saved document's directory (an absolute path
inside it is accepted) and is stored relative. Every use re-resolves it through
the grant policy and requires a regular, granted file still inside the document
directory after symlinks are resolved. Bad headers, truncated data, unsupported
encodings and oversize files are rejected with a typed message and change nothing.

`export_animation` mp4/mov with audio tracks mixes them (deterministic linear
resampling, trim, loop, gain, fades, envelope ducking, then a look-ahead peak
limiter) to 48 kHz stereo and muxes AAC (128 kbit/s) for the video's length; the
export stays synchronous. Loops of the video do not repeat the audio. Without audio
tracks the export path is unchanged. A GIF/APNG/frames export ignores audio.

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

`batch` (`documentId`, optional `atomic` default `true`, `ops`: 1 to 256 entries
of `{"tool": "<name>", "arguments": {...}}`) runs preview operations in order as
one undo step. Every entry is validated exactly like a standalone call before
any is applied; a malformed entry fails the request with a message naming the
op index, tool and argument, e.g. `batch op 2 (element_add): missing required
argument 'template'`. An op may omit `documentId` and inherits the batch's (a
different `documentId` inside an op is rejected):
`{"documentId":"doc-1","ops":[{"tool":"element_add","arguments":{"parentId":"root","template":"rect"}}]}`.
An op has only the keys `tool` and `arguments`. Scene
and native timeline operations may be mixed, and later entries can use ids
given to earlier `element_add` calls (`id`). Success replies
`{"ok":true,"failedCount":0,"results":[{"tool","ok","result"}...]}`. With
`atomic:true` the first failure rolls the whole batch back and replies
`{"ok":false,"failedIndex","error","results":[...applied so far...]}`; with
`atomic:false` the batch continues past failures, each result carries `ok` and
`error`, and the reply has `ok:false` and `failedCount` when any failed. Nested
`batch`, file (`document_open`, `document_close`, `save`, `save_as`), export,
`render_png`, `undo`/`redo` and playback tools are rejected. A batch costs
`1 + ops/8` of the per-minute request budget. 100 `element_add` calls take about
95 ms as one batch against about 117 ms as 100 requests (rig, handler cost only).

`animation_control` (`action`: `play`, `pause` or `seek`; `timeMs` 0-10000 for
seek) drives the SVG animation clock for SVG SMIL documents. On a document with
a native timeline (at least one native track) it drives the native timeline
instead, the same clock the app window's Animate panel samples. Play needs the
Animate mode with the Native timeline selected and otherwise fails with
`native timeline plays in Animate mode on the Native timeline`. Pause holds the
current tick; seek sets the tick, clamped to the loop length. The reply is
`{"ok":true,"playing":<bool>,"tick":<integer>,"timeSeconds":<seconds>}` with the
state after the call. Seek is limited to 10 seconds by the tool.

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
the same undoable edit. Limits of `element_add`: `rect` and `ellipse` may lie
partly or wholly outside the canvas (every edge within +-1000000 of the origin;
width and height must stay positive); a `path` `d` is at most 256 KiB and 16384
segments (arcs unsupported); path `fill`/`stroke` and line `stroke` take
`#rrggbb`, short `#rgb` (expanded to `#rrggbb`) or `none`; a `path` also takes
`stroke-linecap` (`butt|round|square`), `stroke-linejoin` (`miter|round|bevel`)
and `fill-rule` (`nonzero|evenodd`); `rect`/`ellipse` may omit `fill` when
`gradientId` is given (the gradient is the fill, or the stroke with `paint`).
`attribute_set` with `name` `id` renames any element, groups included
(`value` 1-64 characters `[A-Za-z0-9_.-]` starting with a letter or `_`, not used
by another element): `href="#id"`, `xlink:href="#id"` and `url(#id)`/`url('#id')`
references and native timeline tracks that target the element follow, and one
undo reverts all of it. The rename does not rewrite CSS `#id` selectors in
`<style>`, whitespace-padded `url( #id )`, or SMIL `begin="id.end"` references.
`element_group` accepts `elementIds[]` (1-1024 siblings of
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
`keys: [{tick, value, ease?}]` (at most 256, same value forms; `interp` is an
alias of `ease`) to `native_timeline_add_property_track` to create the track and
all its keys in one undoable edit. A `trs` value is either
an array `[tx,ty,rotationDeg,sx,sy,pivotX,pivotY(,skewXDeg)]` or an object with
optional `translate` (`[x,y]` or `{x,y}`; or `tx`/`ty`), `rotate` (degrees; also
`rotation`), `scale` (number, `[x,y]` or `{x,y}`; or `sx`/`sy`), `pivot`
(`[x,y]` or `{x,y}`; or `pivotX`/`pivotY`) and `skewX` (degrees); omitted parts
are identity. Unknown or mistyped members are rejected naming the field
(`value.rotate`). 
Animatable properties: `x y width height cx cy r rx ry opacity fill-opacity
stroke-opacity stroke-width stroke-dashoffset` (scalar), `fill stroke stop-color`
(rgba), `transform` (trs or transform2d), `visibility` (boolean, step), and:

- `d` of a `<path>` and `points` of a `<polygon>`/`<polyline>` with
  `valueType: "path"`; values are path data strings. Every key must have the
  same commands as the first key after normalising to absolute `M L C Q Z`
  (relative commands, `H V S T` are accepted; arcs are not). Blending is
  per coordinate with the track easing; a key tick renders its string verbatim.
  A key with a different structure is rejected and nothing changes. Example:
  `{"trackId":"squash","targetId":"blob","property":"d","valueType":"path","interpolation":"linear","value":"M0 0 C 10 0 20 10 20 20 Z","keys":[{"tick":12,"value":"M0 0 C 20 0 30 10 30 20 Z"}]}`.
- Filter primitives, `targetId` = the primitive element's own `id` (give each
  `feGaussianBlur`, `feOffset`, ... an `id`), scalar: `stdDeviation`
  (`feGaussianBlur`/`feDropShadow`, 0-32), `dx` and `dy` (`feOffset`/
  `feDropShadow`, within 4096), `flood-opacity` (`feFlood`/`feDropShadow`,
  0-1), `values` (`feColorMatrix` with `type` `hueRotate` degrees or `saturate`
  0-1, one number). Values outside the renderer limits are rejected.
- Gradient stops, `targetId` = the `<stop>` id: `offset` (clamped to 0-1 and
  raised to the previous stop's offset so stops stay ordered; later stops are
  raised too), `stop-opacity`, `stop-color` (rgba; also writes `stop-opacity`).
  Scenes with filters render on the CPU route.

Easing: a key's `ease` shapes the segment from that key to the next, for every
value type (scalar, rgba, trs, path morph, filter scalars, gradient stops). Names:
`step`, `linear`, `ease_in`/`ease_out`/`ease_in_out` (quad), the same with
`_cubic`, `_back`, `_elastic`, `_bounce` suffixes (`ease_in_cubic`,
`ease_out_bounce`, ...; aliases `back`, `elastic`, `bounce` = the out forms), a
string `bezier:x1,y1,x2,y2` or an array `[x1,y1,x2,y2]` (x in 0-1, y in -4..4).
No `ease` keeps the track interpolation. Accepted by
`native_timeline_add_property_track`, its `keys`, `native_timeline_set_property_key`
and `native_timeline_tracks_set`.

Noise: `native_timeline_add_property_track` (and `tracks_set` entries) take
`noise: {amplitude, frequency?, seed?, octaves?}`. It adds seeded smooth noise
(amplitude in property units, frequency in Hz, default 4, octaves 1-4) to scalar,
transform2d (e, f) and trs (tx, ty, rotation) tracks: shake, flicker, wind sway,
hand-drawn boil. It is a pure function of tick and seed, identical across runs and
exports.

Camera: `native_timeline_set_camera` (`keys: [{tick, x?, y?, zoom?, rotate?,
ease?}]`, `shake: {amplitude, frequency?, seed?, octaves?, rotate?}`, or
`clear: true`) replaces the camera in one undoable edit. It is a render-time view
transform (pan x/y in user units, zoom, roll in degrees about the viewBox centre)
applied to preview, `render_png` and exports without a wrapper group in the scene.
Without a camera the output is unchanged. The camera lives in reserved tracks
`camera_x`, `camera_y`, `camera_zoom`, `camera_rotate` (target `@camera`).

To create many tracks at once use `native_timeline_tracks_set` (`tracks`: 1-256
entries, each the arguments of `native_timeline_add_property_track` without
`documentId`, at most 16384 keys per call): one undoable, all-or-nothing edit
with a single timeline validation (200 tracks / 3200 keys in about 15 ms). A
rejection names the entry (`tracks[3]: ...`) and changes nothing. It costs
1 + tracks/8 of the per-minute request budget, like a batch of that size, and
the request is still bound by the 2 MiB / 65536-token JSON limit, so very large
scenes need several calls (a trs key as a 7-number array is cheapest).

Then refine keys and
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
`background` (`#rgb` or `#rrggbb`, default white) behind transparency. `alpha`
(boolean, default false) keeps transparency instead: it needs `format` `mov` and
`codec` `hevc` (HEVC with alpha, premultiplied BGRA, `background` ignored) and
is rejected with `mp4` or `h264`. `fps` and
`duration` are not required for video, and `duration` is rejected. Loops longer
than 1440 frames keep every n-th tick so the video stays at or under 1440
frames. The reply carries `path`, `format`, `codec`, `width`, `height`,
`frames`, `fps`, `durationSeconds` and `bytes`. The app's Export popover has the
same export as "Export Video..." (H.264 MP4 at the document size, enabled when
the document has a native timeline).

`export_animation` also writes PNG animations from a native timeline. `format`
`apng` writes one animated PNG (path ends `.png` or `.apng`; 8-bit RGBA, one
pass over the loop, ping-pong included) and replaces an existing file
atomically. `loops` is the play count (1-100; omitted means forever). `format`
`frames` writes one PNG per sampled tick into the path's directory, which must
be a granted, existing directory: `path` is `dir/name_####.png` (four or more
`#` become the zero-padded frame number) or `dir/name` (no extension; files are
`name_0000.png`...). It refuses to start if any target file already exists, and
removes the files it wrote if a frame fails; `loops` is rejected. Both take
`fps` (1-120, default the tick rate), `width` and `height` (1-4096, together,
default the document size with the long edge capped at 4096) and `background`
(`#rgb` or `#rrggbb`); transparency is kept unless `background` is given.
`codec`, `quality`, `bitrate`, `alpha` and `duration` are rejected. At most
1440 frames (longer loops keep every n-th tick); an APNG holds at most 256 MiB
of raw pixels and a sequence writes at most 512 MiB. The apng reply carries
`path`, `format`, `width`, `height`, `frames`, `fps`, `durationSeconds`,
`loops` and `bytes`; the frames reply carries `directory`, `first`, `last` and
the same measures.

Resources expose current document information and rendering data supported by
the active session. Tool results include structured JSON and a text fallback;
rendering can also return an MCP image content block.

## Fragments and `<use>` instances

`element_add_fragment` (`parentId` is `"root"` or any element id, `xml` is one
element of at most 1 MiB) inserts arbitrary allowed markup as the last child in
one undoable edit and returns `elementIds`. Allowed elements: shapes, `g`,
`defs`, `symbol`, `use`, `mask`, `clipPath`, `filter` and `fe*` primitives,
gradients, `stop`, `pattern`, `marker`, `text`, `tspan`, `image`, `title`,
`desc`. Rejected with a typed error: `script`, `foreignObject`, `style`, SMIL
(`animate*`, `set`; use `animation_add`), `on*` attributes, any `href` that is
not a local `#id` (except `data:image/` on `<image>`), any non-local `url(...)`,
duplicate ids, `<use>` reference cycles. A fragment the renderer cannot draw
is undone and refused (see the table), so the document always renders.

`use_add` instantiates an existing element, group or `symbol` by reference:
`{"refId":"star","x":40,"y":30,"id":"star-1","fill":"#00ff00","opacity":0.8,
"transform":{...}}` (`parentId` defaults to `"root"`; `transform` is the
`element_add` object). `x`/`y` are applied in the parent's space: the offset is not scaled or
rotated by the transform (earlier versions applied it in the transformed space).
The transform (scale, rotation) acts about the instance origin at `x`,`y`: with a
transform they are folded into the matrix translation, so `scaleX: 0.5` with
`x: 100` puts the origin at 100 and halves the size around it. It returns the new `elementId`.
The instance is one element: it counts once toward the scene limit (the
referenced content is counted when it is added; expansions are not counted by
`SCENE_LIMIT`, which counts `id` attributes in the source). Expansion is bounded
by the decoder (cumulative `<use>` reference bytes 16 MiB; cyclic references are
a typed decode error, and `use_add`/`element_add_fragment` refuse cycles
up front). The `use` is selectable, hit-testable and movable on the canvas,
undoable, saved with the document, and a transform or opacity track targets its
id. `fill`, `stroke` and `opacity` are inherited by referenced content that sets
none of its own. `use_detach` (convert to a real copy) is not implemented: use
`element_duplicate` on the referenced element.

Measured by `rust/crates/vexel-app/tests/mcp_fragments.rs` (live preview,
512x512 render):

| construct | works | notes |
| --- | --- | --- |
| `<defs><filter>` with `feGaussianBlur` | yes | soft edges |
| `feDropShadow` | yes | offset shadow renders |
| `feColorMatrix` (hueRotate) | yes | |
| `feTurbulence` | yes | renders (see docs/svg-effects.md) |
| `<symbol>` + `<use href x y>` | yes | |
| `<use>` with `transform` and x/y | yes | |
| `<mask>` | yes | |
| `<clipPath>` with `<path>` children | yes | |
| `mix-blend-mode` (attribute or style) | yes | blends against the parent layer (docs/svg-effects.md) |
| `filter`/`mask`/`clip-path`/paint `url(#id)` naming no element | refused | error names the undefined reference kind (for example `undefined filter reference`); define the target first or in the same fragment |
| `stroke` on `<text>` | yes | outline paints |
| `<image>` data URI | yes | `data:image/` only; external hrefs refused |
| `<use>` cycles (self, A to B to A, symbol using its parent) | refused | typed errors; decoder also refuses cyclic sources |

## Performance

`cargo test -p vexel-app --release --test mcp_bench -- --ignored --nocapture`
builds 120, 1024, 2048 and 4096 elements (the scene cap) and 50 opacity tracks x 8 keys
through the headless rig, and prints per-tool and per-phase p50 / p95 ms. The
rig measures handler cost only; it excludes the shell loop's wake/sleep latency.
Phase timers (`vexel_app::profile`) are always on in the bench; `VEXEL_PROFILE=1`
enables them for any run. When off, each phase costs one cached flag check.
Phases nest: `edit_apply` contains `edit_validate` and `edit_ops`; `candidate_open`
contains `scene_scan` and `scene_copy`.

### Scene and source limits (raised 2026-10-25)

`SCENE_LIMIT` is now 4096 elements (was 1024) and the live-preview source limit
4 MiB (was 512 KiB; the native-timeline and animation-frame paths moved from
1 MiB to 4 MiB with it). Measured with `mcp_bench` (release, Apple Silicon, one
run; `element_add` p50 / p95 ms, edit recompose p50 / p95 ms where recompose is
decode + render on the compose worker path):

| elements | element_add | recompose (decode / render) | add_property_track | set_property_key | wall |
| --- | --- | --- | --- | --- | --- |
| 120 | 0.26 / 0.36 | 0.61 / 1.35 | | | 1.0 s |
| 1024 | 0.95 / 1.75 | 3.3 / 5.7 | 1.27 / 1.71 | 1.31 / 2.04 | 5.3 s |
| 2048 | 1.76 / 2.87 | 5.3 / 11.1 | | | 16.9 s |
| 4096 | 2.77 / 5.12 | 10.1 / 20.7 (8.3 / 0.95) | 2.40 / 2.94 | 1.47 / 2.39 | 58.6 s |

Edit recompose p50 at 4096 is 10 ms (target 100 ms); journal phases stay under
0.5 ms (`journal_scene` 0.48, `journal_admit` 0.006); the bench process peaked at
550 MB RSS across all four sizes. 4096 simple rects serialize to about 0.6 MB, so
the 4 MiB source limit is headroom for path-heavy scenes; decode cost is linear
in source bytes (about 8 ms per 0.6 MB), so a full 4 MiB source extrapolates to
about 55 ms (not measured separately). The playback tick path was not re-measured
(`native_play_bench` is unchanged). Responses stay small: `tree_get` and
`element_query` page with `cursor` and `limit` (at most 512 per page, 64 KiB per
reply); `export_svg` still refuses sources over 64 KiB (use `save_as`, or page
the tree).

Baseline (before optimization), p50 / p95 ms:

| tool | n | p50 | p95 |
| --- | --- | --- | --- |
| element_add | 120 | 0.26 | 0.38 |
| native_timeline_add_property_track | 50 | 1.21 | 2.26 |
| native_timeline_set_property_key | 400 | 1.71 | 3.20 |

Total wall 0.80 s (idle machine). The scene cap was 1024 elements then; the
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

### Phase attribution (release, one run, p50 / p95 ms)

Build: one `element_add` per element, then a read-only probe (`scene_scan`,
`recompose`) after each add, as the UI does after a source change.

| phase | 120 | 1024 |
| --- | --- | --- |
| element_add (tool) | 0.38 / 0.47 | 1.31 / 2.55 |
| edit_apply (`mcp.rs` add closure) | 0.14 / 0.21 | 0.85 / 1.66 |
| element_id_scan (`mcp_extras.rs` reserve_add) | 0.01 / 0.02 | 0.04 / 0.08 |
| journal_scene (`journal_integration.rs`) | 0.00 / 0.01 | 0.15 / 0.52 |
| scene_scan (`native_commands.rs` cache miss) | 0.03 / 0.06 | 0.22 / 0.42 |
| recompose (`compose/mod.rs` compose_document) | 0.93 / 1.34 | 4.96 / 9.95 |

Native tracks: 50 `add_property_track` + 400 `set_property_key`, per op.

| phase | 120 tracks | 1024 tracks |
| --- | --- | --- |
| add_property_track (tool) | 1.41 / 2.33 | 2.28 / 3.12 |
| set_property_key (tool) | 1.90 / 3.30 | 2.44 / 3.91 |
| edit_apply | 1.05 / 2.09 | 1.11 / 2.31 |
| edit_validate (`timeline_edit.rs` transact) | 0.18 / 0.33 | 0.19 / 0.35 |
| edit_ops (`vexel-timeline` edit.rs) | 0.88 / 1.78 | 0.90 / 1.92 |
| candidate_open (`native_commands.rs:83`) | 0.26 / 0.46 | 0.30 / 0.52 |
| candidate_reopen (`native_commands.rs:119`) | 0.24 / 0.44 | 0.27 / 0.48 |
| journal_admit (`native_commands.rs:148`) | 0.02 / 0.05 | 0.29 / 0.43 |
| copy_assets, scene_copy, payload_encode | < 0.01 | < 0.02 |

Bulk: one `add_property_track` with 8 keys in one call, 50 tracks, 1024-element
scene: 11.07 / 19.53 ms (120-element scene: 9.95 / 18.61 ms). Its `edit_ops`
runs once per key (about 0.9 ms each), so cost is linear in keys per call.

Hot spots on the request path, ranked by p50 cost per native op at 1024:

1. Full-timeline re-decode on every edit. `vexel-timeline/src/edit.rs:75`
   (`TimelineEdit::new` decodes the source) and `edit.rs:144` (`commit` decodes
   the candidate), reached through `project_session/timeline_edit.rs` `transact`.
   About 1.1 ms per op; grows with timeline keys, not scene size.
2. Candidate re-open and re-validate, twice per op: `native_commands.rs:83`
   (initial open) and `:119` (re-open of the result), both
   `timeline_edit.rs` `ProjectTimelineCandidate::open` (classify + decode).
   About 0.57 ms per op. `copy_assets` is 0.00 ms, so it is not a hot spot.
3. Journal admission scales with retained states: `project_session/history.rs:395`
   `payload_bytes` runs a pairwise identity loop, called from `history.rs:223`
   `prepare_payload`. 0.29 ms with about 1024 retained states at 1024 elements,
   0.003 ms with about 100 states in the bulk run. History cap: `history.rs:9`.

After the redundant-decode, double-reopen and journal-identity fixes (release,
1024 scene, p50 / p95 ms): `add_property_track` 2.28 / 3.12 to 1.48 / 2.13,
`set_property_key` 2.44 / 3.91 to 1.45 / 2.42, `journal_admit` 0.29 / 0.43 to
0.009 / 0.013, 8-key bulk add 11.07 to 7.60 p50 (p95 unchanged at about 19 ms).
The hot-spot list above describes the state before that change.

Largest per-add cost, off the request path: `recompose` at 4.96 ms p50 for 1024
elements (`compose/mod.rs:117`, full re-render per source change on the compose
worker). `edit_apply` for element_add also grows with scene size (0.14 to 0.85 ms).
Tool totals exceed the phase sum by roughly 0.3 to 0.5 ms (rig JSON-RPC and
per-request thread overhead).

## Historical/unsupported transport notes

Older versions of this page described a `vexel-mcp` sidecar, a per-user Unix
socket, socket relay behavior, and sidecar-specific relay fixtures. Those
descriptions do not apply to the current Rust app and are retained here only
as a record that the old launch flow is unsupported. The current flow is the
app's stdio server launched with `--mcp-preview` as shown above.

## Render safety limits

Rendering refuses (typed `limit_render`) a document that exceeds any of these:

- Stroke build steps: 6,000,000 per document (was 4,000,000; `MAX_STROKE_STEPS`).
- Stroke primitive bytes: 192 MiB per document (`MAX_STROKE_BYTES`).
- Pattern or image tile: 4096 px per axis and 8,388,608 px area (was 1,048,576; `PATTERN_MAX_PIXELS`).
- Pattern tiles alive at once: 1024 tiles and 128 MiB of RGBA in total (`PATTERN_MAX_TOTAL_BYTES`).

Raised values were measured in release on the real corpus: france-osm-mapnik 1.6 s and 1.50 GB peak RSS,
columbiarivermap 0.9 s and 217 MB.
