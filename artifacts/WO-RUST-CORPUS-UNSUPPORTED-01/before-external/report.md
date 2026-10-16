# Rust corpus measurement

These are verdict counts and timings only, produced by one run on one host. They are not fidelity, conformance or performance claims. The corpus bytes are not in this repository; only manifest metadata, verdicts and measurements are recorded.

- tool: vexel-corpus, base HEAD `393ae68f54f1bcc15d4a25ab0b754bd4fcc4a496` (working tree may be dirty)
- manifest: `~/Desktop/Freelance/vexel-corpus-real/manifest.json` sha256 `9fbf9a2f384d70a41da72e99ac259d2c5d137ffd3c8a01ce4be4741c1c29bc30`
- render size: 128x128, skip over 33554432 bytes, timeout 60 s, RSS limit 2048 MiB
- host: ProductName: macOS ProductVersion: 27.0.1 BuildVersion: 26A434; arm64

## Verdict histogram

| verdict | files |
|---|---|
| ok | 5 |
| unsupported | 2 |
| limit | 29 |
| xml | 2 |
| timeout | 1 |
| rss_exceeded | 0 |
| skipped | 19 |
| crash | 0 |
| guard_failed | 0 |

Total: 58 files, summed wall time 192526 ms.

## Slowest five (wall ms)

- `generated-cnc_t1` 60002
- `generated-giant_path_t1` 55334
- `generated-cad_t1` 45524
- `generated-text_t1` 17858
- `generated-use_t1` 6730

## Largest peak RSS five (bytes)

- `generated-text_t1` 389890048
- `generated-giant_path_t2` 339705856
- `generated-cad_t1` 213827584
- `generated-science_t1` 74694656
- `generated-use_t1` 62423040

## Per file

| id | tier | kind | bytes | verdict | wall ms | peak RSS | detail |
|---|---|---|---|---|---|---|---|
| generated-cad_t1 | T1 | generated | 997801 | ok | 45524 | 213827584 | 128x128 frame, 0 disclosure(s) |
| generated-cad_t2 | T2 | generated | 10592198 | limit | 10 | 18366464 | document exceeds a safety limit |
| generated-cad_t3 | T3 | generated | 109719583 | skipped | 0 | - | not read: 109719583 bytes exceeds --max-bytes 33554432 |
| generated-cnc_t1 | T1 | generated | 990772 | timeout | 60002 | - | killed after 60 s wall-clock timeout |
| generated-cnc_t2 | T2 | generated | 10162188 | limit | 11 | 17940480 | document exceeds a safety limit |
| generated-cnc_t3 | T3 | generated | 102580014 | skipped | 0 | - | not read: 102580014 bytes exceeds --max-bytes 33554432 |
| generated-embedded_t1 | T1 | generated | 912235 | unsupported | 20 | 15761408 | unsupported in V1: image (<image> at byte 893) |
| generated-embedded_t2 | T2 | generated | 9371925 | limit | 15 | 17154048 | document exceeds a safety limit |
| generated-embedded_t3 | T3 | generated | 93970295 | skipped | 0 | - | not read: 93970295 bytes exceeds --max-bytes 33554432 |
| generated-giant_path_t1 | T1 | generated | 920236 | ok | 55334 | 60030976 | 128x128 frame, 0 disclosure(s) |
| generated-giant_path_t2 | T2 | generated | 8340030 | limit | 330 | 339705856 | document exceeds a safety limit |
| generated-giant_path_t3 | T3 | generated | 91963466 | skipped | 0 | - | not read: 91963466 bytes exceeds --max-bytes 33554432 |
| generated-giant_path_t4 | T4 | generated | 827663466 | skipped | 0 | - | not read: 827663466 bytes exceeds --max-bytes 33554432 |
| generated-inkscape_t1 | T1 | generated | 979796 | unsupported | 26 | 19759104 | unsupported in V1: marker (<marker> at byte 688) |
| generated-inkscape_t2 | T2 | generated | 10062253 | limit | 14 | 17842176 | document exceeds a safety limit |
| generated-inkscape_t3 | T3 | generated | 100883678 | skipped | 0 | - | not read: 100883678 bytes exceeds --max-bytes 33554432 |
| generated-map_t1 | T1 | generated | 1310972 | xml | 2218 | 42205184 | malformed SVG: invalid value at byte 0 |
| generated-map_t2 | T2 | generated | 14191241 | limit | 15 | 21970944 | document exceeds a safety limit |
| generated-map_t3 | T3 | generated | 150627424 | skipped | 0 | - | not read: 150627424 bytes exceeds --max-bytes 33554432 |
| generated-nested_t1 | T1 | generated | 978645 | xml | 13 | 10174464 | malformed SVG: XML at byte 16843 |
| generated-nested_t2 | T2 | generated | 9430992 | limit | 15 | 17203200 | document exceeds a safety limit |
| generated-nested_t3 | T3 | generated | 93956332 | skipped | 0 | - | not read: 93956332 bytes exceeds --max-bytes 33554432 |
| generated-science_t1 | T1 | generated | 1374397 | ok | 3844 | 74694656 | 128x128 frame, 0 disclosure(s) |
| generated-science_t2 | T2 | generated | 14116675 | limit | 13 | 21889024 | document exceeds a safety limit |
| generated-science_t3 | T3 | generated | 117860153 | skipped | 0 | - | not read: 117860153 bytes exceeds --max-bytes 33554432 |
| generated-science_t4 | T4 | generated | 635202435 | skipped | 0 | - | not read: 635202435 bytes exceeds --max-bytes 33554432 |
| generated-text_t1 | T1 | generated | 887035 | limit | 17858 | 389890048 | document exceeds a safety limit |
| generated-text_t2 | T2 | generated | 9024701 | limit | 15 | 16793600 | document exceeds a safety limit |
| generated-text_t3 | T3 | generated | 90488308 | skipped | 0 | - | not read: 90488308 bytes exceeds --max-bytes 33554432 |
| generated-use_t1 | T1 | generated | 986323 | limit | 6730 | 62423040 | document exceeds a safety limit |
| generated-use_t2 | T2 | generated | 10036526 | limit | 15 | 17809408 | document exceeds a safety limit |
| generated-use_t3 | T3 | generated | 100542256 | skipped | 0 | - | not read: 100542256 bytes exceeds --max-bytes 33554432 |
| generated-use_t4 | T4 | generated | 1005597835 | skipped | 0 | - | not read: 1005597835 bytes exceeds --max-bytes 33554432 |
| commons-1-42-polytope-7-cube-svg | T3 | real | 29836045 | limit | 15 | 37617664 | document exceeds a safety limit |
| commons-2015-louisiana-gubernatorial-election-runoff-by-precinct-svg | T2 | real | 16162430 | limit | 15 | 23937024 | document exceeds a safety limit |
| commons-aegean-sea-anatolia-and-armenian-highlands-regions-large-topographic-basemap-svg | T3 | real | 22666147 | limit | 15 | 30441472 | document exceeds a safety limit |
| commons-amhara-topographic-map-svg | T2 | real | 15593284 | limit | 15 | 23363584 | document exceeds a safety limit |
| commons-bulgaria-in-1915-geographic-map-en-svg | T2 | real | 15165804 | limit | 10 | 22937600 | document exceeds a safety limit |
| commons-cambodia-geographic-map-en-svg | T3 | real | 20142590 | limit | 15 | 27918336 | document exceeds a safety limit |
| commons-chuaattractormodified-svg | T3 | real | 20763720 | limit | 10 | 28524544 | document exceeds a safety limit |
| commons-coat-of-arms-of-the-kingdom-of-naples-1808-svg | T3 | real | 25708531 | limit | 15 | 33488896 | document exceeds a safety limit |
| commons-coat-of-arms-of-villa-alemana-svg | T3 | real | 42628426 | skipped | 0 | - | not read: 42628426 bytes exceeds --max-bytes 33554432 |
| commons-columbiarivermap-svg | T2 | real | 16687292 | limit | 13 | 24461312 | document exceeds a safety limit |
| commons-comunas-occitanas-d-alemanha-svg | T3 | real | 43507208 | skipped | 0 | - | not read: 43507208 bytes exceeds --max-bytes 33554432 |
| commons-corsica-geographic-map-svg | T2 | real | 14927948 | limit | 11 | 22708224 | document exceeds a safety limit |
| commons-corvuscoraxiucn-svg | T3 | real | 26496549 | limit | 11 | 34275328 | document exceeds a safety limit |
| commons-dibujo-piu-svg | T3 | real | 87755815 | skipped | 0 | - | not read: 87755815 bytes exceeds --max-bytes 33554432 |
| commons-dukesofhohenbergarms-svg | T3 | real | 28075813 | limit | 15 | 35848192 | document exceeds a safety limit |
| commons-france-osm-mapnik-svg | T2 | real | 19491319 | limit | 11 | 27262976 | document exceeds a safety limit |
| commons-gaussian-2d-svg | T2 | real | 16746052 | limit | 15 | 24526848 | document exceeds a safety limit |
| commons-geographic-map-of-norway-sweden-finland-and-denmark-svg | T3 | real | 20101913 | limit | 11 | 27869184 | document exceeds a safety limit |
| commons-germany-districts-municipalities-location-map-2013-svg | T3 | real | 26486961 | limit | 15 | 34258944 | document exceeds a safety limit |
| commons-ideogram-human-chromosome-20-svg | T3 | real | 21285516 | limit | 15 | 29065216 | document exceeds a safety limit |
| commons-koppen-geiger-map-a-present-svg | T3 | real | 79058833 | skipped | 0 | - | not read: 79058833 bytes exceeds --max-bytes 33554432 |
| commons-municipalities-of-italy-svg | T3 | real | 91915099 | skipped | 0 | - | not read: 91915099 bytes exceeds --max-bytes 33554432 |
| commons-spain-languages-de-svg | T3 | real | 90812529 | skipped | 0 | - | not read: 90812529 bytes exceeds --max-bytes 33554432 |
| natural-earth-world-map-blank | T1 | real | 241682 | ok | 106 | 18726912 | 128x128 frame, 0 disclosure(s) |
| natural-earth-world-map-countries | T1 | real | 260823 | ok | 171 | 28147712 | 128x128 frame, 0 disclosure(s) |
