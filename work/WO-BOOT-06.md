---
id: WO-BOOT-06
revision: 1
status: READY
milestone: M0
owner: project-owner
executor: codex-gpt-6-luna
base_sha: 6d087941486cc5834369d07eff0058b59866fb78
spec_sha256: 71620f9467a91276729eacc581d4b05b1ebc092aaac37daceef2cc4b77db0e6c
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

# Verification sequence
1. Run `python3 -m unittest tests.test_golden_diff`.
2. Run the pinned identical-image comparison and verify its JSON; store evidence under the named artifact directory and transient outputs under `.cache/`.
3. Run tolerance and negative-path tests, including two-run byte determinism.
4. Run `./dev verify` and the ledger/work-order/diff checks in AC-6.
5. Confirm `tests/golden/manifest.lock`, every baseline PNG, the corpus manifest, and all oracle files are byte-identical to the task base.

# Non-goals
No golden creation/update, rendering engine, UI, CI/workflow change, image library dependency, comparison against unavailable renderers, or public performance claim.

# Budget
Executor max runs: 2; verifier max rounds: 2; elapsed hard limit: 120 min; dependencies/network: none.

# Failure and escalation
Stop if the frozen manifest is invalid, a locked PNG is not in the declared profile, or a failure test shows a protected file changed. Do not repair or regenerate the oracle in this work order; retain exact evidence and report the blocking fixture.

# Output
Commit convention: `test: add deterministic golden image comparison`. Keep all evidence under `artifacts/WO-BOOT-06/`; update `STATE.md`, `LEDGER.md`, and `ledger.jsonl` with observed results only.
