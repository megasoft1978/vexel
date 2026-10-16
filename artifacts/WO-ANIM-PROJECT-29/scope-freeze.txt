---
id: WO-ANIM-PROJECT-29
revision: 1
status: READY
milestone: M5
owner: project-owner
executor: codex-gpt-6.1-sol
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 8fa77319ee8edd3e3948ca73a83e4f1cc904c66ca4ae9b0c1461209f6a737396
parallel_safe: false
network: false
---
# Goal
Give newly created drawable objects stable, document-unique SVG IDs so users and MCP agents can key and animate them after save and reopen.

# Context
- Follow `OWNER-DIRECTIVES.md`, `AGENTS.md`, `docs/CODING_STANDARD.md`, ADR-0018 through ADR-0021, and the active product plan.
- `document_edit_insert_template` and `document_edit_insert_xml_fragment` are shared by canvas drawing, text/path creation, clipboard, and MCP; new objects without IDs cannot be targeted by native property tracks.
- Preserve the existing 249-path dirty worktree. Do not revert or reformat unrelated changes.
- `src/document/document_edit_ops.c` is already 798 lines; keep every source file below the 800-line limit by extracting cohesive insertion code where needed.

# Existing contracts and invariants
- Existing source bytes outside the inserted element remain unchanged; all insertion and ID creation is one undoable edit.
- Keep supplied IDs unchanged. For a new top-level graphics element lacking an ID, generate a valid `vexel-object-N` ID absent from the current document. Generation is checked and bounded; fail without mutation on exhaustion or allocation failure.
- Do not add IDs to existing/imported elements or nested descendants. Do not rewrite references, namespaces, styles, or user-supplied fragment bytes other than adding the root ID.
- A supplied ID collision is rejected rather than creating an ambiguous new reference target.
- macOS Apple Silicon only, C17, no dependencies, no broad UI/E2E or corpus work.

# Files allowed to touch
- `work/WO-ANIM-PROJECT-29.md`
- `src/document/document_edit.h`, `src/document/document_edit.c`, `src/document/document_edit_internal.h`
- `src/document/document_edit_ops.c`, `src/document/document_edit_insert.c` (new)
- `src/document/document_edit_text_insert.c`
- `src/document/README.md`
- `src/mcp/mcp_broker_edit.c`, `src/mcp/README.md`
- `tests/edit/test_document_edit.c`, `tests/edit/README.md`
- `tests/mcp/mcp_native_timeline_test.c`
- `CMakeLists.txt`, `tests/test_map.json`
- `artifacts/WO-ANIM-PROJECT-29/**`, `STATE.md`, `LEDGER.md`, `ledger.jsonl`

# Protected paths
- Preserve all existing changes outside this allowlist.
- Do not change imported-source loading, project schema, animation semantics, SVG parsing/rasterization, clipboard ID rewriting, permission grants, platform support, oracle/golden files, or prior frozen work orders/evidence.
- Do not run the comprehensive UI/E2E suite, broad CTest suite, corpus, fuzz, performance, or release acceptance.

# Required implementation
1. Ensure template insertion and one-root XML/text-fragment insertion add a generated `id` only when the inserted root is an SVG graphics element and has no ID. Leave definition/non-drawing metadata elements unchanged.
2. Generate a syntactically valid `vexel-object-N` value that is absent from the current document, using checked increment and a documented finite attempt bound. Preserve unique caller-provided IDs; reject a caller-provided ID already present in the document with a specific edit error.
3. Keep generated ID bytes within the new object insertion's existing single undo transaction. Undo restores exact prior bytes; redo restores the same ID. Preserve all other source bytes and supplied fragment markup.
4. Confirm canvas shapes/text/paths and MCP template/fragment creation expose the assigned ID in their returned/selected element. Add regressions for collision skipping, supplied unique ID, duplicate supplied ID rejection, rootless nested source preservation, undo/redo stability, and both shared insertion APIs.
5. Keep files below 800 lines; extract insertion functionality into a focused module as needed. Document ID generation and limits in the document and MCP guides.

# Verification
- Confirm focused CTest names from `tests/test_map.json` before running.
- Build `document_edit_test`, `mcp_native_timeline_test`, and `svgworkbench` in Debug, ASan/UBSan, and configured UBSan directories.
- Run only `document-edit` and `mcp-native-timeline` CTests in those three configurations; run the focused editor canvas test if it is required to verify selected IDs.
- Validate `python3 scripts/wo_validate.py WO-ANIM-PROJECT-29 --ready`, `python3 scripts/ledger.py validate ledger.jsonl`, JSON syntax of `tests/test_map.json`, source line limits, changed-path scope, and `git diff --check`.
- Record every exact command and result in `artifacts/WO-ANIM-PROJECT-29/verification.txt` and update `STATE.md`, `LEDGER.md`, and `ledger.jsonl`.
- Broad UI/E2E remains deferred.

# Output
Leave changes uncommitted. Keep the full product goal active and report exact focused evidence and remaining dependencies.
