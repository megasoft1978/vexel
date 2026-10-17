# Rust corpus measurement

These are verdict counts and timings only, produced by one run on one host. They are not fidelity, conformance or performance claims. The corpus bytes are not in this repository; only manifest metadata, verdicts and measurements are recorded.

- tool: vexel-corpus, base HEAD `2b0aa7abc8a1d93018a00eefcc253ca9a775e0ab` (working tree may be dirty)
- manifest: `bench/corpus/manifest.json` sha256 `a05dcf0da95bd7c53ce6faec6704f7b739d6cbd9cf829dfff74b50dfb8359bbd`
- render size: 128x128, skip over 33554432 bytes, timeout 60 s, RSS limit 2048 MiB
- host: ProductName: macOS ProductVersion: 27.0.1 BuildVersion: 26A434; arm64

## Verdict histogram

| verdict | files |
|---|---|
| ok | 24 |
| unsupported | 0 |
| limit_decode | 0 |
| limit_render | 0 |
| xml | 0 |
| timeout | 0 |
| rss_exceeded | 0 |
| skipped | 0 |
| crash | 0 |
| guard_failed | 0 |

Total: 24 files, summed wall time 447 ms.

## Slowest five (wall ms)

- `stress-long-path` 113
- `stress-text-collection` 30
- `text-basic` 26
- `path-arcs` 15
- `path-relative-commands` 15

## Largest peak RSS five (bytes)

- `stress-text-collection` 18415616
- `text-spans` 15286272
- `text-positioning` 15204352
- `text-basic` 14991360
- `style-gradient` 11976704

## Per file

| id | tier | kind | bytes | verdict | wall ms | peak RSS | detail |
|---|---|---|---|---|---|---|---|
| clip-path | - | project-authored | 353 | ok | 12 | 10895360 | 128x128 frame, 0 disclosure(s) |
| image-embedded-data | - | project-authored | 382 | ok | 12 | 8847360 | 128x128 frame, 0 disclosure(s) |
| metadata-namespaces | - | project-authored | 400 | ok | 12 | 8814592 | 128x128 frame, 0 disclosure(s) |
| metadata-rdf-like | - | project-authored | 421 | ok | 12 | 8814592 | 128x128 frame, 0 disclosure(s) |
| namespaces-custom-attributes | - | project-authored | 359 | ok | 11 | 8830976 | 128x128 frame, 0 disclosure(s) |
| path-arcs | - | project-authored | 317 | ok | 15 | 8814592 | 128x128 frame, 0 disclosure(s) |
| path-line-curves | - | project-authored | 310 | ok | 13 | 8798208 | 128x128 frame, 0 disclosure(s) |
| path-relative-commands | - | project-authored | 265 | ok | 15 | 8798208 | 128x128 frame, 0 disclosure(s) |
| shapes-basic | - | project-authored | 324 | ok | 12 | 8830976 | 128x128 frame, 0 disclosure(s) |
| shapes-lines-polygons | - | project-authored | 330 | ok | 15 | 8814592 | 128x128 frame, 0 disclosure(s) |
| shapes-rounded | - | project-authored | 283 | ok | 15 | 8830976 | 128x128 frame, 0 disclosure(s) |
| stress-long-path | - | project-authored | 1513 | ok | 113 | 8978432 | 128x128 frame, 0 disclosure(s) |
| stress-many-shapes | - | project-authored | 9816 | ok | 11 | 9682944 | 128x128 frame, 0 disclosure(s) |
| stress-nested-groups | - | project-authored | 326 | ok | 15 | 10960896 | 128x128 frame, 0 disclosure(s) |
| stress-text-collection | - | project-authored | 2004 | ok | 30 | 18415616 | 128x128 frame, 0 disclosure(s) |
| style-css-class | - | project-authored | 335 | ok | 15 | 8814592 | 128x128 frame, 0 disclosure(s) |
| style-gradient | - | project-authored | 393 | ok | 12 | 11976704 | 128x128 frame, 1 disclosure(s) |
| style-opacity-transform | - | project-authored | 342 | ok | 15 | 8880128 | 128x128 frame, 0 disclosure(s) |
| style-presentation-attributes | - | project-authored | 313 | ok | 11 | 8798208 | 128x128 frame, 0 disclosure(s) |
| text-basic | - | project-authored | 257 | ok | 26 | 14991360 | 128x128 frame, 0 disclosure(s) |
| text-positioning | - | project-authored | 334 | ok | 15 | 15204352 | 128x128 frame, 0 disclosure(s) |
| text-spans | - | project-authored | 311 | ok | 12 | 15286272 | 128x128 frame, 0 disclosure(s) |
| use-local-reference | - | project-authored | 359 | ok | 13 | 10813440 | 128x128 frame, 0 disclosure(s) |
| viewbox-preserve | - | project-authored | 304 | ok | 15 | 8830976 | 128x128 frame, 0 disclosure(s) |
