# Project state

- Current work order: none; `WO-BOOT-01` is complete and verified.
- Last accepted code commit: `beb0cdd` (`build: scaffold macOS C17 project`).
- Working branch: `main` (BOOT-04 worktree tooling does not exist yet).
- Platform: Apple Silicon macOS 13+ only, per `OWNER-DIRECTIVES.md`.
- Build: Debug, Release, and ASan/UBSan smoke builds pass on Apple Clang 21.0.0.
- Test inventory: no test suite exists yet; BOOT-01 has CLI acceptance checks.
- Initial Release CLI: 33,568 bytes; median `--version` process latency 1.997 ms over 31 runs; max RSS 1,884,160 bytes (single `/usr/bin/time -l` run).
- Blockers: none identified.
- Next three work orders: `BOOT-02`, `BOOT-03`, `BOOT-04`.

Resume by reading `AGENTS.md`, this file, the assigned work order, and the r4
operating model. The owner-provided `OWNER-DIRECTIVES.md` is currently present
in the workspace as an untracked file and must be preserved.
