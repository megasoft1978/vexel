> Owner direction2026-10-14: former C delivery stopped. The active replacement is [full Rust port and product delivery](rust-port-goal-and-plan.md). This document is retained as historical product/reference scope.

# Vexel goal and delivery plan

## Product goal

Build a high-quality native macOS vector drawing/editor and 2D cartoon animation
application: a credible alternative to Inkscape for vector creation and editing,
with an animation workflow and a live, observable MCP surface for Claude Code,
Codex, and compatible agents. A final product deliverable is a recorded live
example of Codex creating a character and animating it through MCP while Vexel
updates visibly; the reviewed video will demonstrate these capabilities on the
product website and in this repository's README. The goal is not a feature
checklist detached from quality. Artwork, animation, imported metadata, assets,
and user intent must survive real editing workflows; large files must remain
responsive; people must be able to see and control agent work in the same
application.

The platform and engineering scope remains Apple Silicon macOS 13+, C17,
custom-drawn UI, owned SVG parsing/rasterization, minimal dependencies, and
MPL-2.0. The product decision is governed by
[ADR-0018](../adr/0018-product-goal-and-completion-gates.md).

## Current implementation status

This inventory is based on the current working tree, accepted work-order
verification, repository reports, and MCP documentation as of 2026-10-13. It
describes implemented capability separately from release evidence.

- **Editor and interface:** Vexel has drawing, selection/transforms, layers,
  path-node editing, style controls, undo/redo, task workspaces, rulers/guides,
  keyboard commands, a command palette, tabs, and a custom-drawn macOS interface.
  The Blender-inspired direction is a reference for workspaces and timeline
  organization, not a copied interface. Accessibility, inspector coherence,
  complete keyboard workflows, and representative-user review still need final
  validation.
- **Native project:** the bounded `.vexel` v1 container stores project metadata,
  exact SVG scene bytes, a timeline payload, and named assets. Open/save and
  atomic file publication are integrated. ADR-0020 defines a v1 animation JSON
  schema; supported timelines are validated and edited in a session with
  undo/redo. ADR-0021 defines crash-recovery capsules. Background capture and
  explicit restore/discard/cancel for a changed or matching existing source are
  implemented for native projects. Missing-source recovery, retention policy,
  large-file recovery memory evidence, migration, and compatibility UI remain
  open.
- **Animation:** native drawing tracks/cels and property tracks/keys, drawing
  assets, deterministic integer-tick frame composition, onion-skin preview,
  once/loop/ping-pong playback, and native GIF/PNG-sequence MCP export exist.
  MCP supports native track/cel/property authoring and history operations.
  Timeline UI supports a zero-track native project, selected-object opacity,
  transform, fill, stroke, and visibility keying, drawing-track/cel creation
  from a project SVG asset, and item retiming. MCP can add drawing assets to
  the project; direct user-facing capture/import of a drawing as an animation
  asset still needs review. Stable
  IDs are assigned to new graphics roots through shared document insertion;
  MCP returns the generated `elementId`. Standard Undo/Redo prefers native
  timeline history in Animate when available, then falls back to SVG history;
  other workspaces use SVG history. The Undo History popover lists SVG edits
  only. Broader direct property authoring and a complete user-authored character
  workflow remain unfinished. Stable IDs unlock targeting but do not prove an
  end-to-end animation workflow. SVG SMIL editing/export is a separate
  interchange path.
- **MCP:** the local stdio sidecar and private same-user socket let an agent
  inspect and edit the app-owned document, selection, SVG animation, native
  timeline, save, render, and export. Native mutations are undoable and update
  the live viewer. File grants and bounded input/output limits are enforced.
  Per-tool completion/failure entries appear in the operator log and status
  strip. Long-operation progress/cancellation, streamed or larger render
  results, and verified live sessions with Codex, Claude Code, and an
  independent client remain open.
  WO-MCP-ASYNC-03 r2 stopped before product-code edits at the recorded
  transport/scope audit. WO-MCP-TRANSPORT-01 r4 implements accepted bounded stdio
  multiplexing: ordered client forwarding, independent broker notifications and
  responses, retained partial/coalesced lines, half-close/EOF draining, and a
  30-second inactivity bound while requests await broker responses. A fresh
  integrated host-shell run passes all 21 public relay fixtures and both focused CTest
  registrations (2/2). Earlier nested executor runs skipped because socket
  binding was denied. This transport slice does not implement async tasks,
  export workers, app progress, or cooperative cancellation.
  The tested bounded task store is preserved and already synchronized with a
  pthread mutex. WO-MCP-SNAPSHOT-01 now provides an accepted owned native-project
  snapshot with current dirty scene/timeline and base+overlay assets, capped at
  16 MiB scene and 64 MiB encoded bytes. Its integrated focused ownership and
  invalid-input checks pass; owner-thread preparation cost remains an async
  responsiveness probe. This is groundwork for export workers, not integrated
  async export. Sol's focused review found an
  unbounded silent-broker wait and an overstated E2E assertion preservation
  claim; transport r2 was superseded and r3 corrects both review findings.
  R3 passed its fixtures but was rejected after accounting review; r4 is now
  accepted with 256 slots/2 MiB bounded ID accounting and 21 focused fixtures.
  Revise WO-MCP-ASYNC-03 r2 to make
  the verified transport its prerequisite before integration starts. That
  slice uses request-negotiated
  `2026-07-28` Tasks
  (`tasks/get`, `tasks/cancel`) or `2025-11-25` progress tokens/request cancellation
  with no response after cancellation; it is limited to two active/eight retained
  tasks, 15 minutes, 16 MiB scene and 64 MiB bundle snapshots. These behaviors
  remain unimplemented. See the
  [r2 scope evidence](../../artifacts/WO-MCP-ASYNC-03/scope-block-execution-r2.txt)
  and [transport verification](../../artifacts/WO-MCP-TRANSPORT-01/verification-r4.txt).

- **Animation workflow boundary:** the UI timeline selects a compatible SVG
  asset and adds a drawing track/cel; MCP can add new SVG assets. A user-facing
  way to create or revise the cel artwork from current canvas work remains the
  small missing authoring seam. Scene and timeline history are separate; review
  expected Undo/Redo behavior across mixed drawing/animation edits.

## Main gaps to close

- **SVG correctness and preservation:** the accepted report in
  `reports/M4/render-diagnostics.json` renders 3 of 39 probed files in a
  58-file manifest; 19 files above 32 MiB were skipped. The partial
  `WO-CORPUS-04` candidate renders 5 of 39, with no reported regressions, but is
  not accepted. WO-SVG-PAINT-02 reduced the manifested generated Inkscape T1
  comparison from 12.099% to 0.783% differing pixels above tolerance 2; the
  maximum channel delta remains 149. MARKER-01 later reduced the maximum delta
  on its isolated filtered-path fixture to 15, but explicitly did not rerender
  the full T1 source. These are single-file candidate results, not corpus
  acceptance. T1 renders at native 1600x1000; T2 still needs a fresh probe
  after the dash-cache and renderer changes. Preserve the accepted
  3/39 baseline and unaccepted 5/39 candidate separately. The current clip
  resolver honors inherited `clip-rule` on clipping geometry through supported
  CSS. Expand owned-parser support using full-document comparisons, preserve
  Inkscape namespaces and unknown constructs through edits, and diagnose
  unsupported or approximated content explicitly. Evidence:
  `artifacts/WO-SVG-PAINT-02/` and `artifacts/WO-CORPUS-05/` through `13/`.
- **Native project integrity:** background recovery capture and explicit
  restore/discard are implemented for an existing source, including the dirty
  scene, timeline, and assets. Missing-source recovery, bounded retention,
  recovery peak-memory evidence, interrupted-save fault injection, and
  compatibility/migration behavior remain open. Continue round-trip and
  failed-save checks with scene, animation, metadata, and assets together.
  Evidence: `artifacts/WO-PROJECT-05/` through `08/`.
- **Animation authoring:** finish user-facing property and drawing exposure
  workflows, timeline navigation and track/key operations, undoable editing,
  deterministic playback/export behavior, and accessibility. Validate both
  user-authored and MCP-authored projects.
- **Large-document interaction:** responsiveness gates remain unproven. The
  latest M4 head-to-head report has no first preview for its 100 MiB test band;
  its open/index measurements are not full-render evidence. The app load path
  reads and decodes the whole source and rasterizes before returning a loaded
  document, so background loading is not progressive first content. Rebaseline
  and measure app-visible content, main-thread stalls, pan/zoom, edit latency,
  memory, and cancellation independently from visual fidelity.
- **UI cohesion and access:** refine contextual inspectors, animation-specific
  workflows, Mac-native command placement, discoverability, keyboard coverage,
  semantic accessibility, and visual polish. UI-GAP-01/02 repair evidence
  exists, but their native smoke was blocked by the locked GUI session;
  UI-GAP-03 remains ready and identifies status-log accuracy plus opening and
  closing the log pane in the native smoke. Complete task-based review at
  supported display scales.
- **MCP observability/control:** add bounded in-progress reporting and
  cancellation for long operations, close error-recovery gaps, and verify live
  interoperability across Codex, Claude Code, and a test client. Per-tool
  completion/failure status already appears in the operator log and status
  strip. Keep local permissions; remote transport requires a separate security
  design.
- **Release and showcase:** complete software gates and owner review first. Then
  record an example (one video or coordinated short clips) of Codex creating a
  sample character and animating it through live MCP, with Vexel's visible
  updates captured as they happen. Edit/caption the footage, review it for
  accessibility and accuracy, and feature it on the product website and in the
  README so users can see the product's capabilities. Only demonstrate
  capabilities that have passed acceptance. No such recording is currently
  complete or published. This order is governed by
  [ADR-0018](../adr/0018-product-goal-and-completion-gates.md).

## Fastest dependable delivery sequence

The shortest route to a credible, demonstrable product is a vertical slice
supported by independent quality tracks. The repository is already broad; the
remaining schedule is driven by end-to-end workflow closure and evidence, not by
adding another large feature list.

1. **Unblock async MCP integration.** WO-MCP-TRANSPORT-01 r4 passes its 21
   relay cases, including the silent-broker bound, and both focused stdio CTests.
   R4 and the independent owned-project snapshot are accepted after isolated
   implementation, review and integrated focused checks. Revise blocked
   WO-MCP-ASYNC-03 r2 around their frozen worker/protocol API contracts and implement
   bounded native GIF tasks, progress, cancellation, and app status.
2. **Prove the animation vertical slice.** Use the existing fragment/element
   tools, stable IDs, project-asset and drawing-track/cel tools, timeline keys,
   undo, save/reopen, and export to create one small character. Fill only the
   gaps that prevent a user and agent from completing that same workflow; verify
   actual user-facing asset capture/import rather than assuming it exists.
3. **Close native project integrity.** Finish missing-source recovery,
   retention, migration/compatibility, interrupted-save behavior, and
   large-project recovery memory evidence. This is required before release even
   if the demo vertical slice works earlier.
4. **Run a cheap fidelity/scale probe, then optimize measured blockers.** The
   accepted full-render report is 3/39; the 5/39 WO-CORPUS-04 result remains an
   unaccepted candidate. WO-SVG-PAINT-02 leaves 0.783% of T1 pixels above
   tolerance 2 (maximum channel delta 149); refresh T2 and diagnose the next
   pinned common-feature failures. Reprobe the 100 MiB band before selecting
   performance changes: the current report has 0/12 first previews, while its
   other responsiveness metrics pass. Full SVG correctness and scale evidence
   can proceed beside the vertical slice only in isolated worktrees with
   disjoint allowlists and build outputs.
5. **Close UI cohesion and accessibility.** Resolve or explicitly supersede the
   blocked UI acceptance orders, then review the representative drawing and
   animation tasks, keyboard paths, semantic access, and supported display
   scales.
6. **Run final acceptance, then record and publish.** After feature work, run
   the comprehensive UI/E2E, corpus, performance, accessibility, sanitizer, and
   release gates. Resolve findings before recording Codex creating and animating
   the accepted character through live MCP; publish the reviewed example on the
   website and link it from README.

The main critical path is steps 2–6; SVG fidelity and performance must
advance alongside it. WO-ANIM-PROJECT-28 closed standard Animate Undo/Redo
routing, and WO-ANIM-PROJECT-29 closed stable-ID creation for new graphics.
The MCP guide reconciliation is complete in WO-PRODUCT-PLAN-03. The broad UI/E2E pass remains deferred until the feature tracks are ready.
Current evidence does not justify a completion date: corpus coverage remains
partial, the full T1 render has not been refreshed after MARKER-01, and T2 plus
the larger files need new probes. Use small work orders and revise the
estimates after each measured slice.

### Cheap probes before committing to larger implementation slices

Run these as bounded investigations and freeze any resulting code changes in
their own work orders:

1. **Real-client protocol check (30–60 minutes):** connect Codex, capture
   initialize negotiation, inspect the document, add one shape, key it, save,
   and undo. Verify whether the client uses the Tasks extension or legacy
   progress/cancellation before finalizing async protocol routing.
2. **Async ownership/backpressure probe:** use a tiny multi-frame native
   project, then export while the user edits or switches documents, disconnects
   the client, and withholds socket reads. Measure owner-loop responsiveness,
   stale result suppression, cancellation, and atomic publication. The current
   broker socket `send_all` can wait up to two seconds on backpressure; a
   nonblocking stdio relay alone does not prove the async order's 100 ms owner
   poll bound.
3. **Human animation/history walkthrough:** draw two shapes, key motion and
   color/visibility at tick zero and a later frame, play/scrub/onion-skin, then
   undo/redo scene and timeline edits, including deleting and restoring a target.
   This checks whether the accepted controls form a coherent workflow while
   scene and timeline history remain separate.
4. **Full project round trip:** save and reopen a `.vexel` containing scene
   edits, keys, two small cel assets, unknown SVG metadata and provenance; compare
   the payload and representative rendered frames. Include one failed publish
   case to expose app-boundary behavior.
5. **Preview/export agreement:** compare preview frames at integer ticks with
   decoded GIF frames and timing, then cancel a bounded heavier export and
   retry. This decides whether the demo is visually correct and cancellation
   behaves cleanly.

The first real-client check and bounded ownership probe are the highest value
early unknowns: the existing task-store test does not exercise worker rendering,
protocol negotiation, UI cancellation, or output cleanup.

## Design references and what to learn

Use Blender's [workspaces](https://docs.blender.org/manual/en/latest/interface/window_system/workspaces.html)
to organize Draw, Nodes, Animate, and Inspect tasks; Blender's [Dope Sheet](https://docs.blender.org/manual/en/4.4/editors/dope_sheet/introduction.html)
for channels and timing; Figma's [contextual properties](https://help.figma.com/hc/en-us/articles/360039832014-Design-prototype-and-explore-layer-properties-in-the-right-sidebar)
for selection-sensitive controls; and Apple's [toolbar HIG](https://developer.apple.com/design/human-interface-guidelines/toolbars)
for grouping and discoverability. Study Krita's [animation timeline](https://docs.krita.org/en/reference_manual/dockers/animation_timeline.html)
and [frame animation workflow](https://docs.krita.org/en/user_manual/animation.html),
plus Blender [Grease Pencil frame-by-frame animation and onion skins](https://docs.blender.org/manual/en/3.4/grease_pencil/animation/introduction.html).
Synthesize these into an original Vexel interface; preserve macOS keyboard and
accessibility behavior and do not copy product assets or complete layouts.

Inkscape's [FAQ](https://inkscape.org/es/learn/faq/) and [SVG overview](https://inkscape.org/en/develop/about-svg/)
describe SVG as its document basis and explain editor-specific namespace data.
SVG/Inkscape SVG is the import and interchange path; verify preservation of
namespaced and unknown content. The `.vexel` v1 container and native animation
schema are defined in [ADR-0019](../adr/0019-vexel-project-container-v1.md) and
[ADR-0020](../adr/0020-native-animation-schema-v1.md). [ADR-0021](../adr/0021-native-project-recovery-capsules.md)
defines recovery capsules; missing-source recovery, retention, and project
migration/compatibility behavior remain incomplete. Apple's [document packages](https://developer.apple.com/library/archive/documentation/CoreFoundation/Conceptual/CFBundles/DocumentPackages/DocumentPackages.html)
are a design reference, not an adopted format. The [W3C SVG specification](https://www.w3.org/TR/SVG/)
remains the interchange semantics reference. Adobe Animate is in maintenance
mode per its [timeline documentation](https://helpx.adobe.com/animate/desktop/workspace-and-workflow/timeline.html),
so it is not the strategic product foundation.

For MCP, preserve the protocol version and transports already documented in
[`docs/mcp.md`](../mcp.md). The [2026-07-28 MCP announcement](https://blog.modelcontextprotocol.io/posts/2026-07-28/)
is the current project reference. Keep stdio local-first and file grants
explicit; remote transport needs authentication and a threat model.

## Delivery sequence

Each implementation stage is a separately frozen work order with focused
correctness checks. Do not run the comprehensive UI/end-to-end suite until
feature work is complete; the final acceptance stage includes that suite.

| Stage | Work | Evidence to exit |
| --- | --- | --- |
| 0. Product and architecture | **Defined:** product goal, `.vexel` v1 container, animation schema, SVG preservation policy, MCP/local-security requirements | accepted ADRs and a current implemented/partial/planned inventory |
| 1. Fidelity and scale | Continue SVG support and source preservation; finish the partial corpus candidate at final acceptance; profile large-document load/render/edit paths and add bounded improvements where measurements support them | pinned full-document comparisons and per-file diagnostics; repeatable 10/100 MiB benchmarks with first content, stalls, pan/zoom, RSS, and cancellation |
| 2. Project integrity | **Partly implemented:** v1 codec, worker load, atomic create/replace, Save As, supported timeline validation/editing, scene/timeline/asset persistence, background recovery capture, and existing-source restore. **Outstanding:** missing-source recovery, retention, migration, compatibility workflow, and complete failure/memory evidence | round trips preserve artwork, animation and assets; recovery and migration are demonstrated, including interrupted-save cases |
| 3. Drawing and animation | **Partly implemented:** native cels/property keys, frame composition, onion preview, playback, export, and MCP authoring. Complete user-facing authoring, accessibility, workflow consistency, and export evidence | undoable representative cartoon workflows; deterministic frame and export checks; accessibility and keyboard evidence |
| 4. UI completion | Refine task workspaces, inspector, timeline, navigation, command palette, native dialogs, help and accessibility | representative task review, semantic/keyboard coverage, and visual review across supported display scales |
| 5. Live MCP operation | **Partly implemented:** local live document editing, selection, native/SVG animation, save, render, export, and per-tool completion/failure status. Add in-progress reporting/cancel, error recovery, and interoperability evidence | live Codex, Claude Code, and test-client sessions with visible updates, permissions, cancellation, and undo evidence |
| 6. Final acceptance and release | After feature work, run comprehensive automated UI/E2E and release checks; audit fidelity, projects/recovery, animation, performance, security/accessibility, packaging, and owner acceptance. Then record and review example footage of Codex creating a sample character and animation through live MCP while Vexel updates visibly | all ADR-0018 gates pass; then publish/feature the reviewed, accessible, accurate example video(s) on the website and link or embed them in README |

Dependencies are intentional: project semantics precede migration/recovery;
animation UI uses the native timeline model; MCP drives those same operations;
performance and SVG fidelity can proceed alongside the workflows. The
`WO-CORPUS-04` candidate remains partial and unaccepted until its frozen
acceptance and the final user-requested broad verification stage are complete.

## Quality bars and evidence discipline

- Full-document render coverage is the fidelity metric. Element previews do not
  count as whole-file success. Every claim includes manifest/version/hash,
  per-document status, and explicit tolerances.
- Proposed responsiveness targets to rebaseline on named Apple Silicon hardware:
  first visible content under 2 seconds on a representative 100 MiB file,
  main-thread pauses under 100 ms, and pan/zoom p95 under 33 ms. These targets
  are not achieved results.
- Record memory/RSS and file-size limits alongside performance. Keep resource
  limits finite, diagnosable, and cancelable; do not trade away correctness or
  project integrity for a benchmark.
- Test saving and reopening SVG and `.vexel` projects separately. Unknown SVG
  data and Inkscape namespaces must not disappear silently.
- Show the user what the MCP agent changed, where it is working, whether it is
  waiting/running/complete/failed, and how to cancel or undo it.
- Maintain a visible split between implemented, partial, planned, and verified
  functionality. Work-order evidence outranks roadmap prose.

## Execution plan for the shortest reliable delivery

The goal is the product described above: a polished native Mac vector editor
and cartoon-animation workflow, faithful and source-preserving Inkscape SVG
exchange, responsive large documents, and live, visible Codex/Claude control.
The final example must show Codex creating and animating a character through
MCP while Vexel updates on screen. Completion means evidence for these
workflows, final acceptance, and the reviewed showcase on the website and in
README; the showcase is not a substitute for acceptance.

Use one frozen work order per bounded change. Each estimate below is focused
engineering time for a small work order, excluding waiting for review, hardware
access, new discoveries, and the final broad acceptance run. They are planning
ranges, not promises. After the first two work orders, update the ranges from
observed throughput and newly surfaced parser or UI blockers.

| Order | Work and dependency | Focused estimate | Exit evidence |
| --- | --- | ---: | --- |
| A. SVG blocker slices | Reproduce one reported construct with a synthetic fixture; fix one owned-parser or renderer cause at a time. Run after each accepted slice only the mapped focused tests. Do not sweep the corpus yet. | 0.5–2 days per construct; unknown total | Focused regression, source-preservation assertion, specific unsupported/limit behavior, unchanged corpus candidate status |
| B. Native project completion | Finish missing-source recovery discovery and retention; define and implement migration/compatibility policy. Recovery capsule and startup restore already exist for an existing source. | 3–6 days | Interrupted-save/reopen recovery, version migration and older/newer project behavior demonstrated with scene, animation and assets |
| C. Animation workflow | Close the highest-priority user authoring gaps, then make one representative cartoon workflow usable end to end with undo, playback and export. Reuse the accepted native timeline/session model and existing cel/key controls. | 3–7 days | Focused timeline/render/export checks for user-authored edits and keyboard/accessibility behavior |
| D. MCP driveability | Add bounded long-operation progress/cancel, close error-recovery gaps, and verify actual Codex and Claude Code sessions against the running app. Depends on the matching animation workflow being ready for agent use. | 3–6 days | Cancellation and error recovery evidence; real-client session records showing visible updates, undo, and scoped file access |
| E. Large-document performance | Establish a reproducible baseline on named Apple Silicon hardware for first visible content, pan/zoom, edit stalls, memory and cancellation; optimize only measured bottlenecks. Can run alongside A–D if machine and source scopes are separate. | 2–4 days to baseline; 1–5+ days per measured bottleneck | Raw repeated benchmark records for representative 10/100 MiB files against the rebaselined targets |
| F. UI coherence and access | Review supported task flows, inspector and animation navigation, keyboard paths, accessibility and display scales. Existing blocked UI acceptance orders need an explicit gap-order or supersession decision before execution. | 3–5 days after the gap scope is frozen | Task-based evidence, semantic/keyboard coverage and visual review; no hidden attributes or unreachable core actions |
| G. Final verification and release | Finish the deferred corpus acceptance and full-file diagnostics, run the comprehensive automated UI/E2E and release gates once feature work is complete, and close all goal criteria. | 3–7 days plus fixes and reruns | All ADR-0018 gates pass, including per-file SVG results, recovery, animation, performance, security and accessibility |
| H. Product example | Record Codex creating a character and animation through live MCP with Vexel visibly updating. Edit/caption, review accessibility and accuracy, then publish/link it on the website and in README. Depends on G. | 1–2 days | Reviewed example footage demonstrates only accepted capabilities and is available from both destinations |

**Elapsed estimate:** 3–6 focused weeks is an optimistic planning envelope for
one engineer before major rework. A deep repository review puts known closure
work nearer **4–7 focused engineer-weeks**, while SVG coverage and large-file
optimization remain additional unknowns with no defensible upper bound. The
small technical character/MCP vertical slice is roughly **7–16 focused days**
before its final recording, but it does not clear ADR-0018's project, SVG,
performance, UI, or release gates. Record and publish the showcase only after
the broader final acceptance, as ADR-0018 requires.
The worktree is substantially modified and uncommitted. Preserve its current
changes; serialize implementation in this checkout. Independent work can reduce
calendar time only in isolated worktrees with disjoint allowlists and separate
build outputs. Serialize shared state/ledger edits, hardware measurements, and
acceptance runs; do not run competing processes against the same checkout or
build directories.

For a short demonstration slice, constrain the character to one small canvas,
simple shapes, explicit supported paints/transforms, and a short movement/blink
animation. Keep async scope to native GIF export, local stdio, and the current
task bounds. Defer general asset-library management, remote MCP, unified scene
and timeline history, and streamed large renders. These are schedule cuts for
the vertical slice only; they do not remove the SVG, performance, project,
accessibility, or release gates in ADR-0018.

### Critical path and next orders

The immediate next order freezes the async worker/protocol API contracts around
accepted transport r4 and WO-MCP-SNAPSHOT-01, then revises WO-MCP-ASYNC-03 into
bounded parallel worker/protocol slices. Prepare the small character example
script concurrently as scoped planning work. Next close any
asset-authoring or project-recovery gaps exposed by the complete workflow;
reprobe T2 and the 100 MiB files early so fidelity and performance blockers are
measured before selecting more features. UI/accessibility closure and full
corpus coverage remain release gates. Do not overlap implementation in the
current heavily modified checkout; parallel implementation needs isolated
worktrees, disjoint allowlists, and separate build outputs.

The broad UI/E2E pass remains explicitly deferred until feature work is
complete. Focused checks specified by each work order continue during
implementation.

## References

- Existing product review: [`reports/PROJECT-REVIEW-2026-10-12.md`](../../reports/PROJECT-REVIEW-2026-10-12.md)
- Existing interface decision: [`docs/adr/0017-ui-direction-blender-aligned.md`](../adr/0017-ui-direction-blender-aligned.md)
- Existing MCP contract and known limits: [`docs/mcp.md`](../mcp.md)
- Earlier architecture and engineering roadmap: [`docs/research-r4/05-revised-roadmap.md`](../research-r4/05-revised-roadmap.md)

## Owner-approved Luna execution policy (2026-10-13)

This 2026-10-13 worker ceiling is superseded by the owner-approved Rust
migration policy in [OWNER-DIRECTIVES.md](../../OWNER-DIRECTIVES.md) and the
active [Rust port and product plan](rust-port-goal-and-plan.md). Use as many
useful disjoint Luna workers as available capacity and subscription allowance
permit; keep one writer per module and serialize shared registrations and
ledgers. The full ADR-0018 scope and final UI/E2E staging remain required.
