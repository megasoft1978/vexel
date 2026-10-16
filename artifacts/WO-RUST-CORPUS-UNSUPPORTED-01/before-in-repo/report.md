# Rust corpus measurement

These are verdict counts and timings only, produced by one run on one host. They are not fidelity, conformance or performance claims. The corpus bytes are not in this repository; only manifest metadata, verdicts and measurements are recorded.

- tool: vexel-corpus, base HEAD `393ae68f54f1bcc15d4a25ab0b754bd4fcc4a496` (working tree may be dirty)
- manifest: `bench/corpus/manifest.json` sha256 `a05dcf0da95bd7c53ce6faec6704f7b739d6cbd9cf829dfff74b50dfb8359bbd`
- render size: 128x128, skip over 33554432 bytes, timeout 60 s, RSS limit 2048 MiB
- host: ProductName: macOS ProductVersion: 27.0.1 BuildVersion: 26A434; arm64

## Verdict histogram

| verdict | files |
|---|---|
| ok | 23 |
| unsupported | 1 |
| limit | 0 |
| xml | 0 |
| timeout | 0 |
| rss_exceeded | 0 |
| skipped | 0 |
| crash | 0 |
| guard_failed | 0 |

Total: 24 files, summed wall time 440 ms.

## Slowest five (wall ms)

- `stress-long-path` 111
- `stress-text-collection` 25
- `clip-path` 15
- `image-embedded-data` 15
- `metadata-rdf-like` 15

## Largest peak RSS five (bytes)

- `stress-text-collection` 18366464
- `text-spans` 15269888
- `text-positioning` 15171584
- `text-basic` 14942208
- `style-gradient` 10223616

## Per file

| id | tier | kind | bytes | verdict | wall ms | peak RSS | detail |
|---|---|---|---|---|---|---|---|
| clip-path | - | project-authored | 353 | ok | 15 | 9322496 | 128x128 frame, 0 disclosure(s) |
| image-embedded-data | - | project-authored | 382 | unsupported | 15 | 8404992 | unsupported in V1: image (<image> at byte 207) |
| metadata-namespaces | - | project-authored | 400 | ok | 11 | 8781824 | 128x128 frame, 0 disclosure(s) |
| metadata-rdf-like | - | project-authored | 421 | ok | 15 | 8781824 | 128x128 frame, 0 disclosure(s) |
| namespaces-custom-attributes | - | project-authored | 359 | ok | 11 | 8781824 | 128x128 frame, 0 disclosure(s) |
| path-arcs | - | project-authored | 317 | ok | 15 | 8798208 | 128x128 frame, 0 disclosure(s) |
| path-line-curves | - | project-authored | 310 | ok | 15 | 8798208 | 128x128 frame, 0 disclosure(s) |
| path-relative-commands | - | project-authored | 265 | ok | 15 | 8765440 | 128x128 frame, 0 disclosure(s) |
| shapes-basic | - | project-authored | 324 | ok | 14 | 8798208 | 128x128 frame, 0 disclosure(s) |
| shapes-lines-polygons | - | project-authored | 330 | ok | 15 | 8765440 | 128x128 frame, 0 disclosure(s) |
| shapes-rounded | - | project-authored | 283 | ok | 12 | 8814592 | 128x128 frame, 0 disclosure(s) |
| stress-long-path | - | project-authored | 1513 | ok | 111 | 9011200 | 128x128 frame, 0 disclosure(s) |
| stress-many-shapes | - | project-authored | 9816 | ok | 15 | 9633792 | 128x128 frame, 0 disclosure(s) |
| stress-nested-groups | - | project-authored | 326 | ok | 10 | 9240576 | 128x128 frame, 0 disclosure(s) |
| stress-text-collection | - | project-authored | 2004 | ok | 25 | 18366464 | 128x128 frame, 0 disclosure(s) |
| style-css-class | - | project-authored | 335 | ok | 10 | 8798208 | 128x128 frame, 0 disclosure(s) |
| style-gradient | - | project-authored | 393 | ok | 11 | 10223616 | 128x128 frame, 1 disclosure(s) |
| style-opacity-transform | - | project-authored | 342 | ok | 15 | 8880128 | 128x128 frame, 0 disclosure(s) |
| style-presentation-attributes | - | project-authored | 313 | ok | 15 | 8798208 | 128x128 frame, 0 disclosure(s) |
| text-basic | - | project-authored | 257 | ok | 15 | 14942208 | 128x128 frame, 0 disclosure(s) |
| text-positioning | - | project-authored | 334 | ok | 15 | 15171584 | 128x128 frame, 0 disclosure(s) |
| text-spans | - | project-authored | 311 | ok | 15 | 15269888 | 128x128 frame, 0 disclosure(s) |
| use-local-reference | - | project-authored | 359 | ok | 15 | 9224192 | 128x128 frame, 0 disclosure(s) |
| viewbox-preserve | - | project-authored | 304 | ok | 15 | 8830976 | 128x128 frame, 0 disclosure(s) |
