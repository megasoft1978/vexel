# Performance baseline

These measurements establish a local starting point. The `--version` command
only measures process startup and is not an SVG rendering benchmark.

| Work order | Build | Binary bytes | Latency (workload) | Max RSS | Machine |
|---|---|---:|---|---:|---|
| BOOT-01 | Release | 33,568 | `--version`: 1.910 ms median (31 runs) | 1,884,160 bytes (one run) | Apple Silicon, macOS 27.0, Apple Clang 21.0.0 |
| WO-M1-03 | Release | 94,752 | placeholder replay: 100.976 ms; Metal replay: 179.164 ms | not measured | Apple Silicon, macOS 27.0, Apple Clang 21.0.0 |

Raw latency metadata is in `bench/results/WO-BOOT-01.json`. RSS came from
`/usr/bin/time -l`. No SVG throughput or renderer performance is measured yet.

## WO-M1-04 task queue lifecycle

The macOS queue bounds capacity to 1–64 tasks and worker count to 1–4. Context
memory is borrowed and must remain alive through callback completion or queued
cancellation. Cancellation is cooperative through an atomic token. Generation
advances suppress results from older work. Shutdown requests cancellation,
wakes and joins workers, then destroy releases storage. The owner quiesces all
callers before destroy; an uncooperative callback can delay join. Lifecycle
timings are correctness bounds only and are not performance claims.

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
