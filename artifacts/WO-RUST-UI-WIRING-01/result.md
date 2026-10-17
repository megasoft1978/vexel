# WO-RUST-UI-WIRING-01 result (candidate)

Undo, Redo, Save, Export and the inspector toggle now act by pointer and keyboard.
Logic: new `src/toolbar_actions.rs` (press, key, edit, Exports), `Model::{button,menu_item,pointer_action,nav,...}`, `layout/menu.rs` (Export popover), history from the core cursor (`history.rs` has no counter).

Export paths: GIF = `export_gif` via `ExportJob` (24 fps, loop duration) into a fresh private staging dir that is the sole grant of a one-off AccessPolicy, then renamed over the chosen file; PNG = the MCP `render_png` writer (McpHost with the same one-dir policy), renamed; SVG = source bytes, temp sibling renamed. No STOP.

Deviations: (1) `Model::button` takes (Button, layout); popover state lives in Model (`export_open`, `focus_button`, `menu_focus`) because `layout/state.rs` is protected. (2) `route_pointer` stays in the binary: tests drive `toolbar_actions::press` plus the splitter phases (Shell::route). (3) NativePanels' Save panel appends `.vexel`; export strips it and ensures the format extension (platform crate protected; follow-up: extension-aware panel). (4) Inspector toggle was painted disabled while the inspector was hidden; now enabled iff window >= 900 wide. (5) Clicks under an open popover (MCP too) are swallowed, canvas skipped.

Pins updated: tests/ui_polish_behaviour.rs (redo test now cursor-based; all_targets gains menu rows), tests/ui_motion_adapter.rs (FOCUS_ORDER last index). REGISTERED 24->27, FOCUS_ORDER 7->12.
Goldens: new candidates only in review/ (8 frames); no existing golden changed.
Gaps: `cargo fmt --check` for vexel-ui already fails at base (protected crate, 51 diffs); vexel-app is clean. Quitting during a GIF export may leave a `.vexel-export-*` dir. No reduce-motion accessor.
