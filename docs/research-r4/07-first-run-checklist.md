# First ten actions

1. **Owner:** confirm locked choices and set monthly Codex ceiling/alert. Record actual allowance shown by Codex /status and account dashboard; do not assume a public fixed limit.
2. **Sonnet:** freeze ADRs and M0 success/kill criteria from the verdict.
3. **Luna:** execute BOOT-01, scaffold C17/CMake, prove two clean builds.
4. **Luna, Sonnet reviews:** execute BOOT-02 for AGENTS.md, STATE.md, ADRs, WO template and ledger.
5. **Luna:** execute BOOT-03 for one-command build/test/verify and CI matrix; keep fast loop <30 seconds.
6. **Luna + fresh verifier:** execute BOOT-04; test wrappers with fake Codex, timeouts, schemas, worktrees/protected paths, then one low-risk authenticated smoke.
7. **Luna:** execute BOOT-05, BOOT-06, BOOT-07 in order: tests/sanitizers/fuzz → image oracle → UI driver. No app feature work before replay is deterministic.
8. **Luna:** execute M0-01 and M0-02 for licensed/hash-pinned corpus, stress generator and oracle. Do not fetch Inkscape source.
9. **Luna:** execute M0-03; the owner supplies any product interview/corpus evidence. Mark missing Donner evidence unavailable.
10. **Owner + Sonnet:** review M0 packet and decide whether to spend allowance on M1–M3; record continue/stop/pivot ADR.

## Minimum Mac prerequisites

- Apple Silicon Mac, supported current macOS, Xcode Command Line Tools and SDK; choose minimum OS before M1.
- Git, CMake, Ninja or Make, Python 3 stdlib, Clang/Apple toolchain; jq optional.
- Current Codex CLI authenticated in dedicated profile; pin version and confirm exec/resume, JSON schema, image input, workspace-write and read-only flags via help.
- Dedicated unprivileged macOS account or disposable VM; no secrets in checkout; network policy; disk for build/artifacts/fuzz corpus.
- GitHub Actions if remote CI is used; verify plan minutes/concurrency. Rates and runner inventory change.
- Owner-controlled usage cap and stop mechanism.

Before first code task: preflight CLI flags; verify clean git; prove sandbox writes only in task worktree; exercise timeout/schema with fake runner; check no shell interpolation/eval; verify network policy and account credential isolation.
