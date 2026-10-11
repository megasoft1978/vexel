---
id: WO-BOOT-06
revision: 2
supersedes: 1
status: READY
milestone: M0
owner: project-owner
executor: codex-gpt-6-luna
base_sha: dfdfab99c062a70016fda7618cda87b305e54287
spec_sha256: d5e9fe8139e732af4e77de5aec8831dd93e8a410a910665d2ab9c566cd05a570
parallel_safe: false
network: false
---
# Goal
Add deterministic, dependency-free PNG golden comparison tooling that produces scalar metrics, a visual diff image, and a metadata-linked JSON report without changing the frozen oracle.

# Context pointers
- Read: `AGENTS.md`, `OWNER-DIRECTIVES.md`, `STATE.md`, this file, `docs/research-r4/01-operating-model.md`, `docs/research-r4/02-work-order-template.md`, `docs/research-r4/05-revised-roadmap.md`, `docs/CODING_STANDARD.md`, `tests/README.md`, `tests/golden/manifest.lock`, `tests/test_golden_oracle.py`.
- Relevant ADRs: `docs/adr/0001-product-and-platform-locks.md`, `docs/adr/0002-source-preservation-and-provenance.md`.
- Frozen inputs: `tests/golden/manifest.lock`, `tests/golden/fixtures/**`, `tests/oracle/**`, `bench/corpus/manifest.lock` and all pinned corpus fixtures. Read only.
- External docs: none required; use the PNG specification only if a chunk or filter detail is unclear. Do not fetch code or sample images.

# Existing contracts and invariants
- Apple Silicon macOS 13+ only; C17 project remains warning-clean and dependency-free.
- Do not rewrite, re-encode, replace, or weaken any golden PNG, manifest, corpus fixture, or oracle.
- Golden input hashes must match the frozen manifest before comparison. Missing baseline, malformed PNG, unsupported PNG encoding, missing metadata, or mismatched SVG identity is an error.
- The comparison program uses Python standard library only. It writes reports and heatmaps only under repo-local `.cache/`.
- Exact scalar measurements are always reported. Edge tolerance is explicit, bounded, and separately reported; it never changes the exact metrics.
- A baseline change requires a separate READY work order and a recorded reason. This work order has no baseline-update operation.

# Files allowed to touch
- `scripts/golden_diff.py`
- `scripts/README.md`
- `tests/test_golden_diff.py`
- `tests/README.md`
- `docs/golden-images.md`
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`
- `artifacts/WO-BOOT-06/**`

# Protected paths
- `OWNER-DIRECTIVES.md`, `work/**`, `app/**`, `src/**`, `platform/**`, `CMakeLists.txt`, `CMakePresets.json`, `dev`
- `tests/golden/**`, `tests/oracle/**`, `bench/corpus/**`, `.github/**`
- All paths outside the explicit allowlist
- Any non-macOS implementation or test path

# Required implementation
1. Add a CLI that compares an actual PNG against one named fixture in the frozen golden manifest. Reject unknown fixture IDs, absent baselines, baseline hash mismatch, malformed chunks/CRCs/filters, dimension mismatch, and unsupported image encoding.
2. Decode the locked 8-bit RGBA non-interlaced PNG profile. Compare every channel exactly and report exact equal/different pixels, differing channels, maximum channel delta, total absolute channel delta, and mean absolute channel delta.
3. Add explicit `--edge-radius` (0–2 pixels, default 1) and `--edge-channel-tolerance` (0–16, default 1). A pixel is edge-tolerated only when its maximum channel delta is within the declared channel tolerance and it is within the declared Chebyshev radius of an 8-neighbor color/alpha boundary in the expected image. Report tolerated and non-tolerated counts separately. Exit 0 only when all differences are tolerated; otherwise exit 1. Input/format/metadata errors exit 2.
4. Write a deterministic RGBA heatmap (unchanged transparent, tolerated yellow, failing red) and a JSON report. Include fixture ID, expected/actual PNG SHA-256, pinned SVG SHA-256, build SHA, scale, frame, font, backend, dimensions, tolerances, exact metrics, tolerance counts, and pass/fail result. Metadata values are explicit required CLI inputs; validate SHA and numeric bounds.
5. Require all output paths to remain under repo-local `.cache/`; refuse symlinks and overwrite of existing outputs. Never provide a command that modifies a golden baseline.
6. Add tests for identical images, a small edge-only channel difference, a one-pixel shift with zero tolerance, missing baseline, malformed PNG/CRC/filter, metadata mismatch, deterministic report/heatmap output, path escape, and refused output overwrite.
7. Document PNG support limits, metrics, tolerance semantics, output schema, invocation examples, and the separate-WO requirement for baseline changes.

# Acceptance criteria (machine-checkable)
- AC-1 `python3 -m unittest tests.test_golden_diff` exits 0 with zero skips; evidence `artifacts/WO-BOOT-06/tests.txt`.
- AC-2 An identical pinned `shapes-basic` PNG comparison exits 0 and its report has `exact_equal: true`, zero differing pixels, and matching pinned hashes; evidence `artifacts/WO-BOOT-06/identical.json` and `.cache/` heatmap.
- AC-3 Synthetic edge-only and shifted images produce the exact declared tolerated/non-tolerated counts; no tolerance is implicit; evidence `artifacts/WO-BOOT-06/tolerance-tests.txt`.
- AC-4 Missing baseline, corrupt CRC/filter, metadata/source mismatch, unsafe output path, and output overwrite each fail without changing frozen files; evidence `artifacts/WO-BOOT-06/failure-tests.txt`.
- AC-5 Heatmap and JSON output are byte-identical across two identical invocations; evidence `artifacts/WO-BOOT-06/determinism.txt`.
- AC-6 `./dev verify`, `python3 scripts/ledger.py validate ledger.jsonl`, `python3 scripts/wo_validate.py WO-BOOT-06 --ready`, and `git diff --check` exit 0; evidence `artifacts/WO-BOOT-06/local-checks.txt`.

# Verification commands
Run these sequentially from the repository root; the full suite must not overlap the benchmark harness tests.

```text
python3 -m unittest tests.test_golden_diff
python3 scripts/golden_diff.py --fixture shapes-basic --actual tests/golden/fixtures/shapes-basic.png --report .cache/WO-BOOT-06/identical.json --heatmap .cache/WO-BOOT-06/identical.png --svg-sha256 6f546464f23d6cd95be30fe2c98130b84b0f75796e4578b330b0695dcf6de5c7 --build-sha 1111111111111111111111111111111111111111111111111111111111111111 --scale 1 --frame 0 --font "System Sans" --backend scalar
python3 -m unittest tests.test_golden_diff.GoldenDiffTests.test_small_channel_difference_on_edge_is_tolerated tests.test_golden_diff.GoldenDiffTests.test_shift_with_zero_tolerance_is_not_tolerated
python3 -m unittest tests.test_golden_diff.GoldenDiffTests.test_missing_baseline_is_an_error tests.test_golden_diff.GoldenDiffTests.test_bad_crc_and_filter_are_rejected tests.test_golden_diff.GoldenDiffTests.test_svg_metadata_mismatch_is_rejected tests.test_golden_diff.GoldenDiffTests.test_output_path_escape_and_symlink_are_rejected tests.test_golden_diff.GoldenDiffTests.test_existing_output_is_refused
python3 -m unittest tests.test_golden_diff.GoldenDiffTests.test_report_and_heatmap_are_byte_deterministic
./dev verify
python3 scripts/ledger.py validate ledger.jsonl
python3 scripts/wo_validate.py WO-BOOT-06 --ready
git diff --check
```

Afterward confirm every protected baseline/lock/oracle path is unchanged from the task start.

# Non-goals
No golden creation/update, rendering engine, UI, CI/workflow change, image library dependency, comparison against unavailable renderers, or public performance claim.

# Budget
Executor max runs: 2; verifier max rounds: 2; elapsed hard limit: 120 min; dependencies/network: none.

# Failure and escalation
Stop if the frozen manifest is invalid, a locked PNG is not in the declared profile, or a failure test shows a protected file changed. Do not repair or regenerate the oracle in this work order; retain exact evidence and report the blocking fixture.

# Output
Commit convention: `test: add deterministic golden image comparison`. Keep all evidence under `artifacts/WO-BOOT-06/`; update `STATE.md`, `LEDGER.md`, and `ledger.jsonl` with observed results only.
