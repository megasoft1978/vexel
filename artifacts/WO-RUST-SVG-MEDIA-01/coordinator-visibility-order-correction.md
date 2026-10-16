# Preserve separate image and text visibility order

The coordinator's earlier blanket instruction to skip visibility-hidden text
before providers was too broad. Preserve original behavior, not that instruction.
No original frozen input/assertion should change.

Original `src/svg/shape/text.c:475-574` performs layout, cumulative glyph/command
admission, x/y/dx/dy parsing and word-spacing checks before invoking
`svg_shape_builder_add_custom_element` for nonzero outline commands. That helper
checks visibility first (`shape_custom.c:20-29`). Hidden/collapse therefore skips
text geometry/publication but still validates font/layout/position input,
charges successful logical text use and advances cursor (`text.c:580-584`).
Unsupported visibility is likewise only checked if custom outline publication
is attempted; a no-outline layout does not invoke it. Display:none is the
separate early traversal skip. Image custom geometry wraps its provider and
therefore does skip the image provider for visibility:hidden/collapse.

Current text_geometry.rs:227 early hidden branch skips layout/counters/cursor,
which changes legacy behavior. Fix at the per-outline custom-publication stage,
retain the image early branch and display:none semantics. New incorrect hidden
text test expectations may be corrected explicitly; original34 functional/
atomicity predicates and original603/157 evidence remain frozen. Add provider
call/count/cursor and malformed font/layout/position precedence cases for hidden
text, no-outline invalid visibility, hidden image no-provider and rollback.

This is an owned MEDIA correction, independent of the genuine missing shared
pre-geometry shape admission operation. It does not require a new dependency.
