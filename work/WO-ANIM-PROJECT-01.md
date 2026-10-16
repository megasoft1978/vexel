---
id: WO-ANIM-PROJECT-01
revision: 1
status: READY
milestone: M7
owner: project-owner
executor: codex-gpt-6.1-sol
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
parallel_safe: false
network: false
spec_sha256: ca9c6a078db507b20956285608859bad4fab8bcfe553dca6685a5dd3043091d2
---
# Goal
Implement bounded validation and borrowed-view access for the accepted Vexel native animation timeline schema v1.

# Context pointers
- Read `AGENTS.md`, `OWNER-DIRECTIVES.md`, `STATE.md`, `docs/CODING_STANDARD.md`, and `docs/testing.md`.
- Read accepted `docs/adr/0018-product-goal-and-completion-gates.md`, `docs/adr/0019-vexel-project-container-v1.md`, and `docs/adr/0020-native-animation-schema-v1.md`.
- Read `src/project/project_bundle.h`, `src/project/README.md`, `src/mcp/mcp_json.h`, `src/document/document_model.h`, and the owning SVG parse entry points.
- Do not read, fetch, or copy GPL SVG editor source.

# Existing contracts and invariants
- C17, macOS Apple Silicon only, no new dependencies, pointer+length slices, checked arithmetic.
- Container v1 remains unchanged. Decode borrows input and never mutates it.
- JSON payload is capped at 64 MiB. The MCP JSON parser's 2 MiB/65,536-token caps are insufficient; do not reuse it.
- Scene `document_handle` values are process/model lifetime only. Serialized references use unique SVG `id` attribute strings.
- Empty timeline means empty animation. Unknown non-empty schema/version remains opaque in existing load/save; timeline decode reports unsupported and does not rewrite bytes.

# Schema contract
The root object has exactly: `schema` (`"vexel.animation"`), `version` (1), `ticks_per_second` (integer 1..240), `duration_ticks` (integer 1..14,400,000), `fps` (`numerator` and `denominator`, positive integers whose frame boundaries map exactly to ticks), `loop` (`once`, `loop`, `ping_pong`), `onion` (`before`, `after` 0..8; `opacity` finite 0..1), and `tracks` (array, <=256).

Each track has `id` (1..128 ASCII alphanumeric/underscore/hyphen), `kind` (`drawing` or `property`), `target` (unique scene SVG id, <=256 UTF-8 bytes), optional `name` (<=4096 UTF-8 bytes), `visible` (boolean), and exactly one kind payload. Drawing payload `cels` is an ordered array of `{tick, asset}`; asset is a safe bundle asset name whose media type is `image/svg+xml`. Property payload has `property`, `value_type` (`scalar`, `transform2d`, `rgba`, `boolean`), `interpolation` (`step`, `linear`, `cubic`), and ordered `keys` of `{tick, value}`. Values are finite typed JSON values. Ticks are unique and strictly increasing in `[0,duration_ticks)`; first drawing cel/key is at tick 0. Total cels+keys <=1,000,000; entries per track <=100,000; max JSON nesting 32; every other string <=4 KiB. Unknown fields, duplicate JSON keys, duplicate track IDs, duplicate scene ids, dangling target/assets, malformed UTF-8/JSON, and overflow are rejected with category and byte offset.

# Files allowed to touch
- `src/project/project_timeline.h`
- `src/project/project_timeline.c`
- `src/project/README.md`
- `tests/project/project_timeline_test.c`
- `CMakeLists.txt` (the `# lane: project-format` block only)
- `tests/test_map.json`
- `work/WO-ANIM-PROJECT-01.md`
- `artifacts/WO-ANIM-PROJECT-01/**`
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`

# Protected paths
- `tests/oracle/**`, `tests/golden/**`, prior artifacts, release artifacts, and all existing in-progress candidates.
- App/UI/MCP code, container layout, scene/document editing code, scripts, presets, and all paths outside the allowlist.
- Do not run or modify broad UI/end-to-end suites.

# Required implementation
1. Add a separate bounded streaming JSON parser for this schema; no generic unbounded object graph and no dependency. Bound bytes, nesting, string length, item totals and parse work; process must remain linear in payload size.
2. Provide timeline decode/view API with categorized errors and byte offsets. Empty input yields an empty view; unsupported schema/version is distinguishable and leaves bytes untouched.
3. Validate exact fields/types, integer timing arithmetic, rational frame-rate representability, finite numeric values, ordering, track/property enums, duplicate IDs/keys, target SVG id uniqueness/existence, and drawing asset name/media type existence. Use caller-supplied scene/asset validation inputs; no transient document handles are serialized.
4. Provide borrowed accessors for track/cel/key records and preserve source spans. Do not serialize or edit timeline data in this order.
5. Add focused positive/negative vectors including 64 MiB cap handling, deep JSON, duplicate/unknown fields, UTF-8, invalid numeric values, tick/rate overflow, duplicate/dangling references, and unchanged source bytes.
6. Document schema/API ownership and distinguish native tracks from SVG SMIL.

# Acceptance criteria
- AC-1: project timeline target builds with strict project warnings.
- AC-2: focused `project-timeline` CTest passes positive and negative vectors.
- AC-3: ASan/UBSan focused `project-timeline` CTest passes.
- AC-4: `python3 scripts/wo_validate.py WO-ANIM-PROJECT-01 --ready`, `python3 scripts/ledger.py validate ledger.jsonl`, and `git diff --check` pass.
- AC-5: changed paths satisfy the allowlist; no UI/MCP or broad UI/E2E suite is touched or run.

# Verification sequence
1. Add parser and semantic regression cases and capture the focused initial failure.
2. Run the project timeline target and CTest in Debug.
3. Run the same focused target and CTest under ASan/UBSan.
4. Validate frozen spec, ledger, changed-path allowlist, and diff whitespace.
5. Save exact output under `artifacts/WO-ANIM-PROJECT-01/`.

# Non-goals
Timeline serialization/editing, native drawing asset authoring, SVG import/export changes, SMIL changes, playback, onion-skin rendering, UI, MCP, migration UI, recovery journal, comprehensive UI/E2E tests, and commits.

# Budget
One executor run, one read-only verifier, no network, no commit.

# Failure and escalation
Do not weaken tests or reinterpret unknown payloads. If a requirement conflicts with the accepted ADR or current platform contracts, stop with exact evidence and a proposed split; keep existing unknown-payload pass-through untouched.

# Output
Leave changes uncommitted. Report exact changed paths, focused checks, and remaining native-animation integration work.
