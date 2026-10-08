# Revised roadmap, estimates, and work orders

All dates/runs are estimates; no implementation throughput data exists for this exact model pair and product. A run means one headless Luna session, not a token quota. Normal WOs: Luna drafts a candidate from this roadmap; Sonnet checks scope and acceptance. M0–M3 end with a progressive Mac viewer and a go/no-go. This is a work queue, not a promise of full SVG fidelity.

## Schedule and budget estimates

| Scope | Optimistic | Expected | Pessimistic | Luna runs | Sonnet tokens |
|---|---:|---:|---:|---:|---:|
| M0–M3 progressive viewer proof | 4–5 wk | 6–8 wk | 10–14 wk | 75–105 including retries/verifiers | 45k–90k |
| Mac basic editor after proof | 5–8 mo | 9–15 mo | 18–28 mo | 180–350 | 180k–420k |
| Linux + Windows after Mac beta | 3–5 mo | 6–10 mo | 12–18 mo | 100–220 | 100k–240k |
| Animation after static editor | 4–7 mo | 8–14 mo | 18–30 mo | 90–200 | 90k–220k |

Ordinary WO: 2–5 Luna runs total; platform/geometry: 5–10. Estimate 25–40% of WOs need one repair and 5–10% require split/rewrite. Optimistic assumes >75% first-pass acceptance; pessimistic ~45%, rate-limit pauses and platform integration failures. Critical path: corpus+goldens → source-preserving parser/index → deterministic renderer → UI driver/platform → fidelity comparison → owner gate. UI is parallelizable only after contracts stabilize.

### Token and cost model by milestone (estimates)

Luna token ranges include prompts/context/tool-return text at rough order of magnitude; actual CLI account usage may meter differently. Published API-equivalent uses GPT-6 Luna standard $0.10/M input and $0.50/M output, excludes hosted-tool charges and is **not** the Codex subscription invoice. Sonnet costs are token budgets only because the specified deployment/rate is not supplied.

| Milestone | Luna runs | Luna input tokens | Luna output tokens | Sonnet planning/review tokens |
|---|---:|---:|---:|---:|
| M0 | 35–45 | 15–40M | 0.5–2M | 20–35k |
| M1 | 10–14 | 5–15M | 0.2–0.7M | 8–12k |
| M2 | 16–23 | 8–25M | 0.3–1.2M | 10–18k |
| M3 | 15–22 | 8–25M | 0.3–1.2M | 10–18k |
| M4 | 30–55 | 15–60M | 0.6–3M | 18–30k |
| M5 | 20–35 | 10–40M | 0.4–2M | 12–20k |
| M6 | 25–45 | 15–60M | 0.6–3M | 18–30k |
| M7 | 20–35 | 10–40M | 0.4–2M | 12–20k |
| M8 | 25–45 | 15–55M | 0.6–3M | 18–30k |
| M9 | 20–35 | 10–40M | 0.4–2M | 12–20k |
| M10 | 15–30 | 8–30M | 0.3–1.5M | 12–20k |
| M11 | 25–50 | 15–60M | 0.6–3M | 18–30k |
| M12 | 25–50 | 15–60M | 0.6–3M | 18–30k |
| M13 | 30–60 | 20–80M | 0.8–4M | 20–35k |
| M14 | 15–30 | 8–30M | 0.3–1.5M | 10–18k |

Whole M0–M14 order-of-magnitude: 326–544 Luna sessions, 182–660M input and 7.8–30.1M output tokens, about $22–81 API-list-price equivalent at current standard Luna rates; 200–356k Sonnet tokens. This is a planning envelope, not a quote. Tool calls, retries, context-cache behavior, model price changes, and ChatGPT-plan Codex allowance all move the real cost. Rebaseline from actuals after the first 10 WOs.

Published at research time: OpenAI API pricing lists GPT-6 Luna standard input $0.10/1M and output $0.50/1M; that is not the billing model for ChatGPT-plan Codex. Codex allowances vary by plan and current usage; /status and account usage are authoritative. GitHub Actions hosted rates: Linux 2-core x64 $0.006/min, Windows 2-core x64 $0.010/min, macOS standard $0.062/min; partial minutes round up. Local Mac is primary; CI validates builds/tests but not all native behavior. Recheck prices/limits at kickoff.

## Bootstrap and milestone map

| Milestone | Ordered WOs (runs; risk) | Machine gate | Owner gate |
|---|---|---|---|
| **M0 bootstrap + benchmark** | BOOT-01 C17 scaffold (2; low); BOOT-02 AGENTS/ADRs/state/ledger (2; medium); BOOT-03 CI/dev scripts (3; medium); BOOT-04 wrappers (4–7; high); BOOT-05 tests/sanitizers/fuzz (3; high); M0-01 corpus provenance/fetch (3; high); M0-02 stress generator/oracle (3; medium); M0-03 competitor bench CLI (3–5; high); BOOT-06 golden images (3; high); BOOT-07 UI script driver (3–5; high); M0-04 evidence packet (2; medium) | clean builds; corpus hash-locked; benchmark artifacts; replay deterministic | **Yes:** user/corpus signal and continue/stop |
| **M1 Mac platform path** | M1-01 Cocoa shim (3; high); M1-02 CPU tile + Metal blit (4; high); M1-03 deterministic screenshots/replay (3; high); M1-04 cancellation/teardown (3; high) | native scripted open/resize/quit; pixel oracle; teardown | No |
| **M2 bounded source/index core** | M2-01 checked slices/arena (3; high); M2-02 XML tokenizer/source spans (5; high); M2-03 parser fuzz CI (3; high); M2-04 lossless no-op save (4; high); M2-05 rect/path subset (4; high) | limits, sanitizers, byte equality, render fixture | No |
| **M3 progressive viewer** | M3-01 incremental index/cancel (4; high); M3-02 visible set/cache (4; high); M3-03 worker snapshots (4; high); M3-04 nav/progress UI (4; medium); BOOT-08 size/perf/RSS gates (2; medium); M3-05 proof report (3; high) | TTFP/stall/pan/RSS, replay, golden, source roundtrip | **Yes:** review demo, pilots, continue |
| **M4 renderer fidelity** | geometry/path grammar, CSS, stroke/fills/gradients/clips/use, backend bake-off; 30–55 runs | selected fixtures vs independent references, fuzz gates | Yes: coverage/scope |
| **M5 document model + preservation** | stable IDs, overlays, namespace policy, minimal diff save, recovery; 20–35 runs | preservation differential and crash recovery | No |
| **M6 UI core/accessibility** | layout/paint/event/focus/semantics, keyboard, virtualized tree, text/IME shim; 25–45 runs | script + semantic tree, native smoke | Yes: visual/interaction |
| **M7 selection/transforms** | hit test, selection handles, multi-select, move/scale/rotate; 20–35 runs | scripted replay and geometric invariants | No |
| **M8 editing panels** | fill/stroke/opacity/gradients, layers, path node edit; 25–45 runs | scripts, source delta, undo invariants | Yes: discoverability |
| **M9 undo/save/recovery** | transaction log, undo/redo, save-as, atomic replacement, journal; 20–35 runs | random action sequences, fault injection | No |
| **M10 Mac editor gate** | full corpus/security/perf audit, package/pilots/competitor check; 15–30 runs | supported fixtures, fuzz/crash report | **Yes:** ship/pivot/stop |
| **M11 Linux** | Wayland/X11 shims, portal, text input, AT-SPI, Weston headless; 25–50 runs | Linux CI + headless scripts | Yes: compositor spot-check |
| **M12 Windows** | native input, DirectWrite, IME/UI Automation, packaging; 25–50 runs | Windows CI + scripts | Yes: real machine/IME |
| **M13 animation** | separate demand/semantics ADR then bounded SMIL/CSS timeline, deterministic frames; 30–60 runs | frame goldens and round-trip | Yes: demand gate |
| **M14 maintenance/release** | installer, signing, SBOM, update/security/fuzz process; 15–30 runs | reproducible package/install/update | Yes: release |

## Complete work-order texts: M0–M3

Each text below is a ready-to-freeze Sonnet seed. In each, protected paths are `tests/oracle/**`, `tests/golden/manifest.lock`, `bench/corpus/manifest.lock`, and `out3/**` unless the specific WO explicitly establishes a baseline. All WOs require clean starting tree, path allowlist, frozen spec SHA, JSON report, verifier, and one commit if complete.

### BOOT-01 — Repository scaffold and C17 build
**Goal:** deterministic minimal C17 repository and version CLI on Apple Silicon macOS. **Context:** locked decisions in out3 ADR; design rules in 04. **Allowed:** CMake, cmake/**, app/**, src/base/**, README, gitignore, non-oracle tests. **AC:** clean `./dev build` passes; C17 required and warnings-as-errors; `svgworkbench --version` prints `svgworkbench 0.0.1`; repeat clean build changes no tracked generated files; no configure network/dependencies. **Verify:** clean twice, diff check, dependency graph. **Non-goals:** UI/parser/raster. **Budget:** 2 runs, 1 verifier, 90 min, 10k output tokens. **Escalate:** compiler warning policy issue. **Commit:** `build: scaffold C17 project`.

### BOOT-02 — AGENTS, ADR, state and ledger
**Goal:** durable concise instructions and validated state files. **Context:** 01, 02. **Allowed:** AGENTS.md, STATE.md, docs/adr, work-orders, scripts/ledger, docs. **AC:** AGENTS <=200 lines; exact build/test commands and safety rules; initial ADRs record locked choices; ledger schema/transition validation and atomic append; invalid transition fails; STATE has current SHA/next three WOs. **Verify:** malformed JSON, duplicate ID, concurrent append, bad transition tests. **Non-goals:** model invocation. **Budget:** 2 runs/100 min. **Commit:** `docs: establish agent operating state`.

### BOOT-03 — CI, dev commands and reproducible checks
**Goal:** local one-command build/test and CI matrix. **Context:** scaffold; official CMake/Actions docs. **Allowed:** .github/workflows, scripts/dev, CMakePresets, docs. **AC:** `./dev build`, `./dev test-fast`, `./dev verify`; Linux x64, Windows x64, macOS arm64 build/test jobs (if hosted runner unavailable, report); pinned actions; no secrets for forks; artifacts identify compiler/OS/build; fast suite <30 sec; caches hash-keyed and optional. **Verify:** all local commands and workflow YAML lint; state honestly that hosted job pending if not run. **Non-goals:** signing. **Budget:** 3 runs/2h. **Commit:** `ci: add deterministic cross-platform checks`.

### BOOT-04 — Runner, verifier, ledger and worktrees
**Goal:** implement scripts exactly per 03. **Context:** template/schema 02; pin CLI. **Allowed:** scripts/wo-*, scripts/ledger*, wrapper tests/schemas/docs. **AC:** fake Codex tests cover success/schema error/timeout/bad session/dirty tree/path escape/protected oracle/concurrency/ledger crash; flags preflight; argv arrays, no eval; separate worktree; executor workspace-write, verifier read-only; known-ID resume only; no danger-full-access; max two retries; dry-run works unauthenticated; no auto-merge. **Verify:** network-disabled fake tests. **Non-goals:** release. **Budget:** 4–7 runs/4h. **Commit:** `tools: add bounded work order runner`.

### BOOT-05 — Tests, sanitizers and fuzz foundation
**Goal:** fast tests and sanitizer/fuzz profiles. **Context:** C17 rules. **Allowed:** tests/framework, fuzz, CMake test config, scripts, CI. **AC:** machine-readable results; ASan/UBSan profile; deterministic seed logging; 3 sec local fuzz smoke and scheduled 60 sec; crash input and sanitizer trace retained; no silent skips; fast loop <30 sec. **Verify:** temporary injected assertion and memory fault are detected. **Non-goals:** parser. **Budget:** 3 runs/2.5h.

### M0-01 — Corpus manifest and provenance fetcher
**Goal:** legally usable, hash-pinned corpus. **Context:** out3 sources and benchmark plan; no Inkscape source. **Allowed:** bench/corpus, bench/fetch, docs/corpus. **AC:** each fixture records origin/license/hash/features/time; verify fetch fails on hash mismatch; ambiguous licenses excluded; >=20 fixtures across basic/path/metadata/style/text/image/stress; queries deterministic; no private uploads. **Verify:** clean fetch, tamper negative test, license scan. **Non-goals:** standards completeness. **Budget:** 3 runs/2h. **Owner gate:** unclear rights.

### M0-02 — Stress generator and immutable oracle
**Goal:** deterministic stress families and independent golden oracle. **Context:** M0-01. **Allowed:** bench/generate, tests/oracle, tests/golden, corpus locks in this dedicated baseline WO. **AC:** seeded size/object/path/depth/image generators; 10MB, 100MB, bounded 1GB files; hashes stable; expected resource-limit behavior; reference renderer/version/license recorded; CI rejects oracle mutation outside baseline WO. **Verify:** generate twice, tampered hash refusal, reference render. **Budget:** 3 runs/2h.

### M0-03 — Competitor benchmark CLI
**Goal:** compare identical tasks on project, Inkscape, Donner where runnable. **Context:** corpus/oracle; black-box only. **Allowed:** bench/harness/scripts/report schema. **AC:** exact versions/hardware/OS/fixture hashes; cold/warm >=5 trials; TTFP if observable, RSS, completion, pan/zoom, crash; raw JSON/CSV plus median/p95; unavailable reason explicit; smoke works without competitors; no unlike feature comparison. **Verify:** fake process adapter and local smoke. **Budget:** 3–5 runs/3h. **No-go claim:** never generate public speed messaging here.

### BOOT-06 — Golden image tooling
**Goal:** deterministic PNG comparison and diff artifacts. **Context:** M0-02. **Allowed:** tests/golden (excluding lock unless dedicated baseline), diff script, CI/docs. **AC:** exact scalar comparison plus declared edge tolerance; heatmap + JSON metrics; metadata records build/fixture/SVG/scale/frame/font/backend; baseline change requires WO and reason; absent baseline fails. **Verify:** identical/one-pixel/shift/missing tests. **Budget:** 3 runs.

### BOOT-07 — UI test driver protocol
**Goal:** test UI through script before widgets exist. **Context:** product architecture. **Allowed:** app/test_driver, tests/ui, docs, minimal app loop. **AC:** JSONL open/resize/pointer/key/text-commit/frame/progress/accessibility/screenshot/save/quit; virtual clock + seed; invalid command line error; placeholder app outputs PNG and semantic tree; identical replay byte-identical. **Verify:** repeat and malformed input tests. **Non-goals:** full UI. **Budget:** 3–5 runs/3h.

### M0-04 — Product evidence packet
**Goal:** consolidate initial user/corpus signal, competition baselines, and open questions. **Context:** benchmark results plus owner-provided product evidence. **Allowed:** reports/M0, STATE, ledger. **AC:** report explicit evidence counts and source, no invented interview outcomes; request at least five representative files/tasks through the owner; disclose missing responses; include benchmark gaps and owner decision prompt. **Verify:** fresh session checks citation/artifact linkage. **Budget:** 2 runs. **Owner gate:** proceed or stop.

### M1-01 — Cocoa event/window shim
**Goal:** C app creates/resizes/closes native macOS window and emits normalized input. **Context:** official AppKit docs and platform contract. **Allowed:** src/platform/macos, headers, smoke tests. **AC:** arm64 debug/release build; resize/DPI/key/pointer events; C app logic only; 100 open-close cycles; UI driver controls lifecycle; clean quit. **Verify:** script and leak test. **Non-goals:** widgets/accessibility/dialogs/Metal. **Budget:** 3 runs/3h.

### M1-02 — Scalar tile + Metal blit
**Goal:** render deterministic CPU pixels and present through Metal. **Context:** M1-01; no graphics dependency. **Allowed:** src/raster, Metal shim, tests/goldens. **AC:** exact RGBA fixture; readback equals CPU tile; resize/reset safe; 1000 frames no validation error. **Verify:** headless golden and on-device readback. **Non-goals:** SVG parser/GPU compute. **Budget:** 4 runs/4h.

### M1-03 — Deterministic frame and replay
**Goal:** stable frame-step screenshots and event replay. **Context:** BOOT-06/07, M1-01/02. **Allowed:** driver/platform/tests. **AC:** virtual clock drives frames; 20 repeats same hashes; event order stable under scheduling jitter; no sleep synchronization. **Verify:** repeated replay. **Budget:** 3 runs.

### M1-04 — Cancel and teardown stress
**Goal:** bounded task cancellation and clean teardown. **Context:** platform/raster contracts. **Allowed:** lifecycle code/tests. **AC:** queued/running cancel; stale generation cannot publish; workers join <2 sec; 1000 cycles ASan clean; injected allocation failure safe. **Verify:** stress/sanitizer. **Budget:** 3 runs.

### M2-01 — Slices, checked arithmetic, arena
**Goal:** safe foundational byte/memory API. **Context:** C17 standard. **Allowed:** src/base and tests. **AC:** pointer/length invariants; overflow rejection; aligned arena bounds; failure tests; lifetime/thread README; sanitizer clean. **Verify:** property boundary tests. **Budget:** 3 runs.

### M2-02 — Bounded XML tokenizer/source spans
**Goal:** safely tokenize bounded XML with byte offsets and retained original source. **Context:** W3C XML 1.0 link and preservation ADR. **Allowed:** src/svg/xml, tests (not oracle), docs. **AC:** comments/PI/names/quoted attributes/text/CDATA/namespaces spans; explicit depth/node/attribute/text caps; no external entity/network; checked offsets; malformed input category+offset; no serializer. **Verify:** corpus + malformed generation + 60 sec ASan fuzz + comparison with pinned parser on valid corpus. **Non-goals:** DTD/SVG semantics. **Budget:** 5 runs/4h.

### M2-03 — Fuzz CI and regression retention
**Goal:** scheduled tokenizer fuzz with bounded resources. **Context:** M2-02 and BOOT-05. **Allowed:** fuzz targets/seeds, scheduled workflow/docs. **AC:** PR smoke + 10min scheduled run; limits; minimized/preserved crash plus stack fingerprint; no network mutation; regression maps to issue ID. **Verify:** known crash seed and timeout kill. **Budget:** 3 runs.

### M2-04 — Lossless no-op save
**Goal:** unchanged document saves byte-identically. **Context:** M2-02, preservation ADR. **Allowed:** source_save implementation/tests outside oracle/docs. **AC:** equality on metadata/namespaces/comments/whitespace/mixed line endings; parse failure never writes; temp+fsync+atomic replace; permissions preserved; injected write/rename errors. **Verify:** compare bytes/hash. **Non-goals:** edited serialization. **Budget:** 4 runs/3h.

### M2-05 — Rect/path subset and scalar render
**Goal:** decode root, rect and declared M/L/H/V/C/Q/A/Z path subset into CPU tile. **Context:** reference interface. **Allowed:** src/svg/shape, src/geom, src/raster, tests. **AC:** subset documented; unsupported syntax diagnostic/source retained; simple goldens pass; huge/nonfinite coordinates bounded; degenerate input safe; deterministic output. **Verify:** resvg comparison + sanitizer fuzz. **Non-goals:** CSS/strokes/gradients parity. **Budget:** 4 runs/4h.

### M3-01 — Progressive index and cancellation
**Goal:** display first supported visible element before complete index. **Context:** M2; benchmark protocol. **Allowed:** src/svg/index, loading UI, tests/bench. **AC:** per-frame work budget, cancel/stale generations safe, progress visible; proposed 10/100MB TTFP gates on named Mac; no frame block >100ms; 1GB safe explicit limit. **Verify:** scripted progress/cancel at 10/50/90%, repeated benchmarks. **Budget:** 4 runs/4h.

### M3-02 — Visible query and bounded cache
**Goal:** query only visible elements and maintain bounded tile cache. **Context:** incremental index. **Allowed:** src/spatial and raster/cache/tests. **AC:** query equals naive oracle; byte cap enforced; generation/scale invalidation; randomized differential tests; memory plateaus during pan; p95 goal recorded. **Verify:** pressure/property/perf artifacts. **Budget:** 4 runs.

### M3-03 — Worker scheduling and snapshots
**Goal:** parallel parse/render without mutable UI sharing. **Context:** concurrency ADR. **Allowed:** src/jobs, snapshots, tests. **AC:** bounded queue/backpressure, cancellation/generation tags, immutable snapshots, deterministic one-worker mode, randomized schedule tests, TSan where supported, join <2sec. **Verify:** 1000 open/cancel cycles. **Budget:** 4 runs/4h.

### M3-04 — Pan/zoom/fit/progress UI
**Goal:** custom-drawn viewer navigation with accessible progress/cancel. **Context:** driver/design tokens. **Allowed:** src/ui, app, driver/goldens. **AC:** drag pan, wheel/pinch zoom around cursor, fit, keyboard equivalents, semantic progress/cancel, golden screens, 10k items no per-frame allocations. **Verify:** scripted events/accessibility snapshot. **Non-goals:** selection/edit. **Budget:** 4 runs/3h.

### BOOT-08 — Size, performance and RSS gates
**Goal:** report binary, bundle, dependency, TTFP, pan and RSS. **Context:** verdict targets. **Allowed:** scripts/gates, CI, benchmark docs. **AC:** build/fixture/hardware metadata; raw+summary results; compare last green; two reproducible violations before fail to damp noise; stretch package sizes identified as stretch. **Verify:** injected regression/noisy run. **Budget:** 2–3 runs.

### M3-05 — Viewer proof and decision packet
**Goal:** consolidate exact evidence for owner continuation decision. **Context:** all prior artifacts. **Allowed:** reports/M3, STATE, ledger. **AC:** hashes, hardware, TTFP/stall/pan/RSS, source preservation, fuzz/sanitizer, competitor data/gaps, user signal, failures/confidence; fresh verifier checks every claim; gate script deterministic. **Budget:** 3 runs. **Human gate:** required before M4/M5.

## M4–M10 work-order titles and machine acceptance

- **M4-01 Geometry primitives:** affine/bounds invariants; property tests vs independent double precision.
- **M4-02 Path grammar/curves:** declared SVG commands parse with source offsets; reference fixture differential and bounded errors.
- **M4-03 Fills/antialiasing:** fill rules and deterministic raster pass goldens/tolerance.
- **M4-04 Stroke engine:** cap/join/dash corpus vs reference; degenerate fuzz; explicit unsupported limits.
- **M4-05 CSS/style subset:** cascade precedence tests; unsupported declarations preserved.
- **M4-06 Gradients/clip/use:** selected fixtures vs independent renderer; recursion/cycles bounded.
- **M4-07 Renderer bake-off:** PlutoVG vs scalar reference, license/build/fidelity/perf table and ADR.
- **M4-08 Fidelity gate:** coverage matrix and risk report; owner chooses fidelity scope.
- **M5-01 Stable handles:** stable IDs and stale handle rejection tests.
- **M5-02 Attribute overlays:** unknown subtrees/namespaces preserved; randomized invariants.
- **M5-03 Minimal-diff save:** only intended source ranges change; roundtrip oracle and write-failure tests.
- **M5-04 Recovery journal:** committed edits recover after forced crash; corrupt journal safely rejected.
- **M6-01 UI layout/paint:** deterministic screenshots across scale/resize.
- **M6-02 Focus/keyboard/semantics:** all core controls named, reachable and keyboard operable; snapshots pass.
- **M6-03 Native accessibility/text adapters:** Mac smoke build; owner performs real VoiceOver/CJK IME check before beta.
- **M6-04 Virtualized tree/XML inspector:** 100k rows remain within measured frame/RSS and stable semantic IDs.
- **M7-01 Hit testing:** randomized hit results agree with naive reference and z-order.
- **M7-02 Selection overlay:** scripted selection/handles screenshots and semantic states.
- **M7-03 Transforms:** move/scale/rotate operation properties and replay pass.
- **M8-01 Style controls:** supported edits match render and localized source delta.
- **M8-02 Layers/tree:** visibility/lock/order scripts and roundtrip preserve subtrees.
- **M8-03 Path node editing:** targeted path delta, undo and geometry suite pass.
- **M8-04 Workflow gate:** scripted task suite; owner reviews discoverability.
- **M9-01 Transactional undo/redo:** generated action sequences invert within model invariants; bounded history.
- **M9-02 Atomic save/save-as:** fault injection never corrupts prior valid file.
- **M9-03 Crash recovery:** termination at transaction boundaries restores latest committed state.
- **M10-01 Static corpus audit:** all supported fixtures pass; unsupported features reported/preserved; zero silent drops.
- **M10-02 Security/perf audit:** long fuzz, sanitizers, analyzer, RSS/perf and crash triage attached.
- **M10-03 Mac package proof:** signed/notarized package, SBOM, clean install/update smoke.
- **M10-04 Pilot gate:** five target pilots complete named tasks; owner decides continue/pivot/stop.
