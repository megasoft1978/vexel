# Ledger utility

`ledger.py` validates the machine-readable `ledger.jsonl` history and appends a
single transition. The schema is `docs/schemas/ledger-entry.schema.json`;
transition rules reject invalid event order, repeated run IDs, and entries that
refer to missing or modified frozen work orders. The frozen hash covers the
UTF-8 file except its own `spec_sha256` line.

```sh
python3 scripts/ledger.py validate ledger.jsonl
python3 scripts/ledger.py append ledger.jsonl --entry-file .cache/next-entry.json
python3 scripts/ledger.py show ledger.jsonl WO-BOOT-01
python3 scripts/ledger.py summary ledger.jsonl
```

Appends serialize through an advisory `fcntl` lock in ignored `.cache/ledger/`.
Each update is written and fsynced to a temporary file in the ledger directory,
then atomically replaces the log while the lock is held. Invalid input is
rejected before the ledger is changed. The utility uses only Python's standard
library and accepts ledger paths inside this repository.

## Test runner and sanitizers

`test_runner.py` discovers the complete Python `unittest` suite and emits
`docs/schemas/test-results.schema.json` results: UTC start/end, monotonic
elapsed seconds, outcome counts, and a status per test ID. `./dev test-fast`
writes transient JSON to `.cache/test-results/latest.json`, fails on skipped
or unsuccessful tests, then validates the append-only ledger. It uses only the
Python standard library.

`./dev asan` uses the `mac-asan` CMake configure/build/test presets with
AddressSanitizer and UndefinedBehaviorSanitizer. `./dev ubsan` uses
`mac-ubsan` with UndefinedBehaviorSanitizer. Both profiles require Xcode Apple
Clang and retain the project's warning-as-error options. CTest runs
`svgworkbench --version`. `scripts/sanitizer_probes.py --profile asan-ubsan`
injects an assertion and heap overflow; its exact C sources, stdout/stderr,
diagnostics, and JSON summary are retained in `artifacts/WO-BOOT-05/probes/`.
These deliberately failing probe processes are expected detections; ordinary
test failures remain regressions. Later verification reruns preserve that
committed evidence and write fresh traces/results under ignored
`.cache/sanitizer-probes/`, so successful post-commit checks leave the candidate
tree clean.

## CLI fuzz smoke

`./dev fuzz [--duration SEC] [--seed INTEGER]` builds the ASan/UBSan profile,
then runs the no-network argv fuzzer on the CLI. The default seed is `12648430`,
and the JSON reports the selected seed, iterations, actual duration, and any
failure. Duration defaults to three seconds and is capped at sixty seconds.
The runner targets 48 cases per requested second, stops starting cases at the
deadline, and grants an in-flight case at most 0.1 seconds of completion grace.
This fixed case budget keeps repeat runs comparable while the deadline bounds
slow runs. Exact valid `--version` and help output are accepted. Usage-error exit is expected. Signals,
sanitizer findings, timeouts, and unexpected exits fail and retain argv plus
stdout/stderr in `artifacts/WO-BOOT-05/fuzz-failures/`. The scheduled workflow
runs a 60-second smoke; pull requests and manual dispatch run three seconds.
Its checker enforces the macOS Apple Silicon runner, bounded runtime, and
pinned actions.

## Work-order runners

`wo-run.sh` and `wo-verify.sh` are thin delegates to `wo_driver.py`. They are
macOS-only. The driver records CLI version and help before work, creates an
isolated `.worktrees/WO-ID` task checkout, persists session IDs as JSONL events
arrive, and checks candidate commits, reports, path scope, and repository state.
The current CLI bypass exception and its limitations are recorded in
`docs/adr/0003-codex-cli-compatibility-amendment.md`.

Each work order provides a `# Verification commands` fenced `text` block. The
verifier runs those argv lines without shell evaluation in a detached worktree
at the exact candidate commit, then starts a fresh read-only Codex review with
the verifier schema. Review input is capped at 250 KB; prior verifier event,
stderr, and verdict artifacts are omitted from the review diff. Executor
reports must cite commands that the wrapper itself ran successfully. Outputs
are retained under `artifacts/runs/`.
