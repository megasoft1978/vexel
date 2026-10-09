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
at the exact candidate commit. For a full repository checkout, it configures
and builds Debug and Release in that clean worktree before running the frozen
commands, so native tests never rely on ignored build outputs from the executor.
It then starts a fresh read-only Codex review with the verifier schema. Review
input is capped at 250 KB; prior verifier event,
stderr, and verdict artifacts are omitted from the review diff. Executor
reports must cite commands that the wrapper itself ran successfully. Outputs
are retained under `artifacts/runs/`.

Executor commands use `-s workspace-write` by default. Only an exact
`network: true` field in the hash-verified frozen work order selects the
owner-approved unsandboxed CLI mode; absent or malformed values remain
workspace-write. The independent verifier always uses `-s read-only`.

The runner validates the active order's SHA-256 from the repository-root copy,
uses a byte snapshot for the executor prompt, and rejects root-copy changes
before accepting output. Candidate and root scope checks hard-deny `work/**`,
`tests/oracle/**`, both frozen manifest locks, and `OWNER-DIRECTIVES.md` before
evaluating work-order allowlists. The active work order is tolerated in a
candidate diff only when it is byte-identical to the validated root copy.

## Corpus fetcher

`python3 scripts/fetch_corpus.py --verify-only` validates the checked-in
manifest digest, schema, licenses, coverage, exact fixture count, and local
fixture hashes without network access. `--fetch` skips existing exact files
and can fetch only manifest entries whose source URL is an HTTPS raw file under
the pinned WPT revision's `svg/` directory. Redirects are rejected, the
per-file cap is 2 MiB, and installation is atomic after digest verification.
`--list --feature TAG` emits compact deterministic JSON in fixture-ID order.

The fetcher uses Python's standard library. It is an explicit corpus
maintenance command; the SVG application does not call it or retrieve
document resources.

## Local Release performance gate

`./dev perf-gate` builds the Release index benchmark, runs the fixed five
cold-order and ten warm-order trials for each 10 MiB and 100 MiB case, and
writes a report plus per-metric streak history under `.cache/perf-gate/` by
default. `--output` and `--history` can select other paths under `.cache/`.
The command validates the pinned M3-01 report hash before use and compares
only exact host/toolchain and input-hash matches. One comparable breach warns;
the same metric must breach again on the next comparable observation to fail.
A healthy measurement resets its own streak; unavailable, incomparable, and
noisy observations do not update streaks. Trial CV above 10% marks that metric
noisy per the benchmark protocol. The report includes Release executable
bytes, `otool -L` dependencies, and explicit app-bundle and pan/zoom timing
unavailability. Binary, bundle, and cached pan targets remain report-only.
Raw observations and generated result reports use exact nested schemas: unknown
fields, non-finite or negative measurements, wrong trial counts, and boolean
schema versions are rejected before a result is written.

## Golden PNG comparison

`golden_diff.py` compares an actual PNG with one fixture from the frozen
`tests/golden/manifest.lock`. It verifies the baseline SHA-256 before decoding
either image and checks that the supplied SVG hash names that same fixture.
The decoder accepts the locked 8-bit RGBA, non-interlaced profile, PNG filters
0–4, standard zlib image data, and the manifest's optional six-byte `bKGD`
chunk. It checks chunk CRCs, ordering, exact decompressed size, and rejects
other color modes, bit depths, interlacing, chunks, malformed data, and images
above its byte or pixel bounds. It uses Python's standard library only.

All scalar results are exact: equal/different pixel counts, differing channel
count, maximum channel delta, total absolute channel delta, and mean absolute
channel delta across all RGBA channels. Edge tolerance is separate and explicit.
A differing pixel is tolerated only when its maximum channel delta is within
`--edge-channel-tolerance` and it is within Chebyshev distance
`--edge-radius` of an 8-neighbor color or alpha boundary in the expected image.
Defaults are radius 1 and channel delta 1; radius accepts 0–2 and channel delta
0–16. Radius zero considers only differing pixels that are themselves boundary
pixels; larger radii include nearby pixels. Exit status is 0 when all differences
are tolerated, 1 when any are not, and 2 for input, metadata, format, or output
errors.

The deterministic RGBA heatmap uses transparent pixels for unchanged areas,
yellow for tolerated differences, and red for failing differences. The JSON
report schema is version 1 and includes fixture and input hashes, pinned SVG
hash, build SHA, scale/frame/font/backend metadata, dimensions, tolerances,
all exact metrics, tolerance counts, and result. Metadata is required; hashes
must be 64 lowercase hexadecimal characters, scale must be 0.01–64, frame
0–1,000,000, and font/backend are bounded printable labels. Report and heatmap
paths must be new paths under repository `.cache/`; symlinks and overwrites are
refused. The utility has no baseline-writing mode.

Example (replace the actual PNG and build SHA with the render under review):

```sh
python3 scripts/golden_diff.py \
  --fixture shapes-basic \
  --actual .cache/render/shapes-basic.png \
  --report .cache/golden/shapes-basic.json \
  --heatmap .cache/golden/shapes-basic-diff.png \
  --svg-sha256 6f546464f23d6cd95be30fe2c98130b84b0f75796e4578b330b0695dcf6de5c7 \
  --build-sha 0000000000000000000000000000000000000000000000000000000000000000 \
  --scale 1 --frame 0 --font 'System Sans' --backend scalar \
  --edge-radius 1 --edge-channel-tolerance 1
```

Changing a golden baseline requires a separate READY work order and a recorded
reason. This comparison work order cannot update or regenerate baselines.

## UI test driver

`app/test_driver/driver.py` replays bounded JSONL placeholder events on macOS
and emits deterministic PNG, semantic accessibility, and saved-state files
under a new `.cache/` directory. It uses only Python's standard library; it
does not open files named by the script or render SVG. See
[`docs/ui-test-driver.md`](../docs/ui-test-driver.md) for the exact command
schema, limits, and invocation.
