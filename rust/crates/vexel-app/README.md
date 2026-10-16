# Vexel shell and live MCP preview

The normal launch remains the macOS Vexel workbench shell. The opt-in
`--mcp-preview` launch opens a basic-shape live preview and serves MCP over the
launcher's stdin/stdout. Connect a compatible MCP client to the process to
inspect its blank 512×512 document, add bounded rectangles or ellipses, and
undo or redo those edits. Successful changes are answered only after the native
window accepts the corresponding frame.

This is a bounded preview, not V1 completion. It does not provide general SVG
rendering, save/reopen, import/export, animation, or the full editor controls.

The app crate is safe Rust. Its native boundary is provided by
`vexel-platform-apple`, including bounded CoreText text rasterization and owned
RGBA frame presentation.

## Opening an SVG

`vexel <file.svg>` opens one SVG (ADR-0029 subset, at most 8 MiB) in the shell stage:
decode, scene, scalar render at 1 pixel per point, then presentation. Deferred
features are reported in the status bar, never dropped silently. Retina output
is pixel-replicated, not re-rendered. There is no File > Open... menu yet.
