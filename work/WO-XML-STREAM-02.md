---
id: WO-XML-STREAM-02
revision: 1
status: READY
milestone: M7
owner: project-owner
executor: codex-gpt-6.1-sol
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
parallel_safe: false
network: false
spec_sha256: 8bb100b93d868417d7eb36768ee314f2427d3c493999270d8aa0ea30554990f9
---
# Goal
Add a bounded discard-mode visitor to the resumable owned XML tokenizer and refactor attribute entity decoding into a shared public XML API. This enables large native project scenes to collect IDs and validate animation targets without retaining a document-wide token vector.

# Context pointers
- Read `AGENTS.md`, `OWNER-DIRECTIVES.md`, `STATE.md`, `docs/CODING_STANDARD.md`, `docs/testing.md`, XML tokenizer architecture and incremental stream tests.
- WO-ANIM-PROJECT-03 revision 1 is superseded before implementation because current full-token APIs cannot meet its 256 MiB scene inventory budget.
- WO-XML-STREAM-01 is superseded before implementation: its allowlist omitted the private decoder call site, and stream start/attribute records are provisional until tag closure.
- No GPL editor source, no UI/E2E suite.

# Contracts
- Apple Silicon macOS 13+, C17, no dependencies, pointer+length ownership.
- Preserve existing token-vector and stream APIs byte-for-byte in behavior and diagnostic categories/offsets.
- New visitor mode is resumable, cancellation-aware, and discards tokens; retained tokenizer storage is O(depth + maximum attributes on one element), independent of document token count.
- Callback source spans borrow immutable input and remain valid only for that input lifetime. Callbacks receive complete validated start/empty tags, emitted as start/empty token followed by attributes in source order.
- Earlier callbacks may have occurred when later document syntax fails; visitor callers must treat the final stream status as authoritative. A failed/canceled stream is not a successfully validated document.
- Entity decoding supports the same five predefined entities and decimal/hexadecimal numeric references as current shape text decoding. No recursive expansion or external resource access. Return exact input error offset and checked output capacity.

# Required implementation
1. Add a token visitor and context to a distinct resumable visitor stream API. It must not allocate/grow a document-wide token vector. Preserve existing full-token stream unchanged for current clients.
2. Stage only the active element start token and its bounded attributes until `>` or `/>`; then emit finalized element first, attributes afterward in source order. Duplicate checks use bounded per-tag scratch. Document exact retained-memory bound and callback/cancellation semantics.
3. Reuse common scanner validation for UTF-8, XML characters, syntax, byte budgets, cancellation, limits, error offsets, counts, and all token kinds. XML declaration validation must not rely on retained tokens.
4. Move attribute entity decoding from `src/svg/shape/text.c` into the XML library public bounded decoder and update shape text normalization to call that implementation. Preserve shape diagnostic offsets and accepted behavior.
5. Add focused regressions for token kinds and source spans across tiny budgets, complete tag callback ordering, cancellation, late malformed input after prior callbacks, duplicate attrs, visitor storage independent of total tokens, and entity decoding of named/decimal/hex/invalid/overflow/surrogate/capacity cases.
6. Update XML tokenizer API documentation.

# Files allowed to touch
- `src/svg/xml/xml_tokenizer.h`, `src/svg/xml/xml_tokenizer.c`, `src/svg/xml/xml_tokenizer_api.c`, `src/svg/xml/xml_stream_internal.inc`
- `src/svg/shape/text.c`
- `tests/svg/index/test_token_stream.c`, `tests/svg/xml/test_xml_tokenizer.c`
- `CMakeLists.txt` XML/index/shape library and test registration lanes only
- `src/svg/xml/README.md`
- `work/WO-XML-STREAM-02.md`
- `artifacts/WO-XML-STREAM-02/**`, `STATE.md`, `LEDGER.md`, `ledger.jsonl`

# Protected paths
All existing unrelated candidates/artifacts, project/app/document/UI/MCP implementations, renderer except the shared text decoder callsite, test oracles/goldens, scripts, presets, release files, and paths outside this allowlist. Do not change animation behavior. Do not run broad UI/E2E suites.

# Acceptance criteria
- AC-1: strict Debug XML, shape, and application dependency targets build.
- AC-2: focused XML tokenizer, token-stream, and shape text tests pass.
- AC-3: the same focused checks pass under ASan/UBSan.
- AC-4: visitor tests prove token storage stays bounded independent of total token count and ordering/spans remain correct across chunking/cancellation.
- AC-5: fresh read-only review reports no blocking/high findings.
- AC-6: frozen work order, ledger, allowlist, source line limits, and `git diff --check` validate.

# Verification commands
- Build strict Debug XML/shape dependency targets.
- Run focused `xml-tokenizer`, token-stream, and relevant shape CTests.
- Run the same focused CTests under ASan and UBSan.
- Run independent read-only review restricted to allowed paths.
- Record exact command lines and outcomes under `artifacts/WO-XML-STREAM-02/`.

# Budget
One implementation pass and one independent verifier; no network, no commit.

# Failure and escalation
Never alter frozen tokenizer oracles. If complete-tag callback semantics or a bounded per-tag storage ceiling cannot preserve current validation behavior, stop before application code changes and record the exact constraint.

# Output
Leave changes uncommitted. Report changed paths, focused checks, review result, retained-memory bound, and the next native timeline integration step.
