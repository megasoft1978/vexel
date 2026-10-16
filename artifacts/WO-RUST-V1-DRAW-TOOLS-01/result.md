# WO-RUST-V1-DRAW-TOOLS-01 result (candidate)

Window-free tool layer: Select/Move/Delete, Rectangle, Ellipse, Line, Pen
(polyline and cubic Bezier), fill/stroke recolour. Every action is one
source-preserving, undoable `V1Document` edit.

## Test counts (frozen commands, all exit 0)
- vexel-app: 59 passed (tools 27 new, `model` 1 new; previous tests unchanged)
- vexel-v1-document: 31 passed (14 new in `draw_tests`, 17 previously)
- vexel-document-edit: 17 + 7 + 6 + 2 + 10 + 2 + 1 passed, no source change
- fmt check clean, `cargo check` 0 warnings, wo_validate and ledger validate pass

## APP-COMPOSE-01 mapping status
Not integrated at this base. The tools use the `CanvasMap` trait
(`to_doc(x, y) -> Option<(f64, f64)>`) with a tested `ScaleMap`. Wiring the
compose `ViewMap` function into `CanvasMap` is an UNEXECUTED integration step for
the coordinator.

## Not executed (manual verification)
- Live canvas drawing: `main.rs` only declares `mod tools;`. No native pointer
  Down/Move/Up routing into `ToolController` exists yet; `stage_local` (pure,
  tested) is the intended routing function. No window was launched.
- Rail painting of the new Line slot (`Tool::Line`) is visual and unchecked.

## Deviations to disclose
- `render.rs` (rail glyph/label match) and `preview.rs` (`document_error` match)
  needed one-arm edits because `Tool::Line` and four new `V1DocumentError`
  variants made existing exhaustive matches non-compiling. Neither file is on
  the allowed list; edits are minimal.
- AC2 `vexel-svg-path` decode: not a dev-dependency of any touched crate, so the
  produced `d` is checked with the same `validate_path_data` parser as
  `add_path` only. No Cargo.toml/Cargo.lock change was made.
- Multi-attribute edits (move, stroke) use `Editor::replace_source_span` on the
  minimal span, relabelled with `history_relabel_latest`, so they are one entry.
- Hit testing covers direct children of the root with an `id`; non-translate
  transforms are ignored for hit testing and refuse moves.
- git commit/stage were blocked by the sandbox hook in this session; the work
  is left uncommitted in the worktree.
