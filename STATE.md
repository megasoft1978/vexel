# Project state

- Current work order: `WO-BOOT-05` revision 3 implementation and wrapper validation pass; the first fresh verifier found stale retained test/fuzz evidence and a valid-help false positive. Those findings are fixed; final review is pending.
- Candidate implementation commit: `8b4fccb1a3769cdda6c084c9cf8b8a186f40eafe` (`test: add sanitizer and fuzz foundation`), with runner/path and fuzz fixes on the task branch.
- Working branch: `wo/WO-BOOT-05` in its isolated task worktree; not merged.
- Platform: Apple Silicon macOS 13+ only, per `OWNER-DIRECTIVES.md`.
- Build: Debug, `mac-asan` (AddressSanitizer + UndefinedBehaviorSanitizer), and `mac-ubsan` configure/build/CTest pass with Apple Clang 21.0.0.
- Test inventory: latest `./dev verify` reports 52 Python tests pass with zero failures or skips; targeted ledger/runner/BOOT-05 tests and CTest pass.
- BOOT-05 evidence: `artifacts/WO-BOOT-05/`; refreshed fuzz seed `12648430`, 193 invocations in 3.001 seconds; assertion and heap-overflow probes detected.
- Strict executor schema generation enumerates all nine frozen acceptance IDs and preserves runtime exact-key and command-evidence validation.
- BOOT-04 revision 2 remains complete and merged. Its preflight transcripts remain preserved as compressed raw artifacts plus readable copies with trailing whitespace normalized for the required diff check.
- The new fuzz workflow passed local policy checks and actionlint; no hosted GitHub run was available in this checkout.
- The runner's recursive `/**` path matching is now covered and fixed; the fuzzer accepts exact valid help output without treating it as an error. Retained test and fuzz JSON now match the reviewed candidate.
- Next: run `wo-verify.sh` on the refreshed exact candidate and merge only after it passes.
