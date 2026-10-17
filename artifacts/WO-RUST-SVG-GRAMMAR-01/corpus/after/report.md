# Rust corpus measurement

These are verdict counts and timings only, produced by one run on one host. They are not fidelity, conformance or performance claims. The corpus bytes are not in this repository; only manifest metadata, verdicts and measurements are recorded.

- tool: vexel-corpus, base HEAD `ba2442d65be4fbb3089a9b1be25c981285f6af97` (working tree may be dirty)
- manifest: `~/Desktop/Freelance/vexel-corpus-real/manifest.json` sha256 `9fbf9a2f384d70a41da72e99ac259d2c5d137ffd3c8a01ce4be4741c1c29bc30`
- render size: 128x128, skip over 33554432 bytes, timeout 60 s, RSS limit 2048 MiB
- host: ProductName: macOS ProductVersion: 27.0.1 BuildVersion: 26A434; arm64

## Verdict histogram

| verdict | files |
|---|---|
| ok | 1 |
| unsupported | 0 |
| limit_decode | 2 |
| limit_render | 0 |
| xml | 4 |
| timeout | 0 |
| rss_exceeded | 0 |
| skipped | 0 |
| crash | 0 |
| guard_failed | 0 |

Total: 7 files, summed wall time 1722 ms.

## Slowest five (wall ms)

- `commons-1-42-polytope-7-cube-svg` 1227
- `commons-germany-districts-municipalities-location-map-2013-svg` 135
- `commons-aegean-sea-anatolia-and-armenian-highlands-regions-large-topographic-basemap-svg` 121
- `commons-france-osm-mapnik-svg` 95
- `generated-map_t1` 85

## Largest peak RSS five (bytes)

- `commons-1-42-polytope-7-cube-svg` 213942272
- `commons-aegean-sea-anatolia-and-armenian-highlands-regions-large-topographic-basemap-svg` 72892416
- `commons-germany-districts-municipalities-location-map-2013-svg` 70057984
- `commons-france-osm-mapnik-svg` 55771136
- `generated-map_t1` 47267840

## Per file

| id | tier | kind | bytes | verdict | wall ms | peak RSS | detail |
|---|---|---|---|---|---|---|---|
| generated-map_t1 | T1 | generated | 1310972 | ok | 85 | 47267840 | 128x128 frame, 0 disclosure(s) |
| generated-nested_t1 | T1 | generated | 978645 | limit_decode | 12 | 10141696 | document exceeds a decode safety limit |
| commons-1-42-polytope-7-cube-svg | T3 | real | 29836045 | limit_decode | 1227 | 213942272 | document exceeds a decode safety limit |
| commons-aegean-sea-anatolia-and-armenian-highlands-regions-large-topographic-basemap-svg | T3 | real | 22666147 | xml | 121 | 72892416 | malformed SVG: viewport at byte 70 |
| commons-columbiarivermap-svg | T2 | real | 16687292 | xml | 47 | 41304064 | malformed SVG: XML at byte 917 |
| commons-france-osm-mapnik-svg | T2 | real | 19491319 | xml | 95 | 55771136 | malformed SVG: path data at byte 28855 |
| commons-germany-districts-municipalities-location-map-2013-svg | T3 | real | 26486961 | xml | 135 | 70057984 | malformed SVG: path data at byte 474 |
