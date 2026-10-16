---
id: WO-ANIM-PROJECT-03
revision: 1
status: READY
milestone: M7
owner: project-owner
executor: codex-gpt-6.1-sol
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 36d9e16a92451b12236cbd332378936ba7bfb93a494e33ba884319e0ca322509
parallel_safe: false
network: false
---
# Goal
Integrate accepted native timeline v1 validation into `.vexel` project loading and saving without changing container v1 or losing opaque future timeline payloads.

# Context pointers
- Read `AGENTS.md`, `OWNER-DIRECTIVES.md`, `STATE.md`, `docs/CODING_STANDARD.md`, `docs/testing.md`, ADR-0019/0020, and the complete project/timeline load-save path.
- WO-ANIM-PROJECT-02 contains the bounded parser, cached borrowed views, and accessor API; its independent acceptance review is still pending. Do not claim that work order accepted.
- No GPL editor source; no UI/E2E suite.

# Existing contracts
- macOS Apple Silicon only, C17, no dependencies, checked arithmetic, explicit ownership.
- Container v1 and exact SVG round-trip bytes stay unchanged.
- Empty timeline is valid. Known native animation v1 must validate against scene IDs and asset inventory. Unsupported timeline schema/version remains byte-exact opaque pass-through.
- SVG targets are ID strings. Build a bounded, sorted unique ID inventory from the exact scene source with proper XML tokenization and entity decoding. Reject duplicate IDs for a known native animation payload; do not rewrite the SVG.
- The view borrows the project source. Its lifetime must remain tied to the existing project owner; do not store dangling slices or process-local handles.

# Required implementation
1. Add a project-layer integration function that accepts the decoded bundle plus scene-ID inventory and returns categorized timeline compatibility/validation status without mutating bytes.
2. Extract SVG IDs in bounded storage, validate duplicates, decode XML attribute entities according to the owned tokenizer contract, sort IDs for the timeline validator, and respect SVG/project limits. Do not add a second unbounded token tree.
3. Validate drawing asset references against the bundle table and SVG media type, while allowing unrelated raster/provenance assets.
4. Wire native `.vexel` load so known v1 timeline semantic errors fail with useful bounded diagnostics; unknown schema/version stays loadable and opaque. Plain SVG behavior is unchanged.
5. Wire save to preserve unknown timeline bytes exactly and preserve known v1 timeline bytes exactly after validation. Never canonicalize/re-serialize JSON or silently drop assets/metadata.
6. Add focused project-layer tests for scene ID ordering, escaped IDs/entities, duplicate/dangling IDs, asset media mismatch, known/unknown timeline behavior, unchanged bytes, and load failure ownership cleanup.
7. Document API ownership and compatibility behavior.

# Files allowed to touch
- `src/project/project_timeline.h`, `src/project/project_timeline.c`, `src/project/project_timeline_json.h`, `src/project/project_timeline_json.c`
- new `src/project/project_timeline_integration.h/.c` if needed
- `src/project/README.md`
- `src/app/viewer_load.c`, `src/app/viewer_load.h`, `src/app/viewer_internal.h`, `src/app/viewer_save.c`
- `tests/project/project_timeline_test.c`, new `tests/project/project_timeline_integration_test.c`, `tests/project/project_load_test.c`
- `CMakeLists.txt` project-format lane only, `tests/test_map.json`
- `work/WO-ANIM-PROJECT-03.md`, `artifacts/WO-ANIM-PROJECT-03/**`, `STATE.md`, `LEDGER.md`, `ledger.jsonl`

# Protected paths
All existing unrelated candidates and artifacts, release files, SVG/XML tokenizer and renderer source, document editing internals, UI, MCP implementation, container layout, scripts, presets, and all paths outside the allowlist. Do not run comprehensive UI/E2E tests.

# Acceptance criteria
- AC-1: strict Debug application and focused project targets build.
- AC-2: focused project bundle/file/load/timeline integration CTests pass.
- AC-3: same focused suite passes under ASan/UBSan, including failure cleanup.
- AC-4: unknown timeline payload and exact SVG/metadata/assets remain byte-identical across open/save.
- AC-5: fresh read-only review has no blocking/high findings in allowed paths.
- AC-6: frozen order, ledger, allowlist, source line limits, and `git diff --check` validate.

# Verification sequence
1. Add focused integration tests and record initial failures.
2. Run Debug focused project build/tests.
3. Run ASan/UBSan focused project build/tests.
4. Run independent read-only review restricted to allowed paths.
5. Record exact evidence under `artifacts/WO-ANIM-PROJECT-03/`.

# Non-goals
Timeline edit/serialize UI, native drawing SVG sanitization, playback/export, onion-skin rendering, MCP controls, migration/recovery UI, SVG export changes, broad UI/E2E tests, and commits.

# Budget
One executor pass and one independent verifier; no network, no commit.

# Failure and escalation
Never change frozen tests or make unknown payloads look native. If tokenizer limitations prevent safe entity-decoded ID validation, retain opaque behavior and report the exact limitation for an ADR/work-order split.

# Output
Leave changes uncommitted. Report exact paths, commands, reviewer result, and remaining animation authoring/playback work.
