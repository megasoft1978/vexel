---
id: WO-ANIM-PROJECT-04
revision: 1
status: READY
milestone: M7
owner: project-owner
executor: codex-gpt-6.1-sol
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
parallel_safe: false
network: false
spec_sha256: b19a7f4c3341c4f4de2a06250621d6ebb922eb5dd30bdce21aa46fa9a5a620f6
---
# Goal
Integrate accepted native timeline v1 validation into `.vexel` loading and saving without changing container v1 or losing opaque future timeline payloads.

# Context pointers
- Read `AGENTS.md`, `OWNER-DIRECTIVES.md`, `STATE.md`, `docs/CODING_STANDARD.md`, `docs/testing.md`, ADR-0019/0020, project/timeline load-save code, and XML visitor API.
- WO-ANIM-PROJECT-02 provides bounded timeline validation and borrowed accessors; WO-XML-STREAM-02 is accepted and provides finalized-tag discard callbacks plus shared bounded entity decoding.
- WO-ANIM-PROJECT-03 is superseded before implementation because its project-only allowlist omitted the required tokenizer seam.
- No GPL editor source; no broad UI/E2E suite.

# Existing contracts
- macOS Apple Silicon only, C17, no dependencies, checked arithmetic, explicit ownership.
- Container v1 and exact SVG round-trip bytes stay unchanged.
- Empty timeline is valid. Known native animation v1 must validate against scene IDs and asset inventory. Unsupported timeline schema/version remains byte-exact opaque pass-through.
- SVG targets are ID strings. Build a bounded, sorted ID inventory from the exact scene source with owned XML tokenization and entity decoding. Reject duplicate IDs for known native animation; do not rewrite SVG source.
- Project source and views remain borrowed from the existing owner; no dangling slices or process-local handles.
- Unknown/future timelines are classified before allocating the scene ID inventory.

# Required implementation
1. Add a project-layer integration API that accepts a decoded bundle and scene source, returning categorized compatibility/validation status without mutating bytes.
2. Classify empty, unsupported schema/version, and native-v1 timeline payloads using bounded JSON handling before scene-ID collection. Malformed JSON remains a bounded load error.
3. For native-v1 only, collect unprefixed SVG `id` attributes with the XML visitor, decode references with the shared decoder, enforce the documented ID length/count and scene-size limits, detect duplicates, and produce a sorted inventory. Do not retain a document-wide XML token vector or a second SVG tree.
4. Validate timeline target IDs and drawing-asset references against the sorted ID and bundle asset inventories. Drawing assets must use SVG media type; unrelated image/provenance assets remain allowed.
5. Wire `.vexel` load to reject known-v1 semantic errors with useful bounded diagnostics and preserve unknown schema/version as opaque. Plain SVG behavior remains unchanged. Preserve owner lifetime and failure cleanup.
6. Wire save to preserve unknown timeline bytes exactly and validate/preserve known v1 timeline bytes exactly. Never canonicalize JSON or drop metadata/assets.
7. Add focused project integration/load/save regressions for sorted/escaped IDs, duplicates, dangling targets, SVG entity decoding, media mismatch, known/unknown schema/version, unchanged bytes, cancellation, and failure cleanup.
8. Document API ownership, limits, and compatibility behavior.

# Files allowed to touch
- `src/project/project_timeline.h`, `src/project/project_timeline.c`, `src/project/project_timeline_json.h`, `src/project/project_timeline_json.c`
- new `src/project/project_timeline_integration.h/.c` if needed
- `src/project/README.md`
- `src/app/viewer_load.c`, `src/app/viewer_load.h`, `src/app/viewer_internal.h`, `src/app/viewer_save.c`
- `tests/project/project_timeline_test.c`, new `tests/project/project_timeline_integration_test.c`, `tests/project/project_load_test.c`, `tests/app/project_save_test.c`
- `CMakeLists.txt` project-format lane only, `tests/test_map.json`
- `work/WO-ANIM-PROJECT-04.md`, `artifacts/WO-ANIM-PROJECT-04/**`, `STATE.md`, `LEDGER.md`, `ledger.jsonl`

# Protected paths
All existing unrelated candidates and artifacts, release files, SVG/XML tokenizer source, renderer, document editing internals, UI, MCP implementation, container layout, scripts, presets, and paths outside this allowlist. Do not run comprehensive UI/E2E tests.

# Acceptance criteria
- AC-1: strict Debug application and focused project targets build.
- AC-2: focused project bundle/file/load/timeline integration CTests pass.
- AC-3: same focused suite passes under ASan/UBSan, including failure cleanup.
- AC-4: unknown timeline payload and exact SVG/metadata/assets remain byte-identical across open/save.
- AC-5: fresh read-only review has no blocking/high findings in allowed paths.
- AC-6: frozen order, ledger, allowlist, source line limits, and `git diff --check` validate.

# Verification commands
- Build strict Debug application and project integration targets.
- Run focused project bundle/file/load/timeline integration CTests.
- Run the same focused suite under ASan and UBSan.
- Run independent read-only review restricted to allowed paths.
- Record exact commands and outcomes under `artifacts/WO-ANIM-PROJECT-04/`.

# Budget
One implementation pass and one independent verifier; no network, no commit.

# Failure and escalation
Never weaken frozen tests or make unknown payloads look native. If a safe bounded scene ID inventory cannot be produced under these constraints, stop before altering load behavior and record the exact limit conflict for a new order.

# Output
Leave changes uncommitted. Report exact paths, commands, reviewer result, preserved-unknown-payload behavior, and remaining animation authoring/playback work.
