Candidate implemented and locally verified. Coordinator independent LunaHigh review, protected audit and fresh-main checks remain required before acceptance or seeding.

`SceneBuilder::provisional` retains the canonical source without a viewport or animation. Explicit root initialization validates source/root identity and canonical CSS display. Hidden initialization skips transform/viewport/descendants, rejects visible or invalid-display roots, and has separate pending/finalized states. Normal viewport validation stays strict.

`TraversalView` shares existing storage and table algorithms with ready `SceneView`; optional viewport/animation values expose unresolved state honestly. Its type cannot enter existing frame/render APIs. Existing constructor, ready APIs, literals and all previous test sources remain intact. Animation source/API changes were unnecessary.

`finalize_decode` admits the actual same-source, matching-generation animation once. Checked actual-capacity accounting subtracts existing source capacity once and validates live/peak/history/count bounds before publication. Wrong source, duplicate, cancellation, work and budget failures return the unchanged animation owner and preserve builder admission state. Existing drafts/requests retain their identities across installation; freeze rejects pending scopes. A deferred child request can render finalized child input, complete through the original request, then close scopes and freeze.

`freeze_decode` distinguishes drawable graphs from immutable `HiddenScene`, whose source, actual animation diagnostics, empty queries and hidden frame remain available without manufactured dimensions or a drawable conversion.

API coverage exercises actual lexical/CSS/value/path/animation operations, shape/image/text ownership and source-map validation, typed definition owners, production pattern planning, child drafts/sealing, mapped requests/completion, stale handles, rollback, late parser errors and normal frames. The downstream decoder must preserve supported-attribute/style/display/transform/viewport/root_seen priority, parse animation after successful traversal using the same lexical source/tokens, use traversal reads in lower planners, and finalize before rendering. Its implementation remains downstream.

Verification (all exit 0):

- `cargo +stable check -p vexel-scene -p vexel-svg-animation --locked --offline` — [check-final-3.log](check-final-3.log).
- `cargo +stable test -p vexel-scene -p vexel-svg-animation --locked --offline` — [test-final-4.log](test-final-4.log): 173 Scene + 21 animation checks; prior 156+21 preserved, 16 new integration tests and one new compile-fail guard.
- `python3 scripts/wo_validate.py WO-RUST-SCENE-DECODE-INITIALIZATION-01 --ready` — [wo-final-2.log](wo-final-2.log), frozen revision/hash valid.

[verification.json](verification.json) records counts/API coverage. [worker-baseline-comparison.json](worker-baseline-comparison.json) records candidate hashes and comparison against 1,717 baseline files: no missing or unexpected changed paths; 1,681 protected entries unchanged. Maximum Scene source/new-test length: 720 lines. Earlier failed development logs remain preserved. This establishes contract evidence; complete renderer/native app/performance/UI acceptance remains downstream.
