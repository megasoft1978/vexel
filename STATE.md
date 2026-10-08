# Project state

- Current work order: `WO-BOOT-HARDEN-01` revision 1 implementation and local acceptance checks pass; fresh exact-commit verification is pending.
- BOOT-05 revision 3 is accepted and merged at `178f0e57554cf1eae8182a9cf89330297b647ffb` (merge includes the previously verified candidate and the equivalent root-scope path matcher cleanup).
- Working branch: `main`; no active task worktree.
- Platform: Apple Silicon macOS 13+ only, per `OWNER-DIRECTIVES.md`.
- Build: Debug, `mac-asan` (AddressSanitizer + UndefinedBehaviorSanitizer), and `mac-ubsan` configure/build/CTest pass with Apple Clang 21.0.0.
- Test inventory: latest `./dev verify` reports 52 Python tests pass with zero failures or skips; targeted ledger/runner/BOOT-05 tests and CTest pass.
- BOOT-05 evidence: `artifacts/WO-BOOT-05/`; final fresh verification reran all ten required commands. Fuzz seed `12648430` completed 144 invocations in 3.001 seconds; assertion and heap-overflow probes were detected.
- Strict executor schema generation enumerates all nine frozen acceptance IDs and preserves runtime exact-key and command-evidence validation.
- BOOT-04 revision 2 remains complete and merged. Its preflight transcripts remain preserved as compressed raw artifacts plus readable copies with trailing whitespace normalized for the required diff check.
- The new fuzz workflow passed local policy checks and actionlint; no hosted GitHub run was available in this checkout.
- The runner's recursive `/**` path matching is covered and fixed. The fuzzer accepts exact valid help output, uses a fixed 48-case-per-second budget, and stops at the requested deadline with a 0.1-second per-case grace. Old 51-test output is labeled historical under `artifacts/WO-BOOT-05/history/`.
- The three earlier review failures and the final passing review are retained in `artifacts/WO-BOOT-05/reviews/` and ignored per-run evidence under `artifacts/runs/WO-BOOT-05/`. The final review reported 52 tests passing, zero failures or skips; all ten commands passed.
- No hosted GitHub workflow run was available. Initial Release CLI baseline remains 33,568 bytes; median `--version` latency 1.910 ms over 31 runs; max RSS 1,884,160 bytes (single run).
- Hardening work order source `SECURITY-FINDINGS.md` is frozen by SHA-256 `a156b5187b6741385a1fcfe31a8330e0ac257120c8f963ddc342f3fb732dfc92`; its READY spec hash is recorded in `work/WO-BOOT-HARDEN-01.md`.
- Hardening checks: 38 focused runner tests and 59 full fast tests pass, with zero failures/skips; workspace-write default, read-only verifier, frozen source snapshot, unchanged active root-copy acceptance, and candidate/root hard-deny path tests pass.
- First independent review attempt stopped at CLI preflight because `exec resume --help` omits the inherited short sandbox alias; the preflight now validates it on `exec --help`. The substantive review findings are addressed by a positive active-order scope test and by supplying the validated snapshot directly in the executor prompt.
- Next after hardening acceptance: M0-01 corpus provenance/fetch. Continue through M0-04 and stop for its owner gate.
