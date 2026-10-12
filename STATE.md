# Project state

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

- WO-BOOT-08 revision 5 is complete, independently verified, and merged to `main`. The clean read-only review `verify-db219b3f-a7e7-42af-8a67-fe06942ec5b1` passed all 14 frozen commands with no findings. Debug/Release builds pass; `./dev verify` reports CTest 11/11 and Python 191/191; ASan and UBSan CTest each pass 11/11; strict perf-gate tests pass 10/10.
- The local gate uses five cold-order plus ten warm-order trials on both 10 MiB and 100 MiB workloads. In the final comparable pair, 100 MiB first-preview p95 was 277.57/274.66 ms and passed the pinned relative threshold; peak RSS was 137,904,128/139,526,144 bytes and crossed the relative threshold twice, so run two reports `fail`. Absolute limits pass. The first max-step sample was noisy; the next was healthy. Release `svgworkbench` is 172,032 bytes with eight dynamic-library entries. App-bundle size and pan event-to-present p95 remain unavailable with explicit reasons. Evidence is under `artifacts/WO-BOOT-08/`, `artifacts/runs/WO-BOOT-08/verify-db219b3f-a7e7-42af-8a67-fe06942ec5b1/`, and `bench/results/WO-BOOT-08.json`.
- Revisions 1–4 were superseded only to resolve verifier scope-parser/base semantics; ADR-0009 through ADR-0011 document the amendments. No path guard was weakened. Next: owner review of the completed M3-05 packet; stop at its human gate. M4/M5 require the owner to choose proceed.

- WO-M3-05 revision 2 packet is complete and merged to `main` at `903f4eb5ac65e6e903df6f3409212b814135befe`; see `reports/M3/`; its offline validator checks 41 source hashes, derived claims, schema, and Markdown consistency. The final comparable BOOT-08 pair records 100 MiB first-preview p95 277,571,708/274,657,958 ns; peak RSS 137,904,128/139,526,144 bytes. The second RSS run fails the repeated 5% relative gate while both values remain below the 524,288,000-byte absolute limit. Pan event-to-present p95 and app-bundle size remain unavailable with reasons. All seven frozen machine commands passed in the clean verifier worktree; Debug and Release builds passed. Fresh read-only review `verify-a3acfe99-3db8-4bbb-b058-cbdc81188fd7` passed with no findings, validating all 41 hashes, derived values, allowed paths, and the open human choice. Evidence is under `artifacts/runs/WO-M3-05/verify-a3acfe99-3db8-4bbb-b058-cbdc81188fd7/`.
- M3-05 source preservation is 3/3 byte-equal round trips and failure atomicity is 4/4 cases. M2-03 fuzz completed 2,158,912 and 42,108,425 cases cleanly; M2-05 completed 1,000,000 cases with zero crashes or sanitizer reports. M3-04 review links two screenshots and three semantic snapshots; no live customer pilot is claimed.
- Owner files/tasks, interviews, user outcomes, and pilots remain 0; five representative real files/tasks are requested. The proceed/pause/pivot/stop decision remains OPEN. This packet does not authorize M4/M5; owner review is still required.

- WO-M3-06 revision 4 is independently verified and fast-forward merged to `main` at `c086461583b554f2d09a48570663381813280844` (candidate `1b28e8445aef01886d2419c912ea26d1b35b8bd1`). Fresh read-only review `verify-90542d7e-cd0f-4208-92f8-fe1969307c3c` passed with no findings; all seven frozen commands plus clean Debug/Release builds passed. Review validates all 58 corpus input hashes/sizes, 116 trials, the T1 derivations, and reported limitations.
- The 116 fresh Release trials found 2 supported previews and 56 unsupported outcomes, with no limits/crashes. Both previewable files are Natural Earth T1 maps. All 12 T3 files in the 80–120 MiB band have no supported preview, so the <2 s wedge fails. Maximum main-thread step is 279,583 ns and passes <100 ms; all 12 band files pass the M3-01 RSS allowance (3x source +128 MiB).
- Only T1 has successful comparable rows: Inkscape/Vexel geometric mean 84.447x for first-preview time and 33.711x for RSS. T2–T4 time/RSS ratios are unavailable because no Vexel rows succeeded. The task protocols differ; this is open/index evidence, not full-render parity. No Inkscape rows were rerun. Detailed per-file data and validation are in `reports/M3/head-to-head.md`, `.json`, and `head-to-head-validation.txt`.
- Build/tests: `./dev verify` passes CTest 11/11, Python 191/191, native smoke and 20-frame replay; ASan and UBSan CTest each pass 11/11; instrumented benchmark smoke passes under both sanitizers. WO-M3-06 closeout/merge is in progress. M3-05 owner proceed/pause/pivot/stop gate remains OPEN; stop after M3-06 merge and do not start M4/M5.

- Owner decision recorded in ADR-0012: proceed through M4/M5 using synthetic and public-corpus evidence; no customer pilot has occurred. Start with the M4 feature priority inventory at `reports/M4/corpus-feature-frequency.md` and `.json`; the inventory streamed all 58 external corpus files (4,429,240,515 declared bytes) with no parse failures. It ranks common shapes/group transforms, paths, fill/stroke and CSS/class handling, local references/gradients/clips, then text/images. Corpus inputs remain read-only.

- WO-M4-01 revision 1 is complete and fast-forward merged to `main` at `c7aa4c9fa2966b90deaa08e4cb848c78db29de1b`. Seven fresh read-only reviews were run; six rounds surfaced correctness/evidence findings, all corrected, and the final review passed with no findings. `./dev verify` passes CTest 11/11, Python 192/192, native smoke and 20-frame replay; ASan and UBSan CTest pass 11/11; seeded ASan+UBSan fuzz passed 1,000,000 cases in 24.382 s. Real-corpus report covers 58 files/116 trials: 2 previewable, 56 unsupported, no limits/crashes. The 100 MiB preview gate fails; max main-thread step and RSS gates pass. This is open/index evidence, not full-render parity. Evidence: `artifacts/WO-M4-01/`, `artifacts/runs/WO-M4-01/verify-0f4141d8-d85e-4b74-898c-e72cf7ec88d4/`, `reports/M4/`. Active next work order: WO-M4-02, path grammar and curve coverage.

- WO-M4-02 revision 1 was superseded before execution to correct the shape-fuzz artifact destination; revision 2 was frozen READY at `b6c496f3ca9e1d22946015ad74757d2fefea0097` (spec SHA-256 `2fd9975b8de99eb7e2149011839ccdc163f76341b9bb9b820435188ea462580d`).
- WO-M4-02 revision 2 is complete, independently verified, accepted, and fast-forward merged to `main` at `2d19d27960f524c9ff7fd888185ec7f7b3ca295f`. Fresh read-only review passed with no findings in 58.953 s. Direct checks pass: Debug/ASan/UBSan CTest 11/11, `./dev verify` Python 192/192 with native smoke and 20-frame replay, focused tests 9/9, 1,000,000 ASan+UBSan fuzz cases, ten librsvg fixtures, and corpus/inventory checks. Corpus remains 2 previewable, 56 unsupported, no limits/crashes; this is open/index evidence. The initial managed-runner attempt and its sandbox Metal/process-policy failures are retained separately; direct checks pass. Evidence: `artifacts/WO-M4-02/` and `artifacts/runs/WO-M4-02/verify-bb6f216a-27f7-40a3-8a61-45c5a90b2fce/`. Next: WO-M4-03 fill rules and deterministic antialiasing.

- WO-M4-03 revision 3 is complete, independently verified, and fast-forward merged to `main` at `bf006b4833e66533bcf8eaef45dad8119332eb8b`. The final fresh read-only review passed with no findings; all 18 frozen commands pass. Debug/ASan/UBSan CTest each pass 11/11, `./dev verify` reports Python 192/192 with native smoke and 20-frame replay, the 17 librsvg fixtures meet the raw jointly-covered RGB threshold, and ASan+UBSan fuzz completed 1,000,000 cases in 19.081 s. Release paired benchmarks retain all trials and report median changes of +14.34% simple, +63.16% curved, +77.50% multi-shape; candidate peak RSS is 2,031,616 B. Reruns show timing variance, recorded in `docs/PERF.md`. Corpus remains 2/58 previewable and 56 unsupported; the 100 MiB first-preview gate fails, max-step and RSS gates pass. This is open/index evidence, not full-render parity. Evidence: `artifacts/WO-M4-03/`, `artifacts/runs/WO-M4-03/verify-8cc0b01f-0f50-42bb-9623-41d21d18bc69/`, `reports/M4/`.
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

- WO-M4-05 revision 1 is implemented in candidate `9209ae3b670911470100e97b9538daf83dfa509d`; a fresh read-only review passes with no findings after fixes for CSS comment delimiters, inline-important coverage, and stale verification text. All 19 frozen acceptance commands pass in a fresh clean worktree after the three verifier build prerequisites. Debug/ASan/UBSan CTest 12/12 each; Python 199/199; native smoke and 20-frame Metal replay; 16 focused reference tests; 1,000,000 sanitized fuzz cases with zero reports; Release build; and 58-file corpus run/check (2 previews, 56 unsupported, 0 limit/crash). CSS benchmarks: 21 trials/workload, 27.890/29.662 ms stylesheet median/p95 and 27.483/29.013 ms inline; 2,113,536/2,146,304 B RSS; stable shared output SHA. Open/index evidence only; no comparable pre-WO CSS benchmark. 100 MiB first-preview gate still fails for 0/12 files; maximum main-thread step 0.199 ms and RSS gate pass. The standard verifier's fixed 250 KB diff cap required a focused manual read-only review after it completed all frozen commands. Evidence: `artifacts/WO-M4-05/`, including `verifier-round-02.json` and its clean-worktree command summary.
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
- Update 2026-10-09: WO-M4-06 merge-integration brief 6 removes the blanket `mark_varying_shape_bounds` sampling mask. The three previously failing librsvg fixtures are byte-identical across scratch builds at `0732b8e`, `9bae63e`, and the repaired tree; all 47 comparisons pass unchanged thresholds. The test reference shares the boundary-map sampling decision but still visits every shape and edge. Culled/reference identity passes for the three fixtures, alpha-gradient overpaint, and 120 seeded paint/clip documents. Debug/Release, `shape_test`, focused Python 20/20, and 1M ASan+UBSan fuzz (seed 20261014) pass. Debug/ASan/UBSan/verify CTest are 18/20 due LaunchServices and offscreen Metal; `./dev test-fast` is 202/212 due sandbox benchmark/platform tests. Corpus run/check remains 2 previews / 58 and all hashes unchanged. Four solid paired benchmarks are byte-identical and within noise (+0.72%, +1.42%, -0.46%, -0.13%). Git metadata remains unchanged; post-merge runner guard remains unverified. Evidence and current caveats are in `HANDOFF.md` and `artifacts/WO-M4-06/`.
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
