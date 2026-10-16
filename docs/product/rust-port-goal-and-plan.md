# Vexel: full Rust port and product delivery

## Active repository goal

**Deliver the full Vexel Rust application, fully completed, comprehensively
tested against the agreed product requirements, packaged, documented and ready
for real use on Apple Silicon macOS 13+.** This is the final goal, explicitly
confirmed by the owner on 2026-10-14.

Port all first-party code and in-repo tooling, preserving behavior and supported
file formats, and complete the entire ADR0018 vector-editing/cartoon-animation
product: modern custom UI; Inkscape SVG interchange and source preservation;
native artwork/animation/assets/recovery; measured large-file responsiveness;
live visible user-controlled Codex/Claude MCP; accepted release; recorded
character+animation examples for the website and README.

Owner direction2026-10-14 supersedes C17 delivery and the old two-writer ceiling.
Legacy C checkout and all worker candidates/evidence are preserved. No finished
Rust port or performance gain is claimed. The owner resumed the tracker and the latest API read on2026-10-15
confirms **active**. Earlier usageLimited observations are historical. No billing
change or additional-credit purchase was made.
The tracker retains its language-neutral product objective. The available API
cannot edit an unfinished goal's objective; the full canonical wording below
and this outcome contract govern execution alongside OWNER-DIRECTIVES. Earlier
replacement failures remain historical evidence. No false completion is used.

## Canonical full goal — owner update 2026-10-14

Deliver Vexel as a complete, high-quality Rust vector drawing and 2D cartoon animation application for Apple Silicon macOS 13+, fully implemented, integrated, comprehensively tested, packaged, documented, and ready for real use. Port every first-party application, platform, UI, XML/SVG parser/rasterizer, geometry, scene, document, project, animation, MCP, development, test, benchmark, and release component to Rust while preserving migration references and frozen acceptance evidence. Provide an original modern custom interface informed by Blender and other modern drawing/animation tools; complete vector editing and cartoon animation workflows; faithful source-preserving Inkscape SVG import/export; versioned native projects containing artwork, animation, assets, metadata, and recovery; and measured high performance on large files, including memory, build time, and release bundle size. Provide detailed discoverable, authorized, live and visible MCP control for Codex, Claude Code, and compatible agents, including drawing, editing, animation, undo/redo, save/export, progress, errors, and cancellation. After feature completion, pass comprehensive UI/end-to-end, fidelity, performance, accessibility, malformed-input, recovery, MCP interoperability, and packaged fresh-install acceptance, fix release blockers, and deliver release artifacts and documentation. Record authentic Codex prompts creating a character, revising the drawing, animating it, and revising that animation through successive real MCP calls while the same Vexel project visibly updates live, with reusable projects, prompts, exports, captions/transcript, and accepted website/README presentation. Preserve macOS-only initial support, MPL-2.0, owned SVG/XML/raster code, minimal audited pinned dependencies, no GPL editor source or external SVG fetching, and safe ownership/resource limits. Use useful disjoint parallel Luna Codex exec workers at task-appropriate effort and Standard speed within the existing subscription allowance, with named escalations only; keep focused correctness checks during implementation and reserve broad automated UI/E2E for the final stage. Completion requires all agreed product outcomes and release evidence, not foundation tests or partial functionality.

The machine-readable objective and all nine outcome/seven completion gates are
in [`full-goal.json`](../../artifacts/WO-RUST-GOAL-06/full-goal.json).
This updates the repository's authoritative full scope; it does not claim the
stored tracker objective was changed through an unavailable operation.


## Detailed outcome contract — 2026-10-14

**Build a complete Rust vector editor and 2D cartoon animation application for
Apple Silicon macOS 13+, operated by a person or by an authorized Codex/Claude
Code session with edits visible in real time.** Both the migration and the
finished product are required. ADR0018 governs product quality; ADR0022
supersedes its historical C17 clause.

### Required deliverables and completion evidence

| Area | Required outcome | Evidence needed for completion |
|---|---|---|
| Complete Rust migration | Port application, platform, custom UI, XML/SVG, geometry, scene/raster, documents, projects, animation, MCP and first-party development/test/benchmark/report/release tooling. | Per-module parity maps, accepted work orders, integration checks, closed inventory and a Rust-built release using the Rust workflow. |
| Vector editor | Coherent drawing, path/node editing, shapes, text, selection, groups/layers, styles, transforms, alignment/snapping, supported masks/clips/gradients/effects, zoom/pan and undo/redo. | Representative complete create/edit/save/reopen/export workflows; documented exact tool/feature coverage and recoverable errors. |
| SVG and Inkscape interchange | Owned bounded parsing/rendering; preserve namespaces, editor metadata, unsupported constructs and unknown source whenever an edit can retain them. Report approximations and destructive conversion explicitly. | Full-document corpus outcomes, declared render tolerances, preservation checks and documented import/export limitations; no silent content loss. |
| Native `.vexel` projects | Versioned editable artwork, timeline/animation, assets and metadata; schema compatibility/migration, safe save publication, interrupted-save handling and recovery. | Reopen projects intact; asset and animation integrity; malformed-input, compatibility and recovery evidence. |
| Cartoon animation | Property keyframes/interpolation, frame-by-frame drawings/exposures, timeline editing, onion skinning, deterministic playback and export. Undoable, persistent and controllable through MCP. | A character can be drawn, animated, previewed, saved/reopened and exported with verified appearance and frame timing. |
| Modern original UI | Task-focused drawing/animation workspaces, flexible editor areas, contextual inspector, layer controls, timeline and clear progress/errors/cancel. Apply useful Blender/Figma/animation-tool ideas within macOS conventions. | Integrated workflow review, complete keyboard operation, accessibility semantics, readable contrast, supported display scales and representative-user review. |
| Large-file performance | Measure and optimize loading, first visible content, rendering, pan/zoom, owner-thread pauses, memory/RSS, allocations, cancellation, bundle size and build time. | Named corpus/hardware/build, cold/warm protocol, repeated samples and raw results; bounded resource failures; accepted budgets met. |
| Detailed live MCP | Discoverable schemas and tools for live scene/objects, selection/style/transforms, undoable edits, animation, project save/export and rendering; visible activity/progress/results and bounded cancellation. | Real Codex, Claude Code and protocol-client sessions operating the same document shown in the Rust app; protocol, permission and error-recovery evidence. |
| Release and recorded demos | Complete final acceptance and release artifacts, then record authentic Codex character creation and animation with Vexel updating live; deliver the example through the website and README. | Passed quality gates, reviewed footage, captions/transcript, reusable project/prompts/exports and working website/README links or embeds. |

### Migration and preservation requirements

Account for all 603 legacy code files in the baseline and subsequent first-party
additions: accepted Rust replacement, pending port or explicitly justified
historical-data retirement. No unexplained omissions or shipped first-party
C/Objective-C backend hidden behind Rust. OS frameworks remain external platform
services; retained shader assets must be accounted for explicitly.

Preserve legacy sources and candidate evidence, original fixtures/assertions,
strict limits, diagnostics, file versions and source bytes. Record deliberate
safe Rust API adaptations and behavior changes separately. Inventory closure
does not authorize deleting current work or weakening acceptance oracles.
Existing Python/C tooling remains historical until accepted Rust replacements
become authoritative through dedicated work orders.

Define exact editing, rendering and export coverage before implementation in
ready work orders. The aim is a serious Inkscape alternative; blanket feature
parity requires a coverage matrix and evidence. Keep SVG SMIL/CSS preservation
distinct from native animation authoring where their semantics differ. The
accepted bundle codec is a foundation, not the completed project workflow.

### Performance targets

ADR0018 provisional targets must be rebaselined on named Apple Silicon hardware:

| Metric | Target / evidence |
|---|---|
| First visible content | Under 2 seconds for a representative 100 MB document |
| Main-thread pauses | Under 100 ms |
| Pan/zoom latency | p95 below 33 ms |
| Memory, release bundle size and build time | Measured legacy baseline and explicit regression budgets in performance work orders |
| Resource pressure and cancellation | Bounded, explicit and recoverable outcomes |

These are targets, not achieved results. Rust alone does not guarantee faster
builds, smaller bundles or lower memory use. Each measurement records file hash,
hardware/OS/build, method, repetitions, warm/cold conditions and raw results.
Changing a product target requires a recorded owner decision.

### Live control and demonstration requirements

Use local stdio and explicit file grants by default. Agent operations must obey
user access boundaries, preserve document integrity, be visible in the app and
form coherent undoable transactions. Background jobs show progress/errors and
cancel within documented bounds. Remote transport needs a separately approved
authenticated threat model. The application never fetches external SVG resources.
Mock-backend protocol tests alone cannot establish live integration completion.

After full application acceptance, deliver a video of an authentic Codex
conversation controlling the same live Vexel project through MCP. The recorded
sequence must show:

1. Type a prompt to draw a specific character; show Vexel progressively drawing
   it during successive MCP calls.
2. Type a follow-up to revise the drawing; show those edits appearing live.
3. Type a prompt to animate the character; show animation being authored live
   and the resulting playback in Vexel.
4. Type a follow-up to revise the animation; show the timeline and playback
   updating as Codex iterates through MCP calls.
5. Save and reopen the editable native project and show the exported animation.

Keep the real conversation and application visible together, with enough call
and timing evidence to establish that the changes came from actual MCP
execution. Retain the original recording and privacy-reviewed MCP evidence;
clearly label any cuts or speed changes in the published edit. The showcase
must demonstrate live operation, rather than rely solely on finished artwork.
Codex is the required recorded example; the same drawing, editing and animation
capabilities must work through Claude Code and other compatible MCP agents,
using discoverable tools and the same permissions and document transactions.

Retain the example native project, prompts/instructions and exported assets for
reproduction. Review footage for clarity, privacy and accurate claims; provide
captions or an accessible transcript and a useful preview. Publish the reviewed
video on the product website and link/embed it in the README. This is a required
completion gate, with recording and publication still pending.

### Execution, allowance and acceptance policy

- Run as many useful actual Luna `codex exec` processes as independent ready
  scopes and subscription/host resources allow. Six is the current batch size,
  not a permanent ceiling or a reason to duplicate work.
- Keep one writer per module in an isolated current-source worktree with frozen
  contracts/allowed paths and separate local build/cache/output. Coordinator
  alone owns shared Cargo manifests, lockfile, interfaces and ledgers.
- Use Luna Low for scoped inventory/docs/routine work, Medium for routine ports
  and focused cross-file work, High for strict parsers, ownership/concurrency,
  rendering and unsafe FFI. Escalate a named difficult problem to Sol Medium or
  Astra High only when it reduces expected rework. No recursive worker spawning.
- Use Standard service and existing subscription allowance; no purchased
  credits or API billing switch. Keep contexts/reports narrow and reuse sessions
  for repairs. Record observed CLI usage; token logs do not expose dollar cost
  or remaining allowance. Stop rather than loop on allowance/rate errors.
- During implementation use only focused work-order correctness checks and
  build/integration/ledger checks. Defer comprehensive automated UI/E2E until
  feature completion, then cover fidelity, performance, accessibility,
  permissions/cancellation, recovery and release acceptance.
- A successful worker exit creates a candidate. Accept only after independent
  review, exact scope/source-preservation audit, named main-tree checks and
  recorded evidence. Scaffolding and foundation tests do not prove app parity.
- Keep domain Rust safe; unsafe belongs in small audited platform/FFI modules
  with documented ABI, ownership, thread affinity, lifetime and teardown.

### Definition of done

The final application is **completed, tested and ready to use** only when all
of the following are evidenced:

1. **Full scope delivered.** All nine deliverable areas and agreed feature
   coverage are implemented and integrated. The migration inventory is closed;
   application and first-party tooling use Rust. Required production workflows
   contain no unfinished placeholders or mock backends.
2. **Complete user workflows work.** A user can launch, create/edit vector art,
   import supported Inkscape SVG, animate a character, undo/redo, save/reopen
   projects with assets and animation intact, recover work and export results.
   Actual Codex and Claude Code sessions drive the live app visibly and safely.
3. **Comprehensive final testing passes.** After feature completion, run the
   supported unit/integration/regression checks and full automated UI/E2E pass,
   SVG fidelity/source-preservation corpus, malformed-input/property/fuzz checks
   where specified, project compatibility/recovery, animation appearance/timing,
   MCP interoperability/permissions/progress/cancellation and release checks.
   Record coverage and actual results; skipped or blocked required checks cannot
   count as passes. Preserve frozen assertions and oracles.
4. **Product quality is accepted.** Measured large-file responsiveness and
   memory budgets pass on named hardware. Keyboard/accessibility/display-scale
   and representative-user reviews pass. Fix and rerun affected checks for all
   release-blocking findings, including crashes, corruption/data loss, broken
   required workflows, permission failures and unmet acceptance gates. Document
   remaining nonblocking limitations with evidence; do not claim zero possible
   defects or unlimited SVG compatibility.
5. **The packaged product is usable.** Verify the intended release artifact on
   a clean installation/profile for supported macOS: install, launch, open/save,
   animate/export and connect the supported MCP clients without a developer
   checkout. Validate required platform permissions and distribution behavior
   for the chosen release channel. Deliver versioned artifacts, installation/
   getting-started guidance, supported-feature/format documentation, live-agent
   setup instructions, example projects and release notes.
6. **Showcase delivered.** Review an authentic live Codex recording covering
   character creation, drawing revision, animation creation and animation
   revision through successive MCP calls, with the same Vexel project visibly
   updating. Include playback, save/reopen, exported animation, accessible
   captions/transcript and reusable project/prompts/exports; publish/link the
   accepted video through both website and README.
7. **Final evidence reconciled.** Complete release requirements/coverage matrix,
   accepted work orders, inventory, test/performance results, documented
   limitations and release checklist agree with the shipped version. Completion
   is declared only against that evidence, never from plans, worker exits,
   foundation tests or partially usable functionality.

Focused correctness checks continue during implementation. The comprehensive
UI/E2E campaign remains the final implementation gate, followed by fixing its
findings, release verification and recorded demonstrations. Preserve macOS 13+
Apple Silicon, MPL-2.0, custom UI, owned SVG/XML/raster, minimal approved
dependencies and no GPL editor source.

## First usable V1 milestone

The decision and cutline are defined by [ADR-0029](../adr/0029-v1-vertical-slice-cutline.md)
and [the V1 workflow](vexel-v1-vertical-slice.md). V1 demonstrates one small
character from drawing or a disclosed basic SVG import through revision, one
short animation loop, editable `.vexel` save/reopen, preset GIF export and live,
authorized Codex MCP control of the same open project. Vexel visibly reports
agent activity and edits.

This is an early milestone, not the definition of done. Broad Inkscape fidelity,
advanced editing, project recovery/compatibility, full animation, large-file
performance, multi-client MCP verification, comprehensive final UI/E2E and
release acceptance remain required. The authentic Codex demo video for the
README and website follows full-product acceptance. Current V1 estimate:
10–17 focused engineer-days, with the unaccepted SVG decoder/composition gate as
the main uncertainty.

## Consolidated delivery execution — 2026-10-14

This is one continuous goal: use the work already performed to deliver the
complete tested Rust product defined above. Accepted foundation work remains
part of the deliverable; completed isolated candidates enter the integration
gate below. The research and previous worker runs are not a separate project.

Authoritative execution snapshot:
[`delivery-state.json`](../../artifacts/WO-RUST-GOAL-04/delivery-state.json).
It records the current ledger lifecycle, worker evidence paths/hashes and
scope-specific checks. Later accepted work orders update execution checkpoints;
they do not change the release finish line.

### Already accepted in the main working tree

| Work | Actual accepted scope | Contribution to final product |
|---|---|---|
| Research and Cargo bootstrap | Complete legacy inventory, dependency waves, std-only macOS Rust workspace and shared contracts | Migration accounting and build foundation |
| `WO-RUST-BASE-01` | Bounded safe arena/byte/slice utilities; 7 focused tests | Shared ownership/resource controls |
| `WO-RUST-GEOMETRY-01` | Named affine/point/bounds operations; 5 focused tests | Editing and rendering geometry |
| `WO-RUST-XML-01` | XML characters and reader; 10 focused tests | Source-preserving parser foundation |
| `WO-RUST-JSON-01` | Owned bounded JSON core; 5 focused tests | Protocol and structured project/tool data |
| `WO-RUST-PROJECT-01` | Strict native bundle-v1 codec; 8 focused tests | Project container integrity |
| `WO-RUST-RASTER-01` | RGBA tile storage; 5 focused tests | Rendering/UI pixel storage |
| `WO-RUST-WAVE2-SETUP-01` | Shared registrations and native FFI policy only | Contracts for the six candidate ports |

The six domain foundations total **40 accepted focused Rust tests**. Do not add
bootstrap Span tests again or treat this as complete product testing. Accepted
here means reviewed and verified in the uncommitted main working tree.

### Second-batch candidates at consolidation (historical snapshot)

At consolidation, all six actual Luna High CLI workers had exited 0. Their
initial scoped checks below were worker-reported, not main-tree acceptance.
The resumed checkpoint at the end records subsequent repairs and acceptance.

| Work order / candidate | Worker-reported checks | Required integration review / downstream use |
|---|---|---|
| `WO-RUST-XML-02` — tokenizer/DOCTYPE/stream APIs | 17 tests, including the accepted reader tests | Review exact limits/diagnostics/cancel and streaming parity. Worker reports scan-then-parse and retained visitor tokens; prove the bounded processing/storage contract or repair it before acceptance. Enables XML index/document/SVG decode. |
| `WO-RUST-SPATIAL-01` — immutable spatial index | 7 integration + 1 sizing test | Review finite/inclusive bounds, required-capacity accounting, no partial output and concurrency. Enables picking/culling; does not establish million-object performance. |
| `WO-RUST-SVG-VALUES-01` — numbers/lengths/transforms/viewports | 12 tests | Review strict grammar, source spans, diagnostics, unit/default/transform parity and attribute adaptation. Enables paths/shapes/scene mapping, not full SVG rendering. |
| `WO-RUST-PROTOCOL-01` — owned sync/async MCP protocol | 9 tests | Review envelopes, schemas, negotiation, Tasks/progress/cancel and safe backend contracts. Needs real transports, task jobs, owner bridge and clients. |
| `WO-RUST-UI-CORE-01` — theme/tree/layout/focus/commands | 7 tests | Review core behavior, bounds and failure atomicity. Needs widgets/draw/icons/accessibility, native presentation and real editor flows. |
| `WO-RUST-NATIVE-SYSTEM-01` — entropy/UID/peer credentials | 2 unit + 6 integration tests | Audit native ABI and every unsafe boundary, failure zeroing and borrowed-FD lifetime. Needs remaining filesystem/tasks/native AppKit presentation; does not implement a desktop shell. |

Do not sum candidate counts into a unique integrated/app count: XML includes
existing reader tests, and every candidate still requires acceptance. Detailed
candidate results, verification records and parity maps are linked from the
execution snapshot. Preserve all original runs and narrow repair evidence.

### Immediate gate: close the current six candidates

1. Independently review each frozen scope against legacy source, original
   assertions and its parity map. Keep reviews disjoint and focused; classify
   reported adaptations before accepting them as parity.
2. Repair concrete findings in the original worker session where practical;
   run named scoped regressions and obtain a targeted independent recheck.
3. Audit every changed/new file against the worker baseline, allowed paths and
   current main state. Preserve legacy source, fixtures, manifests and other
   workers' changes; coordinator handles shared conflicts/contracts.
4. Integrate only accepted candidate changes. Run the named main-tree package
   build/check/tests and work-order/ledger validation. Record actual results,
   usage and `run_finished -> verified -> accepted` lifecycle transitions.
5. Freeze the next dependency-ready scopes. Candidate completion alone must not
   unlock consumers of unaccepted or incompatible APIs.

### Remaining parallel lanes and dependencies

Launch only ready disjoint scopes with one writer per module; split a lane into
independent work orders when contracts permit. Shared manifest/API changes and
application composition remain serialized by the coordinator.

| Lane | Prerequisites | Remaining deliverables |
|---|---|---|
| XML/document/scene | Accepted tokenizer and base/geometry | XML index, source-preserving edits/history, selection/object/layer scene state and persistence integration |
| SVG/renderer | Accepted XML/values/geometry/tile; spatial where needed | Paths/shapes, CSS/references/paint/effects/media, scene traversal and owned raster/render pipelines; fidelity/unsupported coverage |
| Projects/animation | Accepted bundle + document/scene contracts | Native editable state, assets/timeline, compatibility/recovery, keyframes/exposures/onion skin/playback and export jobs |
| Native platform/custom UI | Accepted core/system services; frozen scene/command contracts | Filesystem/thread/task services, Apple presentation/input/text, widgets/draw/icons, accessibility, inspectors/workspaces/timeline |
| MCP/jobs/live bridge | Accepted protocol/system plus document/project/job contracts | Stdio/socket transport, task lifecycle and cancellation, snapshots/background work, permissions, undoable live owner bridge and complete tools |
| Rust tooling/release | Stable Cargo/work-order/evidence contracts | Developer/validator/test/benchmark/report/packaging replacements; inventory and provenance accounting; reproducible macOS release |

Do not begin by duplicating completed foundations. Carry forward historical C
work, including async MCP/export candidates, as behavior/evidence references
under their recorded lifecycle. Port through new Rust orders; never revive the
stopped C implementation queue or treat an unaccepted C candidate as an oracle.

### Product requirement coverage and milestones

| Final requirement | Implementing lanes / closure gate |
|---|---|
| All-first-party Rust | All lanes plus tooling; per-file migration inventory closure |
| Complete vector editor | Document/scene + SVG/renderer + UI; actual editable workflows |
| Inkscape/SVG fidelity and preservation | XML/document + SVG/renderer; full-document coverage/corpus and round trips |
| Native projects and recovery | Document + projects + platform; safe save/reopen/migration/recovery |
| Cartoon animation | Scene + projects/animation + timeline UI + exports; appearance/timing and undo/persistence |
| Modern accessible UI | Native platform/UI plus editor composition; keyboard/display/accessibility/user review |
| Large-file performance | Parser/index/renderer/jobs/UI; measured corpus and fixed responsiveness/memory budgets |
| Live Codex/Claude MCP | Protocol/transport/jobs + owner bridge + document/animation; real-client visible sessions |
| Tested release and recorded demos | Tooling/composition + every quality gate; packaged installation and website/README showcase |

1. **Accepted baseline:** research, workspace and first foundations are complete
   in their scopes. The full app is still incomplete.
2. **Current batch accepted:** close the six candidates through the immediate
   review/integration gate, including any XML streaming repair.
3. **Subsystems delivered:** execute the ready parallel lanes above, incrementally
   integrate and retain focused correctness checks. Define exact supported
   tools/formats, memory budgets and remaining native architecture decisions
   in frozen orders/ADRs before implementation.
4. **Feature-complete integrated app:** finish remaining product gaps and Rust
   tooling; reconcile requirements, inventory and usable end-to-end workflows.
5. **Fully tested release candidate:** run the comprehensive final acceptance
   plan, fix release blockers and rerun affected checks. Missing/skipped/blocked
   required results remain open; broad UI/E2E starts at this stage.
6. **Ready-to-use release:** verify the packaged fresh-install app, supported
   MCP setup, documentation, examples and distribution behavior.
7. **Final goal achieved:** deliver the reviewed live Codex character-animation
   recordings and reusable assets on website/README, then reconcile all evidence
   against the shipped version and declare completion.

Use the effort/allowance policy above throughout. Estimates remain provisional;
worker elapsed time and token usage do not establish whole-product delivery time
or remaining subscription capacity. No extra credits or API billing are used.

## Initial execution (historical)

Six read-only Luna codex exec research lanes: build/toolchain/tools; macOS/UI;
owned XML/SVG/raster; document/project/animation/recovery; MCP/concurrency;
complete inventory/dependency/parallel sequencing. Use subscription auth and
Standard service; no API billing switch or credit purchase. Each worker gets a
thin scope and bounded report, no recursive agents. After synthesis, freeze
crate interfaces, dependency/provenance decisions and separate Rust work orders,
then launch as many disjoint ready implementation lanes as resources/allowance
permit. One writer per module; one coordinator for Cargo/lock/interface changes.

## Migration gates

1. Inventory every first-party code file and map ownership/behavior/test evidence.
2. Establish macOS-only Cargo workspace, toolchain, platform/FFI safety policy.
3. Port independent base/XML/geometry/document/protocol foundations in parallel.
4. Port raster/scene, native projects/animation, platform/UI, application+MCP.
5. Port in-repo developer/test/benchmark/release tooling; retain immutable fixtures.
6. Integrate parity workflows, complete existing missing product functionality.
7. Final full UI/E2E/fidelity/performance/accessibility/security/release acceptance.
8. Record and publish reviewed live Codex character+animation demonstration.

Research synthesis, dependency waves and available estimate are recorded below.
Focused checks continue during implementation; broad UI/E2E remains final.

## Research synthesis and frozen first batch

Six actual Luna codex exec processes completed successfully in parallel using
ChatGPT subscription authentication. Reports/evidence: `artifacts/WO-RUST-PLAN-01/research/`.
Coordinator verified official Rust1.99.0 release and installed compiler, correcting
the inventory report’s outdated1.98.1 statement. Initial inventory omitted five
C implementation `.inc` files; corrected coverage is **603code files/137,570lines**.
All protected source and index hashes are unchanged; C candidates remain reference.
Some domain gap descriptions are provisional; migration acceptance uses current
source/accepted work orders and original assertions, not an old roadmap summary.

### Architecture decisions

- Rust1.99.0, Edition2024, Cargo resolver3; AppleSilicon macOS13 only. The installed
  stable alias currently resolves to the exact compiler. Pin1.99.0 in project
  metadata, verify actual version, and use `cargo +stable` on this host to avoid
  an automatic outside-repository toolchain install. No system package install.
- Preserve the custom retained UI and use Rust-native Apple framework bindings
  behind the platform seam. AppKit/CoreGraphics/CoreText/CoreAnimation/Metal
  selection and feature-level macOS13 availability are audited per platform
  order. Embedded Metal shader programs must be explicitly mapped to the Rust
  presentation implementation or retired; they are not hidden C backends.
- Keep owned bounded XML/SVG, geometry/raster and protocol logic. Start foundation
  crates with std only; no parser/GUI/async ecosystem rewrite or dependency
  adoption is required to start. Review exact objc2 framework pins/features,
  crc32fast or serde candidates in separate dependency ADR/provenance records
  before fetching/building. Research version lists are candidates, not approved
  latest-version claims.
- Use immutable owned source bytes and offset spans, checked size arithmetic,
  fallible bounded allocations, typed diagnostics and no self-referential source
  pointers. Domain crates forbid unsafe; small audited platform FFI modules
  document thread affinity, callbacks, retained resources and teardown.
- Owned MCP wire behavior:2026-07-28 per-request Tasks plus established2025-11-25
  deferred tools/progress/cancellation. Do not add legacy experimental Tasks
  methods merely because an SDK supports them. rmcp remains an interoperability
  reference/candidate; no runtime/dependency adoption yet.

### Parallel execution

Owner update, 2026-10-15: there is no fixed project worker ceiling. The six
workers below describe the initial foundation batch only. Continue with as many
useful, independent ready lanes as current environment capacity and existing
subscription allowance permit. Preserve one writer per module and coordinator
ownership of shared manifests, interfaces and ledgers.

Bootstrap the shared workspace once, then run **six disjoint implementation
workers**: base, geometry, XML characters/reader, bounded JSON, native bundle
codec, and RGBA tile storage. This is an early foundation batch, not six complete
product subsystems. Each gets a frozen scope, isolated current-source worktree,
fixed public contracts, its own output/build and focused parity tests. Coordinator
alone owns root Cargo/lock/interface changes and serial integration. Increase
parallelism when additional independent ready scopes exist; never launch multiple
writers for one module or duplicate audits solely to occupy CPU slots.

The research runs inherited unrelated connector config, causing avoidable OAuth
startup noise and large repeated cached contexts. Future CLI workers isolate user
configuration, retain subscription auth and the economy model profile, and use
only local repo instructions/tools needed by their scope. CLI usage is recorded
as token evidence, not a dollar/account allowance meter. Stop rather than loop
on allowance/rate errors; no credit purchase or API billing switch.

### Dependency waves and acceptance

| Wave | Work | Entry/exit condition |
|---|---|---|
|0|Cargo/macOS foundation + shared contracts|Exact compiler, zero external deps, buildable workspace|
|1|Six foundation modules in parallel|Original strict semantics/limits and focused Rust checks per module|
|2|XML tokenizer/index, document edits/history, paths/CSS/references, task registry/socket, project timeline, UI layout/widgets|Accepted underlying contracts; one writer per module|
|3|SVG decode/effects/media and raster, project/recovery/animation, native platform/UI, MCP owner bridge|Safe lifetime/thread boundaries and complete format/tool parity|
|4|Application/editor integration and Rust developer/test/benchmark/report/packaging tooling|Every inventory entry assigned port or justified historical-data retirement|
|5|Parity gaps and remaining full product functionality|No shipped first-party C/ObjC backend; usable complete workflows|
|6|Final broad UI/E2E/corpus/performance/accessibility/security/release|All ADR0018 finish-line evidence, no oracle weakening|
|7|Live Codex character+animation recording, website/README example|Accepted live capability, reviewed accurate accessible footage|

Rough research inference is **60–140+ engineer-days** across migration, integration
and unfinished product work. This is not a wall-clock forecast or a subscription
cost guarantee; revise after the first accepted ports expose translation and
review throughput. Parallel workers overlap independent code, while contracts,
native debugging and final acceptance remain integration bottlenecks.

### Primary source register

- [Rust1.99.0 release](https://blog.rust-lang.org/2026/10/01/Rust-1.99.0/)
- [Apple targets/deployment environment](https://doc.rust-lang.org/rustc/platform-support/apple-darwin.html)
- [Cargo workspace/resolver](https://doc.rust-lang.org/cargo/reference/workspaces.html)
- [Cargo optimization/size/build tradeoffs](https://doc.rust-lang.org/cargo/reference/profiles.html)
- [objc2 framework bindings](https://github.com/madsmtm/objc2), [AppKit0.3.2 metadata](https://docs.rs/crate/objc2-app-kit/0.3.2)
- [XML1.0](https://www.w3.org/TR/REC-xml/), [SVG2](https://www.w3.org/TR/SVG/)
- [Official Rust MCP SDK](https://github.com/modelcontextprotocol/rust-sdk), [current Tasks](https://tasks.extensions.modelcontextprotocol.io/specification/2026-07-28/tasks)

Fetched/checked2026-10-14. Version conflicts corrected or left explicit before
dependency adoption. No performance, GUI parity, or completed Rust port claim.

## Current implementation checkpoint

`WO-RUST-BOOTSTRAP-01` is accepted as scaffolding: std-only workspace compiles,
two safe-span checks and developer info/check pass, exact compiler/macOS target
verified, original603codefiles/index preserved. LunaMedium independent CLI
review accepted after resolving a Low reviewer contract misunderstanding.
Rustfmt is not installed; no global component install or formatting-pass claim.

Six actual isolated Luna codex exec implementation processes were launched:
`WO-RUST-BASE-01` (High), `WO-RUST-GEOMETRY-01` (Medium),
`WO-RUST-XML-01` (High, characters/reader scope), `WO-RUST-JSON-01` (High),
`WO-RUST-PROJECT-01` (High, v1bundle scope), `WO-RUST-RASTER-01` (Medium, tile
storage scope). Process registry is `artifacts/WO-RUST-BOOTSTRAP-01/foundation-processes.json`.
They use lean isolated config/economy profile and subscription auth, no unrelated
connectors or network/dependency downloads. All six have now passed their parity
map, ownership review, exact source audit and main focused checks as recorded below.

## First port batch accepted

All six named foundation ports are integrated after independent reviews, exact
worker/root audits and main focused checks: base7, geometry5, XMLreader10, JSON5,
bundle8, tile5 tests. XML received diagnostic/name-limit assertions; bundle
received a real asset CRC error-offset parity repair. Original source/fixtures/
manifests/index are preserved. These scopes cover foundations, not complete
XML/SVG/raster/project/MCP/application parity. The accepted six contain 40 focused
Rust tests in total; this is not an application acceptance result.

## Second batch and next action — 2026-10-14

Workspace registrations and native-system FFI policy are accepted as setup only.
Six isolated Luna High processes were launched for full XML tokenizer, spatial
index, SVG values, owned MCP protocol, UI core and native system services.
Registry: `artifacts/WO-RUST-WAVE2-SETUP-01/batch.json`.

At the initial consolidation inspection, all six workers, including the MCP
protocol worker, exited 0. These were candidate states, not accepted module ports.
Next: independent
scoped review, repair, exact audits and main focused checks before integration.
Then freeze the next dependency-ready wave. Complete Rust desktop application,
full product workflows, final acceptance and recordings remain outstanding.

The earlier replacement rejection is historical. The owner resumed the existing
tracked goal, and `get_goal` confirmed active status. Implementation continues
under the consolidated Rust plan without redefining the full product finish line.

## Resumed implementation checkpoint — 2026-10-14

Six independent Luna High read-only CLI reviews completed. Five second-batch
ports are now integrated and accepted after scoped repairs/rechecks where needed,
fresh exact candidate/main audits and named main Cargo checks:

| Accepted port | Main focused package tests | Correction / scope |
|---|---|---|
| Native system | 8 | Audited Security/System FFI, bounded entropy, identity/peer credentials |
| Spatial index | 9 | Combined-extent overflow precedes duplicate-ID failure; transient-ID adaptation recorded |
| SVG values | 12 | Exact whitespace grammar, aspect source cap and composition diagnostic coverage |
| MCP protocol | 9 | Text-block fallback content shape; failed-task result versus RPC error envelope |
| UI core | 8 | Full legacy accent/contrast combinations and focus-role/wrap/disabled regressions |

Together with the six foundations, **86 focused Rust tests pass** across
accepted main-tree domain ports. These scopes do not establish full renderer,
desktop UI, real MCP app integration or application acceptance.

XML streaming/visitor storage/QName repairs passed only part of the narrowed
review. One-byte UTF-8 progress, cancelable one-shot parsing and failure token
visibility require a focused Sol6.1 Medium second repair under the authorized
model escalation rule. Its supervisor and CLI evidence are tracked at
`artifacts/WO-RUST-XML-02/repair-v2-status.json`; do not accept or copy its code
until narrowed review and fresh audits/main checks pass. The latest resumed
execution snapshot is
[`resumed-goal-checkpoint.json`](../../artifacts/WO-RUST-XML-02/resumed-goal-checkpoint.json).
Original 603 legacy source hashes, fixtures, HEAD and index are preserved.
Shared manifests/lock change only through accepted serialized registrations.
No broad UI/E2E testing ran. Next: finish XML acceptance and advance the ready
downstream work below without repeating completed ports.

## Third parallel batch — 2026-10-14

`WO-RUST-WAVE3-SETUP-01` is accepted as registrations/contract setup only: four
new std-only crate shells, six frozen scopes, offline locked workspace check and
metadata/source audits. Existing accepted implementations remain intact.
Six actual CLI workers launched from current-source overlays:

| Work order | Complete named scope | Model/effort |
|---|---|---|
| `WO-RUST-SVG-PATH-01` | Path commands/arcs/curves, transformed flattening, subpaths and source-number provenance | Luna High |
| `WO-RUST-TIMELINE-01` | Native timeline schema/JSON decoding, reference validation, accessors/iterators and cancellation | Luna High |
| `WO-RUST-UI-DRAW-01` | Full named tree/scalar painting, icon kit/legacy masks/hash and basic widget wrappers | Luna High |
| `WO-RUST-ATOMIC-FILES-01` | Safe existing-file replace and no-replace new-file publication, durability/cleanup outcomes | Luna High |
| `WO-RUST-TILE-CACHE-01` | Bounded owned tile cache, metadata/pixel charges, ownership, LRU/clock/epoch behavior | Luna Medium |
| `WO-RUST-TASK-QUEUE-01` | Bounded owned/shared native task queue, generation/cancel/poll/shutdown/cleanup | Luna High |

Registry: `artifacts/WO-RUST-WAVE3-SETUP-01/batch.json` and `processes.json`.
Each has one crate writer, separate worktree/build/cache, frozen contracts and
focused checks. The XML correction remains separate; these scopes consume only
accepted contracts. The task queue and atomic-files workers use separate crates.
No third-batch implementation is accepted from setup or worker launch.

Current process/goal checkpoint:
[`third-batch-goal-checkpoint.json`](../../artifacts/WO-RUST-WAVE3-SETUP-01/third-batch-goal-checkpoint.json).
Full native UI presentation, document/scene editing, complete SVG rendering,
animation authoring/playback/export, real live MCP and Rust tooling remain
required downstream work. All final acceptance/release/demo gates remain intact.

The owner's schedule question received a provisional several-weeks-to-few-months
range: current batch 2–6 hours, remaining migration/integration 3–8+ weeks,
missing product workflows 2–6+ weeks partly overlapping, final tests/fixes/release/
demos 1–3+ weeks. This is a low-confidence planning estimate, not a guaranteed
date, whole-product throughput measurement or allowance promise. Re-estimate
after the first usable Rust desktop build and named integration benchmarks.


## Current integration checkpoint — 2026-10-14

All six third-batch implementation scopes are accepted after independent review,
scoped repair where needed, exact preservation audits and named main checks:
SVG paths (9 package tests), native timeline decoding (14), UI draw/icons (12),
tile cache (11), task queue (11), and atomic publication revision 2 (25).
The additive raster capacity/import contract has 8 package tests. These package
counts include earlier accepted tests; the unique current domain total is
**155**, rather than a sum of historical work-order counts.

XML tokenizer repair-v4 remains live under Sol6.1 Medium. Its remaining review
focus is bounded accounting of source-backed PI/end-name comparisons; it is not
accepted or copied into main. Evidence: `artifacts/WO-RUST-XML-02/repair-v4-status.json`.
The next gate is independent narrowed review, preservation audit and named main
checks. Full native desktop/document/scene integration and every final product,
acceptance, release and recording gate above remain required. Broad UI/E2E has
not run. The latest goal-tool read reports blocked; prior active tracker reports
are historical, and the consolidated repository goal remains authoritative.


## Fourth parallel batch / XML acceptance — 2026-10-14

XML tokenizer/DOCTYPE/streaming is accepted after Sol6.1 Medium repair and final
independent Luna High recheck: 29 main package tests pass, including the original
10 reader tests. Name decoding, PI classification and two-operand equality have
real bounded source-read accounting; cancellation and fixed visitor storage are
retained. Named check/test and exact source/index audits pass. The unique current
main domain total is **174 focused tests**. This supersedes the earlier XML
pending checkpoints; full corpus/fuzz and application acceptance remain later.

`WO-RUST-WAVE4-SETUP-01` is accepted as registrations/contracts only after
Luna Medium review, locked offline workspace check and preservation audits.
Six actual disjoint Luna CLI implementation workers are live:

| Work order | Full named scope | Effort |
|---|---|---|
| WO-RUST-TIMELINE-EDIT-01 | Timeline creation/edit/move/loop/onion, history and owned source leases | High |
| WO-RUST-UI-WIDGETS-01 | Interactive text/number/color/slider/dropdown and complete named widget painting | High |
| WO-RUST-SVG-COLOR-01 | Owned CSS/SVG color parser, original named table and grammar | Medium |
| WO-RUST-MCP-TASK-STORE-01 | Bounded owner-isolated concurrent task lifecycle, results/TTL/cancellation | High |
| WO-RUST-MCP-ACCESS-01 | Canonical existing/new-save/directory paths and environment/config file grants | High |
| WO-RUST-PROJECT-IO-01 | Native project publication and full recovery capsule codec/save | High |

One writer per crate, no unfinished peer dependency, Standard subscription only.
Each candidate needs independent review, preservation audit and main checks.
Current state/handles: `artifacts/WO-RUST-WAVE4-SETUP-01/current-goal-checkpoint.json`.
The tracked goal is active after the owner resumed it. Every full product,
final acceptance, packaging and recording gate remains required. No broad UI/E2E
has run. Accepted XML now enables the next XML-index/document/scene interfaces.


## Current continuation checkpoint — 2026-10-14

SVG colors and MCP task store are accepted after independent review, exact source
integration audits and named main checks (3 and8 package tests respectively).
Unique current main domain total: **185 focused tests**. Timeline editing is a
completed candidate under review; widget input/painting is still implementing.
MCP grants and project I/O are repairing raw non-UTF8 paths and explicit Rust
allocation-error mapping respectively, with original assertions preserved.

`WO-RUST-WAVE5-SETUP-01` registrations/contracts passed independent Luna Medium
review, offline workspace check and original source/index audits. Two additional
actual Standard Luna High writers are live for full named SVG index and document
model ports, now that XML and native taskqueue are accepted. Index borrows caller
Arc<Vec> source while its real background validation worker owns a cloned Arc;
no unsafe borrowed thread pointer or unconditional whole-source copy. Document
model preserves source-backed spans, bounded tables and stale-safe handles.
They are unaccepted candidates; source editing/scene/native desktop work follows.

Latest handles and authoritative current counts:
`artifacts/WO-RUST-WAVE5-SETUP-01/current-goal-checkpoint.json`.
The full goal is active. Broad UI/E2E remains the final gate after functionality;
all product/release/demo acceptance requirements remain unchanged.


## Scientific evidence added — 2026-10-14

The owner explicitly requested science-paper-search. The accepted targeted
review is [rust-rendering-research.md](rust-rendering-research.md), with eight
successful paced API envelopes, identifier/version reconciliation and five
primary PDFs. Independent Luna Low review/recheck and source/evidence checks
passed. It does not claim exhaustive coverage, algorithm adoption or measured
Vexel performance.

Future rendering/performance orders must include spatial preprocessing and
animated invalidation, actual reserved capacity and end-to-end presentation;
measure first feedback and final frames separately for drawing, dragging,
pan/zoom, playback and live-agent transactions. Existing product budgets and
source-preserving SVG/customUI/ownedrenderer constraints remain unchanged.
Full UI/E2E and all release/recording gates remain final requirements.


## Current accepted continuation — 2026-10-14

All eight fourth/fifth batch named ports are accepted after focused reviews,
scoped corrections, exact preservation/input-delta audits and main checks.
Latest package counts: timeline29 (15new), UI17 (5new), MCPgrants12, projectIO8,
SVGindex12, document9; colors3 and MCPtasks8 accepted previously. The separate
XML allocation correction has31package tests (2new). Unique current domain
total: **248focused tests**, across21 packages. Historical per-order counts
are not added again. All current CLI handles are terminal, not waiting workers.

Resource/geometry corrections preserved original limits, constructor behavior,
C error/offset precedence and accepted assertions. Rejected unaccepted fixture
expectations are archived; raw-path tests assert actual native rejection on this
filesystem and preserve success assertions when supported, without skips or
mocked success. No broad UI/E2E or full application acceptance occurred.

Next dependency work: source-edit/history, SVG style/reference/scene, owned
application integration and Rust/AppKit presentation. The legacy-parity
document model retains16MiBinputcap; the100MB editable-product workflow needs
a separately scoped large-source integration API and evidence. This is an open
product gap, not a reduced goal or proof of large-file support. Full rendering,
animation authoring/playback/export, real live MCP, Rust tooling, acceptance,
release and demos remain required. Current evidence/handles/counts are recorded
in `artifacts/WO-RUST-XML-STORAGE-01/current-goal-checkpoint.json`.


## Latest execution checkpoint — 2026-10-14

Wave6 owned-document/large-source, SVG reference and source-save ports are
accepted after independent review, focused corrections and exact main-tree
checks:13/11/5 package tests respectively. Document13 includes9 previously
accepted tests; unique main domain total is **281 across24 packages**. Large
source correctness includes a100MiB input with shared immutable storage. The
complete large-file edit/save workflow and measured product budgets remain open.

CSS is accepted after source-provenance and exact-value assertion repairs,
independent recheck and13 main focused tests. Wave7
source-editor registration is accepted and its isolated LunaHigh CLI writer
covers all nine named C implementations plus the inline document-view APIs,
including piece tables, snapshots, complete history and source-preserving edits.
Native desktop proposal is accepted as planning only; pinned bindings/ADR,
macOS13 symbol audit and actual platform implementation follow separately.

Scientific directions remain recorded in
[rust-rendering-research.md](rust-rendering-research.md): spatially local work,
accounted cache/preprocessing memory, dynamic edit invalidation and measured
input-to-visible feedback. These are testable inferences, not AppleSilicon
performance results or external renderer adoption. All nine product outcomes
and comprehensive final acceptance/release/authentic recordings remain required.
Current execution evidence:
[`current-goal-checkpoint.json`](../../artifacts/WO-RUST-WAVE7-SETUP-01/current-goal-checkpoint.json).


## Dependency-ready animation and native audit — 2026-10-14

Wave8 registration is accepted: one std-only `vexel-svg-animation` crate using
accepted base/XML, the full SMIL/motion parse/eval work order, and a separate
primary-source native-binding audit. Offline workspace checks, frozen-order
validators, ledger/diff and exact source/index preservation pass. A setup
review's pre-acceptance sequencing concern was resolved by a narrow recheck:
older integrations accept shared hashes only after setup acceptance.

Actual CLI processes run in parallel: source-editor LunaHigh, SVG animation
LunaHigh, and native-binding Sol6.1Medium. The editor's intermediate candidate
is explicitly incomplete, with remaining ID/structural/allocation/oracle gaps;
no acceptance follows passing its small initial checks. The native audit writes
only dependency/SDK evidence and a report. It cannot add packages, adopt bindings
or claim runtime compatibility. Renderer/animation authoring, desktop composition,
large edit/save performance, live agents and all final product gates remain.
Execution handles and evidence:
[`current-goal-checkpoint.json`](../../artifacts/WO-RUST-WAVE8-SETUP-01/current-goal-checkpoint.json).


## Latest accepted SVG animation checkpoint — 2026-10-14

`WO-RUST-SVG-ANIMATION-01` is accepted in main after independent whole-scope
review, invalid-base diagnostic and duplicate-mpath-ID corrections, narrow
recheck, exact preservation audit and9 named main tests. The accepted scope
covers original SMIL/motion parsing/evaluation and source-relative data;
renderer composition, SVG authoring and live cartoon workflows remain later
required integration. Unique accepted main domain total: **290 across25 crates**.

The initial source-editor candidate finished incomplete with7 small tests;
it was preserved, not integrated. Sol6.1Medium now closes its named full-scope
structural/ID/history/allocation/original-oracle gaps. The native audit's sandbox
DNS failure was resolved through a successful coordinator network route:16
published archive checksums and4 exact-revision license statements verified.
Earlier failed routes are retained. No dependency adoption/installation or
native GUI compatibility follows from this evidence. Complete closure,
reviewed ADR/pins/features/licensing and native implementation remain required.
Accepted animation changes during the read-only native audit are recorded as
coordinator-authorized source deltas, not attributed to that writer.


## Current editor/native continuation — 2026-10-14

The full source-editor work order is accepted:112Csymbols mapped, initial
incomplete candidate retained, independent final review, exact root audit and
53focused tests pass. Both original200000-case sequences, fallible-allocation
rollback, source-preserving operations, nested IDs, snapshots/concurrency,
history/viewpagination and actual native saves retain their assertions. Unique
main domain total is **343 across26packages**; this does not establish100MB
editing/save performance or whole-app completion.

Native audit and ADR0026 pass independent review. Setup R2 is accepted:16selected
native units compile; Cargo locks/caches2 additional inactive optional records.
Exact versions/checksums/active tree and the original R1 scope mismatch are
recorded separately. Native exception handling permits the explicitly audited
thirdparty C-unwind helper inside the private platform fence; public operation
and inbound callback exits must contain converted exceptions/Rust panics.
Firstparty code remains Rust, with unsupported cleanup/abort cases explicit.
A real Foundation/mainthread boundary is assigned to LunaHigh; no AppKit window
or macOS13runtime compatibility follows from registration compilation.

The complete renderer plan covers69files/678signature locations and24frozen
goldens, plus necessary additive owned/cursor/large/control APIs. LunaHigh is
reviewing it before root freezes its full implementation scopes. Owner steering:
use Luna as much as possible for scoped work and review; Sol only for concrete
architecture/debugging that reduces rework. BroadUI/E2E/fidelity/performance/
release/live-MCP recorded demos remain final gates. Current evidence:
[`current-goal-checkpoint.json`](../../artifacts/WO-RUST-NATIVE-BINDINGS-SETUP-01/current-goal-checkpoint.json).


## Renderer dependency progress — 2026-10-14

Full renderer contract plan accepted after independent source/context-lifetime
corrections;69files/678signature locations,157suite/fixture/golden/manifest hashes
and603preserved migration sources audited. Planning acceptance adds no renderer
functionality. Its proposed OwnedAnimation/AnimationView names bind to accepted
vexel-svg-animation OwnedDocument/DocumentView; no duplicate owned parser.

Native boundary, owned animation views and shared path geometry are accepted
with independent reviews and fresh named checks. Current recorded focused total
is356 across27packages (including2compile-fail boundary checks); prior tests are
counted once. Values policy/cursor and base work-control/cancellation remain
unaccepted worker/review tasks. Subsequent controlled/resumable lower engines,
shared owned scene interfaces, full decode/paint/definitions/media and scalar
render partitions are required before product composition. Native shell/media,
100MB editable/save/render performance, detailed live MCP, full final acceptance,
packaging and authentic recorded character-animation demos remain required.

Implementation continues under owner authorization. The goal-tracker tool's
older blocked goal cannot be replaced through create_goal while unfinished;
no false completion/resume was recorded to bypass that restriction. Current
implementation evidence/handles are in the linked machine checkpoint.


### Controlled engine continuation

Values policy/cursor and shared base budgets/cancellation are now accepted:
19values tests and14base tests include7new cases each; current unique focused
main total370 across27packages. Effective1GiB/4096-attribute value policies retain
16MiB/256 defaults, absolute spans and original grammar. This does not lift the
editor/save workflow's remaining16MiB limits. Real thread cancellation, checked
work counters, typed sticky stops and shared control APIs are accepted; existing
engines require their own actual in-loop control/resumption implementations.

Four disjoint controlled-engine orders are running. Reference build/find and
animation parse/evaluate stay LunaHigh; initial CSS/path candidates returned
incomplete and remain unaccepted, with specific state-machine complexity handed
to Sol6.1Medium. Full scope/original assertions/real progress remain frozen;
no successful baseline tests are substituted for missing implementations.


### Controlled engine gates accepted — latest checkpoint

All four named lower-engine gates are accepted in main: reference18, CSS63,
path36 and animation21 package tests pass after independent reviews/exact
protected audits. Total460 named focused test entries across27packages includes
original CSS cases run through two added controlled routes. It does not mean
460different input fixtures, full rendering coverage or full-product QA.

Reference/CSS/path retain genuine byte/token/grammar/geometry/cascade cursors;
animation is a controlled one-shot parser/evaluator with actual polling inside
its long scans, with no resumable-session claim. Zeroquantum trivial-reference
cancellation, original test formatting, animation precharges/stale verification
and CSS integration oracle input omissions were repaired with their initial
records preserved. Sol6.1Medium completed CSS/path state machines after incomplete
Luna attempts; Luna completed/reviewed the other gates and all final reviews.
All original603migration-source hashes/index and frozen renderer goldens remain.

The next implementation stage is the coordinator-owned scene/shared rendering
contracts and then full parallel decoder, paint, definition, media, selection,
scalar-kernel/effect/animation/composition owners. Native shell/text/presentation/
menus/accessibility and real Rust executable/liveMCP are still missing. Large
editing/save policies/performance, all-first-party Rust tooling, final comprehensive
acceptance, packaged fresh-install readiness, documentation and authentic
recorded character creation/animation remain required. No Rust desktop or final
product completion is claimed. The earlier blocked tracker snapshot is now
historical: the owner resumed the existing goal and the latest read is active.
Its stored objective remains language-neutral; the canonical full goal and
outcome contract above supply the Rust/product/release/demo requirements.


### Owned scene model accepted — latest checkpoint

WO-RUST-SCENE-MODEL-01 implements the complete owned scene DATA MODEL gate:
all19 private data tables and7 insertion-serial tables, immutable checked views
including source/animation/sealed children, exact per-edge colors/path rebases,
fallible table builders, actual capacity/transient-overlap accounting and
scene-local logical rollback. Child links require sealing; source and animation
share one Arc<Vec<u8>>. Handles reject cross-graph/scene and removed slot reuse.
Complete freeze validation includes source/ranges/finite geometry, image layout,
filter dependencies and ordered/disjoint sibling opacity-group painter ranges.

Independent actual LunaHigh review/rechecks end ACCEPT after metadata and group
ordering fixes. The initial source-capacity concern was disproved against the
accepted animation storage implementation and an exact new regression; adding
source again would double count it. Fresh main27focused tests (6unit+21integration),
offline local-path lock/check, frozen-order/ledger, source603/HEAD/index and final
review-hash audits pass. No broad UI/E2E or workspace-wide suite was run.
Current recorded aggregate is487named focused entries across28accepted packages,
with prior CSS extra-route entries still qualified. This is not full-product QA.
Evidence: [`result.md`](../../artifacts/WO-RUST-SCENE-MODEL-01/result.md),
[`verification.json`](../../artifacts/WO-RUST-SCENE-MODEL-01/verification.json),
[`current-goal-checkpoint.json`](../../artifacts/WO-RUST-SCENE-MODEL-01/current-goal-checkpoint.json).

Next implement full shared kernel and decoder-continuation contracts, then
release dependency-ready disjoint full decoder/paint/definitions/media/selection/
scalar-index/sample/effect/animation ports for real application composition.
Native window/UI/text/presentation/menu/accessibility, measured large-file
editable/save/render workflow, detailed live Codex/Claude MCP, complete first-party
Rust tooling, comprehensive final acceptance and blocker fixes, packaged
fresh-install release, documentation and authentic recorded website/README demos
remain mandatory. Goal active; no full Rust desktop/product completion claimed.


### Shared decoder/scalar contracts accepted — latest checkpoint

WO-RUST-KERNEL-CONTRACTS-01 completes the named shared data/interface gate:
canonical source/token ownership and real accepted CSS/reference query state,
source-vs-context borrow lifetimes, bounded pattern tickets, fallible capacity
leases, all scalar/traversal/delegate/cache data and effective frame overlays.
All43main package tests pass, including unchanged27model and16newcontract cases.
Independent LunaHigh review/recheck ACCEPT after correcting untrusted element
source pairing and malformed dash ranges while preserving global high-bit join
encoding. Original603/157frozenoracles/HEAD/index hashes pass. Recorded aggregate
503named focused entries across28packages; no broadUI/E2E or full renderer proof.

The concrete full engines and adapters remain required; no unfinished decoder or
synthetic renderer/provider is registered. Exact next shared dependency: safe
mark-scoped draft-only shape/clip/paint/group finalization operations for deferred
postoperations before the decoder writer starts. Immutable-data renderer owners
can proceed after ready registration. Native-window readiness runs in parallel.
All native/UI/animation/liveMCP/large-file/fullRusttooling/finalacceptance/release/
recorded-demo outcomes remain required; goal active and application incomplete.
Evidence: [result.md](../../artifacts/WO-RUST-KERNEL-CONTRACTS-01/result.md),
[api-coverage.json](../../artifacts/WO-RUST-KERNEL-CONTRACTS-01/api-coverage.json),
[current-goal-checkpoint.json](../../artifacts/WO-RUST-KERNEL-CONTRACTS-01/current-goal-checkpoint.json).


Native-window implementation readiness is accepted in WO-RUST-NATIVE-WINDOW-PLAN-01
R2 after independent Luna review/recheck. The concrete plan includes app-owned
retained records, weak Window generations, reentrancy-safe callback routing,
owned IME snapshot/range/candidate geometry, bounded pending event delivery and
sequenced fault/reset/close acknowledgement. Current accepted ADR0026 dependency,
canonical grant, setup and real Foundation boundary evidence cleared a stale
initial-audit concern. No package or feature change is adopted. Initial controller
allowlist/lifecycle bookkeeping failures remain archived; product/native/UI/603/
index hashes pass. This is a ready implementation lane, not native-window code
or mac13runtime proof. File/menu/AX/text/presenter/preferences/dialog/welcome and
real desktop integration remain mandatory.
Plan: [ready-window-order.md](../../artifacts/WO-RUST-NATIVE-WINDOW-PLAN-01/ready-window-order.md).


### Scoped decoder draft gate and live-demo clarification — latest checkpoint

WO-RUST-SCENE-DRAFT-01 is accepted. Opaque value tokens authorize only new-record
finalization within bounded64-level LIFO scopes, support actual deferred decoder
postoperations, reject pending publication and roll back scope-created children.
A concrete independent Luna review blocker found ignored notices outside draft
rollback; private count/offset snapshots and nested regressions resolved it.
Independent recheck ACCEPT and fresh main57focused tests pass, including43
byte-preserved existing cases and14newdraft cases. Protected603migration files,
157frozen renderer oracles, source limits and HEAD/index audits pass. Recorded
aggregate517entries across28packages; this is focused evidence, not full app QA.

The full goal now explicitly requires an authentic recorded Codex conversation
that draws and revises a character, then creates and revises its animation through
successive real MCP calls while the same Vexel project visibly updates. Required
playback/save/reopen/export, reusable projects/prompts/assets and accessible video
on website/README remain pending final deliverables. See live demonstration
requirements above and WO-RUST-GOAL-06; all nine areas/seven finish gates remain.

The initial Luna native-window source candidate is preserved as incomplete.
ADR0027 accepts only five features of already-pinned bindings, with unchanged
16active/18lock packages and original7focused tests. Native WINDOW R2 consumes
that approved coordinator-only seed, while protecting worker manifests. Actual
same-session Sol6.1Medium repair is running for missing callbacks, full IME,
event dispatch, timers, Quartz geometry, cursor/material behavior and teardown.
It must pass independent review and main checks before integration; no real
native-window/application/runtime completion is claimed.

Next register useful disjoint complete decoder/paint/definitions/media/selection/
scalar-index/sample/effect/animation engine ports against the accepted contracts
and scoped draft finalizers. Keep focused correctness checks; broad automated
UI/E2E, measured large-file workflows, actual live agents, packaged fresh-install
release and authentic recordings are still required. Goal remains active.

Current accepted evidence and actual worker status:
[current-goal-checkpoint.json](../../artifacts/WO-RUST-SCENE-DRAFT-01/current-goal-checkpoint.json).


### Eight full engine partitions running — latest execution checkpoint

WO-RUST-RENDER-WAVE-SETUP-01 is accepted as registration only. Eight separate
crates retain the complete accepted owner assignments:35source-file rows,
416symbols, original suite assignments and shared operation semantics. All depend
only on accepted source/scene/draft/decoder/scalar/frame contracts; later root
adapters must supply real implementations. No fake provider/renderer is adopted.
Independent LunaMedium review ACCEPT, lockedoffline workspace build/frozen orders/
ledger/diff and603legacy/157frozenoracles/oldRust/HEAD/index audit pass. Exact
external pins/checksums remain unchanged. The new libraries register no engines;
source acceptance and full renderer/application proof remain outstanding.

Eight actual isolated Luna source processes are confirmed live: DECODE, PAINT,
DEFINITIONS, MEDIA, SELECTION, RASTER_INDEX, RASTER_SAMPLE and ANIMATION. Query
scope uses Medium, seven strict complex algorithm scopes High; Standard existing
subscription only. Native-window Sol6.1Medium R2 repair runs separately, for nine
useful source writers. Full-scope candidates require independent review, exact
preservation audits and fresh main focused checks before integration. Initial
launch assembly referenced a wrong historical review filename, before starting
any new writer; that evidence/partial checkout was preserved, the exact path was
corrected and assembly continued without restarting a live process.

Aggregate accepted evidence remains517focused entries across28packages. Full
flat rendering/effects/real coordinator adapters, native media/export, window/UI/
accessibility/files/text/presentation, application/live MCP, performance and all
first-party Rust tooling remain required. Comprehensive UI/E2E stays after feature
completion, followed by fixes, packaged fresh-install acceptance and authentic
iterative Codex drawing/animation demo on website/README. Goal remains active.

Current process handles and exact registration/source baselines:
[current-goal-checkpoint.json](../../artifacts/WO-RUST-RENDER-WAVE-SETUP-01/current-goal-checkpoint.json).


### Native source and real consumer publication gates accepted — latest checkpoint

Current accepted evidence is579focused named entries across28packages: scene82,
nativeApple41 andCSS66. This is scoped evidence, not full application testing.
Consumer contracts now support source-coherent full-context cache entries with
owned stops, zero-edge checked handle enumeration and atomic finished-frame clip
updates retaining the original configured byte budget. Exact4marker style-copy
clears preserve all other values/spans/source and explicit child cascade.

Complete native-window source R2 is integrated after SolMedium implementation,
LunaHigh review, narrow LunaLow reserved16 queue repair and independent ACCEPT
recheck; fresh main41checks retain7original boundary proofs. LiveGUI/mac13/IME/
display/timer/teardown runtime remains a final gate, as do remaining native files,
menus/AX/shaping/preferences/welcome/Metal/UI/application/MCP components.

A confirmed additional PAINT gap is now closed by an accepted complete graph-stop
publication API: opaque validated source/definition/viewport template identity,
explicit ResolvedPaint handoff, sorted per-scene sharing, correct append-order
ordinary/nested rollback and stale/foreign witness checks, actual capacity and
source-once accounting. SolMedium implementation, independent LunaHigh ACCEPT,
fresh main82tests (66unchanged+16new), protected603/157/HEAD/index and source-scope
audits pass.40000instances share2graph stops; full32768template/65536stop bounds
admit. Real allocator CapacityOverflow is tested, not whole-process OOM injection.

All initial eight owner candidates are preserved as incomplete and unaccepted.
They do not reduce the required decoder/paint/definitions/media/query/scalar/
animation scope. PAINT now resumes its same actual Standard LunaHigh session
with accepted publication inputs after the previous process was confirmed
terminal. Fixture constructor dependencies, media use-host source maps, full
marker/definition algorithms and raster job-ledger/control seams are concrete
next fixes; full rendering/effects/adapters/composition remain mandatory.

Goal remains active with all nine outcomes/seven completion gates. Measured
large-file workflows, first-party Rust tooling, comprehensive final UI/E2E and
blocker fixes, packaged fresh-install release, real Codex/Claude/compatible MCP
and authentic iterative drawing/animation video on website/README remain required.

Current source processes, accepted counts and exact next actions:
[current-goal-checkpoint.json](../../artifacts/WO-RUST-PAINT-PUBLICATION-01/current-goal-checkpoint.json).


### Real fixture and media instance contracts unblock five source lanes

Current recorded evidence is583focused entries/28accepted packages, scene86.
ADR0028/WO-RUST-RENDER-FIXTURE-SETUP-01 registers only already accepted local path/
animation devdependencies to construct real owned source/token/animation/path/
SceneBuilder fixtures. Independent LunaLow metadata ACCEPT and workspace/protected
checks pass; no external package/version/runtime dependency change.

WO-RUST-MEDIA-SOURCE-MAP-01 is accepted after independent LunaMedium review,
fresh main86tests (82bytepreserved+4new) and603/157/HEAD/index audits. Text/image
requests retain canonical referenced tokens and explicit distinct use-host
SourceMap; validation checks source/jobledger/span/finite independent transforms/
writable scene/live transform identity/cancellation before provider or append.
Actual MEDIA/DECODE must apply mappings; this is not media-provider completion.

Accepted coordination archives four terminal candidates and seeds only exact
accepted shared/fixture/marker/source-map inputs. Same actual Luna sessions resume
full ANIMATION, DEFINITIONS, SELECTION and MEDIA scopes, threeHigh/oneMedium.
PAINTHigh remains live with publication API, for five confirmed useful source
writers. Full algorithms/original pure assertions on real fixtures, independent
review and freshmain checks are still required before source acceptance. Scalar
job reservation/control seams, full decoder, real adapters/raster/effects/native
media/UI/app/MCP, measured performance/tooling and final testing/release/authentic
website/README recorded showcase remain required. No full app completion claimed.

Current handles/accepted counts/prerequisites:
[current-goal-checkpoint.json](../../artifacts/WO-RUST-RENDER-CONSUMER-RESUME-01/current-goal-checkpoint.json).

## Accepted raster ownership/control and current production prerequisites

WO-RUST-RASTER-JOB-CONTRACTS-01 is accepted as a shared source gate: independent
coordinator LunaHigh review, exact integration and fresh main111checks preserve
86prior assertions and add25proofs. Actual same-job owning buffers, metadata,
capacity admission/overlap/lifetimes and controlled lower interfaces are complete
within that scope. Accepted total is608focused entries across28packages.
Original603source files/157frozen oracle inputs and HEAD/index remain intact.

Full scalar index and sample engine sessions resumed on those accepted inputs
in isolation at LunaHigh; their full original scopes and pure assertions remain
required. This does not accept a renderer or show application performance.

The next shared source gate is WO-RUST-DECODE-PUBLICATION-CONTRACTS-01 R2:
request-local rollback preserving outer draft/prefix, typed emptygroup filter
ownership, real cache/pattern production publication and public query buffer
reservation/identity. Latest MEDIA evidence additionally requires cumulative
document4Mi text-command accounting independent of262144cache admission, with
request rollback. Root audits this prerequisite before MEDIA completion; current
shared source has no acceptance claim.

Paint no-copy capacity/40000instance graph proof is independently substantiated;
malformedfill error parity remains in a focused LunaLow repair. Animation has
repeated budget-policy/bounds-cancellation findings and uses named SolMedium
repair. Media/query/definitions candidates remain incomplete and preserved.
Whole decoder/renderer/effects/native/custom UI/app/MCP/performance/tooling/final
acceptance/fresh-install release and authentic Codex character/animation demo
for README+website remain required; broad UI/E2E stays in the final stage.

Owned animation frame/clip R2 source is now accepted after independent LunaHigh
review and fresh main31checks;639focused entries/29packages total. Explicit
caller budget and true variable-loop cancellation replace the rejected default
policy; separate64MiB clip cap preserved. Whole renderer pixels/CLI/authoring/
fuzz/app/runtime/performance/MCP/finalQA/release/demo remain required.
Scalar index uses named SolMedium for repeated incomplete full implementations;
sample needs safe same-job cache-read/scratch-write composition before its full
implementation completes. No source gate accepts its current oldVec paths.

## Accepted publication/paint and full shared integration gate — 2026-10-15

Owned PAINT R2 source is accepted after fullLunaHigh review, narrow spanparity
Low recheck and fresh main26checks, including the unchanged3graph proofs made
discoverable by cargo using alreadyaccepted devdependencies. Shared publication
R2 is accepted with135mainchecks (111unchanged+24new).689focusedentries across
30packages are recorded;603legacy source/157frozen oracles and HEAD/index intact.
These remain scoped source gates; no fullapp/renderer/runtime/performanceclaim.

WO-RUST-RENDER-INTEGRATION-CONTRACTS-01 now runs as the sole shared sourcewriter
after those accepted prerequisites. Entire confirmed gaps: cumulative4Mi text
commands with requestcleanup, same-job validated cache-read/scratch-write without
per-sample fullvalidation, canonical original post-Z subpath metadata, explicit
maskpost targets and honest shared atomic quantum semantics. No smallercaps,
fake yields, topology rewriting or silent validation weakening is authorized.

Full query/definitions sessions resumed on exact accepted135inputs after
terminal-candidate preservation. Source index continues namedSolMedium full
scope. Decoder preserves its original failing smoothpath assertion; media/sample
and decoder retain incomplete source/pureproof work beyond known shared gaps.
Actual rootadapters/effects/native media/customUI/application/liveMCP, largefile
performance, allfirstpartyRust tooling, fullfinalacceptance/freshinstall release
and real Codex character/animation recording for README+website remain required.

## Latest accepted source checkpoint — 2026-10-15

720 recorded focused entries across31 main packages. Scene query13 and shared
Scene153 source gates are accepted after independent review, exact protected
audits and fresh main checks.153includes135unchanged plus16runtime/2compilefail.
The coordinator preserved terminal candidates before seeding exact accepted
Scene153 to full decoder/media/scalar-sample sessions; two named SolMedium
repairs and one LunaHigh implementation run independently. An additive required
Fill/Stroke production-planning contract runs under LunaMedium as the sole
shared writer; definitions adoption/review remains pending. No pending candidate
is accepted by process exit or source registration alone.

The index compact-number negative fixture correction is a concrete proposal
awaiting explicit owner approval. All603legacy files and157frozen acceptance
files/fixtures/goldens/manifests, HEAD and index are unchanged. Native readiness
R2 is still rejected for incomplete actual cached API/minOS and individual
assertion mappings. Broad UI/E2E has not run. Every full-product outcome and
release/demo completion gate above remains required. Current process snapshots
and preservation evidence are in the current-goal checkpoint, not old process
rows in historical sections.

### Explicit-channel source acceptance — 2026-10-15

723recorded focused entries across31mainpackages. SharedScene156 source contract
accepted after independent root LunaHigh review,1677protectedhash audit and fresh
main check/test. All153prior assertions unchanged. Required explicit-channel
patterns and combined nonpaint planning are usable; eight opacity/default
publications and legitimate distinct host/definition roles are proven. The
blanket-equality R2 requirement was corrected from actual source and frozen mask
evidence; no original assertion changed. Lower decoder/definitions adoption and
every full-product completion gate remain required. No broadUI/E2E run.

### Current dependent execution after channel acceptance

Five isolated source sessions: source-owned initialization/decoder/styled-media
use named SolMedium; full scalar rows and definitions adapters use LunaHigh.
Initialization is the sole shared writer, starts after accepted main156.
Definitions now receives exact accepted156source. Media34-test candidate is
not accepted: independent review found common style/delegate and diagnostic gaps.
Scalar18-test candidate is not accepted: original interval/run work remains;
100earlier unintended protected format edits were archived and exactly restored,
without changing main. Full strict scope and original frozen assertions remain.

### Full owned definitions acceptance — 2026-10-15

763recorded focused entries across32mainpackages. Full79symbol definitions source
is accepted after full LunaHighreview, markerdimension LunaLowrepair/recheck,
root1685protected audit/exactsource-diff and freshmain40checks. Actual URL and
channel-aware pattern publication retains distinct owner/reference roles.
All existing release/demo outcomes remain. Initialization rootviewport proof
repair active; two shared cache/shape publication workorders ready only. Native
readiness still incomplete. No fullrenderer/native/runtime/UI acceptance.

### Source-owned initialization acceptance — 2026-10-15

784recorded entries/32mainpackages. Fresh177Scene+21unchangedanimation checks and
fourconsumer compile checks pass after independentfullHighreview, provenance
Mediumrecheck and root1672protectedaudit/exactdiff. Pending traversal data and
actual lateanimation remain separate from ready/renderable data; rootviewport
comes from canonical source parsing. No fake viewport/emptyanimation adopted.
Decoder/media/definitions adoption, cachegeneration replacement and shape
publication remain. Fullproduct outcomes/release/authenticdemo unchanged.

### Cache publication source acceptance — 2026-10-15

792recorded entries/32packages. IndependentLunaHigh/protectedaudit/freshmain
Scene185 pass;177old assertions+7runtime+1compilefail. Explicitcachegenerations
retainoriginal percache16MiB/cell/ref limits while physicalold/new chargedhonestly
tocallerledger. Scalaradoption remains. Staticpattern/scopedmedia/bounds and
shapepublication nextshared source gates, fullproduct outcomes unchanged.
