# Performance baseline

These measurements establish a local starting point. The `--version` command
only measures process startup and is not an SVG rendering benchmark.

| Work order | Build | Binary bytes | `--version` median | Max RSS | Machine |
|---|---|---:|---:|---:|---|
| BOOT-01 | Release | 33,568 | 1.910 ms (31 runs) | 1,884,160 bytes (one run) | Apple Silicon, macOS 27.0, Apple Clang 21.0.0 |

Raw latency metadata is in `bench/results/WO-BOOT-01.json`. RSS came from
`/usr/bin/time -l`. No SVG throughput or renderer performance is measured yet.

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
