# Project state

## Active Rust/product goal — 2026-10-14

The authoritative detailed goal is
[`docs/product/rust-port-goal-and-plan.md`](docs/product/rust-port-goal-and-plan.md).
Its consolidated execution section integrates the accepted foundation work,
all six completed isolated candidates, review/integration gate, downstream
parallel lanes and milestones through the tested ready-to-use release and demos.
Current process/goal checkpoint:
[`current-goal-checkpoint.json`](artifacts/WO-RUST-RENDER-CONSUMER-RESUME-01/current-goal-checkpoint.json).
Latest checkpoint — 2026-10-15: **910 recorded focused test entries across 35
accepted main-tree packages**. Since the earlier 872-entry checkpoint, accepted
the complete 73-symbol RASTER_INDEX Rust implementation (37 scalar-index tests)
after independent Sol review, exact candidate integration and fresh main checks.
Its production surface excludes legacy Vec-backed entry points while retaining
the allocation-free color lookup. This is an index prerequisite, not a renderer
or application-completion claim. The first Rust macOS app shell (9 app tests)
and Apple platform package (6 new test entries) remain accepted; native window
and font behavior remain unverified. The earlier accepted milestone included complete SVG definitions
(40 entries), scene decode initialization (177 scene entries), and transactional
cache publication (185 scene entries), and static pattern/scoped media/minmax
contracts (199 scene entries: 185 preserved + 9 runtime + 5 compile-fail). Root
owned scalar sampler (24 entries; original19 predicates preserved), and actual
shape admission/canonical geometry (221 Scene package entries, including 22 new
shape-publication proofs). Shape publication has independent Sol Medium review,
full preservation audit and fresh main check/test recorded in its work-order
evidence. These counts do not establish complete application functionality or
final QA.

2026-10-15 coordinator update: the path-number boundary correction is accepted
with 37 passing package tests; project recovery precedence/boundary evidence is
accepted with 9 project-I/O tests; exact C-derived icon hashes for both themes
and three scales are accepted with 12 UI tests. The MCP access evidence follow-up
adds 3 focused cases (15 access integration tests pass in a hash-matched minimal
workspace); the root workspace package commands remain blocked by offline
resolution of unrelated `objc2-image-io`. The latest ledgered decoder candidate
is R8 and remains incomplete: production composition and exact tile-cache
oracle gates are still open. The
independent MCP access review withdrew its suspected long-line quota mismatch
after matching the C `fgets(1024)` and Rust read-loop counting behavior. The
accepted follow-up now covers environment snapshot behavior and exact
allowed-directory/config path-length boundaries. The root workspace's offline
Cargo resolution limitation is documented; it is not a demonstrated production
mismatch.

Owner usage policy has **no fixed worker ceiling**. Run as many useful,
independent Luna agent/CLI lanes as available environment capacity and the
existing subscription allowance permit. Keep one writer per module, isolate
worktrees/builds where needed, and serialize coordinator-owned manifests,
interfaces, shared integration and ledgers. Use Luna for routine work, with
effort matched to the task; reserve Sol Medium for a named hard decision,
repeated failure or final review. Use Standard speed and no purchased credits or
API billing. Current active lanes are tracked by their work orders and candidate
artifacts. The scalar sampler, scalar index and shape publication scopes are
accepted in the main tree. Decoder continuation 9 is the current candidate and
remains incomplete. Engine adapter reseed R4 passes its isolated exact checks
against the current decoder source and 90-test suite (engine 6/6), using the
existing repository offline Cargo cache. Neither candidate is integrated:
decoder-assigned cache-oracle coverage and production contracts remain open.
Renderer output and whole-job gates also remain open.

2026-10-15 continued: WO-RUST-SVG-SCALAR-INDEX-01 revision 2 is accepted in
main after independent Sol Medium review. All 73 frozen symbols map to real
production implementations; the legacy Vec-backed compatibility APIs are
test-only, while `fill_color` remains public and allocation-free. Fresh main
check and 37 focused tests, scoped formatting, both validators and
`git diff --check` pass. Exact evidence is under
[`artifacts/WO-RUST-SVG-SCALAR-INDEX-01/`](artifacts/WO-RUST-SVG-SCALAR-INDEX-01/).

WO-RUST-UI-DRAW-01 R2 corrections are integrated in main and accepted. The full
renderer, editable V1 workflow, animation/GIF, live MCP, app/runtime, final
acceptance, release and recorded Codex demo remain open.

The shared contract is accepted after actual-diff review, full protected audit,
fresh main check/test (199/199), and preservation of the original603source/157
assertions/HEAD/index. It provides typed fixture contracts; production engine
adoption remains required.
Shape admission and canonical geometry publication are accepted at the Scene API
boundary. Lower renderer/media/index integration, native application, modern UI,
drawing/animation, performance, live MCP, Rust tooling, final QA,
packaging and authentic README/website video remain required. Scalar-index
fixture correction still awaits explicit approval; frozen originals remain intact.
Broad automated UI/E2E remains deferred until functionality is complete.
This checkpoint preserves the active goal and its accounting; it is not a pause
or completion. See the concise [handoff](artifacts/WO-RUST-RENDER-CONSUMER-RESUME-01/usage-checkpoint-handoff.md).
The initial consolidation snapshot remains at
[`delivery-state.json`](artifacts/WO-RUST-GOAL-04/delivery-state.json).
Owner-confirmed final finish line: **the full Rust application completed,
comprehensively tested, packaged, documented and ready for real use**. Required
release gates include full final UI/E2E and quality testing, fixes for all
release-blocking findings, fresh-install packaged usability and final evidence.
It requires all-first-party Rust migration and full drawing/animation delivery:
modern custom UI, source-preserving Inkscape/SVG interchange, native projects and
recovery, measured large-file performance, real live Codex/Claude MCP, final
acceptance and recorded character-animation examples for website/README.
The required video explicitly shows Codex prompts drawing a character, revising
that drawing, creating animation and revising it through successive actual MCP
calls, with the same Vexel project visibly updating live. Playback, save/reopen,
exports and reusable prompts/project accompany reviewed accessible footage.
[Live demo acceptance](artifacts/WO-RUST-GOAL-06/full-goal.json) is required and
pending; production MCP control also supports other compatible agents.
Use disjoint actual Luna exec workers within existing subscription allowance;
keep focused correctness checks and defer broad UI/E2E until feature completion.

## Accepted actual shape admission and canonical geometry — 2026-10-15

WO-RUST-SHAPE-PUBLICATION-01 r1 is accepted after independent Sol Medium review,
root preservation audit and fresh main verification. `vexel-scene` passes 221
focused package entries, including 22 shape publication lifecycle, capacity,
canonical source, multi-instance, frame, cancellation, panic-recovery and invalid
source proofs. The APIs reserve real shape/serial capacity before geometry, meter
registry scans, and preserve canonical referenced `<path d>` spans separately
from each use-host SourceMap. All 603 legacy sources, 157 frozen suite/fixture
entries, HEAD and index match their baselines. Evidence is in
[`artifacts/WO-RUST-SHAPE-PUBLICATION-01/`](artifacts/WO-RUST-SHAPE-PUBLICATION-01/).
This does not integrate the production decoder/provider or complete lower renderer,
application, UI, MCP runtime, performance, packaging, final QA or demo.

All foundation/second/third/fourth/fifth named port scopes and the wave6 CSS, owned
large-document, SVG references and source-save scopes are accepted, with
**852 recorded focused Rust test entries passing** across33 main-tree packages.
The owned model shares immutable source bytes and caches query tables; its
large-source API passed a 100MiB correctness case. This does not establish
100MB editing/save performance. Named checks, independent reviews and exact
preservation audits are recorded with each accepted work order.

Scientific literature research requested through science-paper-search is
accepted as research only: [rust-rendering-research.md](docs/product/rust-rendering-research.md).
Eight paced successful API envelopes, five primary PDFs and version/access
reconciliation support testable rendering/resource/latency directions. It makes
no AppleSilicon performance or full-product claim.

The complete source editor is now accepted after independent review and53
main focused tests, including both200000-case sequences, significant allocation
rollback, snapshots/concurrency, history and native save diagnostics. Its initial
incomplete candidate and all original assertions/evidence remain preserved.
SVG SMIL/motion parsing/evaluation is accepted; full rendering/authoring remains.

Native dependency audit and ADR0026 are accepted. Setup R2 compiles exact16
selected native packages; Cargo's18lock records include2 inactive optional
packages with checked cache provenance, not active functionality. R1's download
scope mismatch is retained. The native boundary is now accepted after independent
review and7focused main tests, including real Foundation exception handling,
main-thread denial and adversarial panic-payload cleanup termination. No native
window/UI startup follows from that boundary acceptance.
The complete69file renderer contract plan is accepted after explicit source and
temporary-context lifetime repairs/reviews; original603legacy and157suite/
fixture/golden/manifest hashes are preserved. Owned animation/view support and
safe shared path geometry are accepted, adding3unique cases each. Policy-aware
large-source values/cursor and shared base work-budget/cancellation are accepted,
adding7unique cases each after independent focused reviews and main checks.
The four controlled engines are now accepted after complete implementation,
independent reviews, targeted corrections and fresh main checks: reference18,
CSS63, path36 and animation21 package tests. CSS includes original cases through
two extra controlled routes; totals count named test entries, not distinct
original input fixtures. Reference/CSS/path have genuine resumable state;
animation is controlled one-shot with in-loop polling. Initial incomplete
CSS/path candidates remain preserved; Sol6.1Medium completed their full state
machines and Luna reviewed them. No fake progress or partial renderer accepted.
The complete owned scene-data model is now accepted in WO-RUST-SCENE-MODEL-01:
19 immutable data tables, source/animation/child context, fallible builders,
actual nested capacity/transient budgets, checked stale/cross-scope handles,
atomic path/color publication, scene-local rollback, sealed mask children and
ordered/disjoint nested opacity groups. Independent LunaHigh review/rechecks end
ACCEPT; fresh named main27tests (6unit+21integration) and exact603/index audit pass.
Initial findings and corrections remain preserved. No decoder/renderer/native
provider or desktop completion follows from this data-model gate.
The remaining shared decoder/scalar data/interface contracts are now accepted:
canonical lexical ownership/private element views, independent source/context
lifetimes, capacity leases, effective frame overlays, pattern tickets and full
scalar/traversal/delegate data. Independent LunaHigh review/recheck ACCEPT after
source-pairing and legacy tagged dash-range corrections;43main tests include the
unchanged27model cases plus16contract cases. No engine/backend is supplied here.
Next: mark-scoped draft-only builder finalization operations needed by decoder
postoperations, then full disjoint decoder/raster and native/app composition.
Use Luna for scoped implementation/reviews, escalating only specific architecture
or repeated-debugging issues. No native GUI or macOS13runtime acceptance yet.
The native desktop proposal remains planning evidence; selected bindings are
compiled registrations, and the native shell/text/presentation/menu/AX remain
complete implementation work orders.
Default editor/source-save caps remain16MiB; the complete100MB editable/save
workflow remains required. No Rust desktop application or full
editing/rendering/animation/live MCP acceptance is claimed. Broad UI/E2E remains
the final stage; packaging, full final testing and recorded demos are required.

The owner resumed the goal; the latest API read on2026-10-15 confirms **active**.
Earlier usageLimited tracker evidence is historical; no credit purchase or
billing change was made.
The full canonical goal was refreshed in WO-RUST-GOAL-05, retaining every Rust,
vector/animation/UI/interchange/performance/live-MCP/release/recorded-demo
requirement. The tracker keeps its language-neutral objective because the API
cannot edit an unfinished objective. The authoritative repository plan and
machine-readable full-goal.json supply the complete scope. No full-product
completion is claimed.

## Current product continuation — 2026-10-13

- `WO-MCP-TRANSPORT-01` revision 2 implementation is complete in the shared
  working tree, uncommitted; runtime acceptance is BLOCKED by sandbox Unix
  socket bind denial (`EPERM`). The bounded nonblocking `poll` relay preserves
  ordered client input, partial/coalesced broker lines, independent progress
  and response IDs, overflow handling, and half-close/EOF draining. Only
  `vexel-mcp` and `mcp_headless_broker` were built; both builds exit 0.
  The 11 new focused relay cases all skip before runtime assertions; the
  existing `mcp-stdio-e2e` also reports its documented bind skip. AC-2 is
  therefore unverified, with no runtime PASS or acceptance claim. Frozen order,
  path/mapping/line/ledger checks and exact run-start preservation audit are
  recorded in `artifacts/WO-MCP-TRANSPORT-01/verification.txt`.
  All shared dirty content outside the execution allowlist, task-store slice,
  prior evidence, HEAD, and index are preserved. No staging/commit or broader
  suite. Async tasks/export workers, cooperative cancellation, app progress,
  live-client interoperability, final product acceptance, and the recorded
  showcase remain pending. Run the two focused stdio checks on a host where
  private socket binding is permitted before accepting this transport slice.

- `WO-MCP-TRANSPORT-01` revision 1 execution is BLOCKED before product edits.
  Required item 6 and Output require the product inventory at
  `docs/product/vexel-goal-and-delivery-plan.md`, absent from the frozen allowlist.
  Freeze a scope correction adding that exact path before implementation;
  revision 1 and its scope freeze remain unchanged. Baseline, path audit, and
  preservation evidence: `artifacts/WO-MCP-TRANSPORT-01/verification.txt` and
  `scope-block-execution-r1.txt`. No relay implementation, focused build/test,
  async export, or live-client acceptance is claimed. The complete product goal
  remains active, including final acceptance and the recorded showcase.

- `WO-MCP-TRANSPORT-01` revision 2 is frozen READY at base
  `19234a036ed6a8d40c2ccdaec9b8186806d70eca`, spec SHA-256
  `9fc9af3dfad34463da247326bb93809f184acfe22a0cac6b4458efa2c16e295e`.
  It fixes bounded stdio multiplexing and adds focused public-relay tests for
  progress, cancellation, partial/coalesced lines, and response IDs. It adds
  only the required product-inventory path to revision 1 scope. This is the
  prerequisite for async native export; it does not implement MCP tasks or
  export workers. Scope: `artifacts/WO-MCP-TRANSPORT-01/scope-freeze-r2.txt`.
  Revision 1 is superseded before product edits; its freeze and scope evidence
  remain preserved under `artifacts/WO-MCP-TRANSPORT-01/`.

- `WO-MCP-ASYNC-03` revision 2 is frozen READY at base
  `19234a036ed6a8d40c2ccdaec9b8186806d70eca`, spec SHA-256
  `c964781819f37bf8d36af5486237706916ae8868145b92226b961be2ea653f0f`.
  It adds only `docs/product/vexel-goal-and-delivery-plan.md` to the allowlist
  so required item 6 can update the product inventory. Intent, limits,
  implementation, acceptance, and verification remain identical to revision 1.
  Frozen scope: `artifacts/WO-MCP-ASYNC-03/scope-freeze-r2.txt`.
- Revision 1 is superseded for this scope correction after its recorded block
  before product edits. Its original `scope-freeze.txt`, `scope-audit.txt`,
  `verification.txt`, and all prior evidence remain preserved under
  `artifacts/WO-MCP-ASYNC-03/`. The tested task store and shared product/source/
  test changes are preserved. Revision 2 execution stopped before product-code
  edits: the stdio sidecar waits for one socket line before reading more stdin,
  cannot reliably forward legacy in-flight cancellation/progress, and discards
  coalesced line tails. The required transport path `app/vexel_mcp_main.c` is
  outside the frozen r2 allowlist. Execution is BLOCKED; the frozen order remains
  unchanged. Scope evidence: `artifacts/WO-MCP-ASYNC-03/scope-block-execution-r2.txt`.
  Freeze a focused stdio multiplexing order before resuming async integration.
  No app builds/CTests or live-client acceptance are claimed. Execution checks:
  `artifacts/WO-MCP-ASYNC-03/execution-r2/verification.txt`. Revision checks:
  `artifacts/WO-MCP-ASYNC-03/verification-r2.txt`.

- `WO-MCP-ASYNC-01` was superseded before implementation after scope audit
  found that its UI-cancel acceptance criterion required
  `viewer_editor.c` and `viewer_editor_internal.h`, both outside its frozen
  allowlist. No product code changed under that order. Audit:
  `artifacts/WO-MCP-ASYNC-01/scope-audit.txt`. Corrected scope is frozen in
  `WO-MCP-ASYNC-02`.
- `WO-MCP-ASYNC-02` revision 1 is frozen READY at base
  `19234a036ed6a8d40c2ccdaec9b8186806d70eca`. It preserves the bounded
  asynchronous native GIF export behavior while adding the editor command
  dispatch and per-editor cancellation state needed to make the existing
  status-bar Cancel button functional. Frozen scope:
  `artifacts/WO-MCP-ASYNC-02/scope-freeze.txt`.
- `WO-MCP-ASYNC-02` produced a bounded thread-safe task store and focused
  lifecycle test, which build and pass. Integration was stopped at scope audit:
  `mcp_broker_render.c` is already 735 lines and the frozen allowlist lacks a
  dedicated async-export module needed to honor the 800-line cap. Preserve the
  tested task-store slice; follow-on scope is `WO-MCP-ASYNC-03`.
- `WO-MCP-ASYNC-03` historical revision 1 was frozen READY (now superseded) at base
  `19234a036ed6a8d40c2ccdaec9b8186806d70eca`. It carries forward the passing
  task-store implementation and assigns immutable native GIF export snapshots,
  worker rendering, and atomic publication to dedicated
  `mcp_broker_async_export.c/.h` modules. Frozen scope:
  `artifacts/WO-MCP-ASYNC-03/scope-freeze.txt`.

- `WO-SVG-MARKER-01` revision 2 corrects marker child style inheritance to
  use lexical definition ancestors, while retaining referencing path stroke
  width and placement. A focused regression failed before the fix and passes
  against the definition-root fill and librsvg. Existing marker checks pass
  4/4. Current 1600x1000 filtered-path comparison improves from 2,301 pixels
  and max delta 149 to 2,267 and max delta 15; endpoint ROI pixels above delta
  10 fall from 71 to 7. Full T1 must be rerendered when its protected external
  source is available inside the workspace. Evidence:
  `artifacts/WO-SVG-MARKER-01/verification.txt`.
- `WO-CORPUS-16` independently removed the endpoint marker, blur filter, or
  gradient fill from the local T1 path fixture and compared current Vexel and
  Inkscape renders. Removing the marker cut endpoint-ROI pixels above delta 10
  from 71 to 5 and full-page max delta from 149 to 13; removing blur or using a
  flat fill did not remove the hotspot. This strongly localizes the dominant
  T1 error to marker rendering or inherited marker paint, without separating
  geometry from paint coordinates. Next: focused marker fix and regression.
  No product or test code changed. Evidence: `artifacts/WO-CORPUS-16/`.
- `WO-CORPUS-15` rerendered the existing T1 ablations with the current
  Release renderer. Filtered-path-only retains 2,301 differences (0.144%) and
  all 69 full-T1 pixels above delta 10; gradient-stroke-group-only has 10,523
  differences (0.658%), all at delta 3, with no high-delta pixels. This
  associates the hotspot with the leading filtered path and the distributed
  low-amplitude residual with the gradient-stroke group. The filtered path
  still combines gradient fill, blur, and marker; isolate those separately.
  No source or test changed. Evidence: `artifacts/WO-CORPUS-15/`.
- `WO-CORPUS-14` localizes the residual T1 candidate mismatch from the
  existing 1600x1000 renders: 12,526 pixels differ at tolerance 2; 11,827
  differ by exactly 3 channels, while 69 exceed delta 10. The large errors
  cluster at x596..598/y100..102, and all differences occur in Vexel partial
  or opaque pixels. This is spatial evidence only, not a feature diagnosis.
  Next: rerender the existing filtered-path-only and gradient-stroke-group-only
  ablation fixtures with the current renderer to separate those components.
  No build or tests ran. Evidence: `artifacts/WO-CORPUS-14/`.
- `WO-PRODUCT-PLAN-02` revision 2 refreshes the product delivery plan against current
  accepted work. It removes completed Animate Undo/Redo from next actions,
  qualifies the T1 parity improvement as a candidate, records implemented
  existing-source recovery and its remaining gaps, and corrects MCP docs drift
  about supported text insertion. Broad UI/E2E and the recorded showcase stay
  at the end. Documentation checks only; no application tests or UI/E2E ran.
  Evidence: `artifacts/WO-PRODUCT-PLAN-02/verification.txt`.
- `WO-SVG-PAINT-02` revision 2 fixes per-shape antialias coverage compositing.
  Regression evidence confirms two coincident gradient strokes produce about
  75% alpha and ten saturate to opaque, matching Inkscape. The native-size T1
  comparison falls from 12.099% to 0.783% pixels above tolerance 2; max delta
  remains 149, so this is a fidelity candidate rather than an accepted corpus
  result. Focused `shape-render` and `svg-gradient-stroke` CTests pass 2/2 in
  Debug, ASan/UBSan, and UBSan. Broad UI/E2E remains deferred. Evidence:
  `artifacts/WO-SVG-PAINT-02/verification.txt`.
- `WO-ANIM-PROJECT-27` implements SVG-to-`.vexel` conversion with an editable
  zero-track timeline. Conversion validates and publishes one encoded bundle,
  retains its owner for session lifetimes, and installs the native session only
  after publication. The editor, selection, and SVG undo history remain live.
  Focused `project-save` and `viewer-timeline-bootstrap` tests pass in Debug,
  ASan/UBSan, and UBSan; no broad UI/E2E suite was run. Evidence:
  `artifacts/WO-ANIM-PROJECT-27/verification.txt`.
- Remaining product work includes stable IDs for new artwork, a complete
  user-authored cartoon workflow, MCP
  progress/cancellation and real-client evidence, broader SVG fidelity,
  measured large-document performance, project compatibility/recovery gates,
  and final UI/accessibility/release acceptance. Preserve the comprehensive
  UI/E2E pass for the end.
- `WO-ANIM-PROJECT-28` is implemented and focused-verified: in Animate
  workspace, standard Undo/Redo prefers the native timeline stack per direction
  and falls back to SVG edit history; other workspaces use SVG history. Debug,
  ASan/UBSan, and UBSan each build `svgworkbench` and `viewer_history_test`, and
  pass `viewer-history` (1/1). Evidence: `artifacts/WO-ANIM-PROJECT-28/`.
  The Undo History popover remains SVG-only. Broad UI/E2E remains deferred.
- `WO-CORPUS-05` rebuilt Release and probed only the two manifested generated
  Inkscape-style T1/T2 files after gradient-stroke support. Both now report
  raster `limit` rather than unsupported gradient stroke; neither is yet a
  successful render. Inputs, commands, timing, RSS and exact offsets are in
  `artifacts/WO-CORPUS-05/verification.txt`. No corpus sweep or UI tests ran.
- Fastest dependable product path: (1) diagnose the residual T1 mismatch after
  per-shape compositing (0.783% pixels above tolerance 2; max delta 149), then
  continue from pinned corpus failures. Preserve accepted 3/39 and the
  unaccepted 5/39 candidate separately (58 files; 19 skipped over 32 MiB); (2)
  reconcile MCP documentation with the current tool surface; (3) finish the
  representative character vertical slice (stable IDs, property/cel authoring,
  save/reopen, playback and export); (4) close MCP progress/cancellation and
  verify real Codex/Claude clients, continue measured large-file work, and close
  project recovery/compatibility; (5) run full UI/accessibility/release gates;
  (6) record the accepted MCP character and animation demo for README and
  website. SVG evidence remains inadequate for an Inkscape-quality claim, and
  no live Codex/Claude GUI session is verified. Do not record the final demo
  before its demonstrated workflow passes acceptance.

- `WO-PROJECT-01` implements the v1 native project container codec in
  `src/project/`: bounded, checksummed metadata, exact SVG source, timeline, and
  asset sections. Focused Debug and ASan/UBSan tests pass (1/1 each), as do the
  frozen-order, ledger, and diff checks. A fresh read-only Codex review passes
  after five findings were addressed. This is a codec foundation only; editor
  file open/save, project schema validation, migration, and recovery remain.
  Evidence: `artifacts/WO-PROJECT-01/verification.txt` and `review.txt`.
- Next project-format step: integrate the codec into atomic editor Save/Open,
  dirty-state and file-type handling, and add version migration/recovery. Keep
  the comprehensive UI/end-to-end acceptance pass for the final stage.

- Active product target: `docs/adr/0018-product-goal-and-completion-gates.md`
  and `docs/product/vexel-goal-and-delivery-plan.md` define the owner-approved
  macOS vector drawing, SVG fidelity/preservation, native animation project,
  large-document performance, live MCP, and final acceptance goal. The older
  research roadmap is historical input; its cross-platform stages are retired.
- Documentation order `WO-PRODUCT-GOAL-01` records the new decision and
  reconciles README/roadmap. Its scoped checks pass: frozen work-order validation,
  ledger validation (575 entries), and `git diff --check`. No product
  implementation or broad UI tests were part of this order. Evidence is in
  `artifacts/WO-PRODUCT-GOAL-01/verification.txt`.
- `WO-CORPUS-04` remains partial and unaccepted. Its candidate measurements and
  source changes are retained for the eventual full verification/review pass.

- Completed work order: `WO-M0-02` is merged to `main` at
  `0f24ab6f4a5a9ab6f7ff3846ae3d6339316287a9`. All nine frozen commands pass;
  the read-only review found no remaining code, safety, scope, or test defect.
  It flagged only the ephemeral AC-6 worktree path, normalized in the retained
  evidence and recorded in `artifacts/WO-M0-02/review-nits.md` per planner policy.
- Working branch: `main`; no active task worktree for M0-02.
- Platform: Apple Silicon macOS 13+ only, per `OWNER-DIRECTIVES.md`.
- Build: Debug, `mac-asan` (AddressSanitizer + UndefinedBehaviorSanitizer), and
  `mac-ubsan` configure/build/CTest pass with Apple Clang 21.0.0.
- Test inventory: latest M0-02 `./dev verify` reports 85 Python tests pass,
  zero failures, errors, or skips. M0-02 focused suites report 12 generator
  tests and 42 oracle/runner tests passing.
- M0-02 stress generation: exact 10,000,000 / 100,000,000 / 1,000,000,000
  byte documents. Mixed SHA-256 is
  `895780c50a7e3b6f05e1ad34b68ca6d22684699e6fa1b7425a7f1baaa6caa9e7`.
  Recorded generator times are in `bench/results/WO-M0-02.json`; they measure
  input generation, not Vexel rendering.
- M0-02 oracle: 24 pinned fixtures at 256x256; re-rendered to temporary files
  with librsvg 2.62.3 in 0.701 seconds, with committed goldens unchanged.
  Renderer evidence and environment details are in `artifacts/WO-M0-02/`.
- Completed work order: `WO-M0-03` is merged to `main` at
  `6bff91bec9f1358b9caa9780d5c47ad8139ac7c6`. Its harness validates pinned SVG
  fixtures, requires all four render-task placeholders, restricts adapter
  output and process access, and records recoverable JSON/CSV results.
- WO-M0-03 verification: 113 full tests and 28 focused tests pass with zero
  failures, errors, or skips; ASan/UBSan CTest passes 1/1. Smoke verified all
  24 corpus fixture hashes. Inkscape and Donner are unavailable, and Vexel has
  no compatible render command, so no renderer comparison is claimed. The
  fresh read-only review passed with no findings. Evidence is in
  `artifacts/WO-M0-03/`, `artifacts/runs/WO-M0-03/`, and
  `bench/results/WO-M0-03.*`.
- Completed work order: `WO-BOOT-06` revision 2 merged to `main` at `2d3a0c56ab913495391480e3b7674edd53766316`. The runner-format mismatch in revision 1 was corrected; its initial suite failure did not reproduce on the serial rerun.
- BOOT-06 acceptance: 10 focused golden tests, 2 tolerance checks, 5 negative-path checks, 1 determinism check, and `./dev verify` with 123 tests pass; zero failures, errors, or skips. The two fresh verifier rounds found genuine issues, all corrected. The two-round cap was reached, so no post-fix fresh verifier pass is claimed; review details are in `artifacts/WO-BOOT-06/review-rounds.md`.
- Golden baselines, manifest, and oracle paths were unchanged. No renderer or performance claim is made. Evidence is under `artifacts/WO-BOOT-06/`.
- Completed work order: `WO-BOOT-07` revision 3 merged to `main` at `5ec521fcee0f848cd77db86e365cec65011e2611`. The macOS-only placeholder driver replays bounded JSONL commands and writes deterministic RGBA PNG, semantic-tree JSON, saved state, and acknowledgments. It adds no native window or SVG renderer.
- BOOT-07 acceptance: 6 focused tests, 2 replay tests, 3 invalid-input/path/bounds tests, and the smoke-output contract test pass. The smoke script produced 2 valid PNGs, 1 accessibility tree, and 1 saved document. `./dev verify` reports 129 passed, zero failures/errors/skips.
- The first fresh review round found missing committed AC evidence; evidence was committed, then round 2 passed with no findings. Initial nested sandbox failures in the executor run did not reproduce in the serial host verification. No performance claim is made. Evidence: `artifacts/WO-BOOT-07/` and `artifacts/runs/WO-BOOT-07/verify-604a948f-0dee-4741-8f25-2d3ae1a2b97d/`.
- Completed work order: `WO-M0-04` revision 1 merged to `main` at `a678dd190f6b303af5948c7471020ac30fb437c5`. The M0 evidence packet is prepared: 24 synthetic corpus fixtures and 24 pinned goldens; 3 generated stress tiers; M0-03 has 0 trials, 0 RSS measurements, 0 available competitors, and no compatible Vexel render command. Owner-provided files/tasks, interviews, and outcome records are all 0; five representative examples are requested.
- M0-04 machine checks, all 12 source hashes, count reconciliation, and claim checks pass. A fresh read-only Codex review passed with no findings after the standard runner review diff included the already-frozen work order; the task-start diff was reviewed separately. See `reports/M0/runner-review-note.md` and `reports/M0/verifier-verdict.json`.
- The proceed/pause/pivot/stop owner decision remains OPEN; no owner/user outcomes are recorded. Continue authorized M1-01 through M1-04 engineering while awaiting the requested five representative files/tasks and owner decision.
- WO-M1-01 revision 2 candidate `c7b76fc9b0230876c5c2bfe6c758f4cb69ead1e4` passed the fresh read-only verifier with no findings and is merged to `main`. Debug/Release, 8 focused tests, 137 full tests, 100 ASan/UBSan lifecycle cycles, and five-run native smoke evidence pass. Release executable size is 74,048 bytes; native smoke median is 3.210 s and p95 is 3.400 s. Evidence is under `artifacts/WO-M1-01/` and `artifacts/runs/WO-M1-01/verify-e4cf82af-3c14-4d85-80b5-59daeba28474/`.
- Completed work order: `WO-M0-01` was implemented and accepted on candidate
  `f2a2db5035fda2afb0aa1413eb26c2d246003827`, independently verified, and
  merged to `main` at `6d5af96d73c946a1bd91ab7817a69e22eba79630`.
- BOOT-05 revision 3 is accepted and merged at `178f0e57554cf1eae8182a9cf89330297b647ffb` (merge includes the previously verified candidate and the equivalent root-scope path matcher cleanup).
- BOOT-05 evidence: `artifacts/WO-BOOT-05/`; final fresh verification reran all ten required commands. Fuzz seed `12648430` completed 144 invocations in 3.001 seconds; assertion and heap-overflow probes were detected.
- Strict executor schema generation enumerates all nine frozen acceptance IDs and preserves runtime exact-key and command-evidence validation.
- BOOT-04 revision 2 remains complete and merged. Its preflight transcripts remain preserved as compressed raw artifacts plus readable copies with trailing whitespace normalized for the required diff check.
- The new fuzz workflow passed local policy checks and actionlint; no hosted GitHub run was available in this checkout.
- The runner's recursive `/**` path matching is covered and fixed. The fuzzer accepts exact valid help output, uses a fixed 48-case-per-second budget, and stops at the requested deadline with a 0.1-second per-case grace. Old 51-test output is labeled historical under `artifacts/WO-BOOT-05/history/`.
- The three earlier review failures and the final passing review are retained in `artifacts/WO-BOOT-05/reviews/` and ignored per-run evidence under `artifacts/runs/WO-BOOT-05/`. The final review reported 52 tests passing, zero failures or skips; all ten commands passed.
- No hosted GitHub workflow run was available. Initial Release CLI baseline remains 33,568 bytes; median `--version` latency 1.910 ms over 31 runs; max RSS 1,884,160 bytes (single run).
- Hardening work order source `SECURITY-FINDINGS.md` is frozen by SHA-256 `a156b5187b6741385a1fcfe31a8330e0ac257120c8f963ddc342f3fb732dfc92`; its READY spec hash is recorded in `work/WO-BOOT-HARDEN-01.md`.
- `WO-BOOT-HARDEN-01` is complete, independently verified, and merged at `ee97effa31db1dd0863d847453a6eb9e71df57e0`.
- Hardening checks: 38 focused runner tests and 59 full fast tests pass, with zero failures/skips; workspace-write default, read-only verifier, frozen source snapshot, unchanged active root-copy acceptance, and candidate/root hard-deny path tests pass.
- First preflight attempt stopped because `exec resume --help` omits the inherited short sandbox alias; the runner now validates `-s` on `exec --help`. One substantive review round identified an untested active-order positive case and an executor prompt using the worktree copy; both were fixed before the passing final review.
- M0-01 corpus candidate: 24 project-authored MPL-2.0 fixtures cover shapes, paths, metadata/namespaces, style, text, embedded image, and small stress cases. The pinned WPT 3-Clause BSD notice is retained and hash checked; no WPT test file is included. Fetch/verify/query and 10 focused offline tests pass; full `./dev verify` reports 69 tests, zero failures/skips. Evidence is in `artifacts/WO-M0-01/`.
- M0-01 acceptance: all six frozen machine criteria passed, 10 focused corpus
  tests pass, and `./dev verify` reports 69 passed with zero failures/skips.
  Fresh read-only exact-commit verification passed with no findings; evidence
  is under `artifacts/WO-M0-01/` and ignored per-run records under
  `artifacts/runs/WO-M0-01/`.
- The initial executor report was rejected because two records cited commands
  outside the frozen list. The resumed run corrected the command citations and
  passed runner validation; the accepted report is retained in per-run evidence.
- No renderer throughput, memory, or latency result is claimed for M0-01; its
  synthetic corpus is an input baseline, not a representative user corpus.

- WO-M1-02 revision 2 is merged at `8cba068c0e4fcb5ee19521ff8e5887be8e390bec` after a fresh read-only review passed with no findings. Debug/Release and sanitizer CTest pass; 10 focused and 139 full tests pass; native smoke presents two frames, passes byte-exact Metal texture readback and resize/reset. Release executable is 94,288 bytes; recorded native smoke was 0.171 s, with no throughput or RSS claim. Evidence is in `artifacts/WO-M1-02/`, `bench/results/WO-M1-02.json`, and `artifacts/runs/WO-M1-02/verify-c61e25ca-b213-4393-986c-f69d172c9e09/`.

- WO-M1-03 revision 1 is superseded. Its placeholder replay matched all 20 outputs, but the runner's second execution collided with an existing output directory; native replay also encountered intermittent LaunchServices failures. Revision 2 adds unique replay output directories and reruns the same 20-frame and lifecycle checks. See `artifacts/WO-M1-03/r1-attempt-summary.md`.

- WO-M1-03 revision 2 was superseded before execution because a typo in its declared base SHA failed runner preflight. Revision 3 corrects the base to the verified clean main commit; the prior runner and output-path findings remain documented in `artifacts/WO-M1-03/`.
- WO-M1-03 revision 3 is complete and merged at `734c23ba42d405354d2408dcf8de69ef19a29b39`. The initial visible-window replay failed during AppKit teardown; LLDB and ASan localized the fault to `platform_window_destroy`'s autorelease-pool drain after all 20 frame submissions. The deterministic core now performs headless Metal blits and verifies GPU readback; the separate AppKit/Metal smoke passes. Placeholder replay is 20/20 identical (digest `c46e612bbfa618d18a27693c33a952b9fce16bb941e3d9b6f5048b7bfd280c59`); Metal replay is 20/20 identical (readback `3d636badd712ea35b36c861871517af1a2a0809d0a0a418efd5419b4f4c1196c`). Focused tests: 19 passed; full `./dev verify`: 142 passed, zero failures/errors/skips; ASan and UBSan CTest pass 1/1 each. Fresh read-only verification run `2c6ae867-72ae-4f61-9b04-580079e1d580` passed with no findings after building clean Debug/Release binaries. Release binary: 94,752 bytes; measured placeholder replay 100.976 ms and Metal replay 179.164 ms. Evidence and executor report: `artifacts/WO-M1-03/`, `bench/results/WO-M1-03.json`.
- Verifier runner now builds clean Debug and Release binaries before frozen checks; `python3 -m unittest tests.test_wo_runner` passes all 40 tests. WO-M1-04 revision 3 is merged; its resolved shutdown contract and deterministic native test injection are recorded in ADR-0006 and ADR-0007. Active work order: WO-M2-01 revision 2, which resumes the preserved checked-slice/arena candidate and adds the runner-required Verification commands block. Revision 1's runner parse failure and initial full-suite failures are preserved in its ledger and `artifacts/WO-M2-01/`.
- WO-M1-04 revision 3 is complete and merged at `91ff611782c843428fb5bdc4ba874e3ccdd37f94`. Its two fresh read-only reviews passed with no findings; the first review event needed a ledger status correction, so the second review records the passing transition. All nine frozen commands pass: 20 focused tests, 143 full tests, 1,000 queue cycles in 0.044 s with a 0.000073 s maximum join, ASan/UBSan CTest 1/1 each, and 1,000-cycle task smokes under both sanitizers. Release executable is 97,072 bytes; no SVG throughput or RSS claim. Evidence is under `artifacts/WO-M1-04/` and `artifacts/runs/WO-M1-04/verify-e71955ec-d0a3-4888-8fa2-4355f82ce469/`; benchmark data is in `bench/results/WO-M1-04.json`.
- WO-M2-01 revision 2 is accepted and merged at `1a80ef3bfbff0b6e593c2f435bda05c4f11a9fa1`. The fresh read-only verifier passed all 11 frozen commands with no findings. Debug/Release builds, 143 Python tests, and Debug/ASan/UBSan CTest 2/2 each pass. Release `svgworkbench` is 97,040 bytes; `libvexel_base.a` is 3,504 bytes. Arena boundary and allocator cleanup metrics are in `bench/results/WO-M2-01.json`; full evidence is under `artifacts/WO-M2-01/`. At that checkpoint, next work order was M2-02.

- WO-M2-02 revision 1 is accepted and merged at `a170928f4d116ac7db446a06b7b054ce1ae40da0`. The final fresh verifier passed all 12 frozen commands with no findings; 147 Python tests, Debug/ASan/UBSan CTest 3/3, Expat comparison 24/24, 100,000 malformed cases, and 60-second ASan fuzz passed. Release executable is 97,040 bytes and the XML library is 16,552 bytes. Evidence is in `artifacts/WO-M2-02/`, `artifacts/WO-M2-02/verifier-final/`, and `bench/results/WO-M2-02.json`.

- WO-M2-03 revision 1 is complete and merged to `main` at `f53cc4322d86c7228287578affa6825154f1f1e5`. All 13 frozen acceptance commands passed in the clean verifier worktree; the final read-only review reported no findings. Final-source 30-second and 600-second ASan/UBSan campaigns completed 2,158,912 and 42,108,425 cases cleanly. The 14 focused tests, 161-test full suite, and Debug/ASan/UBSan CTest 3/3 pass; `./dev verify` also passes native smoke and 20-frame Metal replay after the Release prerequisite build. No real tokenizer crash or SVG rendering performance claim is made. Evidence: `artifacts/WO-M2-03/`, `artifacts/runs/WO-M2-03/verify-eebaef9f-4245-4e61-8b87-d81b2f3acb73/`, and `bench/results/WO-M2-03.json`. Active next work order: M2-04.

- WO-M2-04 revision 1 is complete and merged to `main` at `44945019b06bb09e67881abe645c2e453b979427`. All ten frozen acceptance commands and the final fresh read-only review passed. Four focused tests, 165 full tests, and Debug/ASan/UBSan CTest 4/4 pass. The macOS atomic save preserves original bytes and mode, leaves the destination unchanged on parse/write/rename failure, and cleans temporary files. No save latency or renderer performance claim is made. Evidence: `artifacts/WO-M2-04/`, `artifacts/runs/WO-M2-04/verify-16e8fdc9-d718-4d8a-87fe-bc31a406d54b/`, and `bench/results/WO-M2-04.json`.
- WO-M2-05 revision 2 is accepted and merged to `main` at `4a105eb6fd8633a488ee61f8b15f24387b0e0463`; revision 1 was superseded after ADR-0008 moved the fuzz driver from the runner-protected `scripts/**` path. The final fresh read-only review passed all 14 frozen commands with no findings. Eight focused and 173 full tests pass; Debug/ASan/UBSan CTest 5/5, seven reference fixtures, and one million ASan+UBSan fuzz cases plus deterministic aggregate-command-cap exhaustion pass. The fuzz wrapper retains and rejects sanitizer stderr. Release sizes: executable 97,040 bytes, shape CLI 53,408 bytes, shape library 24,744 bytes. End-to-end benchmark: median 15.265 ms, p95 16.618 ms, 4.29 MPixel/s, max RSS 2,244,608 bytes. Evidence: `artifacts/WO-M2-05/`, `artifacts/runs/WO-M2-05/verify-449522aa-185a-41f2-9b91-abde398ee3c0/`, and `bench/results/WO-M2-05.json`.
- WO-M3-01 revision 1 is complete and merged to `main` at `159c7e761ce87862e85742cd25fb61a9aef01da4`. Fresh read-only review `verify-11f6305d-2611-4a4d-a0f4-24799f47b118` passed with no findings after two substantive byte-budget findings were fixed and rechecked. Debug/ASan/UBSan CTest each pass 8/8; full suite 175/175; corpus token equivalence, progressive replay, and all 13 frozen verifier commands pass. Release executable is 151,968 bytes. Latest 10/100 MiB first-preview medians: 27.81/257.05 ms; full indexing: 44.84/334.21 ms; max owner step: 0.44 ms; peak RSS: 33.9/127.0 MB. Evidence: `artifacts/WO-M3-01/`, `artifacts/runs/WO-M3-01/verify-11f6305d-2611-4a4d-a0f4-24799f47b118/`, and `bench/results/WO-M3-01.json`.
- WO-M3-02 revision 1 was superseded before implementation commit: the runner could not parse its verification heading and its focused Python module was outside the allowed paths. The candidate remains uncommitted in `.worktrees/WO-M3-02`; the first attempt's spatial/cache tests pass, but full CTest had an offscreen Metal queue failure and the full Python suite reported 8 failures/2 errors. Evidence: `artifacts/WO-M3-02/r1-attempt.md` and `artifacts/runs/WO-M3-02/8e569860-60d5-4bb3-9e34-88902179424d/`.
- WO-M3-02 revision 3 is complete and merged to `main` at `3a7f02997ad2db5649aa83062ad61d4eaf163e11`; candidate `75a77af5b896ec9f82e8f256c9ef16d6bba26a25` passed the final fresh read-only review with no findings. All 16 frozen verifier commands pass; Debug/ASan/UBSan CTest 9/9 each, `./dev verify` 176/176, focused 2,000-query naive differential and 10,000-insertion cache pressure pass. The LRU uses two-part timestamps with explicit rollover coverage. Release: 35,344-byte benchmark executable; 100k/1k query benchmark build 8.729 ms, p50/p95/max 0.000292/0.001833/0.022583 ms; cache pressure peak 6,000/8,192 bytes. Evidence: `artifacts/WO-M3-02/`, `artifacts/runs/WO-M3-02/verify-4f606ea7-ade1-4813-aac7-3b907ad3deaf/`, and `bench/results/WO-M3-02.json`. Next: prepare WO-M3-03 from the roadmap.

- WO-M3-03 revision 1 is complete and merged to `main` at `9651c616a957e3e6a1a3b38e741b9ed514f5bc98`. All 16 frozen commands and the fresh read-only review `verify-cceacba1-ac19-4ebb-99c8-ae5cfc61965b` passed with no findings. Debug/ASan/UBSan CTest 10/10 each, full suite 177/177, TSan 1/1. Deterministic scheduler-capacity rejection and randomized post-generation in-flight accounting are covered. Final Release 1,000-cycle benchmark: 0.012423 s, maximum join 0.000041 s, binary 70,368 bytes. Evidence: `artifacts/WO-M3-03/`, `artifacts/runs/WO-M3-03/verify-cceacba1-ac19-4ebb-99c8-ae5cfc61965b/`, `bench/results/WO-M3-03.json`.
- WO-M3-04 revision 1 is complete and merged to `main` at `331e27da9f1885bb27d53f7e7a4e3f1fe86264d1`. The final fresh read-only review `verify-aeb69b00-493d-4b43-a808-1569f40960a9` passed all 15 frozen commands with no findings. `./dev verify` reports Debug CTest 11/11 and 181 Python tests; ASan and UBSan CTest each pass 11/11. Native scroll/magnify smoke and lifecycle checks pass. The model test covers cursor-anchored wheel/pinch zoom, verified keyboard pan/zoom/fit, Retina 2x conversion, and 1,000,000 projections with checksum `804ede4d47af0f15`. Release executable size is 172,032 bytes. No RSS or wall-time claim is made for the projection workload. Evidence: `artifacts/WO-M3-04/`, `artifacts/runs/WO-M3-04/verify-aeb69b00-493d-4b43-a808-1569f40960a9/`. Next: complete BOOT-08, then prepare the M3-05 owner decision packet and stop at its human review gate.

- WO-BOOT-08 revision 5 is complete, independently verified, and merged to `main`. The clean read-only review `verify-db219b3f-a7e7-42af-8a67-fe06942ec5b1` passed all 14 frozen commands with no findings. Debug/Release builds pass; `./dev verify` reports CTest 11/11 and Python 328/328; ASan and UBSan CTest each pass 11/11; strict perf-gate tests pass 10/10.
- The local gate uses five cold-order plus ten warm-order trials on both 10 MiB and 100 MiB workloads. In the final comparable pair, 100 MiB first-preview p95 was 277.57/274.66 ms and passed the pinned relative threshold; peak RSS was 137,904,128/139,526,144 bytes and crossed the relative threshold twice, so run two reports `fail`. Absolute limits pass. The first max-step sample was noisy; the next was healthy. Release `svgworkbench` is 172,032 bytes with eight dynamic-library entries. App-bundle size and pan event-to-present p95 remain unavailable with explicit reasons. Evidence is under `artifacts/WO-BOOT-08/`, `artifacts/runs/WO-BOOT-08/verify-db219b3f-a7e7-42af-8a67-fe06942ec5b1/`, and `bench/results/WO-BOOT-08.json`.
- Revisions 1–4 were superseded only to resolve verifier scope-parser/base semantics; ADR-0009 through ADR-0011 document the amendments. No path guard was weakened. Next: owner review of the completed M3-05 packet; stop at its human gate. M4/M5 require the owner to choose proceed.

- WO-M3-05 revision 2 packet is complete and merged to `main` at `903f4eb5ac65e6e903df6f3409212b814135befe`; see `reports/M3/`; its offline validator checks 41 source hashes, derived claims, schema, and Markdown consistency. The final comparable BOOT-08 pair records 100 MiB first-preview p95 277,571,708/274,657,958 ns; peak RSS 137,904,128/139,526,144 bytes. The second RSS run fails the repeated 5% relative gate while both values remain below the 524,288,000-byte absolute limit. Pan event-to-present p95 and app-bundle size remain unavailable with reasons. All seven frozen machine commands passed in the clean verifier worktree; Debug and Release builds passed. Fresh read-only review `verify-a3acfe99-3db8-4bbb-b058-cbdc81188fd7` passed with no findings, validating all 41 hashes, derived values, allowed paths, and the open human choice. Evidence is under `artifacts/runs/WO-M3-05/verify-a3acfe99-3db8-4bbb-b058-cbdc81188fd7/`.
- M3-05 source preservation is 3/3 byte-equal round trips and failure atomicity is 4/4 cases. M2-03 fuzz completed 2,158,912 and 42,108,425 cases cleanly; M2-05 completed 1,000,000 cases with zero crashes or sanitizer reports. M3-04 review links two screenshots and three semantic snapshots; no live customer pilot is claimed.
- Owner files/tasks, interviews, user outcomes, and pilots remain 0; five representative real files/tasks are requested. The proceed/pause/pivot/stop decision remains OPEN. This packet does not authorize M4/M5; owner review is still required.

- WO-M3-06 revision 4 is independently verified and fast-forward merged to `main` at `c086461583b554f2d09a48570663381813280844` (candidate `1b28e8445aef01886d2419c912ea26d1b35b8bd1`). Fresh read-only review `verify-90542d7e-cd0f-4208-92f8-fe1969307c3c` passed with no findings; all seven frozen commands plus clean Debug/Release builds passed. Review validates all 58 corpus input hashes/sizes, 116 trials, the T1 derivations, and reported limitations.
- The 116 fresh Release trials found 2 supported previews and 56 unsupported outcomes, with no limits/crashes. Both previewable files are Natural Earth T1 maps. All 12 T3 files in the 80–120 MiB band have no supported preview, so the <2 s wedge fails. Maximum main-thread step is 279,583 ns and passes <100 ms; all 12 band files pass the M3-01 RSS allowance (3x source +128 MiB).
- Only T1 has successful comparable rows: Inkscape/Vexel geometric mean 84.447x for first-preview time and 33.711x for RSS. T2–T4 time/RSS ratios are unavailable because no Vexel rows succeeded. The task protocols differ; this is open/index evidence, not full-render parity. No Inkscape rows were rerun. Detailed per-file data and validation are in `reports/M3/head-to-head.md`, `.json`, and `head-to-head-validation.txt`.
- Build/tests: `./dev verify` passes CTest 11/11, Python 328/328, native smoke and 20-frame replay; ASan and UBSan CTest each pass 11/11; instrumented benchmark smoke passes under both sanitizers. WO-M3-06 closeout/merge is in progress. M3-05 owner proceed/pause/pivot/stop gate remains OPEN; stop after M3-06 merge and do not start M4/M5.

- Owner decision recorded in ADR-0012: proceed through M4/M5 using synthetic and public-corpus evidence; no customer pilot has occurred. Start with the M4 feature priority inventory at `reports/M4/corpus-feature-frequency.md` and `.json`; the inventory streamed all 58 external corpus files (4,429,240,515 declared bytes) with no parse failures. It ranks common shapes/group transforms, paths, fill/stroke and CSS/class handling, local references/gradients/clips, then text/images. Corpus inputs remain read-only.

- WO-M4-01 revision 1 is complete and fast-forward merged to `main` at `c7aa4c9fa2966b90deaa08e4cb848c78db29de1b`. Seven fresh read-only reviews were run; six rounds surfaced correctness/evidence findings, all corrected, and the final review passed with no findings. `./dev verify` passes CTest 11/11, Python 328/328, native smoke and 20-frame replay; ASan and UBSan CTest pass 11/11; seeded ASan+UBSan fuzz passed 1,000,000 cases in 24.382 s. Real-corpus report covers 58 files/116 trials: 2 previewable, 56 unsupported, no limits/crashes. The 100 MiB preview gate fails; max main-thread step and RSS gates pass. This is open/index evidence, not full-render parity. Evidence: `artifacts/WO-M4-01/`, `artifacts/runs/WO-M4-01/verify-0f4141d8-d85e-4b74-898c-e72cf7ec88d4/`, `reports/M4/`. Active next work order: WO-M4-02, path grammar and curve coverage.

- WO-M4-02 revision 1 was superseded before execution to correct the shape-fuzz artifact destination; revision 2 was frozen READY at `b6c496f3ca9e1d22946015ad74757d2fefea0097` (spec SHA-256 `2fd9975b8de99eb7e2149011839ccdc163f76341b9bb9b820435188ea462580d`).
- WO-M4-02 revision 2 is complete, independently verified, accepted, and fast-forward merged to `main` at `2d19d27960f524c9ff7fd888185ec7f7b3ca295f`. Fresh read-only review passed with no findings in 58.953 s. Direct checks pass: Debug/ASan/UBSan CTest 11/11, `./dev verify` Python 328/328 with native smoke and 20-frame replay, focused tests 9/9, 1,000,000 ASan+UBSan fuzz cases, ten librsvg fixtures, and corpus/inventory checks. Corpus remains 2 previewable, 56 unsupported, no limits/crashes; this is open/index evidence. The initial managed-runner attempt and its sandbox Metal/process-policy failures are retained separately; direct checks pass. Evidence: `artifacts/WO-M4-02/` and `artifacts/runs/WO-M4-02/verify-bb6f216a-27f7-40a3-8a61-45c5a90b2fce/`. Next: WO-M4-03 fill rules and deterministic antialiasing.

- WO-M4-03 revision 3 is complete, independently verified, and fast-forward merged to `main` at `bf006b4833e66533bcf8eaef45dad8119332eb8b`. The final fresh read-only review passed with no findings; all 18 frozen commands pass. Debug/ASan/UBSan CTest each pass 11/11, `./dev verify` reports Python 328/328 with native smoke and 20-frame replay, the 17 librsvg fixtures meet the raw jointly-covered RGB threshold, and ASan+UBSan fuzz completed 1,000,000 cases in 19.081 s. Release paired benchmarks retain all trials and report median changes of +14.34% simple, +63.16% curved, +77.50% multi-shape; candidate peak RSS is 2,031,616 B. Reruns show timing variance, recorded in `docs/PERF.md`. Corpus remains 2/58 previewable and 56 unsupported; the 100 MiB first-preview gate fails, max-step and RSS gates pass. This is open/index evidence, not full-render parity. Evidence: `artifacts/WO-M4-03/`, `artifacts/runs/WO-M4-03/verify-8cc0b01f-0f50-42bb-9623-41d21d18bc69/`, `reports/M4/`.
- WO-M4-04 revision 1 is complete and fast-forward merged to `main` at
  `dac63d6e8f8cd08787f3ee7e71099d0655ee258e`. The final candidate
  `f48c19f017ddf1cbc756e4ff79a9c26d674335fd` passed fresh read-only review
  `verify-3c3ca13c-0382-4d62-8823-d11408b97fa4` with no findings in 89.518 s;
  all 18 frozen commands passed. Debug/ASan/UBSan CTest each pass 11/11,
  `./dev verify` passes 192 Python tests and native smoke/replay, 9 focused
  tests and the native shape suite pass, 26 librsvg differential fixtures pass,
  and ASan+UBSan fuzz completes 1,000,000 cases in 21.767 s with zero reports,
  crashes, or invariants. Evidence: `artifacts/WO-M4-04/`.
- The final corpus run retains 58 source hashes and 116 fresh trials: 2 preview,
  56 unsupported, no limits or crashes. T1 first-preview speed and RSS ratios
  are 42.482x and 33.590x; higher-tier ratios are unavailable. The 100 MiB
  first-preview gate fails (0/12); maximum owner step is 199,375 ns and all
  12 RSS checks pass. This is open/index evidence, not full-render parity.
- Release raster medians are 2.138 ms simple fill, 4.301 ms curved fill,
  5.331 ms multi-shape fill, 2.501 ms line stroke, 10.735 ms curved stroke,
  and 78.994 ms dashed stroke. Existing-fill median changes versus the frozen
  base are +0.73%, +3.86%, and +2.24%; stroke workloads are unsupported by the
  base. Release `svgworkbench` is 189,328 bytes. Full trials and ratios are in
  `artifacts/WO-M4-04/raster-benchmark.json` and `reports/M4/`.
- Active next work order: WO-M4-05, the bounded CSS/style subset.

- WO-M4-05 revision 1 is implemented in candidate `9209ae3b670911470100e97b9538daf83dfa509d`; a fresh read-only review passes with no findings after fixes for CSS comment delimiters, inline-important coverage, and stale verification text. All 19 frozen acceptance commands pass in a fresh clean worktree after the three verifier build prerequisites. Debug/ASan/UBSan CTest 12/12 each; Python 328/328; native smoke and 20-frame Metal replay; 16 focused reference tests; 1,000,000 sanitized fuzz cases with zero reports; Release build; and 58-file corpus run/check (2 previews, 56 unsupported, 0 limit/crash). CSS benchmarks: 21 trials/workload, 27.890/29.662 ms stylesheet median/p95 and 27.483/29.013 ms inline; 2,113,536/2,146,304 B RSS; stable shared output SHA. Open/index evidence only; no comparable pre-WO CSS benchmark. 100 MiB first-preview gate still fails for 0/12 files; maximum main-thread step 0.199 ms and RSS gate pass. The standard verifier's fixed 250 KB diff cap required a focused manual read-only review after it completed all frozen commands. Evidence: `artifacts/WO-M4-05/`, including `verifier-round-02.json` and its clean-worktree command summary.
- Active work order: WO-M4-06 revision 1 is frozen READY against `cd0eb044213dff02f3cff3290d0f414250a9633d` (spec SHA-256 `e3a8b6dc33c470f4306abc227ba2af985b075c551503a79efbc83ac8a1434e56`). It targets bounded local `<use>`/`<symbol>` references, linear/radial gradients, and clipping, ordered by the 58-file feature inventory. Next: execute it on `wo/WO-M4-06`.


- Update 2026-10-09: WO-M4-06 revision 2 is active and blocked. ADR-0013
  narrowly permits synthetic-only scratch buffers up to 4 KiB, independent of
  document size; source intervals may not be copied into generated SVG. The
  rebased candidate `fb09369fa77475707d8a1e9cde0736f033bf7130` passes all 18
  frozen commands, 12/12 CTests, 199/199 Python tests, sanitizers, Release,
  and a fresh 1,000,000-case sanitized fuzz run (73.008 s, zero reports).
  A fresh read-only review still fails AC-9 and identifies SVG gradient
  default/fidelity evidence gaps; see `artifacts/WO-M4-06/manual-review-r2/`
  and `BLOCKERS.md`. M4-06b and later work remain pending until a source-token
  traversal replaces both generated-document paths and the spec/evidence gaps
  are closed.

- Update 2026-10-09: WO-M4-06 revision 4 addressed the five remaining P2 review findings: symbol clip-path now reports unsupported; both gradient href forms are rejected; clipPath child CSS is resolved before nested-clip validation; gradient stop opacity composites over earlier samples; gradient definitions and work visits are cached/counted. Regression tests and an independent librsvg fixture were added. The requested main raster-culling merge is being resolved in this worktree; the final merge/index status and exact verification results are in `HANDOFF.md` and `artifacts/WO-M4-06/`.

- Update 2026-10-09: WO-M4-06 review-repair brief 3 fixes all five P2 findings. The new symbol-clip diagnostic, plain/xlink gradient href rejects, clipPath child cascade check, source-over gradient compositing, and cached/countable gradient parse are covered by focused regressions; the gradient overpaint fixture passes rsvg-convert. Merged renderer content includes main's ae2b673 culling/scanline change through merge commit 10a9878; 200 seeded scalar culling comparisons and 47 librsvg fixtures pass with frozen thresholds. Debug/Release builds, focused 18-test Python suite, shape_test, 1M sanitized fuzz cases, and corpus run/check pass. Debug, ASan, UBSan, and ./dev verify CTest each pass 18/20; native-viewer-smoke is blocked by LaunchServices and index-replay by unavailable offscreen Metal. Merge resolutions are in the worktree, but Git cannot stage the conflicted paths because it cannot create index.lock; maintainer must finalize the merge. The post-merge five-workload paint benchmark also passes its 21-trial hash checks; the frozen-base solid workload is comparable and the other four remain unsupported at base. Evidence is under artifacts/WO-M4-06/.

- Update 2026-10-09: WO-M4-06 merge-integration defect repair brief 5 corrects culling versus paint-server/clip sampling and the zero-edge gradient pointer. Gradient/clip bounds now share the existing one-bit boundary map, keeping the center path for solid shapes; the unculled test renderer samples every pixel through `shape_render_sample.c`. Two named regressions fail against the pre-fix binary and match the new reference byte-for-byte; 120 seeded paint/clip documents also match. The zero-area gradient diagnostic passes `shape_test`, ASan+UBSan CTest, and UBSan CTest. Debug/Release builds, 1M seeded sanitizer fuzz, and corpus run/check (2/58 previews, 58/58 source hashes unchanged) pass. Full focused Python is 19/20 because three partial-gradient/clip fixtures exceed the unchanged raw-RGB threshold (44/47 fixture report); ASan/UBSan/Debug CTest each pass 18/20 with only sandbox LaunchServices and offscreen-Metal failures. Paired 21-trial solid workloads have stable pre/post hashes and median changes from -2.41% to +3.43%; the new 500-shape gradient workload is 288.010 ms median / 290.843 ms p95. Work remains uncommitted; current evidence is under `artifacts/WO-M4-06/`.
- Update 2026-10-09: WO-M4-06 merge-integration brief 6 removes the blanket `mark_varying_shape_bounds` sampling mask. The three previously failing librsvg fixtures are byte-identical across scratch builds at `0732b8e`, `9bae63e`, and the repaired tree; all 47 comparisons pass unchanged thresholds. The test reference shares the boundary-map sampling decision but still visits every shape and edge. Culled/reference identity passes for the three fixtures, alpha-gradient overpaint, and 120 seeded paint/clip documents. Debug/Release, `shape_test`, focused Python 328/328, and 1M ASan+UBSan fuzz (seed 20261014) pass. Debug/ASan/UBSan/verify CTest are 18/20 due LaunchServices and offscreen Metal; `./dev test-fast` is 202/212 due sandbox benchmark/platform tests. Corpus run/check remains 2 previews / 58 and all hashes unchanged. Four solid paired benchmarks are byte-identical and within noise (+0.72%, +1.42%, -0.46%, -0.13%). Git metadata remains unchanged; post-merge runner guard remains unverified. Evidence and current caveats are in `HANDOFF.md` and `artifacts/WO-M4-06/`.
- Update 2026-10-10: WO-M4-06 round 7 adds clip-child and parent-chain edges to the conservative device-space boundary map, restores the shortcut-free all-16 reference, and defines center-evaluated gradient paint with per-sample shape∧clip coverage. The half-pixel target pixel now matches librsvg `(124,0,131,128)`; 160 seeded paint/clip documents match the reference byte-for-byte. Of 48 librsvg fixtures, 47 pass unchanged thresholds; `clip-union` remains over the raw RGB limit (4 vs 1 at pixel (8,4)), blocking acceptance. Debug/Release, shape_test, million-case seed-20261015 ASan+UBSan fuzz, and corpus run/check pass; focused Python is 20/21. Debug CTest and `./dev asan`, `./dev ubsan`, `./dev verify` each remain 18/20 due the two sandbox Metal/LaunchServices checks. Details and evidence are in `HANDOFF.md`, `docs/PERF.md`, and `artifacts/WO-M4-06/`.

- Update 2026-10-11: WO-MOTION-03 and WO-MOTION-04 are implemented in the
  uncommitted `w14-keyframes` worktree. Source-preserving keyframe and track
  authoring uses document edit undo/redo; the timeline has key markers, a
  selected-interval easing graph, bounded numeric controls, presets, and
  accessibility values. `./dev build`, `animation_authoring_test`, and
  `viewer_timeline_test` pass; focused CTest is 3/3, and all three headless
  easing PNGs compare byte for byte with the new goldens. Ledger validation
  passes with 412 entries; `git diff --check` passes. `./dev t --all` is
  incomplete: 69/69 Python tests
  and 56/58 CTests pass; `index-replay` cannot create an offscreen Metal queue,
  and `native-viewer-smoke` failed without a diagnostic in the runner summary.
  The all-tests command invoked that registered native event-injection test;
  no separate manual window review or screenshot was done. See
  `docs/lanes/w14-keyframes.md` and `artifacts/WO-MOTION-03/` / `artifacts/WO-MOTION-04/`.
  Maintainer real-window review remains outstanding; no commit was made.

- Update 2026-10-10: uncommitted W12 polish2 candidate on `lane/w12-polish2`
  refreshes the native editor chrome, pixel grid, and status bar. Live light/dark
  window captures and interaction checks are recorded in
  `docs/lanes/polish2.md`. `./dev t --all` passes 57/57 CTests and 69/69 Python
  tests; `./dev editor-smoke` and `./dev viewer-smoke` pass; the ledger validates
  with 406 entries. This lane has no frozen W12 work order, so no lifecycle event
  was added to `ledger.jsonl`.

- Update 2026-10-11: uncommitted W14 candidate implements UX-09's first-run
  home/local SVG drop flow and UX-10's independent SVG sessions in native
  AppKit windows/tabs. Details and maintainer real-window checks are in
  `docs/lanes/w14-windows.md`. Debug build, focused home/session tests, 100,000
  deterministic layout cases, 5,000 bounded frame paints, and 20 dark/light
  1x/2x home goldens pass. `./dev t --all` reports 59/61 CTests and 69/69
  Python tests; `index-replay` cannot acquire an offscreen Metal queue and
  `native-viewer-smoke` fails in this environment. The ledger validates with
  406 entries and `git diff --check` passes. No frozen UX-09/10 work orders are
  present under `work/`, so no lifecycle entries were added to `ledger.jsonl`.

- Update 2026-10-11: uncommitted W15 visual work on `lane/w15-visual` adds
  system-accent and readable-scale preferences, resizable sidebars, simplified
  toolbar/Inspector hierarchy, scoped Layers focus, full-ID reveal/copy, the
  macOS 26 toolbar material boundary, and interruptible motion. Headless style
  goldens cover dark/light at 100%, 110%, and 120%, 1×/2×; editor chrome and
  selection goldens were reviewed and refreshed. `./dev build`, focused editor
  tests, 200,000-case layout fuzz, all relevant goldens, `./dev shot`/`pixdiff`,
  and ledger validation pass. `./dev t --all` reports 55/57 CTests and 69/69
  Python tests; only `index-replay` (offscreen Metal unavailable) and
  `native-viewer-smoke` fail in that lane's environment. Maintainer real-window
  checks are listed in `docs/lanes/w15-visual.md`. No frozen W15 order is present
  under `work/`, so the ledger has no W15 lifecycle event. No commit was made.

- Integration follow-up for W15 visual: Inspector group accessibility IDs now
  use a reserved range separate from the command palette. The editor document,
  legacy tree, canvas pointer, result-dispatch, and macOS menu wrapper code were
  split into focused files to stay below 800 lines. Combined headless goldens
  were regenerated. `./dev build` passes; `./dev t --all` passes all 73 CTests
  and 69 Python tests.

- Update 2026-10-11: uncommitted W14 rulers/zoom candidate from the supplied
  `r2-interaction.md` work-order text is recorded in `docs/lanes/w14-rulers.md`.
  Added editor-only bounded guides and rulers, zoom HUD presets, Zoom to
  Selection, Space overlay hiding, and light/dark 1×/2× headless goldens. Final
  `./dev t --all`: build passed, 57/59 CTests and 69/69 Python tests passed;
  index-replay and native-viewer-smoke require Metal unavailable on this host.
  Seeded ASan+UBSan shape fuzz completed 1,000,000 cases with zero crashes,
  invariant failures, or sanitizer reports. The task has no matching frozen
  local WO-UX files, so no ledger lifecycle event was added.

- Update 2026-10-11: uncommitted `lane/w14-palette` implements UX-05's Cmd+K
  command palette and shortcut discovery, and UX-08's named undo history
  popover. `./dev build` passes; `./dev t --all` passes 58/60 CTests and 69/69
  Python tests. The remaining CTest failures are `index-replay` (offscreen
  Metal unavailable) and `native-viewer-smoke`. Light/dark palette, history,
  editor-chrome, and UI-chrome goldens pass; `./dev shot`, exact `pixdiff`, and
  a 48,128-case ASan+UBSan shape fuzz pass. The ledger validates with 406
  entries. These UX work orders are from the research scratchpad and are not
  frozen under `work/`, so no lifecycle events were added. Maintainer real-window
  checks are listed in `docs/lanes/w14-palette.md`.
- Update 2026-10-11: uncommitted W14 select candidate implements WO-UX-01 and
  WO-UX-02 from the research interaction report. Selection feedback, cursor
  mapping, contextual actions, bounded snapping, Shift bypass, category controls,
  headless regressions, and dark/light goldens are recorded in
  `docs/lanes/w14-select.md`. Debug build and focused regression passed; all five
  light/dark headless golden verifiers passed. `./dev t --all` passed 58/60
  CTests and 69/69 Python tests; native viewer smoke and index replay are blocked
  by LaunchServices/Metal availability in this environment. The ledger validates
  with 406 entries. The referenced work orders are not frozen under `work/`, so
  no lifecycle event was added to `ledger.jsonl`.
- Update 2026-10-11: W14 implements WO-UX-04 self-revealing path node editing:
  supported-path entry, anchor state shapes, selected handles, path-order
  traversal, one/ten-unit nudge, line insertion hint, contextual Corner/Smooth,
  and protected initial-anchor deletion. Regression, editor/chrome, and node
  option tests pass; headless node-edit goldens cover light/dark at 1x/2x.
  `./dev t --all` reports 55/57 CTests and 69/69 Python tests; the two failures
  are the sandbox offscreen-Metal `index-replay` and `native-viewer-smoke` checks.
  A 1,000,000-case ASan+UBSan shape fuzz run passes with zero reports. Details
  are in `docs/lanes/w14-nodes.md`. No lifecycle event was added because W14
  report proposals are not frozen work orders under `work/`. WO-UX-03 remains
  gated on WO-M4-06 renderer acceptance: round 7 still has the `clip-union`
  reference mismatch (4 vs 1 raw RGB at pixel (8,4)).
- Update 2026-10-11: uncommitted W15 native macOS shell candidate on
  `lane/w15-native` implements r5 WO-1, WO-3, WO-4, WO-5, WO-7, and WO-10.
  `./dev bundle`, the Release CMake app-bundle target, 13 packaging tests, and
  69 Python tests pass. The 14 light/dark offscreen AppKit content and icon
  goldens pass, including Welcome empty and recent-list states. `./dev t --all`
  passes 59/61 CTests and 69/69 Python tests; the two failures are sandbox
  LaunchServices/hiservices and offscreen-Metal failures
  documented in `docs/lanes/w15-native.md`. Native titlebar/menu/full-screen
  review is left for the maintainer because the shared screen must not be
  automated or captured. The r5 orders are not frozen under `work/`, so the
  ledger remains unchanged.

- W15-native integration with W14 keeps `viewer_runtime.c` as the only document
  session manager and routes New SVG, save/edit commands, preferences, and
  Welcome file opens through its existing per-window event path. Recent files
  now use the shared W15 model for the W14 home view, menus, Welcome, and Dock
  menu. Welcome launch hides the host window; closing Welcome reveals the W14
  home state. Removed the duplicate CMake test-layout source and split native
  action/window-shell code to keep source files under 800 lines. The native UI
  golden fixture now uses a checkout-independent relative path. `./dev t --all`
  passes all 77 CTests and 69 Python tests; `python3 scripts/ledger.py validate ledger.jsonl`
  reports 412 entries. Real titlebar, tab, Welcome, and menu review
  remains for the maintainer.

- W15-a11y integration keeps W14's docked timeline and W15 visual/native
  chrome while adding focus-aware keyboard handling, localized editor/status
  text, VoiceOver layer actions, and localized native menu/dialog copy. The
  visual 100/110/120% scale and accessibility 100–200% sizes now share one
  `VexelUIScalePercent` setting, editor field, and View menu; 100, 110, 120,
  125, 150, 175, and 200% are supported. Text and layout scale together, while
  canvas zoom remains separate. The accessibility goldens were refreshed for
  the integrated docked chrome at 200%. `./dev t --all` passes 79/79 CTests
  and 69/69 Python tests; `python3 scripts/ledger.py validate ledger.jsonl`
  reports 412 entries. No lifecycle event was added because r5 WO-6/WO-9 are
  not frozen under `work/`. VoiceOver traversal, keyboard focus order, text
  sizes, and native menus still need maintainer review in a real window.

- W16 integration on `integrate/w16` merges `lane/w14-rulers`, `lane/w14-palette`,
  `lane/w14-select`, `lane/w14-nodes`, `lane/w14-timeline`,
  `lane/w14-keyframes`, `lane/w15-visual`, `lane/w15-native`, and
  `lane/w15-a11y` in the requested order. The merge log and semantic resolutions
  are in `docs/lanes/integrate.md`. Shared document sessions/recents and the
  Welcome flow remain owned by W14's viewer runtime; keyframe edit operations
  are wired into the docked timeline; the visual and accessibility scale
  preferences share one setting. Shortcut collisions were removed while
  retaining the frozen Align Center binding.
  `./dev t --all` passes 79/79 CTests and 69/69 Python tests;
  `./dev t --full` passes 79/79 CTests and 66/66 Python tests;
  `./dev asan` and `./dev ubsan` each pass 79/79 CTests with three host-gated
  skips. `./dev verify` builds Debug and Release and passes Debug CTest 79/79,
  but its Python phase reports 291/302: seven benchmark tests are blocked by the
  outer sandbox denying `sandbox-exec`, one native smoke test cannot use
  LaunchServices, and two Metal tests have no offscreen Metal device. The
  packaging helper, tooltip lifetime, and native shortcut bindings were fixed
  during final integration. `ledger.jsonl` remains at 412 valid entries; no W16
  lifecycle event was added because no W16 frozen work order exists. No live
  window, keyboard/mouse automation, or screenshot was used. Maintainer window
  checks remain listed in `docs/lanes/integrate.md`.

- Update 2026-10-12: WO-M4-06 closure attempt on `main` at `0f615c0`. All renderer gates pass and new exact boundary and gradient tests landed. The ledger records `blocked` because CTest is 78/79: `native-shell-goldens` renders at the host's 2x backing scale against 1x goldens on a Retina host. See `BLOCKERS.md`.

- Update 2026-10-12: WO-UI-GOLDEN-01 merged. The native shell golden check renders headless via `sandbox-exec`, and CTest passes 79/79 on Retina hosts that have WindowServer access.

- Update 2026-10-12: **WO-M4-06 merged** (revision 5, `b04981c`). Bounded references, gradients and clipping are accepted and every frozen gate passes. The corpus supported-file delta is 0: gradient, clip and use support alone makes no additional real-corpus file previewable. Next roadmap items are M4-07 (renderer bake-off) and the M4-08 fidelity gate, which needs an owner decision on scope.

- Update 2026-10-12: **WO-MOTION-03 merged** (revision 4). Spline-track key editing, edge-bound tests and byte-exact undo/redo evidence landed. `authoring.c` was split (`authoring_create.c`, `authoring_internal.h`). Real-window timeline checks remain with the maintainer.

- Update 2026-10-12: **WO-MOTION-04 merged** (revision 3). The easing editor's numeric field editing now works from the keyboard and rejected or uncommitted drafts no longer leak into other fields. Real-window timeline checks remain with the maintainer.

- Update 2026-10-12: **WO-ANIM-FIX-01 revision 1 and WO-M1-01 revision 3 (closure) merged.**
  The M1-01 closure verifier's `./dev asan` run had found an AddressSanitizer
  heap-use-after-free (`authoring.c:113`, reached through
  `animation_authoring_edge_test.c:468`) because the test freed the borrowed animation
  source before `svg_anim_authoring_add_key`. WO-ANIM-FIX-01 corrected the test
  lifetime (`c95ad83`) and both orders were then closed. All frozen commands pass:
  Debug/Release configure and build, Debug CTest 80/80, `./dev asan` 80/80,
  `./dev ubsan` 80/80, `./dev verify` (CTest 80/80, Python 328/328), `./dev t --all`
  (CTest 80/80, Python 328/328), 13 focused platform-driver tests, the 100-cycle native
  smoke (3.174 s), ledger validation (459 entries), `wo_validate`, and
  `git diff --check`. Fresh read-only verification ran in rounds; every finding was an
  evidence defect (a trailing blank line that failed `git diff --check`, a missing AC-7
  verdict, a benchmark record that contradicted the built binary, and a stale value in
  one summary row). All were corrected, the round-1 failures are retained rather than
  erased, and the final rounds passed. No product code changed. Evidence:
  `artifacts/WO-ANIM-FIX-01/r1-verify/` and `artifacts/WO-M1-01/r3-verify/`.
  No work order remains open in `ledger.jsonl`.

- Update 2026-10-12: **Owner-requested deep project review** recorded in
  `reports/PROJECT-REVIEW-2026-10-12.md`. Headline finding: the corpus benchmark's
  "first preview" renders a synthetic one-element document built by
  `svg_index_make_preview_source` (`src/svg/index/index.c:419-440`), so
  `reports/M4/head-to-head.md`'s 2/58 "previewable" result and its 41.6x/30.4x ratios
  do not measure rendering the files. A direct `svgworkbench --render-frame` sweep of
  the 39 pinned corpus files under 30 MB rendered 1. Dominant blockers are policy caps
  (`limit` 26/39) and fatal unknown non-namespaced attributes rather than missing
  renderer features. The review also records governance findings (34 lane merges
  touching 240/310 `src` files with no frozen work order; only 45/432 commits name a
  work order; WO-M6-01/02 have no front matter and cannot be ledgered), status-document
  contradictions, 21 GB of worktrees, a second-root branch lineage, and the fact that
  every owner gate (M0, M3, M4 scope, M6, M8, M10, M13) is still open while M13 shipped
  without its demand ADR.

- Update 2026-10-12: **WO-M4-09** records per-file corpus full-render diagnostics.
  `scripts/corpus_render_report.py` renders every pinned corpus file in full and records the
  error category and source offset for each failure; the report and its Markdown summary are
  committed under `reports/M4/render-diagnostics.*`. 26 focused tests pass; `./dev verify`
  reports CTest 80/80 and Python 328/328; ASan and UBSan each pass 80/80. Of the 58 manifest
  files, 39 were render-tested and 1 renders; 19 files above 32 MiB are recorded as skipped with
  a reason. The headline blocker: the full-render path is hard-capped at 16 MiB by
  `SHAPE_MAX_BYTES` (`src/svg/shape/traverse_internal.h:6`), and 11 files fail at exactly offset
  16777216. Other measured blockers: DOCTYPE declarations (2 files), `<clipPath>` outside
  `<defs>` (2), `stroke="url(#…)"` (2), unknown non-namespaced attributes (2), the glyph,
  path-command, `<use>`-expansion and XML-nesting caps (2 each), `writing-mode` (1), and 2
  unlocatable limits at offset 0. Two runs produced byte-identical JSON. No renderer, parser,
  raster, policy or platform code changed. Evidence: `artifacts/WO-M4-09/`.

- Update 2026-10-12: **ADR-0016 records the owner's product target.** Vexel is a macOS-first
  alternative to Inkscape with animation authoring and MCP integration, not a narrow viewer;
  fidelity is measured by full-document renders; the M0 and M3 gates proceed on synthetic and
  public-corpus evidence without user interviews or user files, knowingly carrying that risk;
  and the M13 animation demand gate is retired in favour of ordinary work orders.

- Update 2026-10-12: **WO-QA-01** records the first complete full-gate baseline and repairs
  `./dev editor-smoke`. Baseline: `verify`, `asan`, `ubsan`, `shape-fuzz`, `mcp-fuzz`, `e2e`,
  `soak`, `viewer-smoke`, `fuzz-xml` and `animation_fuzz` (1,000,000 cases) all passed;
  `editor-smoke` failed on a stale assertion. The W15 accessibility lane added a
  `Selected on canvas` layer-row value, while the scenario asserted an empty row value after
  re-showing and unlocking a row. Selection deliberately survives hiding and locking (the
  scenario's own locked-move step requires it), so the driver gained a `not-contains=`
  condition and the two assertions now check the state each step toggles. No product code
  changed. Sixteen frozen commands pass, including CTest 80/80 and Python 328/328. Evidence:
  `artifacts/WO-QA-01/`.
- Update 2026-10-12: two further work orders are frozen: **WO-M4-10** replaces the 16 MiB
  full-render source cap with a bounded token-storage budget (the largest measured corpus
  blocker: 11 files fail at offset 16777216), and **WO-HYGIENE-01** archives the disconnected
  second-root branch lineage as tags and prunes the 21 GB of stale worktrees.

- Update 2026-10-12: **WO-QA-02** makes every fuzzer part of the final gate and stops routine
  commands from rewriting pinned evidence. `./dev final` now runs shape, XML tokenizer, animation
  and MCP fuzzers plus e2e, soak and both native smokes; `./dev animation-fuzz [cases] [seed]`
  is new (200,000 cases / seed 20261012 default). `./dev fuzz-xml` gained `--output` and defaults
  to `.cache/qa/`, so the WO-M2-03 packet pinned by `reports/M3/validate_packet.py` is no longer
  rewritten by a routine campaign; `tests/fuzz/shape/shape_fuzz.py` defaults under `.cache/qa/`
  too. `.github/workflows/fuzz.yml` gained the `push` trigger that `scripts/check_ci.py`
  requires and writes the pinned packet only through an explicit `--output`. `./dev final` passes
  end to end on this host (CTest 80/80, Python 331/331, fuzzers clean). Evidence:
  `artifacts/WO-QA-02/`.
- Update 2026-10-12: a separate defect was found while running the repository's own validators:
  `python3 reports/M3/validate_packet.py --check` fails with `changed source hash: docs/PERF.md`,
  because the M4 performance work updated that living document after the M3 packet pinned its
  bytes. Exactly one pinned input is stale; every other frozen input still matches. Frozen as
  **WO-QA-03**, which reclassifies living narrative inputs from frozen to recorded-but-not-enforced
  and regenerates the packet under review.

- Update 2026-10-12: **WO-QA-02 revision 6** supersedes revision 5 after the first read-only
  verifier round found scope and record-integrity defects. Revision 6 also fixes
  `scripts/xml_fuzz_campaign.py`, whose failure inputs and tracked canary files were written to a
  fixed `artifacts/WO-M2-03/failures/**` path regardless of `--output`; that directory is now
  derived from the requested report, so a campaign without `--output` writes nothing under
  `artifacts/`. The final-gate assertions are no longer vacuous: each of the four fuzzer steps is
  asserted against its exact bound and seed. `./dev final` passes end to end in 457 s. One
  observation is recorded rather than hidden: the first gate attempt failed inside the
  real-window `editor-smoke` with `saved SVG changed bytes outside the selected transform`;
  the smoke passes standalone and the re-run passed, so it is a timing-sensitive smoke defect,
  not a product regression, and the smoke was not changed by this order.

- Update 2026-10-12: **WO-QA-03** restores a passing M3 owner-packet validator.
  `python3 reports/M3/validate_packet.py --check` had failed with `changed source hash:
  docs/PERF.md`, because the M4 performance work legitimately updated that living narrative after
  the M3 packet pinned its bytes. Exactly one of the packet's pinned inputs was stale; every other
  frozen measurement input still matched. The generator now separates **frozen** inputs (still
  enforced byte-for-byte) from **living** inputs, which are listed in the packet's source table as
  `{"path": ..., "living": true}` with no hash. The packet stays byte-reproducible across later
  edits to the narrative while every measurement hash is still enforced. The regenerated diff
  touched only the `docs/PERF.md` entry. NEW `tests/test_m3_packet.py` covers the frozen-check
  pass, a mutated frozen input failing, a mutated living input not failing, the packet recording
  the living input without a hash, and the committed packet matching the generator. Evidence:
  `artifacts/WO-QA-03/`.

- Update 2026-10-12: **WO-QA-03 merged.** The M3 packet validator passes again. Three non-blocking
  findings from its verifier are open follow-ups for the documentation pass: (a)
  `validate_schema` accepts a living path presented in frozen `{path, sha256}` shape, so the
  frozen/living invariant is not schema-enforced; (b) `tests/test_m3_packet.py`'s living-input
  case asserts the returned source list rather than the packet table directly (the table shape is
  asserted by a separate test); (c) `reports/M3/decision.md`'s boilerplate still reads "Every packet
  source below is repository-relative and verified by the offline validator", which is no longer true
  for the living row rendered beneath it. None affects a measurement or hash.

- Update 2026-10-12: **WO-UI-01** implements the first Blender-alignment increment: task workspaces and
  one consolidated status bar. A workspace strip (`Draw · Nodes · Animate · Inspect`) replaces the
  duplicated tool-hint row; each workspace declaratively selects pane visibility and an inspector group,
  `Control-Tab`/`Control-Shift-Tab` cycle with wrap, the choice persists through the settings store
  honouring `rememberWorkspace`, and an unknown remembered name falls back to `Draw`. The status bar now
  has Blender's three regions: hint, report, readout. Switching workspaces never touches the document,
  asserted by byte comparison in the new `viewer-workspaces` CTest. Ten golden fixtures were updated with
  a recorded baseline change (`artifacts/WO-UI-01/golden-changes.md`, pixdiff comparisons). 81/81 CTests
  pass. Evidence: `artifacts/WO-UI-01/`.

- Update 2026-10-12: **WO-UI-02** adds Blender-style properties tabs to the inspector
  (`Object`, `Fill`, `Stroke`, `Document`). `Object` keeps the name, geometry and every attribute
  no other tab owns, so nothing becomes unreachable; `Document` keeps the document's own
  `width`/`height`/`viewBox` editable alongside the size readout, Fit Canvas and Actual Size; a
  filtered tab is its own group selector. Mutation evidence: making the `Object` tab accept
  appearance attributes fails the structure test with code 46.
- Update 2026-10-12: **WO-UI-03** adds Blender's Info-editor pattern, a bounded
  operator/warning/error log pane above the status bar, toggled from the toolbar or with
  `Option-Command-L`. Commands record their own widget label; errors record their message; the ring
  holds 64 entries of 128 bytes and drops the oldest. The log is UI state and never changes document
  bytes. Mutation evidence: raising `EDITOR_LOG_CAP` fails the overflow assertion. Both orders:
  81/81 CTests pass, `./dev verify`, `./dev t --all` and `./dev editor-smoke` exit 0.

- Update 2026-10-12 (reconciliation): the three Blender-alignment updates above were recorded as passing by their
  implementer. A fresh read-only review at `bcdc3fa` (`artifacts/retro-verification-2026-10-12/`) failed
  WO-UI-01 r2, WO-UI-02, WO-UI-03, WO-BUG-01, WO-M4-10 and WO-HYGIENE-01 on specific acceptance
  criteria; all six are `blocked` in the ledger pending gap-closure orders. Their dates above were
  mislabelled 2026-10-13 and are corrected here. Commits `071e0ec`..`bcdc3fa` are described by
  WO-RETRO-01 (accepted). `./dev final` passed at `bcdc3fa` in 480 s (338 Python tests, 83/83 CTest
  in debug, asan and ubsan, all fuzzers, e2e, soak, both native smokes); the log is `.cache/final-gate-2026-10-12.log`.
  The earlier final-gate failures were a stale 16 MiB fuzz-probe bound, a missing UBSan link option on
  `viewer_workspace_test`, and three hand-counted label lengths (now linted by `tests/test_literal_lengths.py`).
  17 ledger entries (lines 502-518 at the time) carry mislabelled 2026-10-13 timestamps with `duration_s: 0`;
  the ledger is append-only, so they stand uncorrected pending an owner decision.
- Update 2026-10-12 (WO-GAP-M4-10): token-storage exhaustion and cancellation in `svg_shape_decode` now map
  to `SVG_SHAPE_ERROR_LIMIT` (grammar failures stay `xml`); the 256 MiB token budget is documented as 2,796,202
  tokens; `shape-decode-budget` CTest covers >16 MiB decode, tiny budget, 1 GiB + 1 ceiling and allocation
  balance. Corpus render report: rendered 1 -> 3 of 39 against the `5894cb4` baseline (limit 26 -> 13);
  the mapping change moved no file verdict. Evidence: `artifacts/WO-M4-10/` and `artifacts/WO-GAP-M4-10/`.
  The independent verifier verdict is pending.

- Update 2026-10-12 (WO-GAP-UI-01): closes the WO-UI-01 r2 gaps by fixing code to spec. Workspace table is now
  Draw=Object tab, Nodes=Object tab + Geometry open, Animate=keeps tab/section + timeline, Inspect=Document tab;
  `viewer_workspace_test` covers the table, a settings round-trip against a throwaway defaults domain
  (`platform_settings_use_test_domain`), and an unknown stored name loading as Draw with zero writes; the status
  readout shows document size and open/export/render progress goes through
  `viewer_editor_set_operation_progress` (`artifacts/WO-GAP-UI-01/progress-routing.md`); the 10 PNG rows of
  `artifacts/WO-UI-01/golden-changes.md` have side-by-side images; the readout change regenerated four golden
  suites (`artifacts/WO-GAP-UI-01/golden-changes.md`). Debug build, 84/84 CTest, `./dev verify`, `./dev t --all`
  pass. `./dev editor-smoke` was NOT verified: the macOS session was locked (`artifacts/WO-GAP-UI-01/smoke-blocked.md`).

- Update 2026-10-12 (WO-CORPUS-01): unknown non-namespaced attributes, `<meta>`, `<!DOCTYPE>` (internal subset
  skipped, entities never expanded; new `SVG_XML_TOKEN_DOCTYPE`) and `writing-mode` are ignored for rendering and
  reported in `svg_shape_document.ignored[]`; `render-frame` prints `note: ignored <kind> xN, first at byte M` on
  success. New CTests `shape-ignored-constructs` and `render-frame-ignored-note`; two old assertions that pinned
  DOCTYPE rejection were changed (`artifacts/WO-CORPUS-01/test-changes.md`). Corpus sweep: `ok` stayed 3 of 39
  (AC-3 NOT met; stop condition applies); formerly blocked files now fail later, see
  `artifacts/WO-CORPUS-01/residual-causes.md` and `delta.json`. Debug build, CTest, `./dev verify|asan|ubsan|t --all`,
  `./dev fuzz-xml --duration 300 --seed 1` and `./dev shape-fuzz --seed 1` pass (bare `./dev shape-fuzz` exits 2:
  the script requires `--seed`). `./dev editor-smoke` and `./dev viewer-smoke` were NOT run (macOS session locked).
  Independent verifier verdict pending.
- WO-GAP-UI-02: `inspector-tab-reachability` CTest proves each attribute of a probe element is on exactly one inspector tab (pointer-click driven); it found and fixed id shown on Object, Fill and Stroke. `./dev editor-smoke` not run (GUI session locked). Evidence: `artifacts/WO-GAP-UI-02/`.

- Update 2026-10-13 (WO-CORPUS-04 candidate): the raster preflight now uses a finite
  canvas-size-aware work budget, preserving the old 200M ceiling through 1024x1024
  and capping at 16B units. The pinned full-render sweep improves from 3/39 to 5/39:
  `generated-science_t1` and `generated-embedded_t2` now render, with no previous
  `ok` regressions. Release measurements are 13.69 s / 76,775,424 B and 5.77 s /
  381,911,040 B respectively. Debug CTest recorded 88/88 including the focused
  boundary test. The executor attempt is recorded as partial: the full frozen verification sequence
  and independent review are deferred until the planned final test pass; this candidate
  is not marked accepted. Evidence: `artifacts/WO-CORPUS-04/`.

- Update 2026-10-13 (WO-PROJECT-01/02/03): v1 `.vexel` now has a bounded codec,
  atomic create/replace, worker loading with retained container ownership, and
  editor Save As that materializes the edited SVG while preserving metadata,
  timeline JSON, and assets. SVG and `.vexel` are accepted in Save As, Open,
  startup/Finder and file-drop routing. The saved-state checkpoint prerequisite
  preserves undo boundaries. Focused Debug and ASan/UBSan checks pass (5/5 each),
  and independent read-only review passes. Evidence: `artifacts/WO-PROJECT-01/`,
  `artifacts/WO-PROJECT-02/`, and `artifacts/WO-PROJECT-03/`. Native timeline
  editing semantics, migrations, recovery, broader SVG fidelity/performance, and
  final UI/end-to-end acceptance remain incomplete; no comprehensive UI suite was run.

- Update 2026-10-13 (WO-ANIM-PROJECT-02 candidate): added a bounded v1 timeline
  validator, borrowed track/item views, escaped-string handling, strict property
  typing, locale-independent numeric parsing, and sequential item iteration over
  cached spans. Focused project Debug and ASan/UBSan suites each pass 5/5,
  including a 100,000-key traversal regression. This is not yet accepted because
  the required fresh final read-only review could not access local source files.
  Timeline editing/serialization, drawing-asset validation and creation,
  playback/export, onion-skin rendering, UI, and MCP control remain future work.
  Evidence: `artifacts/WO-ANIM-PROJECT-02/`.

- Roadmap alignment update 2026-10-13: the historical M13 animation demand-gate
  row is superseded by owner-approved ADR-0018/0020. Native project animation now
  proceeds through ordinary scoped work orders, with final deterministic frame,
  round-trip, live UI/MCP, and comprehensive UI/E2E acceptance reserved for the
  end. See `docs/research-r4/05-revised-roadmap.md`; timeline load integration
  follows the active XML tokenizer prerequisite.

- Update 2026-10-13: the first timeline/project integration design was stopped
  before implementation because existing XML APIs retain complete token vectors
  and XML attribute entity decoding is private to text layout. That cannot meet
  the 256 MiB project scene ceiling with bounded indexing. WO-ANIM-PROJECT-03
  revision 1 is recorded as superseded before implementation. New prerequisite
  WO-XML-STREAM-01 was superseded before implementation when its allowlist was
  found to omit the shared decoder call site and finalized-tag staging. The
  corrected WO-XML-STREAM-02 now implements those seams; project timeline load
  validation follows. Evidence: `artifacts/WO-ANIM-PROJECT-03/design-decision.txt`
  and `artifacts/WO-XML-STREAM-02/`.

- XML prerequisite scope correction (2026-10-13): superseded WO-XML-STREAM-01 before implementation after source review showed it omitted the existing text decoder call site and did not specify bounded finalized-tag staging. Frozen WO-XML-STREAM-02 includes those required seams; implementation is underway.

- Completed prerequisite 2026-10-13: WO-XML-STREAM-02 adds a bounded finalized-tag visitor and shared XML attribute decoder used by shape text. Focused Debug and ASan/UBSan tests each pass 4/4; final independent read-only review passes. Evidence: `artifacts/WO-XML-STREAM-02/`.

- Accepted in the working tree 2026-10-13: WO-ANIM-PROJECT-04 integrates known native timeline validation into `.vexel` load/save. It builds a bounded scene-ID inventory from exact SVG source, validates target and drawing-asset references, keeps unsupported valid timelines byte-exact, and polls cancellation during long validation. Focused Debug and ASan/UBSan suites pass 6/6 each; fresh independent review passes. Evidence: `artifacts/WO-ANIM-PROJECT-04/`. No commit or broad UI/E2E run.
- Next animation slice: validate and author native SVG drawing assets, then connect authored cels/property tracks to undoable timeline editing and playback. After those core semantics land, implement deterministic export, onion skins, and live observable MCP authoring; reserve broad UI/E2E acceptance for the end.


- Accepted in the working tree 2026-10-13: WO-ANIM-PROJECT-05 validates each referenced native drawing-cel SVG with the owned parser, checks local IDs/references, blocks active scripts and external resource references, preserves unused provenance and opaque future timelines, and validates during project load/save. The additive shape decoder supports cancellation through XML tokenization and traversal; focused Debug and ASan/UBSan project suites each pass 6/6. Final independent review found no blocking/high findings. Remaining limits: existing reference/CSS/SMIL parser phases are only polled between calls; CSS error offsets after entity decoding may not be exact; allocator-failure injection is unavailable. UI/E2E acceptance stays deferred. Evidence: `artifacts/WO-ANIM-PROJECT-05/`. Next: native timeline authoring transactions and undoable cel editing, then playback/render, onion skin/export, and live observable MCP authoring.
- Accepted in the working tree 2026-10-13: WO-ANIM-PROJECT-06 adds owned native-v1 timeline edits for drawing tracks and ordered cel exposures, source-span splicing, and bounded undo/redo with atomic failure behavior. Focused project Debug and ASan/UBSan suites pass 7/7 each; the edit suite exercises 100,000-cel and 1,000,000-total-cel caps; independent review has no findings. Broad UI/E2E remains deferred. Evidence: `artifacts/WO-ANIM-PROJECT-06/`. Next: connect timeline authoring to the UI and MCP, then implement playback/render, onion skin/export, and complete broad UI/E2E acceptance at the end.

- Accepted in the working tree 2026-10-13: WO-ANIM-PROJECT-07 attaches a native-v1 timeline editor and validation context to the loaded project, saves edited timeline bytes, and tracks timeline dirty state through close/reload and save-format routing. Focused Debug and ASan/UBSan project suites pass 8/8 each; independent read-only review found no blocking/high findings. Broad UI/E2E remains deferred. Evidence: `artifacts/WO-ANIM-PROJECT-07/`. Next: expose native timeline authoring through live UI and MCP commands, then playback/render, deterministic export, onion skin, and final broad UI/E2E acceptance.

- Accepted in the working tree 2026-10-13: WO-ANIM-PROJECT-08 exposes the active native-v1 project timeline through live MCP get/add-track/set-cel/remove-cel/undo/redo tools, reports successful changes through viewer dirty/status updates, and routes MCP Save/Save As through the active project writer. Focused Debug and ASan/UBSan MCP/project suites pass 5/5 each; final scoped read-only review found no blocking/high findings. No broad UI/E2E suite or commit. Evidence: `artifacts/WO-ANIM-PROJECT-08/`. Next: native timeline UI/playback/render, deterministic animation export and onion-skin rendering, then final broad UI/E2E acceptance.

- Accepted in the working tree 2026-10-13: WO-ANIM-PROJECT-09 adds bounded deterministic native frame composition at integer ticks. The renderer selects held cel exposures, renders drawing assets with the owned SVG pipeline, and composites visible tracks in order into premultiplied RGBA without modifying project bytes. Focused Debug and ASan/UBSan project suites pass 7/7; the expanded final renderer test passes 1/1 in both configurations. Review found no blocking/high findings; medium pixel/failure-injection coverage limits are recorded. No broad UI/E2E or commit. Evidence: `artifacts/WO-ANIM-PROJECT-09/`. Next: playback and timeline UI integration, target-relative cel placement, deterministic export and onion skin, then final broad UI/E2E acceptance.

- WO-ANIM-PROJECT-10 implementation (2026-10-13): `project_timeline_session_decode` returns current session metadata with explicit mutation invalidation, and `project_timeline_session_render_frame` revalidates current edited timeline bytes against the supplied scene/bundle before rendering without retaining a stale view. Focused session coverage proves decode metadata, current cel rendering across edits, undo/redo, invalid ticks, and source immutability. Debug and ASan/UBSan focused project suites pass 7/7. Independent read-only review and final acceptance remain pending. No UI/E2E suite or commit. Evidence: `artifacts/WO-ANIM-PROJECT-10/`. Next: independent review, then visible native timeline UI/playback integration.

- Accepted in the working tree 2026-10-13: WO-ANIM-PROJECT-11 connects native-v1 tracks and cel start markers to the existing timeline dock, reuses playback/scrubbing controls with bounded integer-tick conversion, exposes native timing/cels to accessibility, and renders current edited artwork through the native frame bridge. Native rows stay read-only. Focused Debug and ASan timeline/MCP/project tests pass 5/5 each; configured UBSan viewer timeline passes 1/1; scoped independent review found no blocking/high findings. No broad UI/E2E suite or commit. Evidence: `artifacts/WO-ANIM-PROJECT-11/`. Next: native row editing, target-relative placement, onion skin, export, then broad UI/E2E acceptance.
- Accepted in the working tree 2026-10-13: WO-ANIM-PROJECT-13 evaluates native property tracks at integer frame ticks with step, linear, and smoothstep-cubic interpolation, endpoint holding, scalar/RGBA/transform/visibility mappings, and temporary scene edits that preserve project bytes. Focused project renderer tests and the strict app target build pass in Debug, ASan, and UBSan. Broad UI/E2E remains deferred; no commit. Evidence: `artifacts/WO-ANIM-PROJECT-13/`. Remaining animation work includes UI property/cel authoring, export, and final broad UI/MCP acceptance. The product goal remains active and includes a requested Codex/Vexel recorded showcase for the website and README.

- Product goal clarification (WO-PRODUCT-GOAL-02, 2026-10-13): final completion now includes an accessible, reviewed live recording of Codex creating a character and animating it through MCP while Vexel updates onscreen. The recording is scheduled after final software acceptance and will be featured on the product website and linked in README. Current docs explicitly say the recording is not yet made; no website, app, or video was changed in this goal-definition order. Evidence: `artifacts/WO-PRODUCT-GOAL-02/`.
- Next frozen implementation order: WO-ANIM-PROJECT-14 adds undoable native property-track/key authoring over MCP so Codex can animate project nodes live. It follows ADR-0020 validation and preserves source bytes; focused project/MCP checks only, with broad UI/E2E deferred.
- WO-ANIM-PROJECT-14 was superseded before implementation because its frozen allowlist omitted a new focused source fragment needed to keep the existing 766-line edit module under the 800-line cap. WO-ANIM-PROJECT-15 includes that module and is the active ready order for native property-key MCP authoring.
- WO-ANIM-PROJECT-15 was superseded after its focused MCP check exposed a missing central dispatch path omitted from its frozen allowlist. WO-ANIM-PROJECT-16 includes that path and is the active ready order; partial W15 implementation remains in the working tree for completion and verification under W16.
- Accepted in the working tree 2026-10-13: WO-ANIM-PROJECT-16 adds undoable native property-track creation and ordered key upsert through MCP. The active project session validates candidates, preserves unrelated JSON bytes, and notifies the live viewer; MCP accepts numeric, boolean, and bounded flat numeric-array values. Focused project edit/session and MCP protocol/native timeline suites pass 4/4 in Debug, ASan, and UBSan; follow-up scalar-key and negative validation cases pass in each configuration. Broad UI/E2E tests remain deferred; no commit. Evidence: `artifacts/WO-ANIM-PROJECT-16/`. Remaining: native UI track/key authoring, animation export, recorded Codex/Vexel showcase, and final broad acceptance.
- WO-ANIM-PROJECT-19 was superseded before implementation after source review found the frozen allowlist omitted a focused module needed to keep the 763-line timeline layout file below the 800-line cap. WO-ANIM-PROJECT-20 corrected the allowlist and module split; evidence: `artifacts/WO-ANIM-PROJECT-19/`.
- WO-ANIM-PROJECT-21 was superseded after its added assertions exceeded the 800-line cap in the frozen general timeline test source. WO-ANIM-PROJECT-22 carries its Add Track/Add Cel implementation forward and extracts native-project coverage into a focused test executable. Debug, ASan/UBSan, and UBSan builds of both focused timeline binaries and `svgworkbench` pass; each changed timeline test source is below 800 lines. Evidence: `artifacts/WO-ANIM-PROJECT-22/verification.txt`.
- Accepted in the working tree 2026-10-13: WO-ANIM-PROJECT-23 connects `export_animation` to the current native `.vexel` session. MCP GIF and PNG-sequence output now samples deterministic integer timeline ticks, including unsaved property keys, drawing cels, and current assets; SVG documents keep the SMIL export route. Focused native and legacy export checks plus strict app builds pass in Debug, ASan/UBSan, and UBSan. Broad UI/E2E remains deferred. Evidence: `artifacts/WO-ANIM-PROJECT-23/verification.txt`.
- Accepted in the working tree 2026-10-13: WO-ANIM-PROJECT-24 makes native `once`/`loop`/`ping_pong` mode persistent and undoable. The viewer loads and edits that project timing setting; `native_timeline_set_loop` lets MCP set it and native timeline reads expose it. Focused Debug, ASan/UBSan, and UBSan checks cover decode, byte-preserving undo/redo, dirty state, UI interaction, MCP schema/dispatch, and undo/redo. Broad UI/E2E remains deferred. Evidence: `artifacts/WO-ANIM-PROJECT-24/verification.txt`.
- Remaining product work includes export quality/limits and format review, SVG/Inkscape fidelity and large-file evidence, UI/accessibility polish, project recovery/migration, live MCP observability/error recovery, and final broad acceptance. After software acceptance, record and review a live Codex/MCP character animation for the website and README. The showcase remains planned, not complete.
- WO-PROJECT-04 implementation (2026-10-13): adds a bounded, versioned recovery capsule around a complete `.vexel` bundle with source path/baseline metadata, CRC checks, exact-path validation, and atomic recovery-file create/replace. Focused project recovery/bundle/file tests and strict app/test builds pass in Debug, ASan/UBSan, and UBSan. Autosave scheduling, Application Support location, cleanup/retention, restart discovery, restore/conflict UX, and broad UI/E2E remain outstanding. Evidence: `artifacts/WO-PROJECT-04/verification.txt`.
- Accepted in the working tree 2026-10-13: WO-ANIM-PROJECT-17 adds an accessible native timeline “Key Opacity” action for a selected SVG node. It captures presentation opacity at the playhead, creates/reuses a stable-ID scalar track, commits later-frame track plus baseline/key atomically, and refreshes the live preview. Focused Debug, ASan/UBSan, and UBSan checks pass; broad UI/E2E remains deferred. Evidence: `artifacts/WO-ANIM-PROJECT-17/verification.txt`. Remaining: broader property authoring, drawing-cel UI, deterministic export, final broad acceptance, and the recorded Codex/MCP character-animation showcase for the website and README.

- Accepted in the working tree 2026-10-13: WO-ANIM-PROJECT-18 adds validated native SVG drawing assets to the live `.vexel` bundle through MCP, with indexed lookup, bounded overlay storage, dirty/save lifecycle, and SVG-save protection. Focused bundle, save/reload, MCP schema/call, and new-track/cel render checks pass in Debug, ASan/UBSan, and UBSan. Broad UI/E2E remains deferred; no commit. Evidence: `artifacts/WO-ANIM-PROJECT-18/verification.txt`. Next: native UI drawing-cel authoring, deterministic animation export, final broad acceptance, and the reviewed Codex/Vexel showcase recording for the website and README.
- Accepted in the working tree 2026-10-13: WO-ANIM-PROJECT-20 adds accessible asset cycling and undoable drawing-cel insertion on a selected native drawing track at the playhead. Incompatible SVG canvas assets are skipped/rejected; same-tick exposure replacement, preview rendering, tick boundary, mutation notification, and undo/redo are covered. Focused Debug, ASan/UBSan, and UBSan timeline checks pass; no broad UI/E2E or commit. Evidence: `artifacts/WO-ANIM-PROJECT-20/verification.txt`. Next: timeline drawing-track creation, deterministic animation export, final acceptance, then the reviewed live Codex/Vexel showcase for the website and README.

- Accepted in the working tree 2026-10-13: WO-PRODUCT-GOAL-03 reconciles the product goal, staged delivery plan, and README with current native animation/MCP capabilities and evidence. The final recorded live Codex character-animation showcase is explicitly planned after software acceptance and is not yet recorded. Corpus/performance status remains qualified; broad UI/E2E stays deferred. Evidence: `artifacts/WO-PRODUCT-GOAL-03/verification.txt`. No application tests or commit.

- WO-ANIM-PROJECT-25 revision 2 adds user-facing native transform key authoring through an accessible timeline control. Users can capture an identified object's identity or `matrix(...)` transform at tick zero, then key the updated transform at later playheads. Unsupported transform syntax and later first keys are rejected without data loss; identical keys are no-ops. Focused native timeline tests and strict app builds pass in Debug, ASan/UBSan, and UBSan. Comprehensive UI/E2E remains deferred. Evidence: `artifacts/WO-ANIM-PROJECT-25/verification.txt`; uncommitted.

- Product showcase clarification (WO-PRODUCT-GOAL-04, 2026-10-13): the final
  user-facing example footage is specified as Codex creating a sample character
  and animating it through live MCP while Vexel updates visibly. The reviewed,
  accessible and accurate video (or coordinated short clips) is intended for the
  website and README after feature work and broad UI/E2E/release acceptance. It
  has not been recorded or published. Evidence: `artifacts/WO-PRODUCT-GOAL-04/`.

- Accepted in the working tree 2026-10-13: WO-ANIM-PROJECT-26 adds atomic,
  source-preserving movement of native cels and property keys by integer tick.
  Timeline pointer drag previews before commit and reports baseline/collision
  failures; MCP exposes `native_timeline_move_item` with live dirty/preview
  updates and no-op reporting. Focused project, native timeline, and MCP CTests
  pass 3/3 in Debug, ASan/UBSan, and UBSan; strict `svgworkbench` builds pass.
  Broad UI/E2E remains deferred. Next product gaps include native project
  recovery/migration, SVG/Inkscape fidelity, long-operation MCP control, and
  large-document performance evidence. Evidence: `artifacts/WO-ANIM-PROJECT-26/`.

- Accepted in the working tree 2026-10-13: WO-PROJECT-05 adds a macOS Foundation-backed private Application Support recovery path keyed by SHA-256 of the canonical source path. Directories are mode 0700; symlinks, unsafe permissions, missing/nonregular sources, and undersized output are rejected. Focused resolver and capsule tests pass 2/2 in Debug, ASan/UBSan, and UBSan. Next: immutable document-edit snapshots so recovery capture can encode large projects on a worker without UI-thread materialization; then periodic scheduling, restart discovery, conflict/restore UI, retention, and final broad acceptance. Evidence: `artifacts/WO-PROJECT-05/verification.txt`.

- Accepted in the working tree 2026-10-13: WO-PROJECT-06 adds immutable document-edit snapshots that copy piece descriptors and retain unchanged source/blob storage, allowing worker materialization while editing continues. Synchronized snapshot concurrency, history stability, bounds, and existing edit/history tests pass 3/3 in Debug, ASan/UBSan, and UBSan; strict app and focused test builds pass. Next: use snapshots in recovery worker jobs and compose current timeline/assets/manifest safely; then schedule capture and add restart discovery/restore UX. Broad UI/E2E remains deferred. Evidence: `artifacts/WO-PROJECT-06/verification.txt`.
- WO-PROJECT-07 implementation (2026-10-13): dirty saved native projects now receive a best-effort 30-second background recovery capture. It uses an immutable scene snapshot, a timeline source lease, copied asset descriptors, and the private Application Support capsule path; source stamps are checked around publication and dirty/undo/saved state remains unchanged. Focused recovery, snapshot, timeline-lease, and capsule tests pass in Debug and UBSan (4/4); ASan/UBSan three focused CTests pass and the timeline history test passes directly (roughly 30 seconds). Restart restore/conflict UI, save/discard cleanup, retention, migration, broad acceptance, and large-file peak-memory evidence remain open. Evidence: `artifacts/WO-PROJECT-07/verification.txt`.

- Accepted in the working tree 2026-10-13: WO-PROJECT-08 adds worker-side recovery discovery for existing native projects, exact capsule/source validation, explicit restore/discard/cancel and conflict handling, complete dirty restore with assets/timeline, and capsule cleanup after successful saves or discard. Focused Debug, ASan/UBSan, and UBSan builds pass; the four required CTests pass in each configuration (12/12 total). Broad UI/E2E remains deferred. Missing-source recovery, retention, migration, and large-file recovery memory evidence remain open. Cleanup success/failure paths have focused coverage; full file-save fault injection remains open. Evidence: `artifacts/WO-PROJECT-08/verification.txt`.

- Accepted in the working tree 2026-10-13: WO-SVG-PATH-01 accepts lexically separable adjacent decimal operands in SVG path data (including `M-.4.6`) while retaining exact source bytes and numeric spans. Compact and spaced forms produce identical geometry; malformed adjacency remains rejected with a stable path diagnostic. The focused parser and shape-render CTests pass 2/2 in Debug, ASan/UBSan, and UBSan (6/6 total). No UI/E2E or corpus sweep was run. The broad SVG/Inkscape fidelity goal remains open; WO-CORPUS-04 is still a partial candidate with measured coverage of 5/58, not an accepted fidelity result. Evidence: `artifacts/WO-SVG-PATH-01/verification.txt`.

- Accepted in the working tree 2026-10-13: WO-SVG-PAINT-01 adds bounded local linear/radial gradient strokes for standard and custom geometry, including document-space sampling for stroke-only gradients, stroke opacity, source-over compositing, and preflight stop accounting. Focused shape-render and gradient-stroke CTests pass 2/2 in Debug, ASan/UBSan, and UBSan (6/6 total); no broad UI/E2E or corpus sweep was run. Solid and pattern stroke handling remains covered by the existing shape-render suite. SVG/Inkscape fidelity remains incomplete. Evidence: `artifacts/WO-SVG-PAINT-01/verification.txt`.

- Accepted in the working tree 2026-10-13: WO-MCP-OBS-01 adds a sanitized per-tool completion/failure entry to Vexel's existing operator log and status strip through the central MCP broker callback. Focused native timeline broker coverage passes 1/1 in Debug, ASan/UBSan, and UBSan (3/3 total). No broad UI/E2E run. Long-operation progress/cancellation and live Codex/Claude interoperability remain open. Evidence: `artifacts/WO-MCP-OBS-01/verification.txt`.

- Accepted in the working tree 2026-10-13: WO-SVG-MEM-01 stores non-default fill paints sparsely behind a bounded per-shape index, preserving gradient, pattern, image, group, and animation rendering. Focused `shape-render`, `svg-gradient-stroke`, and `svg-animation` CTests pass 3/3 in Debug, ASan/UBSan, and UBSan (9/9 total). Default-fill array savings are established by type-size arithmetic; no RSS or corpus rerender was performed. The 65,536-shape limit remains unchanged pending reductions to stroke and other metadata. Evidence: `artifacts/WO-SVG-MEM-01/verification.txt`.

- Accepted in the working tree 2026-10-13: WO-SVG-MEM-02 stores only
  non-default stroke paints behind a bounded per-shape index. Stroke gradients
  and patterns resolve through sparse accessors across raster sampling,
  preflight, grouped rendering, animation copies, and the scalar reference.
  Focused `shape-render`, `svg-gradient-stroke`, and `svg-animation` CTests
  pass 3/3 in Debug, ASan/UBSan, and UBSan (9/9 total). Static arithmetic
  estimates about 17.6 MB less reservation when only one of 65,536 shapes has a
  non-solid stroke; no RSS or corpus rerender was performed. The shape ceiling
  remains unchanged. Evidence: `artifacts/WO-SVG-MEM-02/verification.txt`.

- Accepted in the working tree 2026-10-13: WO-SVG-MEM-03 replaces each stroke's
  inline 16-double dash array with bounded offset/count metadata and a compact
  document-owned values array. Direct stroke containment, seam detection,
  dash-cache construction, scalar validation, and grouped/animation document
  copies use the checked accessor. Focused `shape-render`,
  `svg-stroke-dash-storage`, and `svg-animation` CTests pass 3/3 in Debug,
  ASan/UBSan, and UBSan (9/9 total). The stroke descriptor shrinks by 120 bytes
  on the measured build; no RSS or corpus claim. Broad UI/E2E remains deferred.
  Evidence: `artifacts/WO-SVG-MEM-03/verification.txt`.

- Accepted in the working tree 2026-10-13: WO-SVG-CLIP-01 makes local
  `<clipPath>` elements non-rendering definitions wherever they occur in the
  SVG tree, including inside a rendered group, while retaining original-token
  reference lookup. A synthetic nested-definition fixture verifies clipped
  pixels, scene shape count, exact source retention, and unchanged missing-ID
  diagnostics. Focused `shape-render`, `svg-nested-clip-definition`, and
  `svg-animation` CTests pass 3/3 in Debug, ASan/UBSan, and UBSan (9/9 total).
  No corpus or broad UI/E2E run; corpus status remains partial. Evidence:
  `artifacts/WO-SVG-CLIP-01/verification.txt`.

- Accepted in the working tree 2026-10-13: WO-PRODUCT-PLAN-01 replaces the broad immediate-next-steps list with a dependency-aware delivery sequence, rough focused effort bands, exit evidence, uncertainty and safe parallelization constraints. The product goal remains active. No application tests were run; broad UI/E2E remains deferred until feature work is complete. Evidence: `artifacts/WO-PRODUCT-PLAN-01/verification.txt`.

- Accepted in the working tree 2026-10-13: WO-SVG-CLIP-02 adds inherited SVG `clip-rule` resolution for referenced clip geometry, including presentation attributes and the supported inline/stylesheet CSS cascade. Synthetic pixel tests cover default/nonzero, evenodd, inheritance, non-inheritance from the referencing drawable, invalid-value diagnostics, source identity, and fail-closed unrelated styling. The focused shape-render, nested-clip, animation, and CSS tests pass 4/4 in Debug, ASan/UBSan, and UBSan. No corpus or broad UI/E2E run; corpus results remain partial and unaccepted. Evidence: `artifacts/WO-SVG-CLIP-02/verification.txt`.


- Accepted diagnostic finding 2026-10-13: WO-CORPUS-06 traced the two
  generated Inkscape-style failures before raster work preflight. T1 exceeds the
  dash segment cache by one (65,537 requested / 65,536 cap); T2 reaches the
  65,537th path with the 65,536-shape decode limit. The 16-billion raster work
  cap is not implicated, and raising it will not help. Next slice should address
  each bound with a memory-safe representation and retain adversarial limits.
  This is diagnostic only; corpus counts remain 3/39 accepted and 5/39
  candidate, 19/58 skipped. No UI/E2E or broad suite run. Evidence:
  `artifacts/WO-CORPUS-06/verification.txt`.


- Accepted bounded SVG cache change 2026-10-13: WO-CORPUS-07 removes the
  independent 65,536 dash segment/join count cap while retaining checked
  geometric growth, the 16 MiB combined cache bound, the 1,000,000-step limit,
  and existing grid limits. Focused Release CTests pass 2/2. Generated T1 no
  longer hits the dash-entry refusal but still reports a later raster LIMIT at
  byte zero; full rendering is not established. T2's shape limit is unchanged.
  Next: identify T1's later raster failure before changing another budget.
  Broad UI/E2E remains deferred. Evidence:
  `artifacts/WO-CORPUS-07/verification.txt`.


- Accepted diagnostic finding 2026-10-13: WO-CORPUS-08 traced generated T1
  past dash-cache construction to failure at `raster_fill_scanline_cache_create`
  from the grouped `raster_shape_render_flat` path. The failure is before
  boundary marking and scaled work preflight; the internal return branch is not
  identified yet. No budgets changed. Next: instrument that cache constructor's
  internal early-return branches before changing any more limits. T1 is still
  not proven to render. Evidence: `artifacts/WO-CORPUS-08/verification.txt`.


- Diagnostic correction (WO-CORPUS-09, 2026-10-13): the WO-CORPUS-08
  `flat raster failure line=259` maps to `raster_shape_dash_cache_create`,
  not `raster_fill_scanline_cache_create`; the prior state entry misidentified
  that call. WO-CORPUS-09 confirmed the top-level two-shape fill-cache call
  succeeds. A grouped 34,714-shape subrange fails earlier, before its fill-cache
  call; its dash-cache branch remains uninstrumented. No limits changed.
  `artifacts/WO-CORPUS-09/verification.txt` records the failed diagnostic and
  exact correction.


- Accepted diagnostic finding 2026-10-13: WO-CORPUS-10 identifies T1's current
  refusal after WO-CORPUS-07: 131,073 segments are requested, but doubling to
  262,144 would raise combined segment/join/range capacity to 20,109,760 bytes,
  above the 16 MiB cache budget. The cache had 131,072 segments, 131,072 joins,
  and 262,145 construction steps; raster work was not reached. Next candidate is
  checked 1.5x or byte-aware capacity growth, preserving every byte/step/grid
  limit. T1 remains unrendered pending that candidate. Evidence:
  `artifacts/WO-CORPUS-10/verification.txt`.


- Accepted candidate progress 2026-10-13: WO-CORPUS-11 changes dash-cache
  capacity growth to checked 1.125x increments while retaining the 16 MiB
  combined cache bound, 1,000,000 construction steps, and grid limits. A new
  focused regression builds more than 65,536 dash segments within the byte cap;
  Release shape-render and dash-storage CTests pass 2/2. The exact manifested
  generated T1 now produces a valid 128x128 PPM (1.61 s real, 129,875,968 B max
  RSS in one run). This is a candidate full-render success, not a visual parity
  result and not a corpus-wide count update. T2's 65,536-shape bound and the
  broad UI/E2E gate remain open. Evidence:
  `artifacts/WO-CORPUS-11/verification.txt`.


- Fidelity audit 2026-10-13: WO-CORPUS-12 compares the current candidate T1
  render with installed Inkscape 1.4.4 at 1600x1000. With transparent pixels
  composited over white, 193,585/1,600,000 pixels (12.099%) still differ by
  more than channel tolerance 2; max delta 151, bbox is the full page. Inkscape
  warns that its unknown LPE stack was disabled. The target is renderable but
  not parity-accepted. The next SVG order should isolate the dominant mismatch
  regions/features before attempting another corpus expansion. Evidence, raw
  outputs and difference highlights are under `artifacts/WO-CORPUS-12/` and
  ignored `.cache/WO-CORPUS-12/`.


- Fidelity ablation 2026-10-13: WO-CORPUS-13 isolates T1's mismatch. The
  leading filtered gradient path alone differs from Inkscape on 0.144% of pixels
  above tolerance 2; the inherited gradient-stroke group alone differs on
  11.974%. That group contains 34,714 small paths, so the next renderer slice
  should target inherited/objectBoundingBox gradient stroke coordinates and
  sample/coverage semantics. The ablation locates the feature family but not the
  exact defect. Temporary variants and images are in ignored
  `.cache/WO-CORPUS-13/`; evidence: `artifacts/WO-CORPUS-13/verification.txt`.

- Accepted in the working tree 2026-10-13: `WO-ANIM-PROJECT-29` adds stable,
  document-unique IDs to newly inserted SVG graphics roots without IDs across
  the shared template/fragment edit APIs. Existing unique IDs and untouched
  SVG bytes are preserved; duplicate supplied root IDs fail before mutation.
  Canvas and MCP creation share the operation, and MCP returns the ID as
  `elementId`. Focused `document-edit` and `mcp-native-timeline` tests pass 2/2
  in Debug, ASan/UBSan, and UBSan; the focused targets build in all three.
  No broad UI/E2E, corpus, fuzz, performance, or release suite ran. This
  unlocks object targeting but does not complete the character-animation
  workflow. Evidence: `artifacts/WO-ANIM-PROJECT-29/verification.txt`.

- Accepted in the working tree 2026-10-13: `WO-PRODUCT-PLAN-03` reconciles
  the product roadmap and root MCP guide with accepted stable-ID behavior and
  the current tool surface. The guide now documents fragment/text insertion,
  returned generated IDs, SVG SMIL and native timeline tools, animation
  rendering/export, atomic new-file Save As, visible completion/failure status,
  and the remaining progress/cancellation and live-client gates. The fastest
  sequence keeps SVG fidelity, user-authored animation, project recovery,
  MCP control, hardware-backed performance, UI/accessibility, final acceptance,
  and the post-acceptance showcase explicit. Documentation checks passed; no
  application suites ran. Evidence: `artifacts/WO-PRODUCT-PLAN-03/verification.txt`.

- Accepted in the working tree 2026-10-13: `WO-ANIM-PROJECT-30` adds native
  Animate timeline Key Fill and Key Stroke controls for a single identified
  object with an explicit supported paint attribute. Selected paints parse the
  documented hex/named subset into RGBA; style overrides and unsupported paint
  forms fail closed. Color keys reuse/create native tracks, require tick-zero
  baselines for later edits, and are undoable without changing SVG scene bytes.
  `viewer-timeline-native` passes 1/1 in Debug; `svgworkbench` and the focused
  target build. All changed C files remain below 800 lines. Broad UI/E2E and
  sanitizer/release suites were not run. Evidence:
  `artifacts/WO-ANIM-PROJECT-30/verification.txt`.

- Accepted in the working tree 2026-10-13: `WO-ANIM-PROJECT-31` adds a
  user-facing Key Visibility action to the native Animate timeline. The bounded
  selected-node reader accepts default/explicit visible state and explicit
  hidden/collapse, while rejecting ambiguous IDs, CSS/style overrides, and
  inherited hidden state without a local override. The action authors an
  undoable boolean/step track with a tick-zero baseline, updates later integer
  ticks, and preserves SVG scene bytes. Focused Debug build and
  `viewer-timeline-native` pass; all changed C sources/tests remain below 800
  lines. Broad UI/E2E and sanitizer/release gates remain deferred. Evidence:
  `artifacts/WO-ANIM-PROJECT-31/verification.txt`.

- WO-MCP-TRANSPORT-01 r2 host verification supplement (2026-10-13): the nested
  executor's private-socket bind restriction caused skips, but the same focused
  checks pass from the host shell: all 11 `McpStdioRelayTests` and CTest
  `mcp-stdio-relay` plus `mcp-stdio-e2e` pass (2/2). The relay is now runtime
  verified; this does not implement async tasks/export, cooperative
  cancellation, app progress, or live Codex/Claude sessions. Broad UI/E2E stays
  deferred. Evidence: `artifacts/WO-MCP-TRANSPORT-01/verification.txt`.

- WO-MCP-TRANSPORT-01 r2 is superseded before acceptance after a focused
  gpt-6.1-sol review found the silent-broker wait lacked the preserved 30-second
  bound and the scope verifier mislabeled the intentional expected-tool
  inventory expansion as byte-identical E2E assertions. The host test results
  remain valid for their cases. Revision 3 is frozen to close both findings;
  no broad UI/E2E suite was run. Evidence:
  `artifacts/WO-MCP-TRANSPORT-01/review-r2-sol.txt` and
  `scope-freeze-r3.txt`.

- WO-MCP-TRANSPORT-01 r3 rejected before acceptance: Luna High review found client traffic renewing broker inactivity, response accounting delayed by stdout, and canceled requests remaining pending. The 12 fixtures and 2 focused CTests pass but do not cover these defects. A copied helper overwrote the r2 derived diff; original manifest digest and source/test evidence remain. R4 will close accounting and freeze bounded focused regressions. Evidence: `artifacts/WO-MCP-TRANSPORT-01/review-r3-luna.txt`.

- Owner resumed with existing-allowance Luna parallel execution. `WO-DELIVERY-PARALLEL-01` records full ADR-0018 scope, per-task Low/Medium/High effort with reasoned escalation only, compact contexts, two isolated implementation workers and serialized shared integration. Transport r4 and immutable native snapshot r1 are running with current-source overlay manifests; no acceptance of their code yet. Plan review/metadata/diff checks pass. Broad UI/E2E stays final. See `docs/product/luna-parallel-delivery-plan.md`.

- Accepted uncommitted `WO-MCP-SNAPSHOT-01`: owned immutable encoded native-project snapshot captures current dirty scene/timeline/metadata and base+overlay assets with16MiB scene/64MiB encoded limits. Independent review and invalid-input repair completed; exact isolated delta/conflict audit passes; integrated Debug targets build and `mcp-export-snapshot` passes1/1. Task store untouched. Source bundle validation cost remains a later owner-thread responsiveness probe; async export/protocol/UI integration remains open. Evidence: `artifacts/WO-MCP-SNAPSHOT-01/verification.txt`.

- Accepted uncommitted `WO-MCP-TRANSPORT-01 r4`: broker-only30second inactivity renewal; matching receive-time response and fully-forwarded cancellation retirement;256slots/2MiB ID+metadata budget; escaped string identity and typed numeric text; null-ID errors and server requests retain forwarding. All original12 fixture bodies and broker E2E assertions preserved;21relay fixtures and both integrated focused CTests pass (2/2). Exact worktree delta/conflict audit and coordinator review pass. Main HEAD/index untouched. Async GIF worker/protocol/nonblocking app socket and visible progress/cancel still required. Evidence: `artifacts/WO-MCP-TRANSPORT-01/verification-r4.txt`.

-2026-10-14: active goal confirmed. Frozen isolated Luna High async pair `WO-MCP-GIF-WORKER-01` and `WO-MCP-TASK-PROTOCOL-01` are running from current-source overlays. Root owner-bridge preflight identifies nonblocking app output, actual editor action cancel wiring, owner/generation handling and genuinely atomic GIF publication as mandatory follow-on scope. Existing GIF writer is O_EXCL/no-replace but writes directly to final destination; roadmap corrected. No live async/Tasks or broad UI acceptance claimed.

- Accepted uncommitted2026-10-14 `WO-MCP-GIF-WORKER-01`: snapshot-only native GIF worker with bounded rendering, progress, cancellation and owned output. Reviewed cleanup/token handling and independently decoded frames/timing; queue-only cancel regression confirms cancellation before shutdown. Integrated Debug build and `mcp-export-job`1/1 pass. Integral FPS follows existing MCP/encoder; owner must gate take on uncanceled queue completion and recheck before publication. No live app export claim. Evidence: `artifacts/WO-MCP-GIF-WORKER-01/verification-integrated.txt`.

- Accepted uncommitted2026-10-14 `WO-MCP-TASK-PROTOCOL-01 r2`: opt-in async adapter, per-request capability/readiness, flat Task creation, DetailedTask retrieval, cancel ACK and legacy deferred/cancel/progress. R1 superseded after review found schema gaps. R2 validates content blocks and operation-specific metadata/terminal fields; actual JSON envelopes/IDs tested. Main Debug named build and focused protocol2/2pass; original test SHA/backend layouts preserved. Production remains sync until owner bridge. Evidence: `artifacts/WO-MCP-TASK-PROTOCOL-01/verification-integrated-r2.txt`.

- Accepted uncommitted2026-10-14 `WO-MCP-GIF-PUBLISH-01`: native/SVG GIF exports now recheck canonical grants and use existing atomic no-replace primitive. Published/durable status reflects commit even on postcommit errors; response prebuilt before publication avoids metadata-encoding failure after commit. Added partial-write absence/cleanup, existing-byte preservation, no-grant and response-limit regressions. Main build+source-save/native-timeline2/2pass, C<800. Test fixtures now repo-local .cache. Directory-fsync error classification reviewed but not injected. Async IO/owner bridge still pending. Evidence: `artifacts/WO-MCP-GIF-PUBLISH-01/verification-integrated.txt`.

-2026-10-15: `WO-MCP-SOCKET-IO-01` remains accepted as legacy reference. The reviewed C `WO-MCP-CAPTURE-01` candidate passed focused checks but its brief main-tree integration was reverted after reasserting the Rust-only first-party target. Candidate and evidence remain preserved; reimplement owner capture, worker integration and the live app bridge in Rust. UI cancel/generation behavior, client interoperability and complete application remain open. Broad UI/E2E stays at the end; existing subscription allowance only.

- Accepted uncommitted2026-10-14 `WO-MCP-SOCKET-IO-01`: app hook now uses bounded nonblocking IO with stable perprocess connection IDs, reserved terminal queues and unsent-only progress coalescing. Shared64KiB total/8KiBclient transfer and syscall/line budgets, explicit close, callback slot-reuse and duplicate-fd guards, full allocated queue caps reviewed/repaired. Integrated app+test build and `mcp-broker-io`1/1pass; stalled-reader transport-only poll1000ns. Sync tool callback cost excluded; no live async/Tasks claim. Evidence: `artifacts/WO-MCP-SOCKET-IO-01/verification-integrated.txt`.

- Owner2026-10-14 explicitly stopped former C delivery and requested full Rust port plus many parallel Luna codex exec workers. All old workers stopped/completed; unfinished owned-capture candidate remains isolated/unaccepted and preserved. Old goal tracker paused; replacement API rejected unfinished-goal constraint (not falsely completed). ADR0022/OWNER/AGENTS now make Rust the first-party target; full ADR0018 product/video goal persists. WO-RUST-PLAN-01 running; source/tests unchanged.

- Accepted Rust migration planning `WO-RUST-PLAN-01`: six actual Luna codex exec research runs exit0, corrected complete-code inventory603files/137570lines (fiveinc included), source/index unchanged. Rust1.99/2024/Cargo and owned domain/native platform direction recorded; dependencies remain candidates. Full product/video goal retained. Next bootstrap+six isolated foundation writers, lean config to avoid unrelated connector metadata. Old goal tracker paused/replacement API rejected; repository Rust goal authoritative, no product completion claim.

- Accepted uncommitted Rust scaffold `WO-RUST-BOOTSTRAP-01`: std-only Cargo1.99/2024/resolver3, macOS13AppleSilicon guards, local build/cache/tmp, safe Span/BaseStatus/Point/Affine contracts and Rust developer info/build/check. Cargo check+2Span tests+devinfo/check pass. Low reviewer contract misunderstanding reconciled; Medium recheck accepts. Rustfmt unavailable, no install/pass claim. Six foundation work orders/current-source overlays frozen; actual module ports launch next. No full module/app/Rust parity claimed.

- Six actual lean Luna codex exec module writers launched in isolated Rust worktrees: baseHigh, geometryMedium, XMLreaderHigh, JSONHigh, project-v1bundleHigh and raster-tileMedium. Registry: `artifacts/WO-RUST-BOOTSTRAP-01/foundation-processes.json`; root owns shared manifests/contracts/acceptance. These are partial foundation scopes, not completed subsystems/app. Broader UI/E2E remains final.

- Accepted Rust foundation ports `WO-RUST-GEOMETRY-01` and `WO-RUST-RASTER-01`: affine math/current limits and owned RGBA tile pattern/storage mirrored safely, original cases ported, independent LunaMedium CLI reviews accept. Exact isolated/root audit passes; main per-package checks and5geometry/4tile tests pass. No C/source/fixture/manifest/index change. Base/XMLreader/JSON/bundle candidates under separate High reviews; complete renderer/app still unported.

- Correction to preceding foundation test count: integrated geometry and tile packages each pass5focused tests (tile was inadvertently written4 in summary); exact logs show5. Base/JSON independent High reviews accept candidates; XMLreader needs exact diagnostic/name-boundary tests, bundle needs C-compatible asset CRC error_offset. Two scoped repairs running, no candidate acceptance for these four yet.

- Accepted `WO-RUST-BASE-01` and `WO-RUST-JSON-01`: safe checked byte helpers/bounded aligned arena with explicit C-pointer API adaptations, and owned bounded JSON parse/accessor/string/buffer core. Independent High reviews accept; exact worker/root audits and main checks pass. Focused counts:{'base': 7, 'json': 5}. Reviewer noted nonblocking assertion gaps, retained in evidence. No legacy source/assertion/manifest/index changes; XML/bundle repairs remain.

- Accepted `WO-RUST-XML-01` reader/character module and `WO-RUST-PROJECT-01` strict bundle-v1 codec after High review and one scoped repair: XML exact diagnostics/name-boundary assertions, bundle assetCRC consumed-cursor error_offset corrected. Root audit+main checks pass, focused counts:{'xml': 10, 'project': 8}. All six first foundation ports now integrated; full XMLtokenizer/nativeproject/editor remains unported. Original code/fixtures/manifest/index preserved.

-2026-10-14: accepted WO-RUST-SCENE-MODEL-01 complete owned scene-data gate after independent Luna review and two rechecks, exact source/nested capacity, metadata and sibling-group regressions. Fresh27main tests/protected603/index checks pass; recorded aggregate487entries/28packages. Full kernel/decoder contracts and complete renderer/native/application/performance/finaltesting/release/live-agent demos remain. Evidence: artifacts/WO-RUST-SCENE-MODEL-01/.

-2026-10-14: WO-RUST-KERNEL-CONTRACTS-01 accepted complete shareddata/interfaces after LunaHigh independent review/recheck,43main scoped tests and603/157oracles/index audits. Private canonical lexical owner and true effective frame tables; no fake renderer/decoder/native provider. Aggregate503entries/28packages. Exact deferred draft-finalization mutation dependency found before decoder launch; separate coordinator gate next.

-2026-10-14: WO-RUST-NATIVE-WINDOW-PLAN-01 R2 readiness accepted after LunaHigh planner/Low repair/Medium review/recheck. App-owned native registry with weak Window generations, complete owned IME snapshots, sequenced reset acknowledgement, pending event batches and teardown; required files/menu/AX/text/presenter/preferences/dialog/welcome lanes explicit. Initial stale-license concern disproved against accepted ADR0026/setup/boundary/grant evidence. R1 control-artifact allowlist mismatch and attempted invalid ledger transition preserved/corrected. No native-window code/GUI/mac13runtime acceptance; implementation order draft ready at artifacts/WO-RUST-NATIVE-WINDOW-PLAN-01/ready-window-order.md.

-2026-10-14: frozen WO-RUST-NATIVE-WINDOW-01 running as actual Standard LunaHigh source writer in1660-file isolated current-source overlay, selected local registry seeded, no package/feature/GUI/broadUI changes. Confirmed live by childPID after launch; metadata/tool session in accepted shared-contract checkpoint. Complete real nativewindow lane candidate requires root independent review/main checks before acceptance. Root next shared task is deferred mark-scoped scene draft finalization, independent of this writer.

-2026-10-14: WO-RUST-GOAL-06 clarifies required authentic Codex draw/revise/animate/revise live-MCP video acceptance, reusable assets and README/website delivery; nine outcomes/seven gates retained. Documentation checks only, actual demo pending.
-2026-10-14: WO-RUST-SCENE-DRAFT-01 accepted: opaque bounded64LIFO decoder draft scopes, new-record finalizers, child rollback and capacity, ignored-notice snapshots repaired after concrete Luna review blocker. Independent recheck ACCEPT, fresh57focused tests (43unchanged+14new) and603/157/HEAD/index audit pass. Aggregate517recorded entries/28packages; no full decoder/renderer/product acceptance.
-2026-10-14: ADR0027/WO-RUST-NATIVE-WINDOW-FEATURE-01 accepted metadata-only five-feature prerequisite; exact pins and16active/18lock unchanged, original7native tests and independent LunaLow review/audit pass. Initial native-window Luna source candidate exited INCOMPLETE and is preserved/superseded, not accepted. Frozen R2 now resumes actual same-session Sol6.1Medium repair with approved coordinator seed; childPID confirmed live. Full native callbacks/event-pump/IME/timers/Quartz/cursors/material/teardown and independent review/main checks required. BroadUI/E2E/packagedrelease/liveMCP/demo acceptance remains final.

-2026-10-14: WO-RUST-RENDER-WAVE-SETUP-01 accepted registration only: eight separate whole-crate scopes retain full35owner files/416symbols and original suite/operation assignments; accepted shared/draft APIs, no unfinished peer dependencies. Independent LunaMedium ACCEPT, lockedoffline workspace build/eight ready orders/ledger/diff and603/157/oldRust/HEAD/index audit pass; external pins unchanged. No new engines accepted, aggregate still517focused entries/28accepted packages.
-2026-10-14: Eight actual isolated Standard Luna source writers confirmed live: SVG decode, paint, definitions, SVG text/image glue, scene selection/capacity queries, scalar index/preflight, scalar sample/stroke/grid and animation-frame composition. Scene query Medium, sevencomplex scopes High. Native-window Sol6.1Medium R2 repair separately confirmed live; nineuseful source processes total. Initial overlay proof-filename error occurred before newworker launch; preserved partial checkout/evidence, corrected and continued same assembly without restarting a live worker. Candidate success still requires full-scope review/protected audit/mainfocused tests. Effects/flat-render/rootadapters/composition/native media/platform/UI/app/liveMCP/performance/tooling/fullQA/release/recorded websiteREADME showcase remain required; fullgoal active.

-2026-10-14: Current accepted aggregate579focused entries/28packages: scene82, nativeApple41, CSS66. Consumer cache/source/ownedstops/checkedhandles/atomicfinishedclips R2 accepted, original57+9cases; native source R2 integrated after reserved16 queue fix/independent recheck/freshmain41tests (7old+34new), liveGUI/mac13 runtime remains final. Exact4marker copied-context API accepted after evidence recheck, original63+3new CSS.
-2026-10-14: Shared paint publication API accepted after named SolMedium implementation/LunaHigh review/freshmain82tests (66unchanged+16new)/603/157/source/index audits. Opaque realgradient/source/viewport identity, per-scene sorted stopsharing and atomic fillstroke handoff, append-order correct ordinary/nestedrollback/stale witnesses, actualcapacity/sourceonce. 40000instances share2stops/full32768templates/65536stops. No actual gradientresolver/core/raster acceptance follows.
-2026-10-14: All eight initial render-owner candidates returned incomplete/unaccepted; full owner scope stillrequired. PAINT received accepted publication supplement after terminal process verified and resumes same actual Standard LunaHigh session, nowconfirmed live. Animation API code improved but fixture proof needs declared path dependency; query needs checkedslots/fixtures; definitions marker helper accepted but full algorithms/proofs remain; media use-host mapping and scalar job-ledger/control seams next. No liveprocess restarted for timeout, no partialscope/fullapp/UIQA/release/demo claim. Native source accepted, remainingnativeUI/files/menu/AX/shaping/Metal/application/liveMCP stillrequired.

-2026-10-14: Fixture metadata WO-RUST-RENDER-FIXTURE-SETUP-01/ADR0028 accepted: exact firstparty path/animation devedges on7owner crates, external pins/packages/runtimeedges unchanged. Independent LunaLow review/workspacecheck/protected audit. MEDIA usehost SourceMap contract accepted after LunaMedium review/fresh86tests (82unchanged+4new)/603/157/index audits; canonical reference tokens remain distinct, source/ledger/hostspan/finite independentCTMs/writable scene/livehandle/cancel checked beforeprovider. Aggregate583/28; sourcecontract only, producers/providers/app unaccepted.
-2026-10-14: Accepted coordination WO-RUST-RENDER-CONSUMER-RESUME-01 preserves4terminal candidates and seeds exact accepted shared/fixture/CSS/source-map inputs without rewriting cataloghashes or touching liveinputs. Same-session Luna source handles confirmedlive: animation/definitions/mediaHigh/queryMedium; existingPAINTHigh stilllive, fiveuseful processes total. Fulloriginalalgorithms and realfixture/originalassertion proofs remainmandatory; no currentnewengine/app/finalQA/release/video acceptance.

## Accepted owned definitions — 2026-10-15

Full79symbol definitions source is accepted after complete LunaHigh review,
markerdiagnostic LunaLow repair/recheck, rootexact1685protected audit/source-diff
and fresh main40checks (36retained+4new).763recorded entries across32packages.
Clip/mask/pattern/marker/filter source and actual channel-aware mapped publication
are implemented. Full renderer/native/app/MCP/runtime/performance/QA/release/demo
remain required. Source-owned initialization is in canonicalviewport proof repair;
pregeometry/canonicalpath shape publication and cache-generation ownership are
ready shared orders, not implementation/acceptance. BroadUI/E2E has not run.

## Accepted source-owned initialization — 2026-10-15

784recorded focused entries/32packages. Scene177+unchangedanimation21fresh main
checks pass after fullHighreview/provenanceMediumrecheck/root1672audit/exactdiff.
Original156Scene assertions unchanged. Canonical root parsing, explicit pending
geometry/lateactualanimation, hiddenRoot and no-unresolved-render/freeze are
accepted source APIs. Accepted lowerconsumers compile; actual adoption remains.
Cache-generation transaction then shape-publication orders are ready; sole
sharedwriter starts only after this acceptance. BroadUI/E2E not run.

## Accepted cache generations — 2026-10-15

806recorded focusedentries/32packages. RootSolMedium final review, full1751file
protected audit and freshmain199Scene tests PASS (185prior unchanged+9runtime+
5compilefail). Original603 source files,157 assertion entries, HEAD and index
match their frozen hashes. Static pattern timing/scoped media/minmax contracts
are source interfaces and typed-fixture proof only; actual decoder/media/lower
kernel adoption and shape admission/canonical post-Z remain required. No broad
UI/E2E or full renderer/native/app/release/demo claim.

## V1 vertical slice defined — 2026-10-15

The owner requested a limited first release that makes the complete drawing to
animation workflow visible early. The V1 scope and UI direction are documented
in [`docs/product/vexel-v1-vertical-slice.md`](docs/product/vexel-v1-vertical-slice.md)
and linked from the full Rust plan. It covers a launchable Rust macOS app,
bounded SVG editing/import/export, editable native projects, short animation,
visible local MCP editing, GIF export and one integrated walkthrough. The full
product goal and final UI/E2E/release/video gates remain unchanged. V1
implementation has not started.

An independent Luna review of the isolated SVG media candidate rejected it on
two missing regression proofs: finite cursor overflow and rollback through the
request path, and late geometry offsets propagated from an authored run through
production text rendering. Candidate source and review are preserved under
`.cache/WO-RUST-SVG-MEDIA-01/worker` and
`artifacts/WO-RUST-SVG-MEDIA-01/luna-independent-review.md`; no integration or
acceptance is claimed.


## V1 native presentation seam accepted — 2026-10-15

Accepted `WO-RUST-APPLE-PRESENTATION-01`: the Apple platform crate now exposes
a safe owned RGBA frame API and publishes configured images through a prepared
Core Animation layer with a single guarded hierarchy swap. The `WindowRecord`
owns the published layer; `Window` remains a weak handle. Candidate and main-tree
checks both pass: 48 focused package tests, package check, and frozen work-order
validation. Independent Sol Medium review accepted after source-seed/scope and
evidence corrections. See `artifacts/WO-RUST-APPLE-PRESENTATION-01/`.

This is only the pixel presentation seam needed by the Rust app shell. It does
not deliver a launchable V1 application, editing tools, integrated SVG/project
workflow, timeline, GIF export, or live MCP interface. No broad UI/E2E tests or
AppKit runtime validation were run. Channel order, orientation, compositing,
retention/release, resize/scale behavior, teardown, and native exception
atomicity remain explicitly unverified. Continue with the V1 app shell and retain
the complete product goal.


## V1 Rust app shell accepted — 2026-10-15

Accepted `WO-RUST-V1-APP-SHELL-01` after independent Sol Medium review, frozen
seed/scope audit, and fresh main-tree checks. It adds the `vexel-app` executable,
a responsive custom Draw/Animate workbench, the checked `RgbaFrame` presenter,
CoreText-shaped/clipped system and SF Mono labels, compact layout behavior, and
the verified non-editing keyboard/IME route. The app has 9 passing focused
tests; `vexel-platform-apple` has 42 unit, 5 native-boundary, 1 native-window,
and 6 doc tests passing. Cargo check/test and the frozen validator all exit 0.
Evidence, exact hashes, scope audit, and review are under
[`artifacts/WO-RUST-V1-APP-SHELL-01/`](artifacts/WO-RUST-V1-APP-SHELL-01/).

This is a launchable shell only: drawing tools, editable project workflow,
animation authoring/playback/export, SVG interchange, and working MCP actions
are still unavailable. No native GUI was launched, so actual window startup,
text appearance, frame display, resize, input delivery, and shutdown remain
unverified. The complete Rust application goal remains active, and broad UI/E2E
acceptance stays deferred to the final stage.

## Concise V1 scope accepted — 2026-10-15

`ADR-0029` now sets V1 as one demonstrable character workflow: basic drawing
and a disclosed SVG/Inkscape subset, editable `.vexel` save/reopen, one short
animation loop and GIF export, plus visible authorized Codex MCP edits to the
same open project. The canvas-led Draw/Animate interface keeps the custom
macOS shell and its live agent activity. The estimate from the current state is
14–26 focused engineer-days (roughly 3–6 focused weeks), including one
integrated V1 walkthrough and native launch review. The detailed cutline and
feature estimates are in
[`docs/product/vexel-v1-vertical-slice.md`](docs/product/vexel-v1-vertical-slice.md).
This milestone leaves the full Rust application goal, broad final UI/E2E,
release gates, and the post-acceptance Codex video for README/website unchanged.
Documentation evidence is in `artifacts/WO-RUST-V1-SCOPE-01/`.

## WO-RUST-SCENE-CHILD-REQUESTS-01 accepted — 2026-10-15

The additive nested Scene request API is integrated in the main tree. The legacy
`begin_request` reentrancy rejection remains frozen. Sol Medium accepted the final
contract proof after two targeted regressions: matching consumed child draft and
request tokens reject as stale while the live parent continues; mapped static
pattern tickets reject a pre-request tile under a valid current child scope and
accept a post-request tile. The fresh main `vexel-scene` package suite passes
233 tests across 19 result blocks, with package check, scoped rustfmt, and work
order validator passing. The 375-peer preservation audit has no protected
mismatches. Evidence is in
[`artifacts/WO-RUST-SCENE-CHILD-REQUESTS-01/continuation-5/`](artifacts/WO-RUST-SCENE-CHILD-REQUESTS-01/continuation-5/).
This closes one nested-pattern API prerequisite only; production decoder and
renderer integration, live app workflow, remaining Rust port, UI, performance,
MCP client acceptance, broad final tests, release, and authentic demo remain open.



## WO-RUST-SCENE-OBSERVABILITY-01 accepted — 2026-10-15

The reviewed Scene observability prerequisite is integrated. `DecodeSource` now
forwards actual owned `ReferenceIndex` probe counts, and `ChargeLedger` exposes
fixed-size cumulative allocation history by tag without changing
`RenderMemoryStats` or public struct literals. The `None` history counts net
retained lease-registry capacity growth plus caller-tagged `None` payload; the
existing total allocation history continues to count full physical replacement
allocations. Release, slot reuse, overflow, replacement cancellation and
reference hit/miss behavior have focused proofs. Independent Sol Medium review
accepted after resolving registry-history semantics, overflow atomicity and
edition-2024 formatting findings.

Fresh main-tree `vexel-scene` check and package tests pass: 246 tests across 20
result blocks, including 9 observability integration tests and 10 compile-fail
doctests. Changed Rust files pass edition-2024 rustfmt; work-order validator and
ledger validator pass. The exact scope audit shows all four integrated paths
match the reviewed candidate, all 64 seeded paths match outside the allowed
changes, HEAD is unchanged, and no staged paths exist. `git diff --check` still
reports only the pre-existing protected `STATE.md:1341` trailing blank line;
no unrelated content was changed to mask it. This shared API does not accept
or integrate the SVG decoder. Next: seed this accepted observability API into
the isolated full decoder candidate, continue its exact parity gates, then
resume renderer and application workflow work. Full UI/E2E remains deferred.

## WO-RUST-SVG-PARITY-HARNESS-01 accepted — 2026-10-15

Integrated the standalone nested Rust parity harness for actual cached-gradient
resolution and clip definition planning. The 4,096-comment/512-rectangle/two-stop
fixture records 8,211 actual paint-cache token visits. Clip boundaries cover the
65,536-byte exact span and the 512 KiB aggregate using eight/nine distinct owner
declarations and actual `DefinitionPlanner` calls; the error path also checks
source preservation. Sol Medium review verified the repaired predicates, and the
coordinator refreshed post-review hashes and reran the offline package checks,
format, order validation, 41-file seed audit, root ledger validation, and diff
whitespace check. All pass. This closes a decoder parity prerequisite only;
renderer, full decoder acceptance, tile-cache integration, native app/UI, MCP
runtime, full product QA, release, and demo remain open. Evidence:
[`artifacts/WO-RUST-SVG-PARITY-HARNESS-01/`](artifacts/WO-RUST-SVG-PARITY-HARNESS-01/).

## WO-RUST-SVG-DECODE-01 R2 continuation 7 — candidate only — 2026-10-15

The isolated decoder candidate adds a focused three-input regression through the
actual Decoder: a small source, a 64 KiB comment-padded source, and an equal-size
source with 32 real `<use>` instances. It checks source preservation, actual
source/token/index capacities, bounded allocation growth, and reference probes
from traversal. Its locked offline package suite passes 89 tests; check, format,
and ready-order validation pass. The 113-symbol catalog and <800-line limits
remain intact. This is not integrated or accepted. Decoder production adapter
composition, group-clip resolved-once history, renderer output, tile-cache and
whole-renderer/CLI/oracle gates remain open. See
[`artifacts/WO-RUST-SVG-DECODE-01/current-continuation-7/`](artifacts/WO-RUST-SVG-DECODE-01/current-continuation-7/).


## WO-RUST-SVG-DECODE-01 R2 continuation 8 — historical checkpoint — 2026-10-15

R8 is the latest decoder continuation recorded in the ledger. Its isolated
candidate remains incomplete and unintegrated; the main decoder crate still has
only its library entry point. The separate engine-adapter R2 candidate is also
incomplete and unintegrated. No verified continuation-9 artifact is present in
the current artifact directory or ledger, so no R9 test results are claimed.



## WO-RUST-SVG-ENGINE-ADAPTER-01 R2 — initial run checkpoint — 2026-10-15

The R1 attempt was superseded before edits because its candidate overlapped the decoder order. R2 used a separate isolated workspace with 437 exact-hash inputs: current accepted Rust sources and the terminal R8 decoder candidate. Its initial candidate run has since finished incomplete; see the current status below.


## WO-RUST-SVG-ENGINE-ADAPTER-01 R2 — incomplete isolated candidate — 2026-10-15

The isolated engine candidate's media-offset correction is independently reviewed;
locked/offline engine and decoder checks, five engine integration tests, and
formatting pass. The report now records final source hashes, and coordinator
post-fix log hashes match. The clip-definition case still reports an observed
`InvalidArgument` without proving a successful `DefinitionPlanner` route or its
failure cause. Full decoder acceptance gates also remain open. No candidate code
is integrated into the main workspace. Evidence: [`artifacts/WO-RUST-SVG-ENGINE-ADAPTER-01/current-continuation-1/`](artifacts/WO-RUST-SVG-ENGINE-ADAPTER-01/current-continuation-1/).

## WO-RUST-V1-ESTIMATE-UPDATE-01 accepted — 2026-10-15

Following the owner's request for a shorter first delivery, V1 now describes a
single small-character path: basic drawing and a disclosed SVG subset, one short
loop, editable `.vexel` save/reopen, preset GIF export, and visible authorized
Codex MCP edits to the same open project. Five feature estimates sum to
10–17 focused engineer-days, with the unaccepted SVG decoder/composition gate
called out as the main uncertainty. The full application goal, final gates and
post-acceptance public demo remain unchanged. Work-order, ledger, whitespace,
estimate arithmetic and scope audit pass; no app/UI/E2E tests ran. Evidence:
[`artifacts/WO-RUST-V1-ESTIMATE-UPDATE-01/`](artifacts/WO-RUST-V1-ESTIMATE-UPDATE-01/).

## WO-RUST-SVG-DEFINITION-VIEWS-01 — planner integrated; engine candidate remains pending — 2026-10-15

The reviewed `vexel-svg-definitions` planner and expansion helper now query the
active traversal view. Fresh main-tree check, 36 package tests, and formatting
pass. The isolated actual Decoder clip test reaches charged Progress, Complete,
Drawable finish, and confirms one published clip attached to and resolving from
the owner shape. Independent review passed; the coordinator audit verified all
436 immutable candidate inputs. The engine crate/test remain candidate-only
until the decoder prerequisite is accepted and current-root compatibility is
verified. Evidence: [`artifacts/WO-RUST-SVG-DEFINITION-VIEWS-01/`](artifacts/WO-RUST-SVG-DEFINITION-VIEWS-01/).

## WO-RUST-SVG-PATH-NUMBER-SEPARATOR-01 — revision 2 accepted — 2026-10-15

Revision 2 was independently reviewed and integrated by exact candidate hashes.
The parser now rejects the malformed integer-leading repeated decimal through
direct, controlled and quantum-limited entry points at absolute offset 13,
preserves `.6.5` and the compact arc endpoint fixture, and passes the frozen
parity suite. Fresh main verification passes all 37 package tests, check,
format, work-order and ledger validation. The candidate's out-of-scope
`parser/commands.rs` and `parser/tests.rs` formatting differences were excluded.
Evidence: [`artifacts/WO-RUST-SVG-PATH-NUMBER-SEPARATOR-01/`](artifacts/WO-RUST-SVG-PATH-NUMBER-SEPARATOR-01/).

2026-10-15 coordinator update: WO-RUST-UI-DRAW-01 R2 repairs are integrated.
The exact reviewed production files match their candidate hashes; all five
previously accepted widget regression tests missing from main were restored,
and the new clipped-edge test was added. A hash-matched isolated package workspace passed check and all 18 UI unit tests.
Then the exact root-workspace check and 18-test package run passed with
`CARGO_HOME="$PWD/.cache/cargo"`; the exact locked `objc2-image-io` archive/index
was already present locally. No network or dependency change was needed. Evidence
is recorded in
[`artifacts/WO-RUST-UI-DRAW-01/coordinator-main-verification-r2.txt`](artifacts/WO-RUST-UI-DRAW-01/coordinator-main-verification-r2.txt).
No broad UI/E2E campaign was run. A separate current-source engine reseed passes
all six engine tests but exposes a real SVG path-number mismatch in the decoder;
`WO-RUST-SVG-PATH-COMPACT-OPERANDS-01` is now ready for a focused repair.

2026-10-15 engine reseed review: WO-RUST-SVG-ENGINE-ADAPTER-01 R2 passes scoped
Sol Medium review; no adapter blocker was found. Independent review confirms the
actual planner clip route and all 406 candidate source audit hashes. The remaining
Decoder failure is the frozen-valid `L1.2.4` compact-operand parser regression at
absolute offset 42, reproduced without engine operations and now owned by
WO-RUST-SVG-PATH-COMPACT-OPERANDS-01. The engine candidate remains isolated until
that prerequisite is accepted and its exact raw focused checks are rerun.

## WO-RUST-SVG-PATH-COMPACT-OPERANDS-01 — accepted/integrated — 2026-10-15

The reviewed parser fix accepts the frozen-C-valid `M-.4.6L1.2.4Z` form while
preserving the malformed-number offsets, implicit-group rejection, and the
legacy compact arc endpoint/repeat exception. Exact candidate hashes are
integrated. Main `vexel-svg-path` check and all 40 focused tests pass; independent
Sol Medium review passes. The real decoder regression passes in the isolated
decoder candidate with full-source offsets. The current main decoder package
does not yet expose that integration-test target, so broader decoder composition,
renderer, native app/UI, MCP runtime, full product QA, release, and demo remain
open. No broad UI/E2E run was made. Evidence:
[`artifacts/WO-RUST-SVG-PATH-COMPACT-OPERANDS-01/`](artifacts/WO-RUST-SVG-PATH-COMPACT-OPERANDS-01/).
Next parser-dependent step is to rerun the isolated engine-adapter checks against
the accepted path sources; keep that adapter candidate-only until the decoder
prerequisite itself is accepted and fresh main checks are possible.

## Current continuation — 2026-10-15

R4 refreshes the isolated engine adapter against decoder continuation 9's two
new tests. Decoder production source is byte-identical across the R3/R9 inputs;
the refreshed candidate has 90 passing decoder tests and 6 passing engine
integration tests. The earlier Cargo resolution issue was resolved by using the
existing repository `.cache/cargo`, with no network or dependency changes.
R4 candidate evidence and hashes are under
`.cache/WO-RUST-SVG-ENGINE-ADAPTER-01/reseed-r4/`. Decoder R9 remains incomplete,
so the engine+decoder pair is not integrated or accepted.

Independent decoder review confirms the exact tile-cache C oracle remains open.
`WO-RUST-TILE-CACHE-ORACLE-01` tracks all 29 assertions in a frozen scope. Its
isolated Luna candidate maps and passes every assertion, including 10,000
ownership-checked inserts and an exact JSON-summary match; 16 package tests and
all candidate validations pass. Independent review and fresh main checks remain.
The media candidate has one focused cursor-overflow rollback regression in
progress. Root ledger validation passes at 1,780 entries. These lanes do not
change the full application finish line.


## Parallel Rust follow-ups — 2026-10-15

`WO-RUST-TILE-CACHE-ORACLE-01` is accepted in the main tree after independent
review, exact 29-assertion C-oracle mapping, refreshed hash audit and fresh
locked/offline package verification. `vexel-tile-cache` passes 3 unit + 13
integration tests; scoped formatting, work-order validation and `git diff
--check` pass. Evidence: [`artifacts/WO-RUST-TILE-CACHE-ORACLE-01/`](artifacts/WO-RUST-TILE-CACHE-ORACLE-01/).

The already accepted `WO-RUST-JSON-01` and `WO-RUST-SPATIAL-01` slices were
rechecked in isolated worktrees with independent reviews. JSON passes 5 focused
integration tests; spatial passes 1 unit + 8 integration tests. Their existing
main-tree acceptances remain in the ledger; these checks add no new integration.
The reviewed media candidate is now reconciled into the main workspace, with
the previous untracked crate preserved under
[`artifacts/WO-RUST-SVG-MEDIA-01/pre-integration-root-crate/`](artifacts/WO-RUST-SVG-MEDIA-01/pre-integration-root-crate/).
The main package passes its locked/offline check and all 70 tests. This accepts
the media engine slice only; decoder, renderer, native provider, application,
UI/E2E, packaging, release, and demo gates remain open.

## MCP stdio transport accepted — 2026-10-15

Accepted `WO-RUST-MCP-STDIO-01` after independent review, exact source/scope
audit, and fresh current-root verification. The new `vexel-mcp-stdio` crate
provides bounded caller-owned sync/async-backend line transport around the
accepted MCP protocol. It passes 8 focused tests, including async backend
dispatch and a maximum-size JSON payload with CRLF split across three reads;
check, format, work-order validation, ledger validation, and `git diff --check`
pass. No new external dependency was added. Evidence and logs are in
[`artifacts/WO-RUST-MCP-STDIO-01/`](artifacts/WO-RUST-MCP-STDIO-01/).

This is transport only: it does not connect the application to an MCP backend,
launch stdio, or prove live Codex/Claude Code interoperability. The decoder
remains under implementation; the engine candidate must be reseeded against
accepted decoder sources because its current R5 seed does not match the frozen
work order. The app shell still has no editable document/render/save workflow.
Full UI/E2E, packaging, release, and the recorded agent-driven demo remain open.

## 2026-10-15 — V1 document session and project loading accepted

Accepted `vexel-v1-document`: bounded blank SVG creation, normalized rectangle
insertion through the source editor, source-based undo/redo, save state, and
fixed-buffer coordinate formatting. Independent review passed; fresh root
locked/offline check, all 8 focused tests, format, frozen work-order, source,
ledger, and whitespace checks pass using the existing repository-local
`.cache/cargo` cache. Smallest-subnormal round-trip is covered.

Accepted `vexel-project-io` bounded loading: regular-file and size validation,
bounded reads, complete bundle decoding, and owner-tied borrowed views.
Independent review and fresh locked/offline root check plus all 16 package tests
pass; seven tests cover loading. The first attempt used the default Cargo home
and could not resolve `objc2-image-io`; rerunning with the already-present
`.cache/cargo` resolved it offline without network or dependency changes.

These slices do not connect the app shell to an editable canvas. The next V1
path is production SVG engine composition, then primary-pointer gesture capture,
canvas/document coordinate mapping, `V1Document` updates, and actual raster
presentation. Evidence:
[`artifacts/WO-RUST-V1-DOCUMENT-SESSION-01/`](artifacts/WO-RUST-V1-DOCUMENT-SESSION-01/)
and [`artifacts/WO-RUST-PROJECT-IO-LOAD-01/`](artifacts/WO-RUST-PROJECT-IO-LOAD-01/).

## 2026-10-15 — V1 ellipse editing accepted

Accepted `V1Document::add_ellipse` after independent review caught and closed
a subnormal-span underflow that could otherwise serialize a zero radius. The
API validates finite in-bounds drag geometry and color, normalizes all four
directions, inserts one exact `<ellipse>` source edit, and rejects zero-radius
underflow before mutation. Focused X- and Y-axis tests verify source, history,
cursor, and dirty state remain unchanged on failure. Fresh root locked/offline
check, all 11 package tests, format, work-order/ledger validators, and
`git diff --check` pass. Evidence is under
[`artifacts/WO-RUST-V1-DOCUMENT-ELLIPSE-01/`](artifacts/WO-RUST-V1-DOCUMENT-ELLIPSE-01/).

The owner has requested an interim live-MCP preview to make progress visible
within hours. This is an intermediate demonstration milestone and does not
replace the full V1 or product goal. The app shell still has no live document
canvas, and production SVG composition remains the full renderer path. The
preview must disclose its supported shape subset, show accepted MCP edits in
the native window, and reserve stdout for MCP framing.

## 2026-10-15 — SVG decoder continuation 10 reviewed; continuation 11 active

R10 adds the original zero-area linear-gradient predicate with the correct
resolver-before-fallback ordering, source offset, and request rollback. Its
candidate reports 93 decoder tests passing, locked/offline package checks,
format, and frozen-order validation. Focused independent review passes with a
specific limit: the malformed-gradient and PaintStops error cases inject
resolver errors and do not prove production parser/limit parity. The exact
29-assertion tile-cache oracle is already accepted separately; R11 now maps that
accepted evidence without claiming decoder-path parity and is investigating the
remaining crate-local clip-history and composition gates. Decoder acceptance,
engine integration, renderer, app, UI, and final QA remain open. See
`artifacts/WO-RUST-SVG-DECODE-01/current-continuation-10-review.md` and the
isolated R10 evidence under `.cache/WO-RUST-SVG-DECODE-01/current-continuation-10/`.

## 2026-10-15 — MCP live preview implementation integrated

Integrated the opt-in `--mcp-preview` app path. Its preview-only MCP profile
supports listing/inspecting the blank document, adding bounded rectangles and
ellipses, and undo/redo. The native UI owns the document, shows a basic-shape
canvas and recent activity, and the stdio MCP worker waits for accepted frame
presentation before replying. The normal launch path and full MCP profile are
preserved. Setup instructions and a copyable Codex prompt are in
[`docs/product/mcp-live-preview.md`](docs/product/mcp-live-preview.md).

Fresh root locked/offline check passes. Focused package tests pass: app 16/16,
preview MCP profile 4/4, and existing protocol parity 9/9. Changed Rust files
pass scoped rustfmt; work-order and ledger validation and `git diff --check`
pass. Independent review closed the preview schema-required-attributes and
tool-description mismatches. No broad UI test, native window launch, or actual
Codex MCP session was run, so macOS visual runtime remains unverified. The
preview is ready for a first user-run session; it does not replace full V1.
Evidence is under
[`artifacts/WO-RUST-MCP-LIVE-PREVIEW-01/`](artifacts/WO-RUST-MCP-LIVE-PREVIEW-01/).

## 2026-10-15 — first live Codex preview and animation demo direction

The owner and coordinator ran the preview with Codex 0.162.1 over local MCP
stdio. Initialization, tool discovery, document inspection, and eight
successive basic-shape additions completed against one visible Vexel window.
This proves the initial launch and shape-publication path on this Mac; it does
not complete V1 or establish coverage across macOS versions. The owner-provided
screenshot exposed upside-down canvas text. A CoreGraphics text-space y-flip
correction is in `vexel-platform-apple`; focused platform tests (44 unit, 5
native-boundary, 1 native-window, 6 doc tests), the `vexel-app` check, scoped
formatting, and an independent Luna review pass. The existing visible process
uses the old binary, so visual verification of the fix is pending. Evidence is
recorded in `artifacts/WO-RUST-NATIVE-WINDOW-01/text-orientation-followup.md`.
A new headless CoreText pixel test confirms that an asymmetric “L” glyph's
lower stroke remains in the lower half after rasterization; user-visible
preview confirmation still needs a fresh launch.

The preview guide now records the planned Pip character and a live MCP animation
demo: gentle body bob, two hand waves, one blink, and softly twinkling stars.
The current preview is still static. A credible implementation must connect
source-owned shapes, real animation parsing/evaluation, rendered frames, and
bounded MCP playback controls; the app currently has no production SVG
decode/render composition path. The exact next dependency remains the active
decoder/renderer integration and its shared engine contracts. Do not replace
that work with a separate hardcoded animation model. Full animated editing,
SVG interoperability, native project support, performance, final broad UI/E2E,
release, and the README/website recording remain required.

The demo art direction is refined in `docs/product/mcp-live-preview.md`:
Pip is a small paper-cut sky courier, with a deliberate ink/cobalt/apricot/mint/
coral/gold palette and generous title-card spacing. The 3.2-second loop now has
named beats (body bob, two raised-arm waves, lagging scarf, one blink, and
staggered rising glints), with matched loop endpoints and low-key motion. This
is still a design specification; the current preview does not yet animate.

## 2026-10-15 — source-backed live animation preview started

Ready work order `WO-RUST-MCP-LIVE-ANIMATION-01` extends the demo using the
existing SVG animation parser/evaluator and source-owned SMIL tracks. It adds
stable MCP element IDs, bounded animation authoring/playback, and live frame
publication while keeping the preview's renderer limits explicit. Frozen
source hashes are recorded at
`artifacts/WO-RUST-MCP-LIVE-ANIMATION-01/source-seed.json`. Protocol, V1
document, and app candidates are being developed in separate Luna worktrees;
root manifests, lockfile, product docs/state/ledger, and integration remain
coordinator-owned. Full V1 and the final recorded website/README demonstration
remain active requirements.

## 2026-10-16 — re-baseline and repository security

All work since `19234a0` is now committed and pushed (`2898e15`..), tag `legacy-c-final`
marks the last tree with C. Large artifacts and third-party PDFs are excluded
(`artifacts/EXCLUDED-LARGE.txt`). Six blocked C orders and WO-CORPUS-01/02 are
`superseded` (directive 4).

Baseline (this Mac, `CARGO_HOME=.cache/cargo`; default CARGO_HOME fails offline
resolution of `objc2-image-io`):
- `cargo check --workspace --offline`: failed at start on `vexel-app` (non-exhaustive
  `V1DocumentError` match in `preview.rs`, left by the unfinished live-animation
  integration); fixed by mapping the three animation errors. Check now passes.
- `cargo test --workspace --offline --exclude vexel-app`: 170 suites, 1049 passed, 0 failed.
  `cargo test -p vexel-app`: 16 passed.
- `./dev verify` (C, frozen reference): fails at link — `viewer_editor_byte_length`,
  `_can_redo`, `_can_undo`, `_select_at_source_offset`, `_selection_span` are declared in
  `src/app/viewer_editor.h` but defined nowhere. C is frozen; not fixed. Recorded so the C
  tree is not mistaken for green.

## 2026-10-16 — V1 critical path: decode, render, save accepted

Integrated on main after fresh read-only reviews (all ACCEPT-WITH-AMENDMENT, no blockers): WO-RUST-V1-DECODE-SUBSET-01 (`8ea3e78b`; vexel-svg-engine, decoder 93 / engine 38 / definitions 40 tests), WO-RUST-V1-RENDER-01 (`5e67bb50`; vexel-render, 49 tests, byte-exact vs C goldens on 7 fixtures), WO-RUST-V1-SAVE-01 (vexel-app project_session, 48 app+document tests). WO-RUST-SVG-DECODE-01 superseded. Open: native Save/Open panels, filters/mask-on-group/patterns/dash, app composition, animation+GIF, MCP walkthrough.

## 2026-10-16 — V1 slice accepted (automated level)

Accepted after independent reviews and merged on main (`49ef038f`, 1473 workspace tests, ledger 1884 entries): INTEGRATE-01, PLAYBACK-01, MCP-TOOLS-01, WALKTHROUGH-02 (WALKTHROUGH-01 superseded) on top of DECODE-SUBSET, RENDER, SAVE, NATIVE-PANELS, APP-COMPOSE, ANIM, DRAW-TOOLS. Outstanding: live window verification (manual-checklist.md M1-M10 and per-order lists), legacy C removal, deferred SVG features, render_png.

## 2026-10-16 — legacy C removed

Removed the C implementation and C-bound tooling (`27602ed2`; tag `legacy-c-final`). 1473 Rust tests and 50 Python harness tests pass afterwards. Repository: 1378 tracked files (was ~7,900 untracked + tracked sprawl); top level is `rust/`, `docs/`, `work/`, `artifacts/`, `bench/corpus`, `scripts/` (ledger/WO tooling), `tests/` (harness tests).

## 2026-10-17 — post-V1 waves merged; measured corpus on the merged tree

Merged and accepted since the V1 slice: polish, PNG, render dashes, release/CI/fuzz/perf, live fixes, corpus measurement, masks/groups, patterns, filters, text, renderer performance, SVG wiring, large-document cap, unsupported-cause closure (markers, PNG images). 1727+ workspace tests pass; ledger 1954 entries.

Final external sweep (release build, 128x128, 60 s timeout, 2 GiB RSS bound, manifest-hash guarded, one run on one host; verdict counts only, no fidelity claim): **ok 6 of 39 attempted** (legacy C 3/39, earlier C candidate 5/39): `generated-cad_t1`, `generated-embedded_t1`, `generated-giant_path_t1`, `generated-science_t1`, `natural-earth-world-map-blank`, `natural-earth-world-map-countries`. Other verdicts: timeout 12, xml 9, unsupported 5, limit_decode 6, limit_render 1, skipped 19 (>32 MiB), crash 0, rss_exceeded 0. Louisiana ran at the timeout boundary in earlier runs and timed out here. Render itself is a small share of the time on large inputs (compose of 2000 rects: render ~150 ms of ~2 s): decode, scene validation and compose in `vexel-app` dominate, so the next lever is pipeline speed, then `vexel-svg-decode` `use`/reference limits and the `TextCommands` limit in `vexel-scene`.

## 2026-10-17 — UI upgrade merged; final corpus measurement

Merged and accepted (each after a fresh independent review): UI design direction (`docs/ui/design-direction-2026-10.md`, researched from Apple WWDC25 Liquid Glass guidance, Figma UI3, Linear, Blender 5, Rive, dark-UI elevation guidance), LCH token system, 66 first-party icons (+redraw), hover/press/focus motion, artboard canvas with zoom/pan, new chrome (toolbar, rail, sectioned inspector, status bar), Animate timeline (+ follow-ups), polish (fit to 400%, well contrast, empty states, hover and tooltips), toolbar wiring (Undo/Redo/Save/Export with GIF/SVG/PNG popover), pipeline performance (incremental scene validation), scene limits. Screenshots in `docs/archive/live/`.

Final external corpus sweep (release, 128x128, 60 s timeout, manifest-hash guarded, one run on one host; verdict counts only): **ok 11 of 39 attempted** (legacy C 3/39), timeout 0, xml 10, unsupported 8, limit_decode 8, limit_render 2, skipped 19 (>32 MiB), crash 0, rss_exceeded 0. Remaining causes: XML/viewport/path-data parser strictness (10), unsupported constructs such as `<translate>`/`<circle>`-in-context/`<use>` ordering/tspan quirks/`font-size:11px` tokens (8), `use`/shape/text limits in protected `vexel-scene` (10).

2026-10-17: UI polish round two (no formal order; evidence in `artifacts/WO-RUST-UI-POLISH-02/`): Export popover anchored under the button with surface[5], edge and shadow; 6 px footer scroll thumb with hover; accent elapsed fill and 3 px thumb on the Draw strip; eyedropper, group and node_edit icons redrawn. 457 UI tests pass.

2026-10-17 (final): after the SVG grammar fixes (decode finalize bug, XML limits labelled as limits, path compact-dot grammar per the SVG spec) the external sweep reads **ok 12 of 39** (legacy C 3/39), timeout 0, unsupported 9, limit_decode 12, limit_render 3, xml 3, skipped 19, crash 0, rss_exceeded 0. Most remaining failures are honest limits (65,536 shape clamp and text-command caps in protected `vexel-scene`) or unsupported constructs; root width/height above 4096 and DOCTYPE internal entities are deliberately still rejected (orders deferred).

2026-10-17: first-party app icon added (`resources/AppIcon.icns`, generated by `scripts/make_icon.py`; `vexel-dev bundle` copies it and writes `CFBundleIconFile`). It is a draft for owner brand review; the bundle id `app.vexel.Vexel` remains a placeholder and Developer ID signing/notarization remain owner-gated. Live checks are queued in `docs/ui/live-check.md` (the Mac screen was locked, so automated clicking could not reach the app).
