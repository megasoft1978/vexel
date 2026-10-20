# Agent rules

Read [OWNER-DIRECTIVES.md](OWNER-DIRECTIVES.md) first (esp. #6 lean process); it overrides conflicting roadmap and tooling text. Current status: [STATUS.md](STATUS.md). Follow the slice's allowed and protected paths.

## Locks

- Apple Silicon macOS 13+ only. No Linux/Windows code, shims, CI, packaging or stubs.
- Rust only. No C17 resumption; legacy code is at tag `legacy-c-final`. See ADR-0022.
- Never read, fetch or copy GPL SVG-editor source. Do not fetch external SVG resources from the app. Record provenance for external algorithms.
- Add dependencies only through an ADR and pinned provenance record.
- Preserve unknown SVG/XML source. Keep source files under 800 lines.

## Work

- Never weaken frozen acceptance tests or oracles. Do not invent test or performance results.
- Coordinator alone changes shared manifests, locks, interfaces and ledgers. One writer per module.
- Do not install system packages or touch sibling directories. Generated files go in ignored `build/` or `.cache/`.

## Verify

- `cargo check --workspace --locked`, `cargo test --workspace --locked`
- `python3 -m unittest tests.test_ledger tests.test_wo_runner`
- `python3 scripts/ledger.py validate ledger.jsonl`; `python3 scripts/wo_validate.py --ready WO-ID`
- Browser/UI test: n/a; broad UI/E2E deferred per OWNER-DIRECTIVES.md.

## Orchestration

- Protocol: `~/.codex/AGENTS.md`. Sol Medium only for named hard decisions.
- Push: commit and push when review is CLEAN and tests pass.
