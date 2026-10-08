# Project state

- Current work order: `WO-BOOT-05` revision 3 local implementation and wrapper validation pass. The third fresh review returned two stale-history findings; those records are corrected below and the local suite is refreshed. Final independent review is still required.
- Candidate implementation commit: `8b4fccb1a3769cdda6c084c9cf8b8a186f40eafe` (`test: add sanitizer and fuzz foundation`), with runner/path and fuzz fixes on the task branch.
- Working branch: `wo/WO-BOOT-05` in its isolated task worktree; not merged.
- Platform: Apple Silicon macOS 13+ only, per `OWNER-DIRECTIVES.md`.
- Build: Debug, `mac-asan` (AddressSanitizer + UndefinedBehaviorSanitizer), and `mac-ubsan` configure/build/CTest pass with Apple Clang 21.0.0.
- Test inventory: latest `./dev verify` reports 52 Python tests pass with zero failures or skips; targeted ledger/runner/BOOT-05 tests and CTest pass.
- BOOT-05 evidence: `artifacts/WO-BOOT-05/`; refreshed fuzz seed `12648430`, 144 invocations in 3.010 seconds; assertion and heap-overflow probes detected.
- Strict executor schema generation enumerates all nine frozen acceptance IDs and preserves runtime exact-key and command-evidence validation.
- BOOT-04 revision 2 remains complete and merged. Its preflight transcripts remain preserved as compressed raw artifacts plus readable copies with trailing whitespace normalized for the required diff check.
- The new fuzz workflow passed local policy checks and actionlint; no hosted GitHub run was available in this checkout.
- The runner's recursive `/**` path matching is covered and fixed. The fuzzer accepts exact valid help output, uses a fixed 48-case-per-second budget, and stops at the requested deadline with a 0.1-second per-case grace. Old 51-test output is labeled historical under `artifacts/WO-BOOT-05/history/`.
- The three review verdicts and their findings are retained under `artifacts/WO-BOOT-05/reviews/`. Review rounds 1–3 all returned `fail`; the last findings were stale summary counts and an ambiguously named historical log. These are now corrected, and `./dev verify` reports 52 pass / 0 fail / 0 skip; the retained fuzz result is seed `12648430`, 144 cases, 3.010 seconds.
- Next: start a fresh read-only verifier on the latest clean task-branch commit. If it passes, record acceptance and merge; otherwise fix its findings before merging. No hosted GitHub run was available.
