# Rust corpus measurement

These are verdict counts and timings only, produced by one run on one host. They are not fidelity, conformance or performance claims. The corpus bytes are not in this repository; only manifest metadata, verdicts and measurements are recorded.

- tool: vexel-corpus, base HEAD `842d6ce0373b6da3adf9799a6783da01d326efb5` (working tree may be dirty)
- manifest: `bench/corpus/manifest.json` sha256 `a05dcf0da95bd7c53ce6faec6704f7b739d6cbd9cf829dfff74b50dfb8359bbd`
- render size: 128x128, skip over 33554432 bytes, timeout 60 s, RSS limit 2048 MiB
- host: ProductName: macOS ProductVersion: 27.0.1 BuildVersion: 26A434; arm64

## Verdict histogram

| verdict | files |
|---|---|
| ok | 17 |
| unsupported | 7 |
| limit | 0 |
| xml | 0 |
| timeout | 0 |
| rss_exceeded | 0 |
| skipped | 0 |
| crash | 0 |
| guard_failed | 0 |

Total: 24 files, summed wall time 342 ms.

## Slowest five (wall ms)

- `stress-long-path` 60
- `path-line-curves` 25
- `metadata-namespaces` 12
- `metadata-rdf-like` 12
- `namespaces-custom-attributes` 12

## Largest peak RSS five (bytes)

- `style-gradient` 10043392
- `stress-many-shapes` 9289728
- `stress-nested-groups` 9125888
- `clip-path` 9060352
- `use-local-reference` 8978432

## Per file

| id | tier | kind | bytes | verdict | wall ms | peak RSS | detail |
|---|---|---|---|---|---|---|---|
| clip-path | - | project-authored | 353 | unsupported | 11 | 9060352 | unsupported in V1: clip path |
| image-embedded-data | - | project-authored | 382 | unsupported | 11 | 8306688 | unsupported in V1: image (<image> at byte 208) |
| metadata-namespaces | - | project-authored | 400 | ok | 12 | 8552448 | 128x128 frame, 0 disclosure(s) |
| metadata-rdf-like | - | project-authored | 421 | ok | 12 | 8552448 | 128x128 frame, 0 disclosure(s) |
| namespaces-custom-attributes | - | project-authored | 359 | ok | 12 | 8568832 | 128x128 frame, 0 disclosure(s) |
| path-arcs | - | project-authored | 317 | ok | 12 | 8568832 | 128x128 frame, 0 disclosure(s) |
| path-line-curves | - | project-authored | 310 | ok | 25 | 8585216 | 128x128 frame, 0 disclosure(s) |
| path-relative-commands | - | project-authored | 265 | ok | 10 | 8585216 | 128x128 frame, 0 disclosure(s) |
| shapes-basic | - | project-authored | 324 | ok | 12 | 8585216 | 128x128 frame, 0 disclosure(s) |
| shapes-lines-polygons | - | project-authored | 330 | ok | 12 | 8552448 | 128x128 frame, 0 disclosure(s) |
| shapes-rounded | - | project-authored | 283 | ok | 12 | 8552448 | 128x128 frame, 0 disclosure(s) |
| stress-long-path | - | project-authored | 1513 | ok | 60 | 8716288 | 128x128 frame, 0 disclosure(s) |
| stress-many-shapes | - | project-authored | 9816 | ok | 12 | 9289728 | 128x128 frame, 0 disclosure(s) |
| stress-nested-groups | - | project-authored | 326 | ok | 12 | 9125888 | 128x128 frame, 0 disclosure(s) |
| stress-text-collection | - | project-authored | 2004 | unsupported | 12 | 8437760 | unsupported in V1: text (<text> at byte 219) |
| style-css-class | - | project-authored | 335 | ok | 12 | 8552448 | 128x128 frame, 0 disclosure(s) |
| style-gradient | - | project-authored | 393 | ok | 10 | 10043392 | 128x128 frame, 1 disclosure(s) |
| style-opacity-transform | - | project-authored | 342 | unsupported | 12 | 8437760 | unsupported in V1: opacity group |
| style-presentation-attributes | - | project-authored | 313 | ok | 12 | 8552448 | 128x128 frame, 0 disclosure(s) |
| text-basic | - | project-authored | 257 | unsupported | 12 | 8339456 | unsupported in V1: text (<text> at byte 199) |
| text-positioning | - | project-authored | 334 | unsupported | 11 | 8323072 | unsupported in V1: text (<text> at byte 205) |
| text-spans | - | project-authored | 311 | unsupported | 12 | 8306688 | unsupported in V1: text (<text> at byte 199) |
| use-local-reference | - | project-authored | 359 | ok | 12 | 8978432 | 128x128 frame, 0 disclosure(s) |
| viewbox-preserve | - | project-authored | 304 | ok | 12 | 8568832 | 128x128 frame, 0 disclosure(s) |
