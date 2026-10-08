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
- Active: `WO-BOOT-06` revision 2 is READY from base `dfdfab99c062a70016fda7618cda87b305e54287`. Revision 1 was superseded because its verification heading was incompatible with the runner’s required `# Verification commands` block. Its first full-suite attempt reported benchmark sandbox failures; serial rerun passed all 122 tests.
- BOOT-06 implementation and focused evidence are present in its task worktree. R2 required commands passed sequentially; candidate still needs rebase onto the r2 freeze commit, fresh independent verification, and merge closeout.
- Next roadmap after BOOT-06: BOOT-07, then prepare the M0-04 evidence packet and stop at its owner gate.
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
