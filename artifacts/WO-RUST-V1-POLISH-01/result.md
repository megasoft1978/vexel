# WO-RUST-V1-POLISH-01 result

Candidate; not reviewed. All frozen commands pass (vexel-app lib 178 tests, vexel-protocol suites ok).

- Open filter: `OPEN_EXTENSIONS = ["svg","vexel"]`. Deliberate behaviour change
  (HANDOFF lesson 3): the three `file_flow` tests pinning `open:vexel` (not two)
  now expect `open:svg,vexel`; new test opens an `.svg` (untitled, `svg_origin`
  set) and a `.vexel` (path set).
- Dedup: `file_flow::ShortcutDedup` (`dedup.rs`, `DEDUP_WINDOW` = 150 ms, `now`
  injected as `Duration`), 6 tests. Implemented in both directions (menu then key,
  key then menu) because delivery order is not knowable by code reading; same-source
  repeats and different actions are never suppressed. `main.rs` routes key and menu
  through `file_key`/`file_menu`; a suppressed press returns `Cancelled`.
- Double fire reachable by code reading: not shown. The platform crate (protected)
  installs the menu key equivalents; whether AppKit also delivers `keyDown:` for the
  same press was not provable from `vexel-app`, so the guard lands as defence.
- Preview `element_add` enum/description now name rect, ellipse, line, path. Two
  assertions in `mcp_preview_profile.rs` that pinned the old enum/description were
  updated; non-preview schema untouched.
- Dead code: `McpHost::handle`, `standalone`, `ExternalClock`, `McpHost::is_playing`
  removed (symbol search: no other callers). Tests migrated to `handle_with` +
  `Driver<ManualClock>` with identical expected values (play/pause times via
  `advance(0.5)`). Deleted tests (both only exercised removed code):
  `session::tests_playback::handle_and_handle_with_reply_alike_for_the_same_inputs`,
  `preview_app::tests::reply_follows_staged_state_and_is_only_committed_for_frame`.
  Dropped assertion `!host.is_playing()` (host no longer holds a clock).
  `preview_app.rs` keeps only `start_server` (+ its test).
- Blanket `allow` removed from file_flow, tools, project_session, session, preview,
  preview_app. Retained item-level `cfg_attr(not(test), allow(dead_code))`:
  `preview/animation.rs` `rasterize`, `rasterize_at`, `rasterize_with_opacity`
  (test-only preview rasterizer). Also removed an unused `Path` import in
  `session/tests.rs` exposed by the lint change.

Unexecuted live checks: real Cmd+S in the window, real Open panel showing .svg.
