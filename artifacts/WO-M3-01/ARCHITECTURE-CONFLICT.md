# WO-M3-01 initial runner diagnostic (superseded)

## Historical context

The first runner attempt stopped before implementation. This file preserves
its diagnostic and is not the current work-order status.

## Initial assessment

The required incremental reader cannot be safely implemented as a wrapper
around the existing one-shot scanner. In `src/svg/xml/xml_tokenizer.c`,
`tokenize` first scans all source characters and then invokes token parsers.
`parse_comment`, `parse_pi`, `parse_text`, `parse_cdata`, `parse_end`, and
`parse_start` keep their parser positions in local variables and return a
malformed-input error when a token crosses the end of the current input view.
The open-element stack and result counters live in `xml_scanner`, but the
partial lexical state (including an attribute quote, comment terminator,
CDATA terminator, entity reference, and partial name) does not.

Passing a shorter prefix to the one-shot API would therefore report valid
incomplete tokens as malformed and discard useful state. Passing the full
source would read beyond the 64 KiB step budget. Retrying prefixes would
re-scan the prefix on each call, exceed the work budget, and cannot guarantee
first-preview-before-full-index behavior. Reworking each parser into an
explicit resumable state machine is required to preserve all M2 diagnostics,
offsets, token equivalence, and quote/comment/CDATA states.

Per the frozen work order's failure and escalation instruction, no index code
was written before resolving this architectural conflict. This is not a
claim that a state-machine implementation is impossible; it is the specific
reason a safe incremental implementation cannot be made as a small extension
to the current scanner contract.

## Proposed narrowly scoped amendment

Before implementation, accept a separate incremental scanner contract that:

1. Adds an opaque `svg_xml_reader` with `create`, `step(byte_budget)`,
   `progress`, `diagnostic`, and `destroy` operations while leaving
   `svg_xml_tokenize` and its defaults unchanged.
2. Defines retained partial-token lexical states and a precise budget rule:
   each input byte is examined at most once by the reader, and one step may
   examine no more than the supplied bytes, including delimiters and UTF-8
   validation.
3. Defines how tokens are exposed before a complete containing element is
   known, so callers cannot observe scratch pointers or incomplete source
   spans; completed tokens continue to use absolute borrowed-source offsets.
4. Pins exact M2 token and diagnostic equivalence on the existing fixtures,
   plus chunk-boundary tests for every scanner state and diagnostic category.
5. Clarifies whether the 1 GiB reader cap supersedes only reader input limits,
   while all one-shot limits remain unchanged, and assigns allocation limits
   for retained tokens/index entries.

The index/session and task-queue implementation should begin only after this
scanner contract is accepted. `docs/adr/**` is protected by the supplied work
order, so this proposal is evidence only and does not modify an ADR.

## Verification evidence

The sequential command transcript is in `full-checks.txt`. On the named Mac,
Debug build and CTest pass (5/5), ASan and UBSan CTest pass (5/5 each), Release
configures/builds, ledger validation passes (222 entries), and `git diff
--check` passes. The focused index test module, benchmark command and benchmark
JSON are absent. `./dev verify` runs 173 tests and reports 163 passed, 8
failed, 2 errors, 0 skipped. Exact identities of those failures were not
emitted by the frozen test runner summary. Benchmark gates and progressive UI
criteria remain unmeasured.

The ready-WO validator was not invoked because its implementation loads the
task-worktree work-order file, which the user explicitly prohibited reading
or trusting. The requested per-run report path is outside this worktree's
writable filesystem root; no report was written there.
