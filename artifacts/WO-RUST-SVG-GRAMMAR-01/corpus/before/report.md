# Rust corpus measurement

These are verdict counts and timings only, produced by one run on one host. They are not fidelity, conformance or performance claims. The corpus bytes are not in this repository; only manifest metadata, verdicts and measurements are recorded.

- tool: vexel-corpus, base HEAD `ba2442d65be4fbb3089a9b1be25c981285f6af97` (working tree may be dirty)
- manifest: `~/Desktop/Freelance/vexel-corpus-real/manifest.json` sha256 `9fbf9a2f384d70a41da72e99ac259d2c5d137ffd3c8a01ce4be4741c1c29bc30`
- render size: 128x128, skip over 33554432 bytes, timeout 60 s, RSS limit 2048 MiB
- host: ProductName: macOS ProductVersion: 27.0.1 BuildVersion: 26A434; arm64

## Verdict histogram

| verdict | files |
|---|---|
| ok | 0 |
| unsupported | 0 |
| limit_decode | 0 |
| limit_render | 0 |
| xml | 7 |
| timeout | 0 |
| rss_exceeded | 0 |
| skipped | 0 |
| crash | 0 |
| guard_failed | 0 |

Total: 7 files, summed wall time 1838 ms.

## Slowest five (wall ms)

- `commons-1-42-polytope-7-cube-svg` 1383
- `commons-germany-districts-municipalities-location-map-2013-svg` 126
- `commons-aegean-sea-anatolia-and-armenian-highlands-regions-large-topographic-basemap-svg` 120
- `commons-france-osm-mapnik-svg` 95
- `generated-map_t1` 59

## Largest peak RSS five (bytes)

- `commons-1-42-polytope-7-cube-svg` 213942272
- `commons-aegean-sea-anatolia-and-armenian-highlands-regions-large-topographic-basemap-svg` 72925184
- `commons-germany-districts-municipalities-location-map-2013-svg` 70090752
- `commons-france-osm-mapnik-svg` 55836672
- `generated-map_t1` 44630016

## Per file

| id | tier | kind | bytes | verdict | wall ms | peak RSS | detail |
|---|---|---|---|---|---|---|---|
| generated-map_t1 | T1 | generated | 1310972 | xml | 59 | 44630016 | malformed SVG: invalid value at byte 0 |
| generated-nested_t1 | T1 | generated | 978645 | xml | 10 | 10141696 | malformed SVG: XML at byte 16843 |
| commons-1-42-polytope-7-cube-svg | T3 | real | 29836045 | xml | 1383 | 213942272 | malformed SVG: XML at byte 15241908 |
| commons-aegean-sea-anatolia-and-armenian-highlands-regions-large-topographic-basemap-svg | T3 | real | 22666147 | xml | 120 | 72925184 | malformed SVG: viewport at byte 70 |
| commons-columbiarivermap-svg | T2 | real | 16687292 | xml | 45 | 41287680 | malformed SVG: XML at byte 917 |
| commons-france-osm-mapnik-svg | T2 | real | 19491319 | xml | 95 | 55836672 | malformed SVG: path data at byte 28855 |
| commons-germany-districts-municipalities-location-map-2013-svg | T3 | real | 26486961 | xml | 126 | 70090752 | malformed SVG: path data at byte 474 |
