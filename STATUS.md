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
