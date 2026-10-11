---
id: WO-UI-GOLDEN-01
revision: 1
spec_sha256: 695f2a5f70979723297287961ed5a6bfee1720e4643b13a216667d596bae2e63
status: READY
milestone: UI
owner: project-owner
executor: codex-gpt-6-luna
base_sha: c43fa826c0fbc8910e8dc0c09b4139c8aa615995
parallel_safe: true
network: true
---
# Goal
Diagnose the four UI golden CTest failures on the migrated macOS `main` tree by preserving the exact renderer outputs, measuring differences against the checked-in expected images, and identifying reproducible platform or renderer causes. This order is diagnostic only.

# Context pointers
- Read `AGENTS.md`, `OWNER-DIRECTIVES.md`, `STATE.md`, `docs/CODING_STANDARD.md`, `.github/workflows/ci.yml`, and each verifier named in the four failed CTests.
- Source run: GitHub Actions macOS CI run `38147398376`, commit `c43fa826c0fbc8910e8dc0c09b4139c8aa615995`.
- The run log reports failures for `native-shell-goldens`, `editor-style-goldens`, `editor-snap-overlay-goldens`, and `editor-accessibility-localization-goldens`; it does not retain rendered images.

# Existing contracts and invariants
- Apple Silicon macOS 13+ only; C17 and existing dependencies only.
- Every expected UI image, manifest, golden assertion, comparison threshold, and verifier pass/fail rule is read-only and must remain byte-identical.
- Do not regenerate, bless, rename, or relax any golden or assertion. Do not modify `.github/workflows/nightly.yml`.
- Diagnostic output must be separate from checked-in goldens, bounded, and clearly labeled with source commit, OS/build metadata, dimensions, image hashes, and comparison metrics.
- BetterDisplay or monitor configuration may be recorded as machine context, but no monitor-dependent cause may be claimed without supporting evidence.

# Files allowed to touch
- `work/WO-UI-GOLDEN-01.md` only as this frozen work order.
- `scripts/ui_golden_audit.py`
- `.github/workflows/ci.yml`
- `artifacts/WO-UI-GOLDEN-01/**`
- `STATE.md`
- `LEDGER.md`
- `ledger.jsonl`

# Protected paths
- `OWNER-DIRECTIVES.md`, `AGENTS.md`, all other `work/**`, `tests/golden/**`, `tests/oracle/**`, all UI golden directories and manifests, all verifier scripts and tests, all app/source/platform/CMake paths, `.github/workflows/nightly.yml`, build presets and dev scripts, corpus inputs, all prior evidence, and every path outside the allowlist.

# Required diagnosis
1. Capture the renderer output for the four failed CTest categories using their existing binaries and inputs, without invoking any golden update mode.
2. Compare each actual PNG against the corresponding checked-in expected PNG without changing the verifier. Record dimensions, PNG SHA-256, exact-equal status, differing pixels/channels, maximum and mean absolute channel delta, and bounding box. Keep actual images and visual difference images as diagnostic outputs.
3. Record macOS version/build, architecture, renderer executable hashes, source commit, and relevant display scaling context when available. Distinguish direct observations from hypotheses.
4. Add a CI diagnostic step after `./dev verify` that runs even when verification fails and retains the generated actual images/reports as a downloadable artifact. It may change `.github/workflows/ci.yml`; it must not alter test commands or success conditions. The diagnostic step must fail visibly if it cannot capture an image, while artifact upload itself uses `if: always()`.
5. Run the capture on the exact CI source revision only if retained runner/artifact access permits. If actual images for run 38147398376 cannot be recovered, report that limitation and use a new CI run only after the diagnostic instrumentation commit is available; do not attribute a local reproduction to the historical run.
6. Keep analysis diagnostic. Any proposed implementation fix or golden change requires a separate READY work order.

# Acceptance criteria
- AC-1: `python3 scripts/wo_validate.py WO-UI-GOLDEN-01 --ready` and `python3 scripts/ledger.py validate ledger.jsonl` pass; the READY ledger event records this exact order hash and base SHA.
- AC-2: `scripts/ui_golden_audit.py` captures actual images and machine-readable comparisons for the four categories, writes only to `.cache/ui-golden-audit/` or an explicitly selected new diagnostic directory, and never modifies checked-in goldens.
- AC-3: All available failures have actual-versus-expected PNG evidence, hashes, dimensions, pixel metrics, and clear environment attribution. Historical CI availability is stated precisely.
- AC-4: `.github/workflows/ci.yml` retains artifacts when `./dev verify` fails; test commands and gate semantics remain unchanged. `.github/workflows/nightly.yml` and golden/assertion files remain unchanged.
- AC-5: `git diff --check` passes and the exact allowed-path guard confirms all changes are within this order's allowlist.
- AC-6: `STATE.md` and `LEDGER.md` record the diagnostic outcome and limitations; completion is recorded in `ledger.jsonl` with evidence paths.

# Verification commands
```text
python3 scripts/wo_validate.py WO-UI-GOLDEN-01 --ready
python3 scripts/ledger.py validate ledger.jsonl
python3 scripts/ui_golden_audit.py --output .cache/ui-golden-audit
git diff --check
```

# Budget
Two executor runs; one read-only review; no change to golden/assertion behavior.

# Failure and escalation
If the needed renderer cannot run or an expected image cannot be compared, retain logs, identify the exact missing input/capability, and stop short of causal claims. Do not edit any protected path to work around a missing diagnostic.

# Output
Commit the frozen order first, then commit the diagnostic instrumentation and resulting evidence. Do not push or merge.
