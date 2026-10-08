# Project state

- Current work order: none. `WO-BOOT-01` through `WO-BOOT-03` are complete and verified.
- Last accepted commit: `278c2270341a8eeda6e3538773c7e076257c5a69` (`docs: establish agent operating state`).
- Working branch: `main` (BOOT-04 worktree tooling does not exist yet).
- Platform: Apple Silicon macOS 13+ only, per `OWNER-DIRECTIVES.md`.
- Build: Debug, Release, and ASan/UBSan smoke builds pass on Apple Clang 21.0.0.
- Test inventory: 14 Python ledger/CI tests pass; no application C tests exist yet.
- Initial Release CLI: 33,568 bytes; median `--version` process latency 1.910 ms over 31 runs; max RSS 1,884,160 bytes (single `/usr/bin/time -l` run).
- Blockers: none identified.
- Next three work orders: `BOOT-04`, `BOOT-05`, `M0-01`.

Resume by reading `AGENTS.md`, this file, the assigned work order, and the r4
operating model. The owner-provided `OWNER-DIRECTIVES.md` is currently present
in the workspace as an untracked file and must be preserved.
