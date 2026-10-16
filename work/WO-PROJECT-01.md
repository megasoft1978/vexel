---
id: WO-PROJECT-01
revision: 1
status: READY
milestone: PROJECT
owner: project-owner
executor: gpt-6.1-sol
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 18ca70cf1124b678dcec76dc4f0d0d1ea6da2d781c9bfc42b9ab0d2b7d0452e5
parallel_safe: false
network: false
---
# Goal
Define the first version of Vexel's native project container and implement its bounded, lossless in-memory codec. It stores JSON project metadata, the exact SVG scene source, timeline data, and named assets in one versioned `.vexel` file. The codec is the format foundation; application open/save integration, project schema semantics, migration UI, and recovery are subsequent work.

# Context
- Governing goal: ADR-0018 and `docs/product/vexel-goal-and-delivery-plan.md`.
- Existing editor saves source-preserving SVG but has no native project container. Animation authoring and export currently use SVG/SMIL pathways; the container needs an explicit timeline payload for native semantics to evolve independently.
- Apple Silicon macOS 13+, C17, no third-party dependency, explicit pointer+length APIs, checked arithmetic, bounded user data, source files below 800 lines.
- No archive decompression in this revision. The codec treats manifest/timeline as opaque UTF-8 JSON payloads and validates the SVG scene as non-empty bytes; higher layers own JSON/XML semantics.

# Format v1 contract
- 56-byte little-endian header: 8-byte magic `VEXEL\r\n\x1a`, u32 version (=1), u32 header size (=56), u32 manifest length, u32 asset count, u64 scene length, u64 timeline length, u32 CRC-32/IEEE for manifest, scene and timeline respectively, and u32 flags (=0).
- Payload order: manifest JSON, exact scene SVG bytes, timeline JSON, then asset records.
- Each asset record has a 16-byte little-endian header (u16 name length, u16 media-type length, u32 CRC-32/IEEE, u64 data length), followed by name, media type, and raw data.
- Asset names are unique single path components using ASCII letters, digits, dot, underscore, and dash; `.` and `..` are invalid. Media types are bounded printable ASCII `type/subtype` values.
- Decoder returns borrowed slices into the input buffer; input must outlive the view. Encoder writes into caller-provided storage and never partially writes on invalid input or insufficient capacity.
- Reject trailing bytes, unknown versions/flags, checksum mismatch, malformed lengths, duplicate/unsafe asset names, and all values over limits.
- Limits: encoded project 1 GiB; manifest 1 MiB; scene 256 MiB; timeline 64 MiB; assets 1024 entries, 128-byte names, 128-byte media types, 512 MiB aggregate data and 256 MiB per asset.

# Files allowed to touch
- `docs/adr/0019-vexel-project-container-v1.md`
- `src/project/**`
- `tests/project/**`
- `CMakeLists.txt`
- `work/WO-PROJECT-01.md` (only before READY freeze; frozen now)
- `artifacts/WO-PROJECT-01/**`
- `ledger.jsonl`, `LEDGER.md`, `STATE.md`

# Protected paths
- All existing application, renderer, SVG, document, platform, and MCP sources/tests.
- `OWNER-DIRECTIVES.md`, `AGENTS.md`, all other work orders and ADRs, `README.md`, existing artifacts, corpus and golden/oracle inputs.
- Existing raster candidate files and `artifacts/WO-CORPUS-04/**`; release artifacts; all paths outside the allowlist.

# Required implementation
1. Add `project_bundle.h/.c` with documented limits, borrowed-view structs, error names, encoded-size query, safe encode, decode, and indexed asset access.
2. Use checked arithmetic before every addition, multiplication, narrowing conversion, and buffer operation. Ensure malformed sizes cannot cause overflow or out-of-bounds reads.
3. Verify CRC-32/IEEE for every payload; validate version, header size, reserved flags, all limits, unique asset names, and exact end-of-file consumption.
4. Preserve all opaque metadata, scene, timeline, and asset bytes exactly through decode/encode round trips.
5. Add focused C tests for round-trip equality, zero and multiple assets, boundary sizes, insufficient output, corruption in each section, wrong magic/version/header/flags, duplicate/unsafe names, overflows/limits, and trailing bytes.
6. Document v1 field layout, compatibility promise, and limitations in ADR-0019 and module README. State that future schema changes need explicit version/migration work.

# Acceptance criteria
- AC-1 New focused target builds with C17 and project warnings; source files remain under 800 lines.
- AC-2 Focused project-bundle C test passes and exercises each required validation branch; capture exact command/output.
- AC-3 ASan/UBSan focused target passes if available through existing build presets, without running the comprehensive UI suite.
- AC-4 `python3 scripts/ledger.py validate ledger.jsonl`, `python3 scripts/wo_validate.py WO-PROJECT-01 --ready`, and `git diff --check` pass.
- AC-5 Evidence and state/ledger records identify this as a codec foundation, not full native-project application support.

# Verification commands
```text
cmake --preset mac-debug
cmake --build build/mac-debug --target project_bundle_test
ctest --test-dir build/mac-debug -R '^project-bundle$' --output-on-failure
cmake --preset mac-asan
cmake --build build/mac-asan --target project_bundle_test
ctest --test-dir build/mac-asan -R '^project-bundle$' --output-on-failure
python3 scripts/wo_validate.py WO-PROJECT-01 --ready
python3 scripts/ledger.py validate ledger.jsonl
git diff --check
```

# Non-goals
- File-system save/open, editor menus, native package directories, async loading, recovery journals, UI, or integration with MCP.
- Claiming that the chosen opaque timeline payload is the final animation schema.
- Running broad UI/end-to-end acceptance suites.

# Output
One scoped implementation with ADR-0019, focused codec tests/evidence, and a clear next step for file-system/editor integration.
