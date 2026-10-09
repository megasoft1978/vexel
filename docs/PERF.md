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

<!-- M4-01 generated report start -->

## M4 cumulative shape and corpus workload

Measured on arm64 macOS 27.0, Apple M5 Max, Apple clang version 21.0.0 (clang-2100.3.34.2).
Debug/Release `svgworkbench`: 341648 / 224176 bytes; Release `svg_index_bench`: 159344 bytes;
Debug/Release `shape_render_cli`: 225440 / 139728 bytes; Debug/Release `libvexel_shape.a`: 300128 / 105664 bytes.
Binary sizes are also present in the JSON report and verified by `./dev bench-corpus --check`.

The 58-file run produced 2 previewable, 56 unsupported, 0 limit, and 0 crash outcomes.
The 2 comparable successful T1 rows have an Inkscape/Vexel first-preview geometric mean of 39.987x and RSS ratio of 33.175x. T2–T4 ratios are unavailable when Vexel has no successful outcomes there.
These are directional open/index comparisons against the supplied Inkscape CLI baseline, not full-render parity.
The 100 MiB first-preview gate is fail_no_preview (0/12 eligible files); maximum main-thread step is 0.291 ms (pass); 100 MiB RSS gate is pass (12/12 within the per-file allowance).
Successful T1 measurements: natural-earth-world-map-blank: first preview 9.376 ms, full index 0.886 ms; natural-earth-world-map-countries: first preview 9.467 ms, full index 0.924 ms
Per-file results and aggregate timings are in `reports/M4/head-to-head.json` and `.md`; raw trials and the external manifest pointer stay in ignored `.cache/M4/`.
<!-- M4-01 generated report end -->

## WO-M4-06 candidate status

Five fixed 512×512 Release raster workloads were measured with 21 fresh-process
trials each. The basic solid fill measured 7.624 ms median / 8.016 ms p95 at
34.4 million pixels/s; local `<use>` measured 7.445 / 7.907 ms; linear gradient
8.227 / 8.538 ms; radial gradient 8.338 / 8.642 ms; and clipping 8.299 / 8.481
ms. Peak process RSS ranged from 3,112,960 to 3,194,880 bytes. Every workload
had a stable output SHA-256. The frozen-base CLI measured the basic solid fill
at 10.102 ms median / 10.425 ms p95; it does not expose RSS metrics. Feature
workloads were unsupported at the frozen base and have no direct comparison.
These process-level samples are local evidence, not public speed claims. Raw
trials and output hashes are in `artifacts/WO-M4-06/paint-benchmark.json` and
`bench/results/WO-M4-06.json`.

WO-M4-04 corpus follow-up retains 58 unchanged source hashes and all 116 trials.
Only `natural-earth-world-map-blank` and `natural-earth-world-map-countries`
produce a preview (T1: 2/12); T2 has 0/17, T3 0/26, and T4 0/3. Where
complete-index samples are available, median / p95 and median peak RSS by tier
are: T1 4.549 / 6.138 ms and 8,945,664 B (11 time samples, 12 RSS samples);
T2 46.967 / 61.978 ms and 29,999,104 B (13 / 17); T3 126.351 / 489.044 ms
and 102,211,584 B (15 / 26); T4 2,377.215 / 2,377.215 ms and 829,718,528 B
(1 / 3). Missing timing rows are unavailable because indexing stopped before a
complete-index result. The 100 MiB preview wedge remains `fail_no_preview`
(0/12 eligible T3 files); maximum owner step is 0.199 ms and the per-file RSS
gate passes 12/12. These are open/index results, not full-render parity.

The WO-M4-04 Release raster benchmark ran 21 trials per fixed workload against
base `1e9c7b81f1bda074031af4fd165240e8d5eeba09`; source revision
`b60ef4189f0a20fd5a61cad9e7169a6f49db21d7` was clean during measurement.
Existing simple, curved, and multi-shape medians changed +0.73%, +3.86%, and
+2.24%, respectively. New stroke workloads measured line 2.501 ms median /
2.756 ms p95, curved circle 10.735 / 10.897 ms, and dashed polyline 78.994 /
79.926 ms. The frozen base
rejects direct stroke attributes, so comparative base timings for those three
are unavailable. Candidate output hashes, throughput, RSS (2,031,616–2,048,000
B), toolchain, and all trial values are in
`artifacts/WO-M4-04/raster-benchmark.json`; host model is unavailable from the
benchmark harness. Caching inverse-mapped edge endpoints once per render
keeps inverse transforms out of the per-sample, per-edge loop; the clean-source
measurements above include interior and closed-outline dash-join behavior.


The filled-shape differential fixtures passed with librsvg
`rsvg-convert 2.63.2 (Cairo 1.18.6, Pango 1.58.2, HarfBuzz 14.6.0)`. Fixture
source/output hashes and per-case mismatch counts are in
`artifacts/WO-M4-01/reference-comparison.json`. The final seeded ASan+UBSan
shape campaign result is retained at `artifacts/WO-M4-01/fuzz-report.json`;
it completed 1,000,000 cases in 24.382 seconds with no sanitizer reports,
crashes, or invariant failures. The runner sandbox blocked offscreen Metal
access during its first verification attempts; the complete Debug, ASan,
UBSan, and Python suites pass when run directly in the isolated worktree. The sandbox failure transcript and direct host results are retained in the M4 logs.

## WO-M4-03 deterministic shape raster

The Release CLI benchmark compares the frozen `6cd61dd6570ab556af7d6a1feb8899d3f849f8af`
archive with this 4x4 coverage implementation on the same arm64 macOS 27.0
host and Apple clang 21.0.0; the specific hardware model is unavailable in the
benchmark harness. Each fixed 128x128 workload uses two warmups and
21 measured trials; results include per-trial latencies, median/p95, throughput,
peak RSS, and output hashes in `artifacts/WO-M4-03/raster-benchmark.json`. Each
trial alternates base/candidate order to reduce host clock/load drift. Raster
source revision is `26d6b19d38b23be633c0d85642cd637f4ebdba1e`; the harness records
that source revision so evidence-only commits do not change the identity.

| Workload | Base median / p95 (ms) | Candidate median / p95 (ms) | Median change | Candidate pixels/s |
|---|---:|---:|---:|---:|
| Simple rectangle | 2.065 / 2.167 | 2.361 / 2.496 | +14.34% | 6,937,963 |
| Curved fill | 2.713 / 3.023 | 4.426 / 4.776 | +63.16% | 3,701,867 |
| 16 fractional rectangles | 3.072 / 3.431 | 5.453 / 5.923 | +77.50% | 3,004,493 |

The first implementation made the simple workload 75.30% slower. An interior-pixel
fast path now uses one center sample when no edge bounds cross the pixel, while
boundary pixels retain all 16 fixed samples. The latest campaign measures simple
at 14.34% slower than base; curved and multi-shape are 63.16% and 77.50% slower
because their more numerous edges make boundary classification more costly.
Independent verifier and direct reruns varied within +12.18–14.64% simple,
+57.48–66.24% curved, and +77.50–89.65% multi-shape; each record uses 21 paired
trials and retains per-trial samples. This range reflects short CLI-level measurements,
including process startup, and is recorded instead of implying one exact timing.
The sample grid and work cap remain unchanged. Peak process RSS is 2,048,000 bytes
for base and 2,031,616 bytes for candidate. This is a measured quality/performance
tradeoff, not a speedup claim. The renderer and harness use 128x128 tiles; CLI
timing includes SVG decode, process startup, and rasterization.

The 17 librsvg 2.63.2 fixtures pass with zero interior differences, maximum
alpha delta at most 56, jointly covered raw straight RGB difference at most 1,
and no more than 58 differing edge pixels per fixture. Fixture source, Vexel RGBA, and librsvg
PNG hashes are retained in `artifacts/WO-M4-03/reference-comparison.json`.
The seeded ASan+UBSan raster campaign result is in
`artifacts/WO-M4-03/fuzz-report.json`; it separately counts generated and
rasterized fractional, evenodd, and nonzero cases.

## WO-M4-05 CSS paint cascade

The CSS renderer benchmark ran 21 fresh-process trials for each fixed 128x128,
64-rectangle workload on arm64 macOS 27.0 with Apple clang 21.0.0. The final
stylesheet/class run measured 27.890 ms median and 29.662 ms p95, about 587,442
pixels/s; inline styles measured 27.483 ms median and 29.013 ms p95, about
596,144 pixels/s. Both workloads produced the same stable RGBA SHA-256
`827e21fe25c8fd57db0185583ded7a6b9d988f54846b914cf12c536ba393fd60`; peak
process RSS was 2,113,536 / 2,146,304 bytes (stylesheet / inline). The
benchmark harness could not query the host hardware model. The frozen pre-WO renderer has no equivalent CSS
workload, so no CSS baseline comparison is available. These are local
process-level renderer measurements, not public performance claims. Raw trials
and the existing comparable non-CSS raster workloads are in
`artifacts/WO-M4-05/css-benchmark.json` and `bench/results/WO-M4-05.json`.

The real-corpus run retained 58/58 input hashes and outcomes, with two
previewable T1 files and no new previews. T1/T2/T3/T4 successful comparison
counts are 2/12, 0/17, 0/26, and 0/3. T1 geomean Inkscape/Vexel ratios are
42.437x first-preview time and 33.471x peak RSS; higher-tier comparisons are
unavailable because no files preview. This remains open/index evidence, not
full-render parity.
