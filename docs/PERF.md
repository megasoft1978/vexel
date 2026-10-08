# Performance baseline

These measurements establish a local starting point. The `--version` command
only measures process startup and is not an SVG rendering benchmark.

| Work order | Build | Binary bytes | `--version` median | Max RSS | Machine |
|---|---|---:|---:|---:|---|
| BOOT-01 | Release | 33,568 | 1.910 ms (31 runs) | 1,884,160 bytes (one run) | Apple Silicon, macOS 27.0, Apple Clang 21.0.0 |

Raw latency metadata is in `bench/results/WO-BOOT-01.json`. RSS came from
`/usr/bin/time -l`. No SVG throughput or renderer performance is measured yet.
