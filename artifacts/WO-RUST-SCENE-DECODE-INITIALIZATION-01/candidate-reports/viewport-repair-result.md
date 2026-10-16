Viewport provenance blocker repaired in the same frozen work order. Candidate remains pending root independent re-review, protected audit and fresh-main checks before acceptance/seeding.

`initialize_root(&mut canonical_source, root, control)` now accepts no caller-supplied viewport, attribute list or externally constructible proof. After canonical source/root and CSS display checks, it extracts only the five original relevant unprefixed attributes into fixed stack storage and invokes the accepted `vexel_svg_values::parse_root_viewport`. No XML replay, source copy, alternate parser, heap attribute Vec or graph rebuild is introduced. Original mapping error kind/offset is propagated; publication occurs only after successful parsing, validation and final cancellation checkpoint.

Extraction polls each actual token and admits selected value bytes in bounded batches through WorkControl. The accepted synchronous values parser remains an indivisible bounded operation; no latency guarantee is claimed. Hidden initialization never enters viewport extraction/parsing.

The original 16 initialization fixtures retain their meaningful ownership/accounting/priority/hidden/frame/request/media/planner/child assertions, with positive source fixtures authoring valid dimensions. Added runtime regressions cover direct missing/malformed/zero viewport rejection, all five attributes, namespace/viewBox/aspect diagnostics, canonical source/root binding, prefixed/child attributes, and extraction cancellation/work/overflow with unchanged-state retry. A compile-fail regression rejects the former forged-Viewport call. Prior candidate source/tests are preserved in the before-snapshot artifacts.

All required commands exited 0:

- `cargo +stable check -p vexel-scene -p vexel-svg-animation --locked --offline` — `viewport-repair-check.log`.
- `cargo +stable test -p vexel-scene -p vexel-svg-animation --locked --offline` — `viewport-repair-test.log`: **177 Scene + 21 animation**, including all accepted 156+21 checks, 19 initialization fixtures and both initialization compile-fail guards.
- `python3 scripts/wo_validate.py WO-RUST-SCENE-DECODE-INITIALIZATION-01 --ready` — `viewport-repair-wo.log`: frozen revision/hash valid.

Only initialization source, its new test file, README and own evidence changed during repair. Fourteen protected existing test files match the frozen baseline. Changed source/test lengths are 283/659 lines. Exact commands, logs, candidate hashes and API coverage are recorded in `viewport-repair-verification.json`.

Downstream decoder integration must call normal initialization after original supported-attribute/style/display/visible-transform validation, then parse/install the same-source animation after traversal/root_seen. Existing accepted constructor/signatures/literals, deferred child completion and pending-scope freeze guards remain preserved. Full decoder/renderer/native/product acceptance remains downstream.
