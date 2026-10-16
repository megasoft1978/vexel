---
id: WO-XML-STREAM-01
revision: 1
status: READY
milestone: M7
owner: project-owner
executor: codex-gpt-6.1-sol
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 5b6507e18771b5a214a2a46795c266e4d1866dd1e29cd992ac336cc6226c3c9c
parallel_safe: false
network: false
---
# Goal
Add a low-memory visitor path and reusable XML attribute entity decoder to the owned XML tokenizer so large project documents can build bounded semantic indexes without retaining a document-wide token vector.

# Context pointers
- Read `AGENTS.md`, `OWNER-DIRECTIVES.md`, `STATE.md`, `docs/CODING_STANDARD.md`, `docs/testing.md`, XML tokenizer architecture and existing incremental stream tests.
- WO-ANIM-PROJECT-03 revision 1 was superseded before implementation because its 256 MiB scene ID inventory requirement cannot be met by the existing full-token API or capped retained stream.
- The later native timeline integration order depends on this work. No GPL editor source, no UI/E2E suite.

# Existing contracts
- Apple Silicon macOS 13+, C17, no dependencies, bounded work and explicit pointer+length ownership.
- XML tokenizer validates UTF-8, well-formedness, limits, and source spans. Existing token-vector and stream APIs must remain behavior-compatible.
- New visitor mode must not retain tokens when caller requests discard mode. Callback source spans borrow immutable input and are valid only for the input lifetime.
- Start/empty-element callbacks must expose the finalized element kind and complete source span. Attribute tokens must be emitted in source order after validation. Callbacks do not authorize mutation.
- Attribute entity decoder supports the tokenizer’s accepted XML reference vocabulary and returns UTF-8 into caller storage with exact failure offset and checked capacity. It must not recursively expand entities or load external resources.

# Required implementation
1. Extend the incremental tokenizer with an optional token visitor and context; preserve the existing capped-vector mode. In visitor/discard mode, token memory remains bounded independent of total token count.
2. Emit complete start/empty-element tokens only after the opening tag is fully validated, then emit validated attribute tokens in original order. Preserve cancellation, byte-budget, duplicate-attribute, error-offset, and token-count semantics.
3. Add a public bounded attribute-value decoder for XML predefined and numeric character references, with malformed-reference and output-exhaustion offsets. Match the current text subsystem’s accepted semantics without copying a private second decoder.
4. Add focused regressions for all token kinds across tiny byte budgets, finalized empty/start tags, callback ordering, cancellation, malformed late input after earlier callbacks, attribute entities (named, decimal, hex, invalid, overflow, surrogate, capacity), and visitor mode over a synthetic input with token count above retained-vector limits while proving constant token storage.
5. Document visitor callback lifetime, cancellation/failure semantics, and entity decoder behavior.

# Files allowed to touch
- `src/svg/xml/xml_tokenizer.h`, `src/svg/xml/xml_tokenizer.c`, `src/svg/xml/xml_tokenizer_api.c`
- `tests/svg/index/test_token_stream.c`, `tests/svg/xml/test_xml_tokenizer.c`
- `CMakeLists.txt` XML/index source and test registration lanes only
- `src/svg/xml/README.md` if present; otherwise `src/svg/xml/xml_tokenizer.h` API comments
- `work/WO-XML-STREAM-01.md`, `artifacts/WO-XML-STREAM-01/**`, `STATE.md`, `LEDGER.md`, `ledger.jsonl`

# Protected paths
All existing unrelated candidates/artifacts, project/app/document/UI/MCP implementations, renderer, test oracles/goldens, scripts, presets, release files, and paths outside this allowlist. Do not modify any existing animation behavior. Do not run broad UI/E2E suites.

# Acceptance criteria
- AC-1: strict Debug XML tokenizer and application dependency targets build.
- AC-2: focused `xml-tokenizer` and token-stream CTests pass.
- AC-3: same focused checks pass under ASan/UBSan.
- AC-4: visitor mode proves no token-vector growth and preserves callback ordering/spans under chunking and cancellation.
- AC-5: fresh read-only review has no blocking/high findings.
- AC-6: frozen work order, ledger, allowlist, source line limits, and `git diff --check` validate.

# Verification sequence
1. Add focused visitor/entity regressions, including synthetic high-token-count input.
2. Run strict Debug XML target build and focused XML tests.
3. Run the same under ASan/UBSan.
4. Run independent read-only review restricted to allowed paths.
5. Record exact evidence under `artifacts/WO-XML-STREAM-01/`.

# Non-goals
Project timeline integration, scene ID index, SVG parser/raster behavior changes, document editing, performance benchmark claims, UI/E2E, MCP, dependencies, and commits.

# Budget
One implementation pass and one independent verifier; no network, no commit.

# Failure and escalation
Never alter frozen tokenizer oracles. If finalized callback ordering cannot preserve existing token ordering, document the exact constraint and split a new tokenizer design before implementation.

# Output
Leave changes uncommitted. Report exact paths, focused checks, review result, callback memory bound, and the next native project integration step.
