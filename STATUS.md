# Vexel status (live source of truth; supersedes HANDOFF.md / BLOCKERS.md / STATE.md for planning)

Updated 2026-10-20. Plan: V1 vertical slice per `docs/adr/0029-v1-vertical-slice-cutline.md`, then post-V1 items.

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
