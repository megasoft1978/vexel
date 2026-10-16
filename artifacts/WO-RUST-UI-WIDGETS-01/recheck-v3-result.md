ACCEPT — no remaining blocker in the requested scope.

`widget_paint`, `paint_tree_row`, and `paint_empty_state` preflight their component geometry before writing. The pass checks translated and scaled endpoints with checked arithmetic; valid off-canvas geometry remains valid. Primitive paint methods use the same checked conversion.

The retained assertions cover indent 255’s full content offset (3612) with six guides, near-`u32::MAX` tree-row rejection without pixel mutation, and the valid scale-1 label at `u32::MAX-64` reaching the text callback. The recorded verification reports 17 passing UI tests and the prior repairs retained. I did not rerun checks, as requested. This is not full-app or performance certification.