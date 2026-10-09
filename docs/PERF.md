# Performance baseline

## Local Release performance gate

Run `./dev perf-gate` on Apple Silicon macOS to build the Release index
benchmark and capture the frozen five cold-order plus ten warm-order trials
for each 10 MiB simple and 100 MiB map-like input. Cold/warm are run-order
labels; the command does not evict the OS file cache. Reports and per-metric
violation history default to ignored `.cache/perf-gate/`; use `--output` and
`--history` to choose other paths under `.cache/`.

The gate pins `bench/results/WO-M3-01.json` by SHA-256 and records its source
commit, host/toolchain identity, input hashes, Release executable hash and
bytes, `otool -L` identities, raw trials, and median/p95/maximum-step/peak-RSS
summaries. Results are comparable only when both generated input hashes and
the exact model, chip, memory, architecture, OS, and compiler identity match
the pinned baseline. Incomparable and unavailable observations do not mutate
violation streaks. A healthy comparable value resets only its own metric; one
breach warns and a second consecutive comparable breach fails.

Measured limits are estimates: 10 MiB first preview below 1 s, 100 MiB first
preview below 2 s, maximum owner step below 100 ms, and 100 MiB peak RSS at or
below 500 MiB. The single-architecture stripped executable (10 MiB), app bundle
(25 MiB), and cached pan/zoom event-to-present p95 (33 ms) remain report-only
stretch targets. The current navigation correctness workload does not time
presentation, and no app-bundle packaging target exists; both are reported as
unavailable with reasons. Spatial-query timing is not pan latency.

The baseline record `bench/results/perf-gate-baseline.json` preserves the
M3-01 report hash and measured values without modifying that report. The
implementation evidence and current machine-check output are retained under
`artifacts/WO-BOOT-08/`.

These measurements establish a local starting point. The `--version` command
only measures process startup and is not an SVG rendering benchmark.

| Work order | Build | Binary bytes | Latency (workload) | Max RSS | Machine |
|---|---|---:|---|---:|---|
| BOOT-01 | Release | 33,568 | `--version`: 1.910 ms median (31 runs) | 1,884,160 bytes (one run) | Apple Silicon, macOS 27.0, Apple Clang 21.0.0 |
| WO-M1-03 | Release | 94,752 | placeholder replay: 100.976 ms; Metal replay: 179.164 ms | not measured | Apple Silicon, macOS 27.0, Apple Clang 21.0.0 |
| WO-M1-04 | Release | 97,072 | 1,000 task cycles: 44 ms; max join: 0.073 ms | not measured | Apple Silicon, macOS 27.0, Apple Clang 21.0.0 |
| WO-M2-01 | Release | 97,040 | no renderer workload | not measured | Apple Silicon, macOS 27.0, Apple Clang 21.0.0 |
| WO-M2-02 | Release | 97,040; XML static library 16,552 | parser throughput not measured | not measured | Apple Silicon, macOS 27.0, Apple Clang 21.0.0 |
| WO-M2-05 | Release | 97,040; shape CLI 53,408; shape library 24,744 | 4.29 MPixel/s (256x256, 64 rects; 7 fresh-process trials, median 15.265 ms) | 2,244,608 bytes | Apple Silicon, macOS 27.0, Apple Clang 21.0.0 |
| WO-M3-01 | Release | 151,968 bytes; see `bench/results/WO-M3-01.json` | source read: 10 MiB median 0.890 ms (p95 0.932 ms); 100 MiB 8.626 ms (p95 8.918 ms); validated decode+raster preview: 10 MiB median 27.81 ms (p95 28.80 ms); 100 MiB median 257.05 ms (p95 269.12 ms); full index+XML validation 44.84 ms / 334.21 ms | 33.9 MB / 127.0 MB peak child RSS | Mac Studio Mac17,14, M5 Max, macOS 27.0, Apple Clang 21.0.0 |
| WO-M3-02 | Release | benchmark binary 35,376 bytes | 100,000 records / 1,000 viewport queries: index build 8.729 ms; query p50 0.000292 ms, p95 0.001833 ms, max 0.022583 ms | cache pressure accounted peak 6,000 / 8,192 bytes over 10,000 inserts, including the allocated 100-entry metadata array | Mac Studio Mac17,14, M5 Max, macOS 27.0, Apple Clang 21.0.0 |

Raw latency metadata is in `bench/results/WO-BOOT-01.json`. RSS came from
`/usr/bin/time -l`. WO-M2-05 records one bounded end-to-end shape CLI workload
below; it does not represent general SVG throughput.

## WO-M1-04 task queue lifecycle

The macOS queue bounds capacity to 1–64 tasks and worker count to 1–4. Context
memory is borrowed and must remain alive through callback completion or queued
cancellation. Cancellation is cooperative through an atomic token. Generation
advances suppress results from older work. Shutdown requests cancellation,
wakes and joins workers, then destroy releases storage. The owner quiesces all
callers before destroy; an uncooperative callback can delay join. Lifecycle
timings are correctness bounds only and are not performance claims.

## WO-M3-01 progressive index candidate

The 10 MiB input contains 100,000 simple rectangle elements. The 100 MiB
map-like input contains 100,000 grouped records with bounded attributes. Each
campaign records five first-use and ten warm process trials; OS file caches
were not flushed, so cold/warm labels indicate run order only. First-preview
latency includes shape decoding and tile rasterization on the task worker;
source read and complete-index timings are separate. Both cases remained under
the 100 ms per-step and 128 MiB + 3x-source RSS gates. Raw trials and generated
input hashes are in `bench/results/WO-M3-01.json`.

The resumable reader advances in bounded owner-thread steps and emits complete
tag source spans. Full token spans become queryable after the cancellable
background validator completes; it does not stream the validator's full token
records through the index API. The current app replay remains headless and does
not yet wire an open-document UI to this session. These integration boundaries
remain open for review.

## WO-M3-02 visible queries and tile cache

The fixed-seed Release workload indexes 100,000 caller records and runs 1,000
viewport queries. The packed index build took 8.729 ms; query p50/p95/max were
0.000292/0.001833/0.022583 ms. The named Mac p95 target of 5 ms was met. The
benchmark records every query's result count, compiler, build type, binary
size, and host model in `bench/results/WO-M3-02.json`.

The randomized differential test compares 2,000 query result arrays byte for
byte with the naive source-order oracle. Tile-cache pressure performs 10,000
insertions; maximum accounted storage was 6,000 bytes under an 8,192 byte cap,
including all 100 preallocated entry slots. A deterministic clock rollover test exercises the two-part LRU timestamp. The
benchmark separately verifies that an entry whose payload plus metadata
exceeds its 4,096-byte cap is rejected without taking ownership.

## WO-M0-03 black-box protocol

The standard-library harness in [`bench/harness`](../bench/harness/README.md)
checks all 24 project-pinned corpus fixture hashes before every run. The
comparison tasks use the same three fixtures (`shapes-basic`,
`stress-many-shapes`, `stress-long-path`) at 256x256 for each configured
compatible renderer. It launches five fresh processes for each fixture and
condition, records raw trials, and reports median and nearest-rank p95 only
when five valid observations exist. `cold-process` describes process launch;
the harness does not evict filesystem caches. Commands are argv arrays that
must receive `{input}`, `{output}`, `{width}`, and `{height}`; time, capture,
and output sizes are bounded. Before each run, the harness parses the pinned
SVGs and rejects external references, stylesheet imports, and XML external
declarations.
Adapters run under the macOS sandbox with process forks and network access
denied. Writes are limited to the trial's designated output file under
`.cache`, capped at 2 MiB; at most eight adapters can be configured. Renderers
that require helper processes, sidecar files, or external resources fail
explicitly instead of escaping the measurement boundary. A renderer is
available only after a successful, non-empty version probe.

The current smoke run verified all 24 fixture hashes and the manifest digest
`a05dcf0da95bd7c53ce6faec6704f7b739d6cbd9cf829dfff74b50dfb8359bbd`. Inkscape
and Donner executables were unavailable, so there are no competitor render
measurements. Vexel has no compatible render command, so render measurements
are unavailable. Smoke mode does not launch Vexel; it records the version probe
as unperformed. `--run` separately records five Vexel `--version` process launches;
that startup task is never compared with SVG render latency. Peak RSS is
unavailable from the bounded standard-library child process interface on this
host. No performance advantage is claimed.

## WO-M1-03 deterministic replay

The Release executable is 94,752 bytes. The fixed placeholder JSONL scenario
completed 20/20 identical replays with combined SHA-256
`c46e612bbfa618d18a27693c33a952b9fce16bb941e3d9b6f5048b7bfd280c59`; one
measured invocation took 100.976 ms. Two further outer invocations against the
same output root reserved distinct numbered directories and produced the same
digest. These results cover placeholder event replay, PNG/accessibility/saved
state output hashing, and virtual frame advancement only.

The initial visible-window replay crashed during AppKit teardown after the 20
Metal frames. LLDB and AddressSanitizer localized the fault to
`platform_window_destroy`'s autorelease-pool drain. The replay core now uses
headless Metal blit command buffers while windowed Metal presentation remains
separately smoke-tested. Twenty readbacks matched byte-for-byte with SHA-256
`3d636badd712ea35b36c861871517af1a2a0809d0a0a418efd5419b4f4c1196c`; measured
replay latency was 179.164 ms. No SVG throughput or RSS was measured. Raw
evidence is in `artifacts/WO-M1-03/` and `bench/results/WO-M1-03.json`.

## WO-M1-04 bounded task queue

Revision 3 completed 1,000 create/submit/cancel/join cycles in 0.044 s. The
maximum measured queue shutdown join was 0.000073 s against the 2 s limit;
queued and running cancellation passed and no stale result was published. The
Release executable is 97,072 bytes. Focused tests passed 20/20 and the full
suite passed 143/143; ASan and UBSan CTest each passed 1/1. Test-only pointer
injection dispatches directly to its target view, so the native lifecycle
smoke does not depend on LaunchServices activation. RSS, SVG throughput, and
renderer latency were not measured. Evidence is in `artifacts/WO-M1-04/` and
`bench/results/WO-M1-04.json`.

## WO-M2-01 checked base API

The Release `svgworkbench` executable is 97,040 bytes and the base static
library is 3,504 bytes (Apple Clang 21.0.0.21000334, Apple Silicon arm64,
macOS 27.0). The deterministic arena boundary test uses 16-byte initial
chunks and a 96-byte total payload limit: its alignment sequence (1, 2, 4, 8,
16, 64, 256, 4096) retains 5 chunks with 80 bytes of payload capacity and 13
used bytes including alignment padding. The injected allocation-failure test
records 6 allocator calls, 4 frees after destroy, and zero live allocations;
the failed push leaves arena statistics unchanged.

The base contract test passed in Debug, ASan+UBSan, and UBSan CTest runs. Debug
CTest also passed the existing CLI smoke (2/2 total). `./dev verify` passed all
143 Python tests and both Debug CTest cases. The revision-1 attempt's transient
benchmark sandbox and LaunchServices failures are retained under
`artifacts/WO-M2-01/r1-attempt/`; the revision-2 rerun passed. No SVG
throughput, RSS, or performance-improvement claim is made. Detailed output is
in `artifacts/WO-M2-01/` and the machine-readable result is
`bench/results/WO-M2-01.json`.

## WO-M2-02 XML tokenizer

The tokenizer is an allocation-free, iterative lexical scanner with caller-owned
token storage and a fixed 256-name depth stack. It validates 24 pinned corpus
fixtures against Python 3.12.8 / Expat 2.6.4; element and attribute counts
match on every fixture. The deterministic malformed generator checked 100,000
inputs. The ASan+UBSan fuzz run used seed 20261009 for 60.000003 seconds and
completed 4,318,388 generated inputs with zero sanitizer reports and valid
span/offset invariants. Debug, ASan, and UBSan CTest each passed 3/3.

The full `./dev verify` passed from the host shell: all 147 Python tests,
Debug CTest 3/3, native window smoke, and 20-frame Metal replay. Three earlier
attempts inside the executor's nested workspace-write sandbox failed in
existing sandbox-exec and LaunchServices/XPC checks; those are retained as
transient isolation evidence and did not reproduce on the host. No renderer
throughput, RSS, or performance-improvement claim is made.

## WO-M2-03 bounded XML tokenizer fuzz

The fixed seed 20261009 completed the 30-second smoke in 30.01 seconds with
2,158,912 cases and the 600-second campaign in 600.02 seconds with 42,108,425
cases. Both used the same ASan/UBSan fuzzer binary (SHA-256
`1e90528160b496724303b36cead5e339ad1ab6179252701f0a072a44151608cf`) built
with Apple Clang 21.0.0. No sanitizer output or invariant failure was reported.
These are bounded tokenizer fuzz counts, not rendering performance measurements;
no throughput claim is made. The synthetic canary is explicitly excluded from
parser defect counts. The first independent review also caught missing iteration
preservation for invariant failures and a regression test that did not execute
retained seeds through the native tokenizer; both were fixed and focused tests
now pass 14/14. The second review also found missing native replay flushing,
regression fingerprint validation, and `/var/folders` normalization; all three
are fixed. The initial executor run lacked the Release binary needed
by two native platform tests. After building the Release preset, `./dev verify`
passed all 161 Python tests, native smoke, and 20-frame Metal replay; Debug,
ASan, and UBSan CTest also pass. The final fresh read-only review passed all 13
frozen commands without findings; the work is merged at
`f53cc4322d86c7228287578affa6825154f1f1e5`. Evidence is under
`artifacts/WO-M2-03/` and `artifacts/runs/WO-M2-03/verify-eebaef9f-4245-4e61-8b87-d81b2f3acb73/`.

## WO-M2-04 lossless no-op source save

The no-op save validates bounded XML and atomically replaces an existing
regular file through a same-directory temporary file. C and Python authored
fixtures cover metadata, namespaces, comments, significant whitespace, mixed
LF/CRLF/CR endings, quote styles, processing instructions, and CDATA. Every
recorded source and saved byte count and SHA-256 match. Successful native tests
report temporary-file fsync, atomic rename, and parent-directory fsync; fault
injection confirms parse, write, and rename failures keep the original target
hash/mode and leave no temporary file. No save latency, renderer throughput,
RSS, or performance improvement was measured. Evidence is in
`artifacts/WO-M2-04/` and `bench/results/WO-M2-04.json`.

The managed executor's full-suite attempts failed in environment-dependent
benchmark sandbox and LaunchServices checks. The host-shell `./dev verify`
rerun passed all 165 tests, native smoke, and Metal replay after the Release
build. The final fresh review passed all ten frozen commands with no findings;
the implementation is merged at
`44945019b06bb09e67881abe645c2e453b979427`. Evidence is under
`artifacts/runs/WO-M2-04/verify-16e8fdc9-d718-4d8a-87fe-bc31a406d54b/`.

## WO-M2-05 bounded shape rendering

The supported-shape comparison uses `/opt/homebrew/bin/rsvg-convert`, version
2.62.3 (Cairo 1.18.4, Pango 1.58.2, HarfBuzz 14.3.1). Seven project-authored
fixtures cover rectangles, closed lines, cubic and quadratic curves, elliptical
arcs, even-odd fill, and degenerate geometry. The Python harness renders each
twice and requires byte-identical Vexel RGBA output. Reference comparison is
exact in the interior and allows differences only within one pixel of either
renderer coverage boundary. Edge alpha delta is capped at 160, covered RGB delta
at 1, and differing pixels at 120 per fixture; all seven cases pass. Source and render hashes and
per-case pixel differences are recorded in
`artifacts/WO-M2-05/reference-comparison.json`.

The fixed-seed ASan+UBSan campaign (`20261009`) completed 1,000,000 random cases in
6.579 s wall time (6.522 s process CPU time), with zero sanitizer reports,
crashes, or invariant failures. Random inputs were at most 4,096 bytes; a
deterministic 327,730-byte seed also exhausted the command cap safely. The result is
in `artifacts/WO-M2-05/fuzz-report.json`. Release sizes on Apple Silicon macOS
27.0 with Apple Clang 21.0.0 (`clang-2100.3.34.2`) are recorded in
`bench/results/WO-M2-05.json`: `svgworkbench` 97,040 bytes, shape CLI 53,408,
shape static library 24,744, and XML static library 16,552.

For a deterministic 256x256 SVG containing 64 rectangles, seven fresh-process
Release CLI trials after two warmups had a 15.265 ms median and 16.618 ms
nearest-rank p95 (4.29 MPixel/s using total viewport pixels). Maximum RSS was
2,244,608 bytes. Timing includes process startup, file read, decode, scalar
raster, and raw output write; caches were not evicted. This is not isolated
raster-kernel throughput. Raw trials and hashes are in
`artifacts/WO-M2-05/render-benchmark.json`.

The shape-specific build, CTest, focused differential tests, ASan/UBSan fuzz,
UBSan CTest, Release build, and final `./dev verify` pass. The full suite
reports 173 Python tests with zero failures/errors/skips; Debug/ASan/UBSan CTest
each pass 5/5, and native smoke plus 20-frame Metal replay pass. Three initial
managed runs had sandbox and LaunchServices failures that did not reproduce;
see `artifacts/WO-M2-05/verification-transients.md`. This is no longer a
blocker. No general SVG throughput or performance advantage is claimed.

## WO-M3-03 scheduler lifecycle benchmark

The Release worker test ran exactly 1,000 scheduler create/submit/cancel/join
cycles with one worker. Latest total elapsed time was 0.018880 s; maximum measured
destroy/join time was 0.000110 s, with 1,000 accepted cancellation requests.
The final reviewed test binary is 70,368 bytes, built with AppleClang 21.0.0
(`clang-2100.3.34.2`) for arm64 on macOS 27.0, model Mac17,14. The scan is
bounded deterministic test work, not SVG rendering throughput. Exact metadata
is in `artifacts/WO-M3-03/schedule-benchmark.json` and
`bench/results/WO-M3-03.json`. After adding deterministic scheduler-capacity
and generation-drain accounting checks, the final 1,000-cycle benchmark took
0.012423 s with a 0.000041 s maximum join.

The focused snapshot/scheduler test passes Debug, ASan+UBSan, UBSan, and TSan.
The first isolated executor pass saw transient offscreen Metal and Python
process-policy failures. A later sequential clean-worktree run passed all 16
frozen commands: Debug/ASan/UBSan CTest 10/10, full suite 177/177, and focused
TSan 1/1. The original failure transcripts remain in
`artifacts/WO-M3-03/`; passing command records are under
`artifacts/runs/WO-M3-03/07d39ea9-fcb5-4dac-b5dc-e90eb80ca2ed/.`

## WO-M3-04 navigation projection workload

The deterministic C test projects 10,000 world-space scene coordinates over
100 navigation frames (1,000,000 projections) and records FNV-1a 64-bit
checksum `804ede4d47af0f15`. Navigation and projection functions contain no allocator
calls and retain no buffers; the count is a static API-path allocation claim,
not a process-wide allocation measurement. The workload is correctness and
determinism evidence, not rendering throughput. The original flat-canvas and
semantic baseline remains in `manifest.json`; an additive `manifest-rendered.json`
pins two screenshots with transformed scene geometry and the progress/cancel
overlay. Retina 2x viewport, drag, and cursor conversion is also covered. No wall-time,
rendering throughput, or process-wide allocation number is claimed.

The reviewed WO-M3-04 Release executable is 172,032 bytes after a clean `mac-release` build. This milestone has no RSS or latency result: the projection workload records deterministic operation count and checksum, not elapsed time.

## WO-BOOT-08 local performance gate

The machine-checkable commands passed; the measured gate outcome is a repeated relative regression warning/failure recorded below, even though absolute limits pass.

The gate runs five cold-order and ten warm-order samples on deterministic 10 MiB simple and 100 MiB map-like index workloads. The pinned M3-01 baseline hash and exact host/toolchain/workload identity are checked before comparing. Per-trial raw values, median, nearest-rank p95, maximum owner step, peak child RSS, sample standard deviation, and coefficient of variation are retained in `bench/results/WO-BOOT-08.json`. A metric above 10% CV is reported as noisy and does not advance its repeated-violation streak.

Two comparable runs on Mac Studio Mac17,14 / Apple M5 Max / macOS 27.0 / Apple Clang 21 showed 100 MiB first-preview p95 of 277.57 ms and 274.66 ms against 269.12 ms last-green p95; both observations remain within the 10% relative threshold. Peak child RSS was 137,904,128 and 139,526,144 bytes against 126,959,616 bytes; the repeated >5% change trips the two-run gate. Both absolute targets still pass: 100 MiB first-preview p95 remains below 2 s and peak RSS remains below 500 MiB. The first maximum-step sample had CV above 10% and was marked noisy; the next was healthy. The 10 MiB p95 passed both runs. Only peak RSS repeated its relative breach; its absolute target remains below 500 MiB. The first-preview p95 and maximum owner step remain below their absolute targets. This is a regression signal for review, not evidence of a public speed claim.

The Release `svgworkbench` executable is 172,032 bytes; the `svg_index_bench` harness is 107,136 bytes. `otool -L` records 8 linked dylib/framework entries. Binary/package stretch targets remain report-only; no app bundle exists. Cached pan/zoom event-to-present p95 is unavailable because current UI navigation tests do not time presentation. Full raw runs and gate history are in `artifacts/WO-BOOT-08/` and `bench/results/WO-BOOT-08.json`.

## WO-M4-01 bounded geometry and corpus workload

Debug and Release sizes on arm64 macOS 27.0, Apple M5 Max, Apple Clang 21.0.0
(`clang-2100.3.34.2`) are: `svgworkbench` 265,312 / 172,464 bytes,
`svg_index_bench` 107,552 bytes in Release, `shape_render_cli` 149,040 /
87,904 bytes, and `libvexel_shape.a` 106,888 / 37,960 bytes.

`./dev bench-corpus` now reruns the established fresh-process progressive index
workload, verifies every external manifest hash, and writes M4-specific reports
without changing the frozen M3 report. The 58-file report records 2 previewable,
56 unsupported, no limits, and no crashes. Both previewable files are Natural
Earth T1 maps, so geometry support alone has not increased whole-file coverage;
common class CSS, stroke and local references remain future work. For those two
successful T1 rows, the Inkscape/Vexel first-preview geometric mean is 91.875x
and the RSS ratio is 33.832x. T2–T4 ratios are unavailable because Vexel has no
successful outcomes there. These are directional open/index task comparisons
against the supplied Inkscape CLI baseline, not full-render parity.

The 100 MiB preview gate fails because 0/12 eligible T3 files produced a
preview. The maximum measured main-thread index step was 0.215 ms, under 100 ms;
all 12 eligible files passed the existing per-file RSS allowance. The two T1
first-preview measurements were 4.218 and 3.986 ms; full-index measurements
were 0.851 and 0.879 ms. Host, binary hash, per-file aggregate results and
ratios are in `reports/M4/head-to-head.json` and `.md`; raw per-trial data and
the external manifest pointer stay in ignored `.cache/M4/`.

The filled-shape differential fixtures passed with librsvg
`rsvg-convert 2.63.2 (Cairo 1.18.6, Pango 1.58.2, HarfBuzz 14.6.0)`. Fixture
source/output hashes and per-case mismatch counts are in
`artifacts/WO-M4-01/reference-comparison.json`. The final seeded ASan+UBSan
shape campaign result is retained at `artifacts/WO-M4-01/fuzz-report.json`;
it completed 1,000,000 cases in 24.984 seconds with no sanitizer reports,
crashes, or invariant failures. The runner sandbox blocked offscreen Metal
access during its first verification attempts; the complete Debug, ASan,
UBSan, and Python suites pass when run directly in the isolated worktree. The sandbox failure transcript and direct host results are retained in the M4 logs.
