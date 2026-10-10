# MCP tests

`test_e2e.py` exercises the stdio relay and headless broker against temporary
granted directories. It checks tool discovery, edits and transactions, empty
text-template content updates, validated XML fragments, rendered text and an
embedded PNG, closing a clean non-active document, and Save As to an existing
or new granted path. The native media path needs the macOS CoreText and ImageIO
frameworks. The test skips only when the execution environment denies Unix
domain socket binding.

`mcp_fuzz` runs the deterministic 200,000-case tool argument campaign;
`mcp_protocol_test` covers JSON-RPC framing and schema validation.
`mcp_features_test` calls the broker directly without a Unix socket, covering
media-aware timed PNG rendering, text and fragment edits, animation track list,
add, set, and remove operations, PNG frame sequence and GIF export, atomic
new-path Save As, output timing/frame-count checks, refusal to overwrite output,
granted-directory checks, and closing one clean document while another remains
open. It keeps these feature checks runnable when the host blocks socket bind.
