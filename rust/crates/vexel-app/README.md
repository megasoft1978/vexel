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
