# WO-RUST-V1-PNG-01 result (candidate; AC7/AC8 blocked, see below)

Alpha convention: `AnimatedScene::render_sized` returns straight
(non-premultiplied) RGBA; PNG colour type 6 is straight alpha, so pixels are
written as held, no conversion. `render_png` renders at t=0 (static frame).

Compression (measured in `vexel-png/tests/encode.rs`, bytes of whole PNG):
- flat 256x256: Stored 262488, Fixed 2181 (0.83%)
- random 128x128: Stored 65737, Fixed 65737 (1.00; Fixed falls back to stored
  when its stream is not smaller, so it never exceeds Stored)
- 128x128 repeating 3000-byte random block: Stored 65737, Fixed 4217 (6.4%)

Independent check: Fixed and Stored files decoded identically by Python zlib
and macOS ImageIO (`sips`), CRCs verified.

Blocker: six existing tests pin `render_png` as unlisted/unknown in the preview
profile, contradicting scope 5 and AC7 ("existing tests pass unmodified"):
`vexel-protocol/tests/mcp_preview_profile.rs` (2: listing count 15 and
`!contains("render_png")`; "refuses unknown tool"),
`vexel-app/tests/mcp_tools_edit.rs` (2), `vexel-app/tests/walkthrough.rs` (2,
`!tools.contains("render_png")`). Not edited (protected). Each needs only its
render_png-absence assertion flipped; owner/coordinator decision.

Note: `render_png` already existed in the protocol `TOOLS` table (region/scale/
time, listed in the non-preview list). Not duplicated: the preview listing
overrides description/properties/required, and the table's unadvertised
`fields` gained path/width/height. Non-preview listing is unchanged.

Unexecuted: live MCP client use, GUI, rendering is synchronous on the UI thread.
