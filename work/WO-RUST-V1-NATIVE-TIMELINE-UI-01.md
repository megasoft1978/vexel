---
id: WO-RUST-V1-NATIVE-TIMELINE-UI-01
revision: 2
status: READY
milestone: FULL-RUST-PROJECT
owner: project-coordinator
executor: codex-gpt-6-luna-high-fallback
base_sha: 849371835fa4cb8c3e48ff154778a3ec5d7de196
spec_sha256: 9cccc2fde3473f2fa0e0e702de1fa650483caf8ab44f4c3b7b278c303814b990
parallel_safe: true
network: false
---
# Revision 2 note
Claude delegation failed before process launch because the installed wrapper raised `extra[@]: unbound variable`. This revision assigns the unchanged UI-only scope to Luna high with the frontend-design skill.

# Goal
Implement a custom-drawn native timeline panel for Vexel's native animation tracks, separate from the existing SVG SMIL timeline.

# Context and interface
Read `OWNER-DIRECTIVES.md`, ADR-0017, ADR-0020, ADR-0029, ADR-0032, `docs/ui/README.md`, `vexel-app/src/timeline/{mod.rs,layout.rs,paint.rs}`, and `vexel-ui/src/widget_canvas.rs`. Preserve the custom UI design system and existing Blender-aligned shell. This panel receives immutable `NativeTimelineSnapshot` data and emits `NativeTimelineAction` intents defined in the coordinator-owned parent module. Do not import or call session, history, MCP, file I/O, SMIL, or renderer APIs. Only UI display and transient interaction state belong here. Do not edit the existing `timeline/` SMIL modules.

# Allowed paths
- New `rust/crates/vexel-app/src/native_timeline/model.rs`
- New `rust/crates/vexel-app/src/native_timeline/layout.rs`
- New `rust/crates/vexel-app/src/native_timeline/paint.rs`
- New `rust/crates/vexel-app/src/native_timeline/interaction.rs`
- Work-order artifacts only

# Protected paths
Existing UI modules and all source except the allowed new files; module declarations, shared action/snapshot signatures, app/main wiring, session/history/payload/MCP, manifests/locks, recovery files, ADRs, work orders, ledger, C/Python migration references, and external paths. Do not run builds/tests or GUI checks. Author focused tests where practical, left unexecuted.

# Acceptance
- AC-1: displays native drawing/property tracks, cels/keys, tick ruler, playhead, selection, playback, loop/onion state, and explicit disabled/error states from the supplied snapshot.
- AC-2: supports pointer/keyboard selection, seeking, viewport changes, and authoring intents; drag previews remain transient and release emits at most one edit intent.
- AC-3: layout and painting clip within narrow bounds and use existing design tokens, canvas, focus and input conventions.
- AC-4: does not merge native data with SMIL or perform I/O/session mutations; no dependency added; source files under 800 lines.
- AC-5: focused tests are authored if practical and not run; all verification is deferred until the full implementation wave is complete.

# Deferred verification
`cargo test -p vexel-app --locked` and final native interaction acceptance.
