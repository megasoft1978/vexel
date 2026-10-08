# Project state

- Current work order: `WO-BOOT-05` revision 3 completed; all nine local acceptance criteria pass.
- Last accepted implementation commit: `8b4fccb1a3769cdda6c084c9cf8b8a186f40eafe` (`test: add sanitizer and fuzz foundation`).
- Working branch: `wo/WO-BOOT-05` in its isolated task worktree; not merged.
- Platform: Apple Silicon macOS 13+ only, per `OWNER-DIRECTIVES.md`.
- Build: Debug, `mac-asan` (AddressSanitizer + UndefinedBehaviorSanitizer), and `mac-ubsan` configure/build/CTest pass with Apple Clang 21.0.0.
- Test inventory: 50 Python tests pass with zero failures or skips in 2.565 seconds; 44 targeted ledger/runner/BOOT-05 tests pass. CTest includes the CLI `--version` smoke.
- BOOT-05 evidence: `artifacts/WO-BOOT-05/`; fuzz seed `12648430`, 226 invocations in 3.011 seconds; assertion and heap-overflow probes detected.
- Strict executor schema generation enumerates all nine frozen acceptance IDs and preserves runtime exact-key and command-evidence validation.
- BOOT-04 revision 2 remains complete and merged. Its preflight transcripts remain preserved as compressed raw artifacts plus readable copies with trailing whitespace normalized for the required diff check.
- The new fuzz workflow passed local policy checks and actionlint; no hosted GitHub run was available in this checkout.
- Next: integrate the accepted BOOT-05 candidate from its task branch; no release or deployment is authorized by this work order.
