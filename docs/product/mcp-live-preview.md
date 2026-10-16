# Vexel live MCP drawing preview

> **Preview scope:** This is an interim basic-shape live preview for macOS. It
> is not the Vexel V1 release or the completed product. A live Codex MCP run
> initialized, discovered the preview tools, and added eight shapes to the same
> visible Vexel window. The preview remains static; animation, SVG import or
> export, save or reopen, and the finished demo video are still to come. A
> follow-up fix for upside-down canvas text is in source and needs a fresh live
> preview before its visual result is confirmed.

The preview is designed to open a blank 512 × 512 document in a Vexel window
and let a connected Codex session add simple rectangles and ellipses through
local MCP stdio. Each accepted add should appear in that same window before the
MCP call returns. The UI should show recent activity and operation results.

## Configure Codex

Add this server entry to Codex's MCP configuration. Replace the example
`cwd` with the absolute path to your Vexel checkout. Codex launches the app
process and connects to its standard input and output; JSON-RPC framing is the
only output written to stdout.

```toml
[mcp_servers.vexel_preview]
command = "cargo"
args = ["run", "-p", "vexel-app", "--", "--mcp-preview"]
cwd = "/absolute/path/to/vexel"
```

The preview is local and uses the caller-owned stdio connection. It does not
start a remote service. Start Vexel from Codex's MCP server controls after
saving the configuration. The command is expected to build and launch the
preview process, open its native window, and then serve MCP requests. Keep the
Codex MCP server running while using the preview. When finished, stop the
server from Codex's MCP controls; the Vexel preview process and its window
should close with it. A normal Vexel launch without `--mcp-preview` remains the
standard app launch.

## Available tools

The preview profile should advertise only these implemented operations:

| Tool | Preview behavior |
|---|---|
| `document_list` | List the current preview document. |
| `document_info` | Inspect the current document dimensions and basic document information. |
| `element_add` | Add one rectangle or ellipse with bounded geometry and a six-digit RGB color. |
| `undo` | Undo the latest accepted document edit. |
| `redo` | Redo the latest undone document edit. |

`element_add` accepts only `rect` and `ellipse`. Geometry must be finite,
non-degenerate, normalized, and within the 512 × 512 document. Calls that are
malformed, unsupported, out of bounds, or exceed the 128-shape preview limit
should fail without a partial document update. The MCP frame is bounded by the
existing stdio transport limit.

The renderer is a bounded basic-primitive preview. Tool names and argument
schemas are discoverable through MCP; ask Codex to inspect the tools if the
client does not show their details in its UI.

## Copyable prompt

With the server connected and the Vexel preview window open, paste this into
Codex:

```text
Use the Vexel preview MCP tools to create a simple friendly character in the
open 512x512 document. First inspect the document. Build the character from
rectangles and ellipses only: make a large rounded-looking blue ellipse for the
body, a smaller skin-colored ellipse for the head, two small dark ellipses for
the eyes, and a red rectangle for a simple scarf. Place each shape fully inside
the canvas, use six-digit RGB colors, and add the shapes one at a time so I can
watch them appear in Vexel. After the first version, inspect the document and
revise the character by adding two small yellow ellipses as buttons and a green
rectangle as a little hat brim. Inspect it again and tell me what changed. Use
only the available preview tools; do not claim to save or export the artwork.
```

Codex should use successive `element_add` calls, inspecting as useful, so each
accepted shape is visible in the open Vexel window. Undo and redo can revise the
latest edit. If a requested detail needs an unsupported shape or operation,
Codex should explain the limitation and keep the drawing within the preview
profile.

## Planned animation demo

The preview character is **Pip, the little sky courier**: a friendly paper-cut
mascot framed like a small animated-film title card. Pip wears a cobalt flight
suit, a warm apricot face, a mint pilot cap, a coral scarf, and a round gold
parcel badge. A raised mitten makes the greeting readable at thumbnail size.
Two or three tiny cross-shaped glints and a soft oval ground shadow give the
character a setting without competing with the silhouette. The canvas uses a
cool pale blue-gray field; dark ink-blue under-shapes create a consistent
outline, while smaller colored ellipses and rectangles sit inside them as clean
cel-shaded fills. Keep generous space around Pip and center the figure slightly
below the canvas midpoint so the floating glints have room to move.

Use this compact palette: ink `#25324A`, cobalt `#5B75E8`, apricot `#F2C99F`,
mint `#76D3B4`, coral `#F17868`, and parcel gold `#FFD166`; background is
`#EEF2F8`. Repeat the silhouette inset consistently and keep eyes, badge, and
scarf simple enough to read at a small window size. The art should be built from
visible source elements in drawing order, with stable IDs returned to Codex so
later animation calls can name the parts they affect.

The live MCP sequence should show Codex building Pip piece by piece, then
authoring and playing a 3.2-second loop. At rest, the whole character breathes
up and down by a few canvas units. The raised arm and mitten travel through two
small side-to-side greeting arcs; the scarf tip lags behind the body by a few
frames. Pip blinks once with a quick close and slower reopen. The glints rise a
little and fade in staggered timing. End positions and opacity match the start
so the loop joins cleanly. Keep the motion soft and legible: no camera move,
fast spin, bounce squash, or flashing.

For the recording, show the actual Codex conversation beside the Vexel window:
Codex creates Pip through successive MCP calls, inspects and adjusts one
detail, adds the named animation tracks, starts playback, then changes the wave
timing and lets the updated loop play. Each accepted edit or playback command
must publish a visible frame before the MCP response. This is a proposed design,
not a capability of the current static preview. The follow-up implementation
must use Vexel's SVG source and animation parser/evaluator, keep edits
undoable, and label the primitive preview's rendering limits accurately. The
recording comes after the live drawing and animation have been visually checked.

The intended authoring calls use the stable ID returned by each `element_add`:
`animation_add` attaches a source fragment such as
`<animate attributeName="cy" values="180;176;180" dur="3.2s" repeatCount="indefinite"/>`
to a named ellipse, while `animation_control` plays, pauses, or seeks the
preview. Blink and scarf tracks use the same source fragment route, with bounded
values and matching start/end values for the seamless loop. These tools are
planned; until implementation lands, they are not available in the preview.

## Scope and known limitations

This preview covers blank-document inspection, rectangle and ellipse insertion,
and undo/redo, with basic visible activity feedback. It is deliberately narrow.
It does not provide animation, SVG import or export, save or reopen, broad MCP
editing, or a final demo video. Those remain later work under the Vexel V1 and
full-product goals. It also does not claim arbitrary SVG decoding, remote MCP,
packaged release readiness, or completed macOS native runtime verification.

The running server's tool discovery and one live shape-creation session have
been confirmed on this Mac. That proves the initial preview path and visible
shape publication here; it does not establish compatibility across every
supported macOS version. The text-orientation correction still needs a fresh
native preview launch, as noted above.
