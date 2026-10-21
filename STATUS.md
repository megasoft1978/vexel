# Vexel status (live source of truth; supersedes HANDOFF.md / BLOCKERS.md / STATE.md for planning)

Updated 2026-10-20. Plan: V1 vertical slice per `docs/adr/0029-v1-vertical-slice-cutline.md`, then post-V1 items.

## Status 2026-10-20

- The live-preview MCP allowlist includes six native timeline tools: get,
  add-property-track, set-property-key, set-loop, undo, and redo. Add-track,
  add-drawing-asset, set-cel, remove-cel, and move-item remain unrouted.
- Native timeline undo/redo use shared project history and act on the latest
  edit, including edits outside the native timeline. Native GIF export is
  synchronous; loops over 120 frames are stride-sampled with delay multiplied
  by the stride.
- Select has eight resize handles. The inspector has fill/stroke swatches and
  stroke-width presets. The MCP popover shows Ready/Active and the last request.
- CI runs the pan-timing gate in release mode. Chrome and footer goldens are
  informational on hosted runners.

## Baseline
- `main` @ a072511d: 43 Rust crates, ~218k lines. `cargo check --workspace --locked` passes.
- In-flight dirty work (async export, svg-decode lease/lifetime, timeline, recovery) snapshotted on `wip/async-export` (bce34c83; does not compile: `vexel-svg-decode/src/definition_finish.rs` E0500). Not merged.
- Legacy C is at tag `legacy-c-final`; no C tree in the working copy.

## V1 lanes (disjoint crates)
| Lane | Crates | Exit |
|---|---|---|
| A Editor core | vexel-document-edit, vexel-ui | select/transform/style/undo on shapes and paths |
| B Animation | vexel-timeline, vexel-app native_timeline/native_animation | key edit, loop playback, mixed draw/anim undo |
| C Project IO | vexel-project, vexel-project-io | save/reopen round-trip incl. animation |
| D Export (sync) | vexel-gif, vexel-render | preset GIF export |
| E MCP live | vexel-mcp-*, vexel-protocol, session/mcp.rs | edit open project + activity indicator, real Codex client |
| F SVG import basics | vexel-svg-* | disclosed basic subset, gradients |

## Post-V1
Async export (from wip branch) -> SVG corpus fidelity (21/39) and large-doc design -> recovery/compat, perf, a11y -> Python tooling to Rust -> final UI/E2E, packaging (owner-gated), Codex showcase video.

## Open owner items
Signing/identity/distribution decisions; showcase publication.

## Decisions 2026-10-20 (coordinator, owner delegated)
- Native timeline MCP: V1 stays with the six routed property-track tools; add_track, add_drawing_asset, set_cel, remove_cel, move_item stay unrouted and unadvertised until the character-loop demo needs them.
- Full 42-entry tool schema stays as is (frozen `parity.rs` count); the live-preview list is already limited to routed tools and guarded by `tools_schema_matches_dispatch`.
- Timing-sensitive tests (pan present, cancel latency) gate in release builds on hosted CI; chrome/footer goldens are informational there until goldens are regenerated on a pinned runner image.

## SVG fidelity note (2026-10-21)
- Gradient stop inherited `color` is now resolved lazily (only for `currentColor` stops), removing a per-gradient quadratic token rescan that exhausted `PaintVisits`.
- `ideogram-human-chromosome-20` now decodes past gradients and stops at the clip planner's aggregate cap (`MAX_CLIP_JOB_REFERENCE_BYTES`, 512 KiB, `vexel-svg-definitions/src/planner.rs`). Raising it needs a design decision (lazy/streamed clip expansion), not a cap bump.
- Open: Dukes (`<path>`), Naples (`<use>`), image/text default value limits.

## Live click check (2026-10-21, automated via synthetic input)
Verified in the real window: open char.vexel/ball.vexel, selection + 8 handles, resize + undo/redo, fill swatch, stroke width, timeline scrub/play, cel switching, GIF export from the toolbar (512x512, 48 frames, looping). Fixed: GIF row was disabled for native timelines; a press on a resize handle cleared the selection. Open nits: export save panel suggests `name.gif.vexel` (project panel reused); unwritable export folder reports a generic write error.
- Keep caps: 4096-byte XML root, 32 MiB (owner 2026-10-21)

## Decisions 2026-10-21 (coordinator, owner delegated)
- Dukes `<path>` (f2) dropped: the fix conflicts with a frozen app test (clip-child `fill` stays unsupported). Naples nested `<use>` fixed; output viewport clamps to 4096 px instead of refusing.
- Deferred past V1 (known fidelity gaps, no cap changes): DTD internal entity expansion (columbiarivermap), `<tspan>` text (corsica), ideogram clip aggregate cap, france-osm render limit.
- MCP per-session request budget raised 120 -> 600 per minute (owner 2026-10-21) so scripted scene building is not throttled.
- Live preview tools: 51 (adds `native_timeline_set_duration`); `element_add` accepts any group as `parentId`; path data accepts Q/T/S (arcs unsupported).
- New app icon from generated artwork (`resources/AppIcon-source.png`, `scripts/make_icon.py --from-image`).
