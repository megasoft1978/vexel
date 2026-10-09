# Performance baseline

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
| WO-M3-02 | Release | benchmark binary 35,376 bytes | 100,000 records / 1,000 viewport queries: index build 8.071 ms; query p50 0.000292 ms, p95 0.002000 ms, max 0.006125 ms | cache pressure accounted peak 5,200 / 8,192 bytes over 10,000 inserts, including the allocated 100-entry metadata array | Mac Studio Mac17,14, M5 Max, macOS 27.0, Apple Clang 21.0.0 |

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
viewport queries. The packed index build took 8.071 ms; query p50/p95/max were
0.000292/0.002000/0.006125 ms. The named Mac p95 target of 5 ms was met. The
benchmark records every query's result count, compiler, build type, binary
size, and host model in `bench/results/WO-M3-02.json`.

The randomized differential test compares 2,000 query result arrays byte for
byte with the naive source-order oracle. Tile-cache pressure performs 10,000
insertions; maximum accounted storage was 5,200 bytes under an 8,192 byte cap,
including all 100 preallocated entry slots. The
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
