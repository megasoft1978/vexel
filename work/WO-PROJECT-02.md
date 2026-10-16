---
id: WO-PROJECT-02
revision: 1
status: READY
milestone: PROJECT
owner: project-owner
executor: gpt-6.1-sol
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 983c93737af2af00ab9bfb902cdc182ff4315fb98cdc38396eb5022292f6c633
parallel_safe: false
network: false
---
# Goal
Make v1 `.vexel` projects usable through Vexel's existing open and Save As flows. Preserve the SVG scene, project metadata, timeline payload, and assets across load/edit/save; support `.vexel` through Finder/startup, Open, and drag/drop. Keep SVG import/save/export source-preserving.

# Context
- Governing product goal: ADR-0018 and `docs/product/vexel-goal-and-delivery-plan.md`.
- Format and codec: ADR-0019 plus the verified `WO-PROJECT-01` codec in the current shared worktree (`src/project/project_bundle.*`). This work order intentionally builds on that unmerged predecessor; do not reset or replace its files.
- Existing editor loads SVG on a worker thread and swaps documents only after successful decode. Existing SVG Save As uses an atomic save path and protects dirty-document state.
- macOS 13+, Apple Silicon, C17, no third-party dependency, checked arithmetic, bounded inputs, source files under 800 lines.
- The comprehensive automated UI/end-to-end acceptance pass remains deferred until feature work is complete. Focused load, save, platform routing, and sanitizer checks are required here.

# Files allowed to touch
- `work/WO-PROJECT-02.md`
- `src/project/**`
- `src/app/viewer_load.c`, `src/app/viewer_load.h`, `src/app/viewer_internal.h`, `src/app/viewer.c`, `src/app/viewer_save.c`
- `src/platform/macos/platform_dialog.h`, `src/platform/macos/platform_dialog.m`, `src/platform/macos/platform_window.m`, `src/platform/macos/platform_window_files.m`
- `app/Info.plist.in`
- `tests/project/**`, `tests/platform/platform_dialog_test.m`, `tests/platform/**file*test*`, `tests/app/**project*test*`
- `docs/adr/0019-vexel-project-container-v1.md`, `docs/product/vexel-goal-and-delivery-plan.md`
- `CMakeLists.txt`, `STATE.md`, `LEDGER.md`, `ledger.jsonl`
- `artifacts/WO-PROJECT-02/**`

# Protected paths
- `OWNER-DIRECTIVES.md`, `AGENTS.md`, prior `work/**`, all ADRs except ADR-0019, UI goldens/oracles/fixtures, renderer and raster sources/tests, MCP sources/tests, release artifacts, prior artifacts, and all paths outside the allowlist.
- In particular, preserve the partial `WO-CORPUS-04` candidate without editing or accepting it.

# Required implementation
1. Add atomic native-project file writing using the existing macOS atomic replace/new-file adapters. Enforce `.vexel` bundle limits before allocating; return precise limit, path, durability, and I/O failures. Do not weaken SVG save behavior.
2. Extend worker loading to detect v1 project magic, fully validate the bundle before decoding its SVG scene, and retain the backing container allocation for all borrowed metadata/timeline/asset slices. The loaded document must own exactly the allocation it frees; no interior-pointer free.
3. Store project payload access in the loaded-document state. On Save/Save As to `.vexel`, materialize the latest edited SVG source and combine it with the preserved metadata, timeline, and assets. A document created/imported from SVG uses explicit empty project metadata/timeline defaults. SVG destinations continue using the existing source-preserving save path.
4. Save As must allow both `.svg` and `.vexel`; the Open panel, startup/document routing, and drag/drop must accept both extensions. Add Vexel file type metadata to the app bundle. Never claim the opaque timeline JSON has become an editable native animation schema.
5. Keep opening transactional: invalid bundle, SVG decode failure, canceled load, or failed save leaves the currently open document/path and dirty state intact.
6. Add focused tests for: atomic create and replacement of `.vexel`; exact payload/asset preservation after save and reload; edited scene replacement while metadata/timeline/assets remain unchanged; checksum/corrupt bundle rejection; SVG save path behavior; `.vexel` startup/open path acceptance; both Save As extensions; and failed-save/open invariants where the existing seams allow deterministic checks.
7. Update the product delivery plan to mark the codec and basic open/save integration implemented, and to keep native timeline semantics, version migration, recovery, and end-to-end UI acceptance explicitly outstanding.

# Acceptance criteria
- AC-1 Native project save writes a valid v1 file atomically to a new path and safely replaces an existing regular file; failure before publication preserves the previous file.
- AC-2 Worker load accepts both SVG and v1 project files; project scene bytes feed the existing SVG renderer/editor without copying or freeing an interior slice incorrectly; malformed projects fail before replacing a live document.
- AC-3 Save As can produce `.svg` or `.vexel`; opening, startup association, and file drop route both supported extensions. SVG save remains byte-preserving outside normal edits.
- AC-4 A save/reload round trip preserves exact metadata, timeline, assets, unknown SVG namespaces/source, and edited scene bytes.
- AC-5 Focused Debug and ASan/UBSan tests pass; work-order, ledger, and diff checks pass. Do not run comprehensive UI/end-to-end suites.
- AC-6 Independent read-only Codex review passes; state, ledger, and evidence state clearly that native timeline editing/migrations/recovery remain incomplete.

# Verification commands
```text
cmake --preset mac-debug
cmake --build build/mac-debug --target project_file_test project_load_test platform_dialog_test
ctest --test-dir build/mac-debug -R '^(project-file|project-load|platform-dialog)$' --output-on-failure
cmake --preset mac-asan
cmake --build build/mac-asan --target project_file_test project_load_test platform_dialog_test
ctest --test-dir build/mac-asan -R '^(project-file|project-load|platform-dialog)$' --output-on-failure
python3 scripts/wo_validate.py WO-PROJECT-02 --ready
python3 scripts/ledger.py validate ledger.jsonl
git diff --check
```

# Non-goals
- Designing or interpreting the native timeline JSON schema, adding animation key editing, autosave/recovery journals, SVG renderer changes, remote MCP, or running final UI/end-to-end acceptance.

# Output
Verified basic `.vexel` open/save through the real document pipeline, focused regression evidence, updated product inventory, and a precise next step for animation schema/migration/recovery.
