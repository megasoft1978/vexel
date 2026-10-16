# WO-RUST-V1-PLAYBACK-01 result

Status: candidate. `vexel-app` tests 174 pass (146 before, +28 new, none modified); check, fmt --check, wo_validate, ledger validate pass; `cargo tree` package set identical to INTEGRATE-01; no `unsafe`. Not committed: the sandbox hook refused every git command, so the tree is uncommitted and `git diff --check` was not run.

## Key map (no modifiers; any Cmd/Ctrl/Opt/Shift chord is not a playback key)
Space play/pause (49), Home seek 0 (115), Left/Right step one frame, paused only (123/124), L loop vs play-once (37). Ignored while a text field (none exists yet) or a tool gesture/pen path owns keys (`ToolController::owns_keys`, new) or the document has no loop. Unchanged: Cmd+Z/Shift+Cmd+Z, Cmd+S/O/N, Esc/Delete/Return (tools), D/A (modes).

## Design
- `animation::driver::Driver<C: LoopClock>`: one `Playback`, decoded `AnimatedScene` cached per `revision`, `FrameQueue` (capacity 2), cancel `Arc<AtomicBool>`. `tick(revision, bytes)` renders at most one frame at `playback.time()` (30 fps step), skips when the step is unchanged, returns only the newest queued frame. Frames render synchronously on the UI thread via new `AnimatedScene::render_sized(time, size, cancel)` at the stage size (same view box/aspect as compose; `render` delegates to it).
- Cancel-on-edit: a new revision drops queued frames, re-decodes, and `Playback::set_duration` clamps time (paused at 0 when the loop vanished). Frames carry their revision and are discarded if stale. The cancel flag is consumed by the next tick (`Cancelled`, nothing presented).
- Presentation: `DocumentView::present_animated(revision, Frame)` swaps the pixels of the composed frame (keeps map/hit shapes); `compose::blit_document` is the only blit. Refused for another revision or size. A finished static compose clears the flag, so the shell re-renders at the current time.
- `Playback` gained `looping`, `set_looping`, `set_duration`, `settle` (play-once pauses at the end frame); defaults keep old behaviour.
- Shell: tick each loop pass once the static compose of the current revision is on screen; dirty only when a frame was presented or the play state changed. Idle cost unchanged (same 16 ms sleep).
- Strip (`animation::strip`): pure layout/hit-test, `Slider` drives the drag (pause + seek, release stays paused), drawn with `compose::fill` + `draw_text_rgba` (`fill` made pub). Hidden without a loop.
- Production `PlaybackHost` = `Driver`; the shell calls `AppDocument::request_frame` when a revision is new.

## McpHost clock replacement
`McpHost` lost `playing/time/anchor`; `handle_with(app, op, &mut Driver)` routes play/pause/seek to the shared driver (replies and error strings identical). Deviation: the existing 3-arg `handle(app, op, now)` and `is_playing()` are kept for the unmodified INTEGRATE tests, backed by a lazily created standalone driver on a caller-supplied clock. The shell never uses it.

## Deltas vs the order's assumptions
- INTEGRATE-01 had not unified clocks; `McpHost` still owned them (as assumed).
- `AnimatedScene::render` is document-size only; added `render_sized` (needed so frames match the compose stage mapping; no renderer change).
- MCP `Seek` beyond the loop now clamps (reply still echoes the requested seconds, as before).
- Strip overlays the stage bottom (also in Animate mode, above the placeholder timeline panel); it blocks tool input beneath it.

## Gaps / unexecuted live checks
Rendering is synchronous (a slow frame blocks the UI; no worker). Editing recomposes statically first, so a brief blank/static frame may show after an edit while playing. KeyDown has no repeat flag (held Space may toggle repeatedly). All five order live checks are unexecuted.
