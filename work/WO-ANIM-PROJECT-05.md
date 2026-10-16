---
id: WO-ANIM-PROJECT-05
revision: 2
status: READY
milestone: M7
owner: project-owner
executor: codex-gpt-6.1-sol
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
parallel_safe: false
network: false
spec_sha256: 3e06b7cd27fdef8d8b057e2c128dd2ff4bf7cf1e36557184b9f8ab7a59682c1f
---
# Goal
Validate the content of every SVG asset referenced by known native drawing cels when a `.vexel` project is opened or saved.

# Context pointers
- Revision 1 was superseded after independent review found its protected-path allowlist omitted the cancellable owned shape-decoder seam needed for responsive validation of large assets. The first three findings (event-handler detection, CSS escapes, pool allocation check) are fixed in this revision.
- Read `AGENTS.md`, `OWNER-DIRECTIVES.md`, `STATE.md`, `docs/CODING_STANDARD.md`, `docs/testing.md`, ADR-0019/0020, `project_timeline_integration`, the owned SVG shape/reference APIs, and viewer load/save integration.
- WO-ANIM-PROJECT-04 classifies future timelines before SVG work and validates v1 references and media types. The XML visitor prerequisite is accepted.
- No GPL editor source. Do not fetch or resolve external SVG resources. No broad UI/E2E suite.

# Existing contracts
- macOS Apple Silicon only, C17, no dependencies, checked sizes, borrowed project slices remain owned by the encoded bundle.
- Unknown timeline JSON stays byte-exact opaque and must not trigger drawing-asset decoding.
- Unreferenced assets, including imported image/provenance assets, remain valid opaque payloads.
- A native drawing asset is one complete SVG document in `image/svg+xml`, parsed by the owned SVG implementation, with local references resolved and active scripts/external resource references rejected. Asset bytes are never rewritten.

# Required implementation
1. Add a project-layer drawing-asset validation API returning stable error category and byte offset; it borrows asset bytes and allocates no persistent state.
2. For known v1 timelines only, enumerate drawing tracks/cels, decode each asset name with the timeline string decoder, locate it in the bundle, and validate each referenced SVG asset once per validation call.
3. Reject unsupported/malformed SVG, duplicate or unresolved local IDs/references, scripts, and external resource references. Never fetch resources. Preserve unsupported but valid future timeline bytes without parsing assets.
4. Add an additive cancellable entry point to the owned SVG shape decoder. Thread the cancellation callback through bounded XML tokenization and shape traversal; return a distinct cancellation indication and leave output cleanup valid. Keep existing decoder entry points behavior-compatible.
5. Wire cancellation-aware validation into `.vexel` load and save. Failure diagnostics identify the asset name and source byte offset; cancel during parsing must stop promptly and release temporary ownership.
6. Do not parse unused provenance assets or change the SVG scene, timeline JSON, metadata, or asset bytes.
7. Add focused tests for valid local refs, malformed SVG, duplicate/dangling IDs, script/event-handler/external references, CSS escapes, repeated cel refs, unreferenced invalid provenance, opaque future timeline bypass, allocation failure, load/save failure cleanup, and cancellation during tokenization and shape traversal.
8. Document the SVG drawing-asset validation contract and limits.

# Files allowed to touch
- new `src/project/project_drawing_asset.h/.c`
- `src/project/project_timeline_integration.h/.c`, `src/project/README.md`
- `src/svg/shape/shape.h`, `src/svg/shape/decode.c`, `src/svg/shape/traverse_internal.h`, `src/svg/shape/traverse.c`
- `src/app/viewer_load.c`, `src/app/viewer_load.h`, `src/app/viewer_internal.h`, `src/app/viewer_save.c`
- `tests/project/project_timeline_integration_test.c`, `tests/project/project_load_test.c`, `tests/app/project_save_test.c`
- `CMakeLists.txt` project-format lane only, `tests/test_map.json`
- this work order, `artifacts/WO-ANIM-PROJECT-05/**`, `STATE.md`, `LEDGER.md`, `ledger.jsonl`

# Protected paths
All existing unrelated candidates and artifacts, other SVG/parser/renderer implementation paths, container layout, UI, MCP, document internals, scripts, presets, release files, and paths outside this allowlist. Do not run comprehensive UI/E2E tests.

# Acceptance criteria
- AC-1: strict Debug application and project targets build.
- AC-2: focused project integration/load/save CTests pass.
- AC-3: same focused suite passes under ASan/UBSan, including malformed-asset cleanup and cancellation.
- AC-4: byte-exact scene/timeline/metadata/assets remain unchanged; opaque timelines bypass SVG asset parsing; unused provenance remains accepted.
- AC-5: fresh read-only review has no blocking/high findings in allowed paths.
- AC-6: frozen order, ledger, changed-path allowlist, source line limits, and `git diff --check` validate.

# Verification commands
- Build strict Debug application and project integration/load/save targets.
- Run focused project bundle/file/load/save/timeline integration CTests.
- Run the same focused suite under ASan and UBSan.
- Run a fresh read-only review limited to allowed paths.
- Record exact commands and outcomes under `artifacts/WO-ANIM-PROJECT-05/`.

# Budget
One implementation pass and one independent verifier; no network, no commit.

# Failure and escalation
Do not treat parse failure as safe, weaken SVG safety, rewrite source bytes, or inspect unused provenance. If the owned SVG decoder cannot distinguish active/external content without changing protected paths, stop and record the precise seam needed in a new order.

# Output
Leave changes uncommitted. Report validated asset behavior, exact focused evidence, review result, and remaining native timeline authoring/playback work.
