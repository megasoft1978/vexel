# Project state

## Current product continuation — 2026-10-13

- `WO-MCP-TRANSPORT-01` revision 1 is frozen READY at base
  `19234a036ed6a8d40c2ccdaec9b8186806d70eca`, spec SHA-256
  `8058c8295d94df7ab2381e9b3e3667ab65deb82eaadbf90102225259de08abd4`.
  It fixes bounded stdio multiplexing and adds focused public-relay tests for
  progress, cancellation, partial/coalesced lines, and response IDs. This is
  the prerequisite for async native export; it does not implement MCP tasks or
  export workers. Scope: `artifacts/WO-MCP-TRANSPORT-01/scope-freeze.txt`.

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
