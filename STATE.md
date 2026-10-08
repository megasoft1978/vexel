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
- WO-M1-01 revision 1 is superseded: its allowlist omitted the focused test module path, so the frozen verifier rejected the candidate scope. The tested implementation is committed as `d3c1bcf6815fca24e59bc67ffd4d47eef03cc6e7`; revision 2 will correct the allowlist and re-run exact-candidate verification. Revision 1's initial nested sandbox failures and successful host reruns are recorded in its artifacts.
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
