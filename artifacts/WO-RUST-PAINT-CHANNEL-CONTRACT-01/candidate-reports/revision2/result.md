# Revision 2 result: BLOCKED candidate

Implemented the shared canonical owner/element equality guard before either
lower planner. Existing owner/style/source error priority is preserved. Public
signatures/literals are unchanged. The previous new-test prefix is SHA-256
verified unchanged, retaining combined nonpaint dynamic dispatch and all eight
opacity publication cases.

Added both-route canonical mismatch, zero provider entry, unchanged scene and
ledger accounting, real cancellation/invalid-bounds rejection, and successful
matching-element real mapped request/job/PatternId Fill and Stroke publication.
The three new-target tests pass.

Fresh final required commands after restoring the candidate guard:

- `cargo +stable check -p vexel-scene --locked --offline`: exit 0.
- `cargo +stable test -p vexel-scene --locked --offline`: exit 101.
- `python3 scripts/wo_validate.py WO-RUST-PAINT-CHANNEL-CONTRACT-01 --ready`: exit 0; frozen revision 2 SHA validates.

Exact logs and timestamps: check.log, test.log, wo.log, verification.json.
Additional `--no-fail-fast` run: 155 passed, 1 failed including doctests (original153:
152 passed, 1 failed; new target: 3 passed).

Frozen failure:
`actual_mask_planner_shared_validation_nonempty_expansion_wrong_flavor_and_source_coherence`
unwraps InvalidArgument offset 0 at render_integration_contracts.rs:616:57.
Guard-only removal diagnostic passes that exact test; final source restores the
required guard. Protected test/source was neither read nor edited. Weakening the
identity guard or modifying frozen assertions would violate the order, so full
acceptance cannot be claimed under the current constraints.

Independent Luna High scoped review passed; overall acceptance is BLOCKED.
See luna-review.md, APIcoverage.md, protected-audit.md and integrity.json.
Coordinator protected-baseline audit and integrated fresh-main checks remain
mandatory/pending; the main ref check alone is not fresh-main verification.
No integration, stage/commit, ledger/state changes or purchased credits.
