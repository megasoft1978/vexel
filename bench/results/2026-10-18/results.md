# Vexel vs Inkscape benchmark (2026-10-18)

Host: arm64, macOS 27.0.1. Runs per file: 3 (median).
Vexel: `vexel-corpus --one <file> --size 128x128` (compose only).
Inkscape: Inkscape 1.4.4 (dcaf3e7, 2026-05-05) `--export-type=png --export-filename=/dev/null` (document size).
Different workloads (compose vs PNG export): not a like-for-like speed claim.

| File | Bytes | Vexel median ms | Vexel median peak RSS MiB | Inkscape median ms | Inkscape median peak RSS MiB | Notes |
|---|---:|---:|---:|---:|---:|---|
| files/real/world-map-blank.svg | 241682 | 40.8 | 19.1 | 275.3 | 139.9 |  |
| files/generated/cnc_t1.svg | 990772 | 166.1 | 93.8 | 642.9 | 429.8 |  |
| files/real_inkscape/greater-coat-of-arms-of-blas-de-lezo.svg | 4898525 | - | - | 482.9 | 235.2 | Vexel: compose verdict: VEXEL-CORPUS-RESULT	unsupported	unsupported in V1: unsupported element or attribute (<path> at byte 744649) |
| files/real_inkscape/kymijoki-drainage-basin-map-svg.svg | 17449181 | - | - | 1812.6 | 926.2 | Vexel: compose verdict: VEXEL-CORPUS-RESULT	unsupported	unsupported in V1: unsupported element or attribute (<svg> at byte 152) |
| files/real/geographic-map-of-norway-sweden-finland-and-denmark-svg.svg | 20101913 | - | - | 2332.2 | 1080.8 | Vexel: compose verdict: VEXEL-CORPUS-RESULT	xml	malformed SVG: viewport at byte 70 |
