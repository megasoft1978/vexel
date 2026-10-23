# Claude Code instructions

Project instructions are in [AGENTS.md](AGENTS.md). Read it and the active work
order before making changes. `OWNER-DIRECTIVES.md` has the highest priority.

## Orchestration

- Verify: `cargo check --workspace --locked`; `cargo test --workspace --locked`; `python3 -m unittest tests.test_ledger tests.test_wo_runner`
- Browser/UI test: n/a (native macOS app, no browser UI; broad UI/E2E deferred to final stage per OWNER-DIRECTIVES.md)
- Push policy: commit+push when review CLEAN and tests pass
- Project overrides: Sol Medium only for a named hard decision, repeated failed implementation, or final review; one writer per module; coordinator alone changes manifests, locks, interfaces, ledgers.
