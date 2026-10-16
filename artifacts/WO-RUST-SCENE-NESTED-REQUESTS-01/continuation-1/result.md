# Continuation 1 result

Status: **stopped before implementation; frozen acceptance conflict and unavailable offline dependency prevent compliant execution.**

## Blocking conflict

The immutable existing test `rust/crates/vexel-scene/tests/decode_publication_contracts.rs::request_preserves_parent_context_with_real_nested_child_drafts` asserts at line 571 that `begin_request(&nested, ...)` fails while an outer request is active. Revision 1 requires nested requests to succeed for production nested patterns, and its required proof demands at least three nested request scopes. These are opposite outcomes for the same public API state. The work order protects existing tests/assertions and prohibits weakening them. There is no implementation that can satisfy both requirements without changing protected assertion semantics or falsifying behavior.

## Baseline check

Ran the required focused command before editing:

`cargo +stable test -p vexel-scene --locked --offline request_preserves_parent_context_with_real_nested_child_drafts -- --exact --nocapture`

Cargo exited before compilation: the offline registry lacks `objc2-image-io`, required by workspace package `vexel-platform-apple`. Exact output is in `baseline-focused.log`.

## Integrity

No Scene source or existing test was edited. The hash inventory is in `source-and-protected-hashes.txt`. Current main Scene API source hashes are in `main-scene-api-hashes.txt`. The decoder continuation report was used only as context. No tests, formatter, validator, or implementation checks are claimed as passed.

Implementation, current checks, evidence coverage, independent review, and acceptance-main checks were not run because the contradiction is in the frozen work order contract and its protected existing assertions. Required resolution: revise the work order or accepted test contract so nested `begin_request` has one unambiguous expected behavior; also make the locked offline dependencies available if execution must remain offline.
