# App, platform, and UI semantic crosswalk

Evidence-only candidate map for WO-RUST-PORT-MAP-01. The frozen inventory and reconciliation retain every baseline row as `pending_port`; this report does not change those dispositions or assert parity. Every path below is copied from `baseline.jsonl` and carries its original disposition.

## Behavior crosswalk anchors

Path rows below remain the accounting authority: each gives its candidate classification and unchanged baseline `pending_port` disposition. The following groups connect concrete legacy entry points, Rust candidate symbols, accepted result documents, and frozen tests. A connection is still only a candidate unless the cited result explicitly accepts a same-behavior comparison; none of these results claims C-oracle parity.

| Group and owned path families | Legacy behavior/symbol | Rust path and symbol | Accepted result and Rust evidence | Frozen baseline tests/oracles |
|---|---|---|---|---|
| **A1 compose/frame and app launch** — `src/app/frame_compose*`, `src/app/render_frame_cli*`, `src/app/viewer.c`, `app/main.c`, `app/index_replay*`, `tests/app/viewer_frame_test.c`, `tests/test_viewer_frame.py` | `viewer_allocate_frame`, `viewer_submit_job`, `viewer_request_open`, `viewer_app_tick`; `app_run_index_replay`, `run_native_cycle` | [`rust/crates/vexel-app/src/compose/worker.rs`](../../rust/crates/vexel-app/src/compose/worker.rs) `ComposeWorker`; [`compose/mod.rs`](../../rust/crates/vexel-app/src/compose/mod.rs) `compose_document_owned`; [`src/main.rs`](../../rust/crates/vexel-app/src/main.rs) `main` | [`WO-RUST-APP-SHARED-SOURCE-01`](../WO-RUST-APP-SHARED-SOURCE-01/result.md): owned compose route and worker behavior accepted for its own AC; [`compose/tests_open.rs`](../../rust/crates/vexel-app/src/compose/tests_open.rs) tests borrowed/owned frame equivalence and allocation identity. Does not establish viewer-shell or CLI parity. | [`tests/app/viewer_frame_test.c`](https://github.com/megasoft1978/vexel/blob/legacy-c-final/tests/app/viewer_frame_test.c), [`tests/test_viewer_frame.py`](https://github.com/megasoft1978/vexel/blob/legacy-c-final/tests/test_viewer_frame.py), `app/main.c` native smoke entry points (frozen source oracle). |
| **A2 canvas navigation and editor gestures** — `src/app/viewer_canvas_geometry*`, `src/app/viewer_editor_canvas*`, `src/app/viewer_editor_selection*`, `src/app/viewer_editor_transform*`, `tests/app/viewer_canvas_geometry_test.c`, `tests/editor/editor_canvas_test.c`, `tests/editor/editor_canvas_fuzz_test.c`, `tests/editor/viewer_editor_guides_test.c`, `tests/test_viewer_navigation.py` | `viewer_canvas_navigation_resize`, `viewer_editor_canvas_pointer_down`, `viewer_editor_canvas_pointer_move`, `viewer_editor_canvas_pointer_up`, `viewer_editor_get_canvas_overlay` | [`rust/crates/vexel-app/src/canvas/map.rs`](../../rust/crates/vexel-app/src/canvas/map.rs) `ViewMap`; [`canvas/view.rs`](../../rust/crates/vexel-app/src/canvas/view.rs) `View`; [`canvas/events.rs`](../../rust/crates/vexel-app/src/canvas/events.rs) event routing; [`tools/controller.rs`](../../rust/crates/vexel-app/src/tools/controller.rs) tool gestures | [`WO-RUST-UI-CANVAS-01`](../WO-RUST-UI-CANVAS-01/result.md): `View`/`ViewMap`, zoom/pan and overlays have focused Rust checks and candidate goldens (not accepted legacy goldens); Rust evidence [`tests/ui_canvas_view.rs`](../../rust/crates/vexel-app/tests/ui_canvas_view.rs), [`tests/ui_canvas_pointer.rs`](../../rust/crates/vexel-app/tests/ui_canvas_pointer.rs). | [`tests/app/viewer_canvas_geometry_test.c`](https://github.com/megasoft1978/vexel/blob/legacy-c-final/tests/app/viewer_canvas_geometry_test.c), [`tests/editor/editor_canvas_test.c`](https://github.com/megasoft1978/vexel/blob/legacy-c-final/tests/editor/editor_canvas_test.c), [`tests/editor/editor_canvas_fuzz_test.c`](https://github.com/megasoft1978/vexel/blob/legacy-c-final/tests/editor/editor_canvas_fuzz_test.c), [`tests/test_viewer_navigation.py`](https://github.com/megasoft1978/vexel/blob/legacy-c-final/tests/test_viewer_navigation.py). |
| **A3 chrome, commands, history and file actions** — `src/app/viewer_chrome*`, `src/app/viewer_command_dispatch.c`, `src/app/viewer_commands*`, `src/app/viewer_history.c`, `src/app/viewer_load*`, `src/app/viewer_save.c`, `tests/editor/viewer_chrome_test.c`, `tests/app/viewer_history_test.c`, `tests/platform/platform_file_route_test.m`, `tests/test_ui_chrome_golden.py` | `viewer_chrome_layout_compute`, `viewer_apply_command`, `viewer_command_find`, `viewer_command_search`, `viewer_history_callback`, `viewer_request_open` | [`rust/crates/vexel-app/src/layout/state.rs`](../../rust/crates/vexel-app/src/layout/state.rs) `Layout`; [`layout/paint.rs`](../../rust/crates/vexel-app/src/layout/paint.rs) `draw_chrome`; [`toolbar_actions.rs`](../../rust/crates/vexel-app/src/toolbar_actions.rs) `press`/`run`; [`history.rs`](../../rust/crates/vexel-app/src/history.rs) history cursor; [`file_flow/mod.rs`](../../rust/crates/vexel-app/src/file_flow/mod.rs) file actions | [`WO-RUST-UI-CHROME-01`](../WO-RUST-UI-CHROME-01/result.md) documents candidate layout and changed constraints; [`WO-RUST-UI-WIRING-01`](../WO-RUST-UI-WIRING-01/result.md) documents pointer/keyboard Undo, Redo, Save, Export; [`tests/ui_chrome_goldens.rs`](../../rust/crates/vexel-app/tests/ui_chrome_goldens.rs), [`tests/ui_wiring_toolbar.rs`](../../rust/crates/vexel-app/tests/ui_wiring_toolbar.rs) are Rust checks. Both results retain candidate-only golden status. | [`tests/editor/viewer_chrome_test.c`](https://github.com/megasoft1978/vexel/blob/legacy-c-final/tests/editor/viewer_chrome_test.c), [`tests/app/viewer_history_test.c`](https://github.com/megasoft1978/vexel/blob/legacy-c-final/tests/app/viewer_history_test.c), [`tests/platform/platform_file_route_test.m`](https://github.com/megasoft1978/vexel/blob/legacy-c-final/tests/platform/platform_file_route_test.m), [`tests/test_ui_chrome_golden.py`](https://github.com/megasoft1978/vexel/blob/legacy-c-final/tests/test_ui_chrome_golden.py). |
| **A4 timeline and animation controls** — `src/app/viewer_timeline*`, `src/app/viewer_anim_edit*`, `tests/app/viewer_timeline_test.c`, `tests/app/viewer_timeline_keyframe_test.c`, `tests/app/viewer_timeline_native_test.c`, `tests/ui/viewer_timeline_render.c`, `tests/ui/verify_timeline_goldens.py` | `viewer_timeline_time_from_x`, `viewer_timeline_seek`, `viewer_timeline_toggle_play`, `viewer_timeline_advance`, `viewer_timeline_sync_selection` | [`rust/crates/vexel-app/src/timeline/model.rs`](../../rust/crates/vexel-app/src/timeline/model.rs) `TimelineModel`; [`timeline/layout.rs`](../../rust/crates/vexel-app/src/timeline/layout.rs) layout; [`timeline/paint.rs`](../../rust/crates/vexel-app/src/timeline/paint.rs) paint/input | [`WO-RUST-UI-TIMELINE-01`](../WO-RUST-UI-TIMELINE-01/result.md) documents timeline candidate and focused checks; [`WO-RUST-UI-TIMELINE-02`](../WO-RUST-UI-TIMELINE-02/result.md) records two golden failures pending coordinator review and explicitly candidate status; Rust evidence [`tests/ui_timeline_model.rs`](../../rust/crates/vexel-app/tests/ui_timeline_model.rs), [`tests/ui_timeline_input.rs`](../../rust/crates/vexel-app/tests/ui_timeline_input.rs), [`tests/ui_timeline_golden.rs`](../../rust/crates/vexel-app/tests/ui_timeline_golden.rs). | [`tests/app/viewer_timeline_test.c`](https://github.com/megasoft1978/vexel/blob/legacy-c-final/tests/app/viewer_timeline_test.c), [`tests/app/viewer_timeline_keyframe_test.c`](https://github.com/megasoft1978/vexel/blob/legacy-c-final/tests/app/viewer_timeline_keyframe_test.c), [`tests/app/viewer_timeline_native_test.c`](https://github.com/megasoft1978/vexel/blob/legacy-c-final/tests/app/viewer_timeline_native_test.c), [`tests/ui/viewer_timeline_render.c`](https://github.com/megasoft1978/vexel/blob/legacy-c-final/tests/ui/viewer_timeline_render.c), [`tests/ui/verify_timeline_goldens.py`](https://github.com/megasoft1978/vexel/blob/legacy-c-final/tests/ui/verify_timeline_goldens.py). |
| **U1 retained UI widgets, draw, tokens and icons** — `src/ui/ui_core*`, `src/ui/ui_draw*`, `src/ui/ui_widget*`, `src/ui/ui_widgets*`, `src/ui/ui_icons*`, `src/ui/ui_icon_paths*`, `tests/ui/ui_core_test.c`, `tests/ui/ui_interaction_test.c`, `tests/ui/ui_kit_test.c`, `tests/ui/ui_tokens_contrast_test.c`, `tests/ui/verify_style_goldens.py` | `ui_tree_init`, `ui_tree_add`, `ui_tree_layout`, `ui_tree_render`, `ui_focus_next`, `ui_semantics_snapshot_build`, `ui_widget_add_button`, `ui_tree_paint_into` | [`rust/crates/vexel-ui/src/tree.rs`](../../rust/crates/vexel-ui/src/tree.rs) `Tree::{layout,render}`; [`interaction.rs`](../../rust/crates/vexel-ui/src/interaction.rs) interaction; [`widget_input.rs`](../../rust/crates/vexel-ui/src/widget_input.rs) `TextEdit`, `number_parse`, `hex_parse`; [`widget_canvas.rs`](../../rust/crates/vexel-ui/src/widget_canvas.rs) `widget_paint`; [`draw.rs`](../../rust/crates/vexel-ui/src/draw.rs) drawing; [`tokens.rs`](../../rust/crates/vexel-ui/src/tokens.rs) `Tokens`; [`icons.rs`](../../rust/crates/vexel-ui/src/icons.rs) icon definitions | [`WO-RUST-UI-ICONS-01`](../WO-RUST-UI-ICONS-01/result.md), [`WO-RUST-UI-ICONS-02`](../WO-RUST-UI-ICONS-02/result.md), [`WO-RUST-UI-TOKENS-01`](../WO-RUST-UI-TOKENS-01/result.md) record implementation and Rust checks; their icon/token goldens remain candidates. Rust evidence: [`tests.rs`](../../rust/crates/vexel-ui/src/tests.rs), [`tests_icons.rs`](../../rust/crates/vexel-ui/src/tests_icons.rs), [`tests_tokens.rs`](../../rust/crates/vexel-ui/src/tests_tokens.rs). | [`tests/ui/ui_core_test.c`](https://github.com/megasoft1978/vexel/blob/legacy-c-final/tests/ui/ui_core_test.c), [`tests/ui/ui_interaction_test.c`](https://github.com/megasoft1978/vexel/blob/legacy-c-final/tests/ui/ui_interaction_test.c), [`tests/ui/ui_kit_test.c`](https://github.com/megasoft1978/vexel/blob/legacy-c-final/tests/ui/ui_kit_test.c), [`tests/ui/ui_tokens_contrast_test.c`](https://github.com/megasoft1978/vexel/blob/legacy-c-final/tests/ui/ui_tokens_contrast_test.c), [`tests/ui/verify_style_goldens.py`](https://github.com/megasoft1978/vexel/blob/legacy-c-final/tests/ui/verify_style_goldens.py). |
| **U2 chrome integration and motion** — `src/ui/viewer_controls*`, `src/ui/ui_widget_chrome_paint.c`, `tests/ui/verify_chrome.py`, `tests/ui/verify_home_goldens.py`, `tests/ui/verify_command_overlay_goldens.py`, `tests/ui/verify_guides_goldens.py`, `tests/ui/verify_native_shell_goldens.py`, `tests/ui/verify_navigation.py`, `tests/test_ui_chrome_golden.py` | `viewer_chrome_layout_compute`, `ui_viewer_navigation_key`, `ui_viewer_progress_update`, `ui_viewer_semantics` | [`rust/crates/vexel-app/src/chrome.rs`](../../rust/crates/vexel-app/src/chrome.rs) `chrome_text`; [`layout/paint.rs`](../../rust/crates/vexel-app/src/layout/paint.rs) chrome paint; [`rust/crates/vexel-ui/src/motion.rs`](../../rust/crates/vexel-ui/src/motion.rs) `MotionRegistry`; [`rust/crates/vexel-app/src/motion/paint.rs`](../../rust/crates/vexel-app/src/motion/paint.rs) motion paint | [`WO-RUST-UI-CHROME-01`](../WO-RUST-UI-CHROME-01/result.md), [`WO-RUST-UI-MOTION-01`](../WO-RUST-UI-MOTION-01/result.md) describe implementation candidates and golden status; Rust evidence [`ui_chrome_goldens.rs`](../../rust/crates/vexel-app/tests/ui_chrome_goldens.rs), [`ui_motion_adapter.rs`](../../rust/crates/vexel-app/tests/ui_motion_adapter.rs). | Frozen C golden/render tests under [`tests/ui/`](https://github.com/megasoft1978/vexel/blob/legacy-c-final/tests/ui/) and [`tests/editor/viewer_chrome_test.c`](https://github.com/megasoft1978/vexel/blob/legacy-c-final/tests/editor/viewer_chrome_test.c); specific Python verifier paths are listed at left and remain frozen. |
| **P1 native window, panels, atomic files and tasks** — `src/platform/macos/platform_window*`, `platform_dialog*`, `platform_atomic_file*`, `platform_task*`, `tests/platform/platform_dialog_test.m`, `tests/platform/platform_file_route_test.m`, `tests/platform/platform_recovery_store_test.m`, `tests/platform/platform_png_test.c`, `tests/test_platform_driver.py`, `tests/test_ui_driver.py`, `tests/viewer/native_smoke*`, `tests/viewer/panel_smoke*` | `platform_window_create`, `platform_window_open_panel`, `platform_window_dispatch_action`, `platform_atomic_file_replace`; task callbacks from `viewer_submit_job` | [`rust/crates/vexel-platform-apple/src/native_window/app.rs`](../../rust/crates/vexel-platform-apple/src/native_window/app.rs) `NativeApp::{create_window,poll_events}`; [`panels/mod.rs`](../../rust/crates/vexel-platform-apple/src/panels/mod.rs) `checked_path`, `with_project_extension`; [`rust/crates/vexel-platform-macos/src/atomic_files/mod.rs`](../../rust/crates/vexel-platform-macos/src/atomic_files/mod.rs) atomic replace/create-new contracts; [`rust/crates/vexel-platform-tasks/src/queue.rs`](../../rust/crates/vexel-platform-tasks/src/queue.rs) `TaskQueue::{submit,poll,cancel}` | [`WO-RUST-V1-NATIVE-PANELS-01`](../WO-RUST-V1-NATIVE-PANELS-01/result.md) records focused native-panel implementation/check results; Rust evidence [`native_window.rs`](../../rust/crates/vexel-platform-apple/tests/native_window.rs), [`atomic_files/tests.rs`](../../rust/crates/vexel-platform-macos/src/atomic_files/tests.rs), [`platform-tasks/src/tests.rs`](../../rust/crates/vexel-platform-tasks/src/tests.rs). These do not prove all original event/panel/recovery behavior. | [`tests/platform/platform_dialog_test.m`](https://github.com/megasoft1978/vexel/blob/legacy-c-final/tests/platform/platform_dialog_test.m), [`tests/platform/platform_file_route_test.m`](https://github.com/megasoft1978/vexel/blob/legacy-c-final/tests/platform/platform_file_route_test.m), [`tests/platform/platform_recovery_store_test.m`](https://github.com/megasoft1978/vexel/blob/legacy-c-final/tests/platform/platform_recovery_store_test.m), [`tests/test_platform_driver.py`](https://github.com/megasoft1978/vexel/blob/legacy-c-final/tests/test_platform_driver.py), [`tests/test_ui_driver.py`](https://github.com/megasoft1978/vexel/blob/legacy-c-final/tests/test_ui_driver.py), [`tests/viewer/native_smoke.py`](https://github.com/megasoft1978/vexel/blob/legacy-c-final/tests/viewer/native_smoke.py), [`tests/viewer/panel_smoke.py`](https://github.com/megasoft1978/vexel/blob/legacy-c-final/tests/viewer/panel_smoke.py). |
| **P2 localization, accessibility, media and device services** — `src/platform/macos/platform_accessibility*`, `platform_localization*`, `platform_clipboard*`, `platform_metal*`, `src/platform/media/**`, `src/app/media_providers.c`, `tests/editor/accessibility_localization_test.c`, `tests/editor/editor_media_provider_test.c`, `tests/platform_media/**`, `tests/test_platform_media.py`, `tests/test_w15_accessibility_localization.py` | Supported text subbehavior only: `platform_text` and `app_text_layout`. Unsupported entry points include `platform_accessibility_update`, `platform_localization_copy`, `platform_clipboard_write_svg`, `svg_app_media_providers`, and `app_image_decode`. | Partial candidate: [`rust/crates/vexel-platform-apple/src/text_layout.rs`](../../rust/crates/vexel-platform-apple/src/text_layout.rs) `text_layout` covers the `platform_text` / `app_text_layout` glyph-outline callback only. No candidate is established for image/animation/media providers, Metal, accessibility, localization, or clipboard. | [`WO-RUST-SVG-TEXT-01`](../WO-RUST-SVG-TEXT-01/result.md) explicitly maps legacy `platform_text.m` plus `app_text_layout` to `vexel-platform-apple::text_layout`; it says app wiring was not executed, so this is partial evidence only. No accepted result establishes the other P2 services. Rust evidence: [`text_layout.rs` tests](../../rust/crates/vexel-platform-apple/tests/text_layout.rs). The frozen SVG image/text oracle is cross-referenced to core ownership: [`tests/svg/shape/text_image_test.c`](https://github.com/megasoft1978/vexel/blob/legacy-c-final/tests/svg/shape/text_image_test.c). | Frozen P2 oracles: [`tests/editor/accessibility_localization_test.c`](https://github.com/megasoft1978/vexel/blob/legacy-c-final/tests/editor/accessibility_localization_test.c), [`tests/editor/editor_media_provider_test.c`](https://github.com/megasoft1978/vexel/blob/legacy-c-final/tests/editor/editor_media_provider_test.c), [`tests/platform_media/platform_media_test.c`](https://github.com/megasoft1978/vexel/blob/legacy-c-final/tests/platform_media/platform_media_test.c), [`tests/platform_media/media_probe.c`](https://github.com/megasoft1978/vexel/blob/legacy-c-final/tests/platform_media/media_probe.c), [`tests/test_platform_media.py`](https://github.com/megasoft1978/vexel/blob/legacy-c-final/tests/test_platform_media.py), [`tests/test_w15_accessibility_localization.py`](https://github.com/megasoft1978/vexel/blob/legacy-c-final/tests/test_w15_accessibility_localization.py). `tests/platform_media/generate_fixtures.py` remains unresolved; no generator counterpart is evidenced. |
| **T1 top-level app tools and demos** — `app/editor_chrome_render.c`, `app/editor_command_overlay_render.c`, `app/index_bench.c`, `app/index_replay*`, `app/main.c`, `app/spatial_cache_bench.c`, `app/test_driver/**`, `app/ui_demo*`, `app/ui_focus_bench.c`, `app/vexel_mcp_main.c` | `main`, `app_run_index_replay`, `ui_demo_tree_build`, `ui_demo_render`, `run_native_cycle`; benchmark `main` functions | Partial CLI/UI candidate: [`rust/crates/vexel-app/src/main.rs`](../../rust/crates/vexel-app/src/main.rs) `main`, [`preview_app.rs`](../../rust/crates/vexel-app/src/preview_app.rs) `PreviewApp`, [`compose/worker.rs`](../../rust/crates/vexel-app/src/compose/worker.rs) `ComposeWorker`. No established replacement for the legacy test driver or named benchmark tools. | [`WO-RUST-APP-SHARED-SOURCE-01`](../WO-RUST-APP-SHARED-SOURCE-01/result.md) accepts its source-ownership/worker change only; it does not map these top-level drivers or demos. Rust checks from that result are not native-driver equivalence evidence. | [`app/main.c`](https://github.com/megasoft1978/vexel/blob/legacy-c-final/app/main.c), [`app/test_driver/driver.py`](https://github.com/megasoft1978/vexel/blob/legacy-c-final/app/test_driver/driver.py), [`app/test_driver/native_task_tests.c`](https://github.com/megasoft1978/vexel/blob/legacy-c-final/app/test_driver/native_task_tests.c), `tests/test_ui_driver.py`, and `tests/e2e/**` where applicable. |

### Revision-2 added path rows

These paths were assigned by revision 2 and have one accounting row each here. All preserve the frozen `pending_port` disposition. They are linked to the behavior groups above; the media fixture generator is intentionally unresolved.

#### `app/**` (16 paths)

- `app/editor_chrome_render.c` — partial candidate (T1/U2); inventory disposition `pending_port`.
- `app/editor_command_overlay_render.c` — partial candidate (T1/U2); inventory disposition `pending_port`.
- `app/index_bench.c` — unresolved (T1); inventory disposition `pending_port`.
- `app/index_replay.c` — partial candidate (A1/T1); inventory disposition `pending_port`.
- `app/index_replay.h` — partial candidate (A1/T1); inventory disposition `pending_port`.
- `app/main.c` — partial candidate (A1/T1/P1); inventory disposition `pending_port`.
- `app/spatial_cache_bench.c` — unresolved (T1); inventory disposition `pending_port`.
- `app/test_driver/driver.py` — unresolved (T1/P1); inventory disposition `pending_port`.
- `app/test_driver/native_injection.h` — unresolved (T1/P1); inventory disposition `pending_port`.
- `app/test_driver/native_task_tests.c` — partial candidate (P1); inventory disposition `pending_port`.
- `app/test_driver/native_task_tests.h` — partial candidate (P1); inventory disposition `pending_port`.
- `app/ui_demo.c` — partial candidate (T1/U1); inventory disposition `pending_port`.
- `app/ui_demo.h` — partial candidate (T1/U1); inventory disposition `pending_port`.
- `app/ui_demo_render.c` — partial candidate (T1/U1); inventory disposition `pending_port`.
- `app/ui_focus_bench.c` — unresolved (T1); inventory disposition `pending_port`.
- `app/vexel_mcp_main.c` — unresolved (T1); MCP implementation mapping is owned by the other slice and no app binary parity is accepted here; inventory disposition `pending_port`.

#### `tests/viewer/**` (7 paths)

- `tests/viewer/native_smoke.py` — unresolved (P1); inventory disposition `pending_port`.
- `tests/viewer/native_smoke.sh` — unresolved (P1); inventory disposition `pending_port`.
- `tests/viewer/panel_smoke.py` — unresolved (P1); inventory disposition `pending_port`.
- `tests/viewer/panel_smoke.sh` — unresolved (P1); inventory disposition `pending_port`.
- `tests/viewer/repeat_native_smoke.sh` — unresolved (P1); inventory disposition `pending_port`.
- `tests/viewer/test_native_smoke.py` — unresolved (P1); inventory disposition `pending_port`.
- `tests/viewer/viewer_quality_test.c` — partial candidate (A1/P1); inventory disposition `pending_port`.

#### Newly assigned Python test rows (7 paths)

- `tests/test_platform_driver.py` — unresolved (P1); inventory disposition `pending_port`.
- `tests/test_platform_media.py` — unresolved (P2); inventory disposition `pending_port`.
- `tests/test_ui_chrome_golden.py` — partial candidate (A3/U2); inventory disposition `pending_port`.
- `tests/test_ui_driver.py` — unresolved (P1); inventory disposition `pending_port`.
- `tests/test_viewer_frame.py` — partial candidate (A1); inventory disposition `pending_port`.
- `tests/test_viewer_navigation.py` — partial candidate (A2); inventory disposition `pending_port`.
- `tests/test_w15_accessibility_localization.py` — unresolved (P2); inventory disposition `pending_port`.

## Path-level inventory and classification

Classification is per path: `partial candidate` means a plausible Rust semantic area exists but assertion-level evidence is absent; `unresolved` means no relevant Rust behavior/evidence was established. No row qualifies as `candidate replacement` or `no Rust candidate found` from the evidence available. Original frozen disposition is shown unchanged.

### `src/app/` (109 paths)

- `src/app/feedback_diagnostics.c` — partial candidate; inventory disposition `pending_port`.
- `src/app/feedback_diagnostics.h` — partial candidate; inventory disposition `pending_port`.
- `src/app/frame_compose.c` — partial candidate; inventory disposition `pending_port`.
- `src/app/frame_compose.h` — partial candidate; inventory disposition `pending_port`.
- `src/app/mcp_broker_hook.c` — partial candidate; inventory disposition `pending_port`.
- `src/app/mcp_broker_hook.h` — partial candidate; inventory disposition `pending_port`.
- `src/app/mcp_broker_io.c` — partial candidate; inventory disposition `pending_port`.
- `src/app/mcp_broker_io.h` — partial candidate; inventory disposition `pending_port`.
- `src/app/media_providers.c` — partial candidate for `app_text_layout` only (`WO-RUST-SVG-TEXT-01`); image/media provider callbacks unresolved; inventory disposition `pending_port`.
- `src/app/platform_command_policy.c` — partial candidate; inventory disposition `pending_port`.
- `src/app/platform_command_policy.h` — partial candidate; inventory disposition `pending_port`.
- `src/app/render_frame_cli.c` — partial candidate; inventory disposition `pending_port`.
- `src/app/render_frame_cli.h` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer.c` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer.h` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_anim_edit.c` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_anim_edit.h` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_canvas_geometry.c` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_canvas_geometry.h` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_chrome.c` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_chrome.h` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_chrome_editor.c` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_chrome_inspector.c` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_chrome_render.c` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_chrome_structure.c` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_command_dispatch.c` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_commands.c` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_commands.h` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_display.c` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_display_name.c` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_document_factory.c` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_editor.c` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_editor.h` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_editor_actions.c` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_editor_arrange.c` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_editor_canvas.c` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_editor_canvas_draw.c` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_editor_canvas_pointer.c` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_editor_clipboard.c` — unresolved; no supported Rust symbol/result crosswalk; inventory disposition `pending_port`.
- `src/app/viewer_editor_contextual.c` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_editor_document.c` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_editor_guides.c` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_editor_guides.h` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_editor_guides_paint.c` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_editor_input.c` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_editor_internal.h` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_editor_keyboard.c` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_editor_log.c` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_editor_motion.c` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_editor_multi_edit.c` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_editor_overlay.c` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_editor_paint.c` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_editor_palette.c` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_editor_path.c` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_editor_path_curves.c` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_editor_path_node_commands.c` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_editor_path_nodes.c` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_editor_preview.c` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_editor_result.c` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_editor_reveal.c` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_editor_selection.c` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_editor_selection_groups.c` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_editor_snap.c` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_editor_snap_ui.c` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_editor_status.c` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_editor_structure.c` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_editor_test_layout.c` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_editor_test_layout.h` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_editor_text.c` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_editor_transform.c` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_editor_tree_legacy.c` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_editor_view.c` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_editor_visibility.c` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_editor_widget_paint.c` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_events.c` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_export.c` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_file_watch.c` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_history.c` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_home.c` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_home.h` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_home_controller.c` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_home_render.c` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_internal.h` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_load.c` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_load.h` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_native_actions.c` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_native_state.c` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_quality.c` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_quality.h` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_recovery.c` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_render.c` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_runtime.c` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_save.c` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_sessions.c` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_sessions.h` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_test_layout_dump.c` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_timeline.c` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_timeline.h` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_timeline_accessibility.c` — unresolved; no supported Rust symbol/result crosswalk; inventory disposition `pending_port`.
- `src/app/viewer_timeline_cel.c` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_timeline_easing.c` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_timeline_layout.c` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_timeline_native_move.c` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_timeline_transform.c` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_timeline_ui.c` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_workspace.c` — partial candidate; inventory disposition `pending_port`.
- `src/app/viewer_workspace.h` — partial candidate; inventory disposition `pending_port`.
- `src/app/welcome_model.c` — partial candidate; inventory disposition `pending_port`.
- `src/app/welcome_model.h` — partial candidate; inventory disposition `pending_port`.

### `src/ui/` (20 paths)

- `src/ui/index_progress.c` — partial candidate; inventory disposition `pending_port`.
- `src/ui/index_progress.h` — partial candidate; inventory disposition `pending_port`.
- `src/ui/ui_core.c` — partial candidate; inventory disposition `pending_port`.
- `src/ui/ui_core.h` — partial candidate; inventory disposition `pending_port`.
- `src/ui/ui_draw.c` — partial candidate; inventory disposition `pending_port`.
- `src/ui/ui_draw.h` — partial candidate; inventory disposition `pending_port`.
- `src/ui/ui_icon_paths.c` — partial candidate; inventory disposition `pending_port`.
- `src/ui/ui_icon_paths.h` — partial candidate; inventory disposition `pending_port`.
- `src/ui/ui_icons.c` — partial candidate; inventory disposition `pending_port`.
- `src/ui/ui_icons.h` — partial candidate; inventory disposition `pending_port`.
- `src/ui/ui_icons_legacy.c` — partial candidate; inventory disposition `pending_port`.
- `src/ui/ui_widget_canvas.c` — partial candidate; inventory disposition `pending_port`.
- `src/ui/ui_widget_chrome_paint.c` — partial candidate; inventory disposition `pending_port`.
- `src/ui/ui_widget_input.c` — partial candidate; inventory disposition `pending_port`.
- `src/ui/ui_widget_paint.c` — partial candidate; inventory disposition `pending_port`.
- `src/ui/ui_widget_paint_internal.h` — partial candidate; inventory disposition `pending_port`.
- `src/ui/ui_widgets.c` — partial candidate; inventory disposition `pending_port`.
- `src/ui/ui_widgets.h` — partial candidate; inventory disposition `pending_port`.
- `src/ui/viewer_controls.c` — partial candidate; inventory disposition `pending_port`.
- `src/ui/viewer_controls.h` — partial candidate; inventory disposition `pending_port`.

### `src/platform/` (50 paths)

- `src/platform/macos/platform_accessibility.h` — unresolved (P2); no same-behavior Rust candidate established; inventory disposition `pending_port`.
- `src/platform/macos/platform_accessibility.m` — unresolved (P2); no same-behavior Rust candidate established; inventory disposition `pending_port`.
- `src/platform/macos/platform_atomic_file.c` — partial candidate; inventory disposition `pending_port`.
- `src/platform/macos/platform_atomic_new_file.c` — partial candidate; inventory disposition `pending_port`.
- `src/platform/macos/platform_clipboard.h` — unresolved (P2); no same-behavior Rust candidate established; inventory disposition `pending_port`.
- `src/platform/macos/platform_clipboard.m` — unresolved (P2); no same-behavior Rust candidate established; inventory disposition `pending_port`.
- `src/platform/macos/platform_cursor.m` — partial candidate; inventory disposition `pending_port`.
- `src/platform/macos/platform_dialog.h` — partial candidate; inventory disposition `pending_port`.
- `src/platform/macos/platform_dialog.m` — partial candidate; inventory disposition `pending_port`.
- `src/platform/macos/platform_feedback.m` — partial candidate; inventory disposition `pending_port`.
- `src/platform/macos/platform_localization.m` — unresolved (P2); no same-behavior Rust candidate established; inventory disposition `pending_port`.
- `src/platform/macos/platform_menus.m` — partial candidate; inventory disposition `pending_port`.
- `src/platform/macos/platform_metal.m` — unresolved (P2); no same-behavior Rust candidate established; inventory disposition `pending_port`.
- `src/platform/macos/platform_metal_internal.h` — unresolved (P2); no same-behavior Rust candidate established; inventory disposition `pending_port`.
- `src/platform/macos/platform_png.h` — partial candidate; inventory disposition `pending_port`.
- `src/platform/macos/platform_png.m` — partial candidate; inventory disposition `pending_port`.
- `src/platform/macos/platform_recents.m` — partial candidate; inventory disposition `pending_port`.
- `src/platform/macos/platform_recovery_store.m` — partial candidate; inventory disposition `pending_port`.
- `src/platform/macos/platform_settings.m` — partial candidate; inventory disposition `pending_port`.
- `src/platform/macos/platform_task.c` — partial candidate; inventory disposition `pending_port`.
- `src/platform/macos/platform_welcome.m` — partial candidate; inventory disposition `pending_port`.
- `src/platform/macos/platform_window.m` — partial candidate; inventory disposition `pending_port`.
- `src/platform/macos/platform_window_animation.m` — partial candidate; inventory disposition `pending_port`.
- `src/platform/macos/platform_window_files.m` — partial candidate; inventory disposition `pending_port`.
- `src/platform/macos/platform_window_geometry.m` — partial candidate; inventory disposition `pending_port`.
- `src/platform/macos/platform_window_internal.h` — partial candidate; inventory disposition `pending_port`.
- `src/platform/macos/platform_window_menus.m` — partial candidate; inventory disposition `pending_port`.
- `src/platform/macos/platform_window_native_shell.m` — partial candidate; inventory disposition `pending_port`.
- `src/platform/media/platform_animation.h` — unresolved (P2); no same-behavior Rust candidate established; inventory disposition `pending_port`.
- `src/platform/media/platform_animation.m` — unresolved (P2); no same-behavior Rust candidate established; inventory disposition `pending_port`.
- `src/platform/media/platform_image.h` — unresolved (P2); no same-behavior Rust candidate established; inventory disposition `pending_port`.
- `src/platform/media/platform_image.m` — unresolved (P2); no same-behavior Rust candidate established; inventory disposition `pending_port`.
- `src/platform/media/platform_media.c` — unresolved (P2); no same-behavior Rust candidate established; inventory disposition `pending_port`.
- `src/platform/media/platform_media.h` — unresolved (P2); no same-behavior Rust candidate established; inventory disposition `pending_port`.
- `src/platform/media/platform_text.h` — partial candidate for `platform_text` layout only (`WO-RUST-SVG-TEXT-01`); inventory disposition `pending_port`.
- `src/platform/media/platform_text.m` — partial candidate for `platform_text` layout only (`WO-RUST-SVG-TEXT-01`); inventory disposition `pending_port`.
- `src/platform/platform_atomic_file.h` — partial candidate; inventory disposition `pending_port`.
- `src/platform/platform_atomic_new_file.h` — partial candidate; inventory disposition `pending_port`.
- `src/platform/platform_feedback.h` — partial candidate; inventory disposition `pending_port`.
- `src/platform/platform_localization.h` — unresolved (P2); no same-behavior Rust candidate established; inventory disposition `pending_port`.
- `src/platform/platform_localization_defaults.c` — unresolved (P2); no same-behavior Rust candidate established; inventory disposition `pending_port`.
- `src/platform/platform_localization_internal.h` — unresolved (P2); no same-behavior Rust candidate established; inventory disposition `pending_port`.
- `src/platform/platform_metal.h` — unresolved (P2); no same-behavior Rust candidate established; inventory disposition `pending_port`.
- `src/platform/platform_recents.h` — partial candidate; inventory disposition `pending_port`.
- `src/platform/platform_recovery_store.h` — partial candidate; inventory disposition `pending_port`.
- `src/platform/platform_settings.h` — partial candidate; inventory disposition `pending_port`.
- `src/platform/platform_task.h` — partial candidate; inventory disposition `pending_port`.
- `src/platform/platform_text_ui.h` — unresolved; no supported Rust symbol/result crosswalk; inventory disposition `pending_port`.
- `src/platform/platform_text_ui.m` — unresolved; no supported Rust symbol/result crosswalk; inventory disposition `pending_port`.
- `src/platform/platform_window.h` — partial candidate; inventory disposition `pending_port`.

### `tests/app/` (16 paths)

- `tests/app/feedback_diagnostics_test.c` — partial candidate; inventory disposition `pending_port`.
- `tests/app/platform_command_policy_test.c` — partial candidate; inventory disposition `pending_port`.
- `tests/app/project_save_test.c` — partial candidate; inventory disposition `pending_port`.
- `tests/app/viewer_canvas_geometry_test.c` — partial candidate; inventory disposition `pending_port`.
- `tests/app/viewer_editor_visibility_test.c` — partial candidate; inventory disposition `pending_port`.
- `tests/app/viewer_frame_test.c` — partial candidate; inventory disposition `pending_port`.
- `tests/app/viewer_history_test.c` — partial candidate; inventory disposition `pending_port`.
- `tests/app/viewer_home_layout_fuzz.c` — partial candidate; inventory disposition `pending_port`.
- `tests/app/viewer_home_test.c` — partial candidate; inventory disposition `pending_port`.
- `tests/app/viewer_recovery_test.c` — partial candidate; inventory disposition `pending_port`.
- `tests/app/viewer_sessions_test.c` — partial candidate; inventory disposition `pending_port`.
- `tests/app/viewer_timeline_bootstrap_test.c` — partial candidate; inventory disposition `pending_port`.
- `tests/app/viewer_timeline_keyframe_test.c` — partial candidate; inventory disposition `pending_port`.
- `tests/app/viewer_timeline_native_test.c` — partial candidate; inventory disposition `pending_port`.
- `tests/app/viewer_timeline_test.c` — partial candidate; inventory disposition `pending_port`.
- `tests/app/welcome_model_test.c` — partial candidate; inventory disposition `pending_port`.

### `tests/editor/` (27 paths)

- `tests/editor/accessibility_localization_test.c` — unresolved (P2); no same-behavior Rust candidate established; inventory disposition `pending_port`.
- `tests/editor/editor_canvas_fuzz_test.c` — partial candidate; inventory disposition `pending_port`.
- `tests/editor/editor_canvas_test.c` — partial candidate; inventory disposition `pending_port`.
- `tests/editor/editor_clipboard_ids_test.c` — unresolved; no supported Rust symbol/result crosswalk; inventory disposition `pending_port`.
- `tests/editor/editor_command_palette_test.c` — partial candidate; inventory disposition `pending_port`.
- `tests/editor/editor_followups_test.c` — partial candidate; inventory disposition `pending_port`.
- `tests/editor/editor_media_provider_test.c` — unresolved (P2); no same-behavior Rust candidate established; inventory disposition `pending_port`.
- `tests/editor/editor_selection_render.c` — partial candidate; inventory disposition `pending_port`.
- `tests/editor/editor_snap_overlay_render.c` — partial candidate; inventory disposition `pending_port`.
- `tests/editor/editor_snap_popover_render.c` — partial candidate; inventory disposition `pending_port`.
- `tests/editor/editor_state_test.c` — partial candidate; inventory disposition `pending_port`.
- `tests/editor/inspector_tab_reachability_test.c` — partial candidate; inventory disposition `pending_port`.
- `tests/editor/mouse.py` — partial candidate; inventory disposition `pending_port`.
- `tests/editor/native_smoke.sh` — partial candidate; inventory disposition `pending_port`.
- `tests/editor/native_smoke_actions.sh` — partial candidate; inventory disposition `pending_port`.
- `tests/editor/smoke_supervisor.py` — partial candidate; inventory disposition `pending_port`.
- `tests/editor/test_smoke_supervisor.py` — partial candidate; inventory disposition `pending_port`.
- `tests/editor/verify_accessibility_localization.py` — unresolved (P2); no same-behavior Rust candidate established; inventory disposition `pending_port`.
- `tests/editor/verify_chrome.py` — partial candidate; inventory disposition `pending_port`.
- `tests/editor/verify_selection_goldens.py` — partial candidate; inventory disposition `pending_port`.
- `tests/editor/verify_snap_overlay_goldens.py` — partial candidate; inventory disposition `pending_port`.
- `tests/editor/verify_snap_popover_goldens.py` — partial candidate; inventory disposition `pending_port`.
- `tests/editor/viewer_chrome_test.c` — partial candidate; inventory disposition `pending_port`.
- `tests/editor/viewer_editor_guides_test.c` — partial candidate; inventory disposition `pending_port`.
- `tests/editor/viewer_editor_ux_test.c` — partial candidate; inventory disposition `pending_port`.
- `tests/editor/viewer_visual_test.c` — partial candidate; inventory disposition `pending_port`.
- `tests/editor/viewer_workspace_test.c` — partial candidate; inventory disposition `pending_port`.

### `tests/ui/` (24 paths)

- `tests/ui/native_ui_render.m` — partial candidate; inventory disposition `pending_port`.
- `tests/ui/platform_text_ui_test.c` — unresolved; no supported Rust symbol/result crosswalk; inventory disposition `pending_port`.
- `tests/ui/ui_core_test.c` — partial candidate; inventory disposition `pending_port`.
- `tests/ui/ui_demo_interaction_test.c` — partial candidate; inventory disposition `pending_port`.
- `tests/ui/ui_gallery.c` — partial candidate; inventory disposition `pending_port`.
- `tests/ui/ui_icon_gallery.c` — partial candidate; inventory disposition `pending_port`.
- `tests/ui/ui_interaction_test.c` — partial candidate; inventory disposition `pending_port`.
- `tests/ui/ui_kit_test.c` — partial candidate; inventory disposition `pending_port`.
- `tests/ui/ui_tokens_contrast_test.c` — partial candidate; inventory disposition `pending_port`.
- `tests/ui/ui_widget_fuzz.c` — partial candidate; inventory disposition `pending_port`.
- `tests/ui/verify_chrome.py` — partial candidate; inventory disposition `pending_port`.
- `tests/ui/verify_command_overlay_goldens.py` — partial candidate; inventory disposition `pending_port`.
- `tests/ui/verify_guides_goldens.py` — partial candidate; inventory disposition `pending_port`.
- `tests/ui/verify_home_goldens.py` — partial candidate; inventory disposition `pending_port`.
- `tests/ui/verify_native_shell_goldens.py` — partial candidate; inventory disposition `pending_port`.
- `tests/ui/verify_navigation.py` — partial candidate; inventory disposition `pending_port`.
- `tests/ui/verify_smoke.py` — partial candidate; inventory disposition `pending_port`.
- `tests/ui/verify_style_goldens.py` — partial candidate; inventory disposition `pending_port`.
- `tests/ui/verify_timeline_goldens.py` — partial candidate; inventory disposition `pending_port`.
- `tests/ui/verify_uikit_gallery.py` — partial candidate; inventory disposition `pending_port`.
- `tests/ui/viewer_chrome_layout_fuzz.c` — partial candidate; inventory disposition `pending_port`.
- `tests/ui/viewer_guides_render.c` — partial candidate; inventory disposition `pending_port`.
- `tests/ui/viewer_navigation_test.c` — partial candidate; inventory disposition `pending_port`.
- `tests/ui/viewer_timeline_render.c` — partial candidate; inventory disposition `pending_port`.

### `tests/platform/` (5 paths)

- `tests/platform/platform_dialog_test.m` — partial candidate; inventory disposition `pending_port`.
- `tests/platform/platform_file_route_test.m` — partial candidate; inventory disposition `pending_port`.
- `tests/platform/platform_png_test.c` — partial candidate; inventory disposition `pending_port`.
- `tests/platform/platform_recovery_store_test.m` — partial candidate; inventory disposition `pending_port`.
- `tests/platform/viewer_export_test.c` — partial candidate; inventory disposition `pending_port`.

### `tests/platform_media/` (3 paths)

- `tests/platform_media/generate_fixtures.py` — unresolved; no Rust fixture-generator counterpart evidenced; inventory disposition `pending_port`.
- `tests/platform_media/media_probe.c` — unresolved (P2); no same-behavior Rust candidate established; inventory disposition `pending_port`.
- `tests/platform_media/platform_media_test.c` — unresolved (P2); no same-behavior Rust candidate established; inventory disposition `pending_port`.

### `tests/e2e/` (9 paths)

- `tests/e2e/__init__.py` — unresolved; inventory disposition `pending_port`.
- `tests/e2e/render_soak_driver.c` — unresolved; inventory disposition `pending_port`.
- `tests/e2e/test_dev_final.py` — unresolved; inventory disposition `pending_port`.
- `tests/e2e/test_doctor.py` — unresolved; inventory disposition `pending_port`.
- `tests/e2e/test_e2e_svg.py` — unresolved; inventory disposition `pending_port`.
- `tests/e2e/test_editor_alignment_shortcuts.py` — unresolved; inventory disposition `pending_port`.
- `tests/e2e/test_packaging.py` — unresolved; inventory disposition `pending_port`.
- `tests/e2e/test_soak.py` — unresolved; inventory disposition `pending_port`.
- `tests/e2e/test_workflows.py` — unresolved; inventory disposition `pending_port`.

## Disjoint future work packages and dependencies

- **Viewer/editor behavior**: reconcile `src/app/**`, `tests/app/**`, `tests/editor/**` by behavior families (document lifecycle/history/recovery; canvas/selection/tools; commands/clipboard; inspector/structure; timeline/animation editing; home/workspace/export). Depend on core document/edit semantics and stable `vexel-app` session/model APIs; preserve each C test assertion and visual golden as oracle.
- **UI toolkit and chrome**: reconcile `src/ui/**` and `tests/ui/**` by widget core/input/drawing/icons/tokens, then chrome/navigation/home/command overlay/guides/timeline visual goldens. Depends on app/editor semantic models and stable drawing/theme contracts.
- **macOS platform and media**: reconcile `src/platform/**`, `tests/platform/**`, `tests/platform_media/**` in native lifecycle/input/window, file dialogs/clipboard/atomic save/recovery, text/accessibility/localization, and image/animation/media provider packages. Depends on platform interface decisions and app callback contracts; preserve macOS-only owner directive.
- **End-to-end workflows**: port `tests/e2e/**` after app, UI and platform slices, as workflow drivers depend on integrated app startup, file I/O, rendering and interaction APIs. This slice owns migration of these workflow harnesses and records them as unresolved; tooling may cross-reference this report without classifying or owning these paths.

## Unresolved mapping groups

- All 293 assigned rows remain `pending_port` in the frozen baseline; reconciliation reports zero accepted replacement rows. This report now supplies concrete symbol/result/oracle links by behavior group, but there is still no accepted C-symbol-to-Rust-symbol-to-assertion parity chain for any listed row.
- UI pixel goldens, native shell/menu/window behavior, accessibility/localization, file routing, media decoding/provider behavior, recovery-store durability, and full editor workflows are especially unresolved.
- The app/UI/platform portions of Rust are substantial candidates, but results for their individual work orders are needed alongside frozen C oracle comparison before any path can be accepted.
