Implemented the additive ownership and view support as an uncommitted candidate.

`parse_owned` uses the accepted parser, moves its tables, and clones the source `Arc` only on success. Borrowed and owned documents expose immutable views, and `evaluate` and `evaluate_view` share the same evaluator. Checked storage accounting includes source capacity once and the actual capacities of the track, diagnostic, key, motion-point, and spline vectors.

The original nine acceptance tests remain, with three added tests covering source and table retention, failure owner counts, worker moves, capacity accounting, and evaluation parity. **All 12 tests pass.**

- `cargo +stable check -p vexel-svg-animation --locked --offline` — passed
- `cargo +stable test -p vexel-svg-animation --locked --offline` — passed; 12 integration tests, 0 failed; doc tests passed
- `python3 scripts/wo_validate.py WO-RUST-SVG-ANIMATION-OWNED-01 --ready` — valid, revision 1
- `python3 scripts/ledger.py validate ledger.jsonl` — valid, 1315 entries
- `git diff --check` — passed

Candidate notes are in [candidate-report.md](/Users/megasoft78/Desktop/Freelance/vexel/.worktrees/rust-animation-owned/artifacts/WO-RUST-SVG-ANIMATION-OWNED-01/candidate-report.md). Independent review is still required before acceptance. No shared ledgers or unrelated files were edited.