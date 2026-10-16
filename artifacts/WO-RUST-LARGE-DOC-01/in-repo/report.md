# Rust corpus measurement

These are verdict counts and timings only, produced by one run on one host. They are not fidelity, conformance or performance claims. The corpus bytes are not in this repository; only manifest metadata, verdicts and measurements are recorded.

- tool: vexel-corpus, base HEAD `e0f2f6a03aa9c3c12adda379a11b4a82aab16c66` (working tree may be dirty)
- manifest: `bench/corpus/manifest.json` sha256 `a05dcf0da95bd7c53ce6faec6704f7b739d6cbd9cf829dfff74b50dfb8359bbd`
- render size: 128x128, skip over 33554432 bytes, timeout 60 s, RSS limit 2048 MiB
- host: ProductName: macOS ProductVersion: 27.0.1 BuildVersion: 26A434; arm64

## Verdict histogram

| verdict | files |
|---|---|
| ok | 17 |
| unsupported | 7 |
| limit_decode | 0 |
| limit_render | 0 |
| xml | 0 |
| timeout | 0 |
| rss_exceeded | 0 |
| skipped | 0 |
| crash | 0 |
| guard_failed | 0 |

Total: 24 files, summed wall time 395 ms.

## Slowest five (wall ms)

- `stress-long-path` 57
- `path-line-curves` 27
- `stress-many-shapes` 17
- `clip-path` 15
- `image-embedded-data` 15

## Largest peak RSS five (bytes)

- `style-gradient` 10076160
- `stress-many-shapes` 9404416
- `clip-path` 9093120
- `stress-nested-groups` 9043968
- `use-local-reference` 8994816

## Per file

| id | tier | kind | bytes | verdict | wall ms | peak RSS | detail |
|---|---|---|---|---|---|---|---|
| clip-path | - | project-authored | 353 | unsupported | 15 | 9093120 | unsupported in V1: clip path |
| image-embedded-data | - | project-authored | 382 | unsupported | 15 | 8355840 | unsupported in V1: image (<image> at byte 208) |
| metadata-namespaces | - | project-authored | 400 | ok | 15 | 8585216 | 128x128 frame, 0 disclosure(s) |
| metadata-rdf-like | - | project-authored | 421 | ok | 15 | 8585216 | 128x128 frame, 0 disclosure(s) |
| namespaces-custom-attributes | - | project-authored | 359 | ok | 15 | 8585216 | 128x128 frame, 0 disclosure(s) |
| path-arcs | - | project-authored | 317 | ok | 14 | 8585216 | 128x128 frame, 0 disclosure(s) |
| path-line-curves | - | project-authored | 310 | ok | 27 | 8585216 | 128x128 frame, 0 disclosure(s) |
| path-relative-commands | - | project-authored | 265 | ok | 12 | 8585216 | 128x128 frame, 0 disclosure(s) |
| shapes-basic | - | project-authored | 324 | ok | 15 | 8585216 | 128x128 frame, 0 disclosure(s) |
| shapes-lines-polygons | - | project-authored | 330 | ok | 13 | 8568832 | 128x128 frame, 0 disclosure(s) |
| shapes-rounded | - | project-authored | 283 | ok | 15 | 8585216 | 128x128 frame, 0 disclosure(s) |
| stress-long-path | - | project-authored | 1513 | ok | 57 | 8749056 | 128x128 frame, 0 disclosure(s) |
| stress-many-shapes | - | project-authored | 9816 | ok | 17 | 9404416 | 128x128 frame, 0 disclosure(s) |
| stress-nested-groups | - | project-authored | 326 | ok | 15 | 9043968 | 128x128 frame, 0 disclosure(s) |
| stress-text-collection | - | project-authored | 2004 | unsupported | 15 | 8486912 | unsupported in V1: text (<text> at byte 219) |
| style-css-class | - | project-authored | 335 | ok | 15 | 8585216 | 128x128 frame, 0 disclosure(s) |
| style-gradient | - | project-authored | 393 | ok | 14 | 10076160 | 128x128 frame, 1 disclosure(s) |
| style-opacity-transform | - | project-authored | 342 | unsupported | 11 | 8454144 | unsupported in V1: opacity group |
| style-presentation-attributes | - | project-authored | 313 | ok | 15 | 8585216 | 128x128 frame, 0 disclosure(s) |
| text-basic | - | project-authored | 257 | unsupported | 15 | 8355840 | unsupported in V1: text (<text> at byte 199) |
| text-positioning | - | project-authored | 334 | unsupported | 14 | 8355840 | unsupported in V1: text (<text> at byte 205) |
| text-spans | - | project-authored | 311 | unsupported | 10 | 8355840 | unsupported in V1: text (<text> at byte 199) |
| use-local-reference | - | project-authored | 359 | ok | 11 | 8994816 | 128x128 frame, 0 disclosure(s) |
| viewbox-preserve | - | project-authored | 304 | ok | 15 | 8601600 | 128x128 frame, 0 disclosure(s) |
