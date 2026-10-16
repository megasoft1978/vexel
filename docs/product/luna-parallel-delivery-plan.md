> Owner direction2026-10-14: former C delivery stopped. The active replacement is [full Rust port and product delivery](rust-port-goal-and-plan.md). This document is retained as historical product/reference scope.

# Vexel: Luna parallel delivery plan

Owner direction updated 2026-10-15: the fixed worker ceiling in this historical
plan is removed. Use the existing subscription allowance, Luna at Standard speed,
and choose effort per task. The active policy is in
[OWNER-DIRECTIVES.md](../../OWNER-DIRECTIVES.md). No additional credit purchase,
API billing switch, account changes, or global model configuration changes.

## Finish line

Deliver every requirement and evidence gate in [ADR-0018](../adr/0018-product-goal-and-completion-gates.md):
a polished Apple Silicon macOS 13+ vector editor, faithful and source-preserving
SVG/Inkscape interchange, native artwork/animation/asset projects with recovery,
usable cartoon authoring, measured large-document responsiveness, and visible,
user-controlled live MCP for Codex, Claude Code and a test client. After feature
completion, pass the comprehensive software acceptance gates, then record and
review Codex creating and animating a character live in Vexel and publish the
example on the website and in README. The complete goal is unchanged.

Implementation locks remain C17, custom-drawn UI, owned SVG parser/rasterizer,
minimal pinned dependencies and MPL-2.0. Keep macOS implementations behind the
existing platform interfaces. Never read/fetch/copy GPL editor source or fetch
external SVG resources from the app.

## Execution rules

- **Coordinator:** owns contracts, work-order freezing, shared integration,
  STATE/ledgers, review decisions and release claims. Run as many useful
  disjoint implementation/review lanes as available environment capacity and
  existing allowance permit. Keep one writer per module; coordinator serializes
  shared manifests, interfaces and ledgers. No duplicate work or idle polling.
- **Context:** send workers a short task brief, named paths and exact exit checks;
  start with no full conversation history. Scope repository searches and return
  compact findings, changed paths and evidence. Do not repeat entire-repository
  audits. Reuse completed work and accepted tests.
- **Isolation:** each worker uses a repo-local worktree containing the current
  uncommitted source overlay, with a recorded SHA-256 baseline and independent
  build directory. Worktree HEAD alone does not represent the current app. Workers
  do not edit the main checkout, stage/commit, or modify other workers' files.
- **Integration:** compare the worker to its overlay baseline, check every changed
  path and verify the corresponding main file has not changed since that baseline.
  Merge shared registrations sequentially. Preserve all unrelated dirty changes
  and earlier evidence. Rebuild affected targets in the main tree after integration.
- **Correctness:** the implementation worker runs only mapped focused checks.
  A separate reviewer examines the critical code and test coverage, then the
  coordinator checks the integrated result. Passing tests are evidence for their
  covered cases, not product completion. Run additional tests only for changed
  code, failures or a concrete uncovered risk.
- **Effort:** Luna Low for scoped searches, docs and routine evidence; Medium for
  ordinary app/UI modules; High for memory ownership, concurrency, cancellation,
  renderer defects and critical reviews. XHigh or Max requires a named unresolved
  design/defect after a scoped attempt, with a short recorded reason. No automatic
  switch to Sol/Astra. Effort is selected per worker, not raised for the entire repo.
- **Usage:** use Standard speed and existing allowance. Record actual input/output
  counts from available worker/CLI usage summaries; use null where unavailable.
  Account usage is authoritative in the Codex usage dashboard. Token counts are
  not a dollar meter for included subscriptions, and no $100 completion guarantee
  is made without account billing evidence. Prefer completing one small unit over
  launching several duplicate audits. Do not buy credits or retry rate-limit errors
  in loops; preserve the checkpoint and resume after the allowance resets.
- **Testing sequence:** broad UI/E2E, full corpus, performance campaign, sanitizer
  matrix and release gates remain the final software stage after features. Focused
  regressions and small measured probes continue during implementation.

These are project execution choices. Official [Codex pricing and usage](https://learn.chatgpt.com/docs/pricing)
distinguishes included allowance from token credit rates and API billing, and says
context, reasoning and tools affect consumption. The [configuration reference](https://learn.chatgpt.com/docs/config-file/config-reference)
notes that available effort levels depend on model/client. No local subscription
balance or dollar spend is exposed to this coordinator; do not invent one.

## Work queue and dependencies

The first pair is frozen, implemented in isolation, reviewed and accepted:

- [WO-MCP-TRANSPORT-01 r4](../../work/WO-MCP-TRANSPORT-01.md), worker
  `transport_review_luna`, in `.worktrees/delivery-transport`: owns the relay
  source/helper, relay fixtures and transport README changes. CMake helper
  registration is integrated by the coordinator.
- [WO-MCP-SNAPSHOT-01 r1](../../work/WO-MCP-SNAPSHOT-01.md), worker
  `snapshot_build_luna`, in `.worktrees/delivery-snapshot`: owns
  `mcp_export_snapshot.c/.h` and `mcp_export_snapshot_test.c`. Snapshot
  CMake/test-map additions are integrated by the coordinator.

Their exact allowed/protected paths are in those immutable orders. Overlay
baseline manifests are in `artifacts/WO-DELIVERY-PARALLEL-01/`; worker code paths
are disjoint. Future table entries are queued planning scopes, not already
frozen or running work orders.

Estimates are focused engineering ranges from the existing product audit. They
exclude waiting for hardware/user review and discoveries; they are not calendar
promises or guaranteed model runtimes. SVG/performance totals remain unknown.

| Wave | Task and dependency | Owner / Luna effort | Exit evidence | Estimate |
| --- | --- | --- | --- | --- |
| Accepted A | WO-MCP-TRANSPORT-01 r4: repair broker-only inactivity renewal, receive-time response retirement and canceled-ID accounting | Relay worker / High; separate review / High | Bounded ID memory/capacity, unknown/duplicate/late response cases, idle and deadline regressions, focused stdio tests | one scoped implementation + one repair |
| Accepted B | WO-MCP-SNAPSHOT-01: copy current dirty scene/timeline/metadata and base+overlay assets into a bounded owned native container | Snapshot worker / High; review / Medium | Independence after original sources mutate/free; 16 MiB scene and 64 MiB bundle rejection; mapped focused test | one scoped implementation + one repair |
| Next C | Freeze async worker contract using B; native GIF prepare/render/finalize, frame-boundary cancel, owner-thread atomic publication | Worker / High | Worker never touches live app/session/edit state; deterministic export, cancellation cleanup, lifetime/expiry | part of MCP 3–6 days |
| Next D | Protocol task negotiation and polling/cancel, legacy progress/request cancellation; depends on A and agreed C API | Protocol worker / Medium; review / High | Real negotiated shapes, owner binding, busy/errors, canceled legacy no response | part of MCP 3–6 days |
| Next E | Nonblocking app socket output, task drain, visible progress/cancel and disconnect/doc-switch cleanup; depends on C/D | Coordinator + worker / High | Owner poll <=100 ms, bounded queues, stale result suppression, accessible cancel route | part of MCP 3–6 days |
| Alongside C | Human drawing asset capture/import into cel authoring; freeze transaction/history semantics first | App worker / Medium | Draw/capture/cel/play/undo/save-reopen using exact artwork; failed edits remain unchanged | part of animation 3–7 days |
| After APIs stabilize | Complete motion/color/visibility/drawing exposure authoring and preview/export agreement | App worker / Medium; render review / High | User and agent finish the same character workflow, deterministic frame/timing agreement | animation total 3–7 days |
| Independent bounded slices | Missing-source recovery, retention, interrupted-save behavior and version compatibility | Project worker / Medium; integrity review / High | Full project payload round trips and recoverable publication failures, bounded memory | 3–6 days |
| Early probe / separate lane | Refresh full T1/T2 outcomes, diagnose one supported-feature mismatch at a time; preserve unknown XML/Inkscape data | Renderer worker / High; inventory / Low | Per-file outcomes and tolerances, focused reference regression, exact preserved source | 0.5–2+ days per cause; unknown total |
| Early probe / separate hardware slot | App-visible large-file content and interaction baseline, then measured bottleneck fixes | Worker / High | Named hardware, raw repeated first-content/pan/zoom/edit/memory/cancel samples | 2–4 days baseline; 1–5+ per bottleneck |
| After authoring surfaces settle | Inspector/workspace cohesion, keyboard, semantic access, supported scales, representative-user review | UI worker / Medium | Reachable core tasks, coherent undo/history, keyboard/semantic/display evidence | 3–5 days |
| After feature waves | Live Codex/Claude/test-client interoperability, then complete deferred final software gates and resolve findings | Coordinator + review / High | Every ADR-0018 gate passes; no narrow test substitutes for broad evidence | final 3–7+ days |
| After acceptance | Record, caption and review live Codex character creation+animation; publish website/README links | Showcase worker / Low planning; human/artifact review | Accurate accessible footage of accepted live capabilities at both destinations | 1–2 days |

C and D may overlap once their API and message contracts are frozen. The
human-authoring/project/renderer lanes require separate orders with disjoint
paths. Add independent lanes as capacity permits, keeping one writer per module
and accounting for shared integration work. Shared app runtime/editor and
build/test-map changes are serialized by the coordinator. Hardware measurements
and final acceptance are exclusive machine tasks.

Reuse the existing `platform_animation_encode_gif` owned-byte encoder and
`platform_task_submit_owned`/poll/shutdown lifecycle for C. They already provide
the encoding and worker primitives; a new platform encoder or thread framework
is unnecessary. Keep GIF publication on the owner after a final cancel/grant/
generation check. `mcp_broker_hook.c` still has a two-second synchronous socket
send wait; E must replace that backpressure path before claiming a responsive
live async session.

Known closure work remains approximately 4–7 focused engineer-weeks, with no
reliable upper bound yet for corpus fidelity and large-file optimization. Parallel
agents can shorten independent work; they do not divide all estimates by the
worker count. Re-estimate after each accepted slice from actual evidence.

## Current checkpoint and review findings

- R3 relay's 12 fixtures and two focused CTests passed. Luna High review found
  untested deadline/accounting defects, so r3 was rejected/superseded rather than
  accepted. R4 closes the findings and is accepted:21 relay fixtures,2/2 integrated focused
  CTests, exact-delta audit and preserved original assertions.
- The r3 verifier accidentally overwrote the derived r2 diff. Its original digest
  is in the saved run-start manifest; source baselines, reviews and test/build logs
  remain. The discrepancy is recorded, and no preservation claim is made for the
  lost derivative. R4 begins from a fresh baseline.
- The existing task store already uses a pthread mutex. Reuse it; join all workers
  before destroying it. Do not implement a second synchronization layer.
- Snapshot work is accepted after independent review, one invalid-input repair,
  exact integration audit and the main focused test. It has no protocol/app
  behavior dependencies; async export remains unfinished.
- Existing cel controls only consume stored SVG assets. The next authoring seam is
  a human drawing-to-asset workflow. Capturing a whole scene must not duplicate
  unrelated artwork or silently change the target; define selection, coordinates
  and failure/history semantics before implementation.
- T1's latest full comparison in WO-SVG-PAINT-02 has 0.783% pixels above tolerance
  2, maximum delta 149. The later marker fixture does not prove a refreshed full T1
  result. Full corpus acceptance and actual 100 MiB first content remain open.
- Broad UI/E2E remains deferred. Website destination and publication workflow will
  be identified before the final showcase, rather than inferred from this repo.

## Acceptance recording

Freeze each task once, preflight every allowed/protected path and missing helper
before edits, and record the worker's commands and actual results. A rejected
slice receives a focused repair or a superseding contract if the scope must
change. Only the coordinator writes the append-only ledgers and marks acceptance
once independent review and integrated focused evidence justify it. The full
product goal remains active until every finish-line requirement is proven.

## Checkpoint after the first parallel pair

A and B are accepted in the main checkout. Worker builds and coordinator
integration used separate directories; all shared registrations and ledgers were
serialized. The snapshot focused test passes 1/1; transport has 21 fixtures and
passes 2/2 focused CTests. No broad UI/E2E ran and no credit purchase was made.
Next freeze the async worker/protocol contract for C/D, using the accepted
snapshot and existing GIF/task primitives; keep E’s socket/backpressure and
visible control gate explicit. The full product remains incomplete. The owner re-enabled the goal tracker on 2026-10-14; it now reports active.

## Async implementation checkpoint (2026-10-14)

[WO-MCP-GIF-WORKER-01](../../work/WO-MCP-GIF-WORKER-01.md) and
[WO-MCP-TASK-PROTOCOL-01](../../work/WO-MCP-TASK-PROTOCOL-01.md) are accepted as the next independent Luna High pair, in `.worktrees/async-gif-worker`
and `.worktrees/async-task-protocol`. The protocol worker preserves the original
backend layout and sync handler, exposing an optional async entry point. The
worker owns snapshot-only GIF bytes; owner integration remains a required gate.

Preflight for the subsequent owner bridge must include:

- `mcp_broker_hook.c` input and output budgets: nonblocking queued sends, bounded
  progress coalescing, terminal response retention/explicit failure under pressure,
  bounded messages per owner poll and unique per-connection task ownership. Its
  current send loop waits up to two seconds; multiplexed sidecar alone does not
  solve app backpressure.
- `viewer_editor_actions.c`, editor state/public progress/cancel APIs and hook
  routing: the status Cancel button currently has semantics and presentation but
  no `EDITOR_BUTTON_STATUS_CANCEL` action. Freeze these paths before edits.
- `mcp_broker_render.c` publication: current `write_new_granted_file` creates the
  final file with O_EXCL, writes bytes, then fsyncs. It avoids replacement but is
  not an atomic visibility commit. Use `platform_atomic_file_create_new` for GIF
  publication, preserve grants/no-replace behavior, and report durability errors
  truthfully when publication has occurred. Existing planning claims of atomic
  synchronous GIF publication are not evidence of that behavior.
- Request identity/progress-token ownership, current-task vs legacy cancellation,
  disconnect/document-switch stale-result handling, bounded task retention and
  monotonic-to-UTC task timestamps. Tool-domain errors are completed Tasks with
  isError; only JSON-RPC execution errors use Tasks failed status.
- Snapshot preparation cost: do not claim a 100 ms owner poll from background frame
  rendering alone. Measure/avoid owner-thread CRC/encoding and final file I/O
  stalls; all worker snapshots must already own their borrowed inputs.

Comprehensive UI/E2E remains final. No current production Tasks advertisement or
live async export is claimed from these isolated building blocks.

## Current delivery checkpoint

The native GIF worker and optional protocol adapter (revision2) are integrated and accepted after independent focused reviews, exact source/registration audits and main Debug checks: `mcp-export-job`1/1, protocol/async2/2. Protocol r1 was superseded before acceptance to fix schema gaps. No production Tasks are advertised until the owner bridge is installed.

`WO-MCP-GIF-PUBLISH-01` and `WO-MCP-SOCKET-IO-01` are accepted legacy C slices. The owner requested Rust for all first-party code; preserve these as migration reference and implement the corresponding production path in Rust. Owner capture, generation/cancellation/UI control, live clients and final product gates remain open. Existing allowance only; no purchased credits or API billing.

### Capture and socket follow-through

`WO-MCP-CAPTURE-01` produced a reviewed legacy C candidate with passing focused checks and owner-copy measurements. Its brief main-tree integration was reverted to honor the Rust-only first-party direction. Preserve the candidate and evidence for migration reference; do not continue this C order. Reimplement bounded owned capture and worker finalization in the Rust application. The measurements do not establish large-file app acceptance or live asynchronous export.

Socket review identified the combined read/write budget and missing close-by-owner seam; the consolidated repair also covers callback slot reuse, explicit terminal queue failure and lifetime documentation. The next active slice should port the owner task bridge to Rust, including grant/generation/cancel checks, UI Cancel and final file-I/O responsiveness. Do not execute the historical blocked monolithic C order unchanged.

Socket IO is integrated/accepted in the legacy C tree. Its shared64KiB total and8KiB/client transfer budgets, full allocated queue caps and safe connection ownership are covered by socketpair tests; the1000ns stalled-peer sample excludes protocol/tool callbacks. The C capture candidate's ownership, cancellation cleanup and size boundaries passed focused checks, but it is not the active Rust implementation. The live Rust app bridge and large-document application performance gate remain open.
