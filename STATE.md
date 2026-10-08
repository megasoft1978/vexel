# Project state

- Current work order: `WO-BOOT-04` revision 2 (implementation complete locally; independent verifier and ledger acceptance pending).
- Last accepted commit: `837e88acdc247c790b0b9785efed98f544584fcb` (`docs: record BOOT-03 verification`).
- Working branch: `main`; BOOT-04 changes are being prepared for a scoped commit and fresh read-only verification.
- Platform: Apple Silicon macOS 13+ only, per `OWNER-DIRECTIVES.md`.
- Build: Debug, Release, size, and ASan/UBSan scaffold checks passed on Apple Clang 21.0.0; current `./dev verify` passes.
- Test inventory: 30 targeted ledger/runner tests pass; 36 Python tests pass through `./dev verify`; no application C tests exist yet.
- Initial Release CLI: 33,568 bytes; median `--version` process latency 1.910 ms over 31 runs; max RSS 1,884,160 bytes (single `/usr/bin/time -l` run).
- BOOT-04 revision 2 implements frozen work-order validation, guarded task worktrees, JSONL session caching, resume identity checks, executor report validation, read-only exact-commit verification, and fake-Codex success/failure coverage. The owner-approved CLI amendment is documented in `docs/adr/0003-codex-cli-compatibility-amendment.md`.
- Installed CLI: `codex-cli 0.160.1`; executor preflight passes the owner-approved invocation contract. Approval bypass limitations are documented in the ADR.
- Latest checks: 30 targeted tests, `./dev verify`, ledger validation, actionlint, and `git diff --check` pass. Fresh independent review remains pending.
- Next: review the committed BOOT-04 candidate with the read-only verifier, record acceptance, then prepare BOOT-05.

Resume by reading `AGENTS.md`, this file, the active work order, and the r4 operating model. Preserve the owner-provided `OWNER-DIRECTIVES.md`.
