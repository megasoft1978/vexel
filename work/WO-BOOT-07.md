---
id: WO-BOOT-07
revision: 2
supersedes: 1
status: READY
milestone: M0
owner: project-owner
executor: codex-gpt-6-luna
base_sha: 41fe3393793a1a982591b63092a6f707d541a431
spec_sha256: 2a138e5bd0fc19c081656b04259b4512050d662b3626276e0b61d282f3fafe36
parallel_safe: false
network: false
---
# Goal
Add a deterministic JSONL UI test driver that replays bounded placeholder app events and writes stable screenshots, semantic accessibility snapshots, and saved state.

# Context pointers
- Read: `AGENTS.md`, `OWNER-DIRECTIVES.md`, `STATE.md`, `app/main.c`, `docs/research-r4/01-operating-model.md`, `docs/research-r4/02-work-order-template.md`, `docs/research-r4/04-llm-aware-design-changes.md`, `docs/research-r4/05-revised-roadmap.md`, `docs/research-r3/02-research-gaps.md`, `docs/CODING_STANDARD.md`, `scripts/README.md`, `tests/README.md`.
- Relevant ADRs: `docs/adr/0001-product-and-platform-locks.md`, `docs/adr/0002-source-preservation-and-provenance.md`.
- Frozen inputs: all golden and corpus manifests/fixtures and all `tests/oracle/**`; read only.
- External docs: none required. Use Python standard library only; do not copy external UI or PNG code.

# Existing contracts and invariants
- Apple Silicon macOS 13+ only. Keep the product C17; this driver is a bounded developer test harness and does not introduce a UI framework or third-party dependency.
- The driver is a placeholder, not an SVG renderer or native window. `open` records a bounded document ID; it never reads a user path, fetches resources, or executes content.
- JSONL input, command count, individual strings, dimensions, pixel allocation, and virtual time are bounded. Invalid input exits nonzero with a concise error and never emits a success summary.
- Output paths must be new, non-symlink paths under repo-local `.cache/`. All artifacts are deterministic for identical script and seed; no wall-clock or sleep synchronization.
- PNG output is 8-bit RGBA, non-interlaced, encoded by the Python standard library. Accessibility output is a stable semantic tree with IDs, roles, names, values, states, and bounds.

# Files allowed to touch
- `app/test_driver/**`
- `tests/ui/**`
- `tests/test_ui_driver.py`
- `docs/ui-test-driver.md`
- `scripts/README.md`
- `tests/README.md`
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`
- `artifacts/WO-BOOT-07/**`

# Protected paths
- `OWNER-DIRECTIVES.md`, `work/**`, `app/main.c`, `src/**`, `platform/**`, `CMakeLists.txt`, `CMakePresets.json`, `dev`
- `tests/oracle/**`, `tests/golden/**`, `bench/corpus/**`, `.github/**`
- All paths outside the explicit allowlist
- Any non-macOS implementation or test path

# Required implementation
1. Add `python3 app/test_driver/driver.py --script PATH --output-dir .cache/PATH [--seed N]`. Reject invalid/missing CLI inputs with exit 2. Refuse non-macOS execution and paths outside the repository/cache contract.
2. Read UTF-8 JSONL with a maximum of 8 MiB, at most 10,000 commands, and at most 65,536 UTF-8 bytes per line. Reject blank lines, invalid JSON, duplicate/unknown object keys, unknown commands, wrong field types, missing fields, non-finite values, and out-of-range values. Require one `open` first and one final `quit`; reject commands after quit.
3. Support exact command forms: `{"command":"open","document_id":"..."}`; `resize` with integer width/height; `pointer` with action `move|down|up` and integer x/y; `key` with action `down|up` and bounded key string; `text_commit` with bounded text; `frame` with integer `delta_ns`; `progress` with integer percent 0–100 and bounded label; `accessibility` with no extra fields; `screenshot` with no extra fields; `save` with no extra fields; `quit` with no extra fields. Reject bools where integers are required.
4. Start at 320x240 and virtual time 0. Resize dimensions must each be 1–4096 and total pixels at most 16,777,216. Virtual time accumulates only from `frame`, with each delta 0–1,000,000,000 ns and total at most 1,000,000,000,000 ns. Pointer coordinates must lie inside the current window. Seed is an unsigned 32-bit integer and affects a deterministic placeholder accent.
5. Implement a simple placeholder raster (not SVG rendering) and semantic tree. `screenshot` writes sequential `frame-NNNN.png` files; `accessibility` writes sequential `accessibility-NNNN.json`; `save` writes deterministic `document.json`. The output directory must not exist before invocation. Emit one deterministic JSONL acknowledgment per command on stdout, using relative output filenames.
6. Add tests for every command, bounds and invalid types/order, invalid CLI, output escape/symlink/overwrite protection, output structure, seed sensitivity, and two complete identical replays whose output filenames, bytes, and stdout are identical.
7. Document command schema, limits, deterministic behavior, placeholder limitations, output files, and invocation.

# Acceptance criteria (machine-checkable)
- AC-1 `python3 -m unittest tests.test_ui_driver` exits 0 with zero skips; evidence `artifacts/WO-BOOT-07/tests.txt`.
- AC-2 `python3 -m unittest tests.test_ui_driver.DriverTests.test_all_commands_emit_placeholder_artifacts tests.test_ui_driver.DriverTests.test_identical_replay_is_byte_identical` exits 0; evidence `artifacts/WO-BOOT-07/replay-tests.txt`.
- AC-3 `python3 -m unittest tests.test_ui_driver.DriverTests.test_invalid_cli_and_jsonl_are_rejected tests.test_ui_driver.DriverTests.test_path_escape_symlink_and_existing_output_are_rejected tests.test_ui_driver.DriverTests.test_bounds_and_command_order_are_enforced` exits 0; evidence `artifacts/WO-BOOT-07/failure-tests.txt`.
- AC-4 Run the committed mixed-command script using seed 17 to `.cache/WO-BOOT-07/smoke`, then assert it produced at least one valid PNG, accessibility JSON, and saved document JSON; evidence `artifacts/WO-BOOT-07/smoke.txt`.
- AC-5 `./dev verify`, `python3 scripts/ledger.py validate ledger.jsonl`, `python3 scripts/wo_validate.py WO-BOOT-07 --ready`, and `git diff --check` exit 0; evidence `artifacts/WO-BOOT-07/local-checks.txt`.

# Verification commands
Run these sequentially from the repository root; no tests may overlap.

```text
python3 -m unittest tests.test_ui_driver
python3 -m unittest tests.test_ui_driver.DriverTests.test_all_commands_emit_placeholder_artifacts tests.test_ui_driver.DriverTests.test_identical_replay_is_byte_identical
python3 -m unittest tests.test_ui_driver.DriverTests.test_invalid_cli_and_jsonl_are_rejected tests.test_ui_driver.DriverTests.test_path_escape_symlink_and_existing_output_are_rejected tests.test_ui_driver.DriverTests.test_bounds_and_command_order_are_enforced
python3 -c "import shutil; from pathlib import Path; shutil.rmtree(Path('.cache/WO-BOOT-07/smoke'), ignore_errors=True)"
python3 app/test_driver/driver.py --script tests/ui/mixed-commands.jsonl --output-dir .cache/WO-BOOT-07/smoke --seed 17
python3 -m unittest tests.test_ui_driver.DriverTests.test_smoke_output_contract
./dev verify
python3 scripts/ledger.py validate ledger.jsonl
python3 scripts/wo_validate.py WO-BOOT-07 --ready
git diff --check
```

Afterward confirm every protected path is unchanged from the task start.

# Non-goals
No native window, widgets, SVG parser or renderer, actual file open/save, external dependencies, CI changes, baseline updates, performance claims, or cross-platform implementation.

# Budget
Executor max runs: 2; verifier max rounds: 2; elapsed hard limit: 120 min; dependencies/network: none.

# Failure and escalation
Stop if a test requires changing protected paths, a platform-specific assumption cannot be checked on this Mac, or any frozen oracle changes. Preserve failing evidence; do not weaken tests or broaden path scope.

# Output
Commit convention: `test: add deterministic UI script driver`. Keep all evidence under `artifacts/WO-BOOT-07/`; update `STATE.md`, `LEDGER.md`, and `ledger.jsonl` with observed results only.
