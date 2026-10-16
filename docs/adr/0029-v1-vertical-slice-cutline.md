# ADR-0029: first V1 vertical-slice cutline

- Status: Accepted
- Date: 2026-10-15
- Decision owner: project owner

## Context

The complete Vexel goal is intentionally broad: a Rust vector editor and 2D
animation application, faithful Inkscape interchange, native projects, measured
large-document performance, live MCP, full release verification, and an
accessible Codex demonstration. Waiting for all of that before showing a
working product would delay useful feedback. The earlier V1 outline also spread
into a large feature list before the first complete user workflow existed.

## Decision

V1 is one small, complete, visible workflow: draw and revise a simple character,
animate a short loop, save and reopen the editable `.vexel` project, and export
an animated GIF. Codex controls that same open project over local stdio MCP;
Vexel visibly reports and displays each edit.

V1 includes one deliberately limited SVG/Inkscape subset: paths and basic shapes,
groups, transforms, solid fills and strokes, plus preservation of untouched
unknown source where possible and explicit disclosure of unsupported content.
It does not promise broad SVG fidelity or general Inkscape parity.

The interface keeps the accepted original macOS custom shell: a dominant canvas,
clear Draw/Animate workspaces, a small tool rail, contextual inspector,
attached timeline, and visible agent activity/results. Blender informs useful
workspace and timeline ideas without supplying Vexel's visual chrome. Core
actions use plain labels, keyboard access and visible focus. The design stays
specific to making vector characters and animation rather than expanding into
generic dashboard surfaces.

V1 uses focused correctness checks as each capability is added and one
integrated walkthrough when the workflow is assembled. The broad automated
UI/E2E, accessibility, SVG corpus, performance, security, recovery,
interoperability, packaged-install and release gates remain part of the full
product finish line. The authentic Codex character-and-animation video remains
required after that full software acceptance, for the README and website.

## V1 cutline

Included: one document, one text-free sample character, a few drawing/editing
actions, a short keyframed loop, editable native save/reopen, a basic SVG subset,
GIF export, local authorized Codex MCP, visible activity/results, and undo/redo
for the demonstrated edits.

Deferred to later full-product work: advanced node editing and precision tools,
rich layer/assets management, broad SVG/CSS/effects fidelity, large-file targets,
project recovery and compatibility matrices, onion-skin and advanced animation
controls, MCP remote transport and multi-client acceptance, comprehensive final
UI/E2E, release packaging, and public demo recording. These deferrals do not
remove or weaken the accepted full-product goal.

## Estimate

From the current repository state, remaining V1 work is estimated at 14–26
focused engineer-days (roughly 3–6 focused weeks). The custom app shell is
implemented and package-verified, but its live native appearance has not been
verified. Estimates include completing feature integration and one V1
walkthrough; they exclude the later full-product gates and public recording.
Unknown SVG and native-runtime defects may change the estimate. It is a planning
range, not a promised delivery date.
