# Project state

- Current work order: `WO-BOOT-05` revision 1 (READY; runner bootstrap repair committed, executor run pending).
- Last accepted implementation commit: `05dc456a90ef67a8d9a1f8344fd52fe000f1af9c` (`fix: parse complete work order path rules`); final closeout is on `main` after fast-forward.
- Working branch: `main`; BOOT-04 revision 2 is verified and merged.
- Platform: Apple Silicon macOS 13+ only, per `OWNER-DIRECTIVES.md`.
- Build: Debug, Release, size, and ASan/UBSan scaffold checks passed on Apple Clang 21.0.0; current `./dev verify` passes.
- Test inventory: 37 targeted ledger/runner tests pass at the verified candidate; 43 Python tests pass in the fresh verifier; no application C tests exist yet.
- Initial Release CLI: 33,568 bytes; median `--version` process latency 1.910 ms over 31 runs; max RSS 1,884,160 bytes (single `/usr/bin/time -l` run).
- BOOT-04 revision 2 is complete: frozen work-order validation, guarded task worktrees, JSONL session caching, resume identity checks, executor report validation, read-only exact-commit verification, path guards, and fake-Codex success/failure coverage. The owner-approved CLI amendment is documented in `docs/adr/0003-codex-cli-compatibility-amendment.md`.
- Installed CLI: `codex-cli 0.160.1`; executor preflight passes the owner-approved invocation contract. Approval bypass limitations are documented in the ADR.
- Latest checks: 37 targeted tests; fresh exact-commit verification ran all seven frozen commands, 43 full-suite tests, actionlint, and a read-only review with `verdict=pass`.
- BOOT-05 runner bootstrap repair scopes Codex preflight evidence to each ignored run directory; regression coverage confirms the root path guard accepts a new order without mutating BOOT-04 evidence.
- Next: execute BOOT-05 through `scripts/wo-run.sh` from the clean main branch.

Resume by reading `AGENTS.md`, this file, the active work order, and the r4 operating model. Preserve the owner-provided `OWNER-DIRECTIVES.md`.
