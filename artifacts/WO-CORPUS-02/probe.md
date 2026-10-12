# WO-CORPUS-02 probe (executor, base 974dfed)

Command: build/mac-debug/bin/svgworkbench --render-frame .../generated/cad_t2.svg --size 128x128
Result today: `limit at byte 2680259` (release and debug), exit 1. Not byte 9,365,917 as BLOCKERS.md defect 1 states.

Probe: temporary backtrace()+backtrace_symbols_fd in svg_shape_fail for SVG_SHAPE_ERROR_LIMIT (reverted):
  svg_shape_fail <- svg_shape_builder_begin_shape (shape_geometry.c: shape_count >= SVG_SHAPE_MAX_SHAPES)
  <- svg_shape_builder_add_element <- svg_shape_traverse_render_node <- walk_range (x3 nesting)
  <- decode_source_with_language_and_media_and_bounds <- svg_shape_decode_with_media <- viewer_load_document <- app_run_render_frame

The refusing budget is the 65,536 element budget (SVG_SHAPE_MAX_SHAPES), not the edge array.
Element census of cad_t2 (python regex): path 106,497, rect 53,249, line 53,249, text 42,598 (> 250k renderable elements).
Edges at the refusal point are far below 4,194,304 (refusal is at 2.68 MB of 10.6 MB).

Informational scratch run (reverted): SHAPES=1,048,576 and EDGES=16,777,216 -> still `limit at byte 9448701`,
peak RSS 375,472,128 B (release). So raising only the shape and edge budgets does not make cad_t2 render either;
a further site at ~9.45 MB is not yet identified (not probed further; the order forbids raising other budgets).

Other discrepancies: manifest.json sha256 for generated-cad_t2 is 2ddafd2e...d934 (matches the file on disk);
the order's 9fbf9a2f... does not appear in the manifest. Size 10,592,198 matches.

Conflicts with allowed paths if the storage change were made: tests/anim/anim_test.c and tests/editor/editor_followups_test.c
index/memcpy `document->edges` as a flat array, and tests/fuzz/shape/shape_fuzz.c uses SVG_SHAPE_MAX_EDGES; all are outside the
allowlist, so removing the flat array / the constant would break their compilation.
