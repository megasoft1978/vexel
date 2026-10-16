---
id: WO-ANIM-PROJECT-02
revision: 1
status: READY
milestone: M7
owner: project-owner
executor: codex-gpt-6.1-sol
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
parallel_safe: false
network: false
spec_sha256: 0aaf73edb6fdc4bbe318172f187cdf5d1e6be95186cee11dd5c0ad642fb685d0
---
# Goal
Repair and complete WO-ANIM-PROJECT-01's native timeline validator and borrowed accessors based on independent review findings.

# Context pointers
- WO-ANIM-PROJECT-01 revision 1 is rejected and superseded by this repair order. Reuse its accepted schema contract and tests.
- Read `AGENTS.md`, `OWNER-DIRECTIVES.md`, `STATE.md`, `docs/CODING_STANDARD.md`, `docs/testing.md`, ADR-0019/0020, and current timeline source/header/tests.
- ADR-0020 defines the exact allowed property/value pairs and deterministic numeric contract.
- No GPL editor source. No broad UI/E2E suite.

# Required repairs
1. Split cohesive JSON scanning from schema validation so each C source remains below 800 lines. Parsing must use bounded constant/local storage; no 100,000-entry stack arrays. Add a small-stack pthread regression exercising decode and accessors.
2. Validate all caller-provided asset slices, safe bundle asset names, media types, count limits, and uniqueness before comparison. Add an independent missing-asset test with a valid scene target, plus invalid pointer/unsafe-name tests where representable.
3. Decode JSON string escapes for semantic comparison and decoded-length limits. Reject malformed UTF-8 and invalid UTF-16 surrogate pairs; support valid pairs and escaped schema/field/target/asset strings. Expose a caller-buffer string decode helper while source spans remain byte-exact.
4. Enforce the exact ADR-0020 property vocabulary/value-type pairing and finite/range constraints. `rgba` is four components in 0..1; affine transform is six finite components; booleans must be JSON booleans. Unsupported properties fail.
5. Add borrowed item access by track/index for cel/key records, including tick, asset or raw value span, and source range. Accessors must not allocate large scratch arrays.
6. Classify unknown schema/version before v1-only required-field validation, while duplicate JSON keys remain rejected.
7. Use locale-independent finite decimal parsing without mutating global locale; add a non-C thread-locale regression where available on the supported macOS test host.
8. Report semantic error offsets at the offending value/item and assert exact offsets for timing range errors.
9. Add focused regressions for each repair and other original WO requirements (unknown fields, duplicate track IDs, tick ordering, malformed UTF-8, tick/fps overflow, item/string bounds, and borrowed-source immutability).
10. Clarify ADR-0020 cubic interpolation as deterministic smoothstep between adjacent values, with no authored tangent payload in schema v1.

# Files allowed to touch
- `src/project/project_timeline.h`, `src/project/project_timeline.c`
- `src/project/project_timeline_json.h`, `src/project/project_timeline_json.c`
- `src/project/README.md`
- `tests/project/project_timeline_test.c`
- `CMakeLists.txt` (the `# lane: project-format` block only)
- `tests/test_map.json`
- `docs/adr/0020-native-animation-schema-v1.md`
- `work/WO-ANIM-PROJECT-02.md`
- `artifacts/WO-ANIM-PROJECT-02/**`
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`

# Protected paths
All oracle/golden files, prior artifacts other than the new work-order artifact directory, release files, existing unrelated candidates, app/UI/MCP code, container layout, scene/document code, scripts, presets, and all paths outside the allowlist. Do not run comprehensive UI/E2E tests.

# Acceptance criteria
- AC-1: strict Debug project timeline target builds; no source file exceeds 800 lines.
- AC-2: focused `project-(timeline|bundle|file)` CTests pass.
- AC-3: focused same CTests pass under ASan/UBSan; small-stack regression passes.
- AC-4: fresh read-only review reports no blocking/high findings in allowed paths.
- AC-5: work-order validation, ledger validation, allowlist inspection, and `git diff --check` pass.

# Verification sequence
1. Add regressions for reviewer findings and capture focused expected failures.
2. Run Debug focused project builds/tests.
3. Run ASan/UBSan focused project builds/tests.
4. Run read-only Codex review limited to allowed paths.
5. Record exact commands/results under `artifacts/WO-ANIM-PROJECT-02/`.

# Non-goals
Timeline editing/serialization, native drawing asset creation, playback/export, onion-skin rendering, UI, MCP, SVG import/export changes, SMIL changes, migration/recovery, broad UI/E2E tests, and commits.

# Budget
One repair pass, one fresh read-only verifier, no network, no commit.

# Failure and escalation
Never weaken regressions. If an accepted schema contract cannot be met within these paths, report the exact conflict and leave the timeline payload opaque.

# Output
Leave changes uncommitted. Report changed paths, exact focused checks, reviewer result, and remaining animation integration work.
