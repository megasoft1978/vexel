# Rust corpus measurement

These are verdict counts and timings only, produced by one run on one host. They are not fidelity, conformance or performance claims. The corpus bytes are not in this repository; only manifest metadata, verdicts and measurements are recorded.

- tool: vexel-corpus, base HEAD `497f5b61e864211c4accd01ebbd45ea97431bf88` (working tree may be dirty)
- manifest: `~/Desktop/Freelance/vexel-corpus-real/manifest.json` sha256 `9fbf9a2f384d70a41da72e99ac259d2c5d137ffd3c8a01ce4be4741c1c29bc30`
- render size: 128x128, skip over 33554432 bytes, timeout 60 s, RSS limit 2048 MiB
- host: ProductName: macOS ProductVersion: 27.0.1 BuildVersion: 26A434; arm64

## Verdict histogram

| verdict | files |
|---|---|
| ok | 11 |
| unsupported | 8 |
| limit_decode | 8 |
| limit_render | 2 |
| xml | 10 |
| timeout | 0 |
| rss_exceeded | 0 |
| skipped | 19 |
| crash | 0 |
| guard_failed | 0 |

Total: 58 files, summed wall time 92248 ms.

## Slowest five (wall ms)

- `generated-giant_path_t1` 54704
- `generated-embedded_t2` 24095
- `generated-use_t2` 1331
- `commons-cambodia-geographic-map-en-svg` 1084
- `commons-1-42-polytope-7-cube-svg` 1021

## Largest peak RSS five (bytes)

- `commons-corvuscoraxiucn-svg` 564920320
- `commons-bulgaria-in-1915-geographic-map-en-svg` 561283072
- `commons-2015-louisiana-gubernatorial-election-runoff-by-precinct-svg` 517505024
- `commons-cambodia-geographic-map-en-svg` 486932480
- `generated-text_t2` 458981376

## Per file

| id | tier | kind | bytes | verdict | wall ms | peak RSS | detail |
|---|---|---|---|---|---|---|---|
| generated-cad_t1 | T1 | generated | 997801 | ok | 343 | 228048896 | 128x128 frame, 0 disclosure(s) |
| generated-cad_t2 | T2 | generated | 10592198 | limit_decode | 642 | 249167872 | document exceeds a decode safety limit |
| generated-cad_t3 | T3 | generated | 109719583 | skipped | 0 | - | not read: 109719583 bytes exceeds --max-bytes 33554432 |
| generated-cnc_t1 | T1 | generated | 990772 | ok | 189 | 98418688 | 128x128 frame, 0 disclosure(s) |
| generated-cnc_t2 | T2 | generated | 10162188 | limit_decode | 464 | 249413632 | document exceeds a decode safety limit |
| generated-cnc_t3 | T3 | generated | 102580014 | skipped | 0 | - | not read: 102580014 bytes exceeds --max-bytes 33554432 |
| generated-embedded_t1 | T1 | generated | 912235 | ok | 264 | 24657920 | 128x128 frame, 0 disclosure(s) |
| generated-embedded_t2 | T2 | generated | 9371925 | ok | 24095 | 175849472 | 128x128 frame, 0 disclosure(s) |
| generated-embedded_t3 | T3 | generated | 93970295 | skipped | 0 | - | not read: 93970295 bytes exceeds --max-bytes 33554432 |
| generated-giant_path_t1 | T1 | generated | 920236 | ok | 54704 | 59965440 | 128x128 frame, 0 disclosure(s) |
| generated-giant_path_t2 | T2 | generated | 8340030 | limit_render | 352 | 339886080 | document exceeds a render safety limit |
| generated-giant_path_t3 | T3 | generated | 91963466 | skipped | 0 | - | not read: 91963466 bytes exceeds --max-bytes 33554432 |
| generated-giant_path_t4 | T4 | generated | 827663466 | skipped | 0 | - | not read: 827663466 bytes exceeds --max-bytes 33554432 |
| generated-inkscape_t1 | T1 | generated | 979796 | ok | 182 | 163102720 | 128x128 frame, 34715 disclosure(s) |
| generated-inkscape_t2 | T2 | generated | 10062253 | limit_decode | 523 | 318914560 | document exceeds a decode safety limit |
| generated-inkscape_t3 | T3 | generated | 100883678 | skipped | 0 | - | not read: 100883678 bytes exceeds --max-bytes 33554432 |
| generated-map_t1 | T1 | generated | 1310972 | xml | 55 | 44793856 | malformed SVG: invalid value at byte 0 |
| generated-map_t2 | T2 | generated | 14191241 | xml | 543 | 299499520 | malformed SVG: invalid value at byte 0 |
| generated-map_t3 | T3 | generated | 150627424 | skipped | 0 | - | not read: 150627424 bytes exceeds --max-bytes 33554432 |
| generated-nested_t1 | T1 | generated | 978645 | xml | 15 | 10174464 | malformed SVG: XML at byte 16843 |
| generated-nested_t2 | T2 | generated | 9430992 | xml | 39 | 27099136 | malformed SVG: XML at byte 16843 |
| generated-nested_t3 | T3 | generated | 93956332 | skipped | 0 | - | not read: 93956332 bytes exceeds --max-bytes 33554432 |
| generated-science_t1 | T1 | generated | 1374397 | ok | 109 | 74399744 | 128x128 frame, 0 disclosure(s) |
| generated-science_t2 | T2 | generated | 14116675 | limit_decode | 350 | 213598208 | document exceeds a decode safety limit |
| generated-science_t3 | T3 | generated | 117860153 | skipped | 0 | - | not read: 117860153 bytes exceeds --max-bytes 33554432 |
| generated-science_t4 | T4 | generated | 635202435 | skipped | 0 | - | not read: 635202435 bytes exceeds --max-bytes 33554432 |
| generated-text_t1 | T1 | generated | 887035 | limit_decode | 370 | 391544832 | document exceeds a decode safety limit |
| generated-text_t2 | T2 | generated | 9024701 | limit_decode | 512 | 458981376 | document exceeds a decode safety limit |
| generated-text_t3 | T3 | generated | 90488308 | skipped | 0 | - | not read: 90488308 bytes exceeds --max-bytes 33554432 |
| generated-use_t1 | T1 | generated | 986323 | ok | 255 | 91455488 | 128x128 frame, 0 disclosure(s) |
| generated-use_t2 | T2 | generated | 10036526 | limit_decode | 1331 | 328581120 | document exceeds a decode safety limit |
| generated-use_t3 | T3 | generated | 100542256 | skipped | 0 | - | not read: 100542256 bytes exceeds --max-bytes 33554432 |
| generated-use_t4 | T4 | generated | 1005597835 | skipped | 0 | - | not read: 1005597835 bytes exceeds --max-bytes 33554432 |
| commons-1-42-polytope-7-cube-svg | T3 | real | 29836045 | xml | 1021 | 213975040 | malformed SVG: XML at byte 15241908 |
| commons-2015-louisiana-gubernatorial-election-runoff-by-precinct-svg | T2 | real | 16162430 | ok | 700 | 517505024 | 128x128 frame, 0 disclosure(s) |
| commons-aegean-sea-anatolia-and-armenian-highlands-regions-large-topographic-basemap-svg | T3 | real | 22666147 | xml | 129 | 72925184 | malformed SVG: viewport at byte 70 |
| commons-amhara-topographic-map-svg | T2 | real | 15593284 | unsupported | 390 | 314523648 | unsupported in V1: text (<text> at byte 14693546) |
| commons-bulgaria-in-1915-geographic-map-en-svg | T2 | real | 15165804 | unsupported | 779 | 561283072 | unsupported in V1: unsupported element or attribute (<circle> at byte 15068031) |
| commons-cambodia-geographic-map-en-svg | T3 | real | 20142590 | limit_decode | 1084 | 486932480 | document exceeds a decode safety limit |
| commons-chuaattractormodified-svg | T3 | real | 20763720 | unsupported | 531 | 157679616 | unsupported in V1: unsupported element or attribute (<translate> at byte 2053) |
| commons-coat-of-arms-of-the-kingdom-of-naples-1808-svg | T3 | real | 25708531 | unsupported | 157 | 84393984 | unsupported in V1: unsupported element or attribute (<use> at byte 457167) |
| commons-coat-of-arms-of-villa-alemana-svg | T3 | real | 42628426 | skipped | 0 | - | not read: 42628426 bytes exceeds --max-bytes 33554432 |
| commons-columbiarivermap-svg | T2 | real | 16687292 | xml | 54 | 41287680 | malformed SVG: XML at byte 917 |
| commons-comunas-occitanas-d-alemanha-svg | T3 | real | 43507208 | skipped | 0 | - | not read: 43507208 bytes exceeds --max-bytes 33554432 |
| commons-corsica-geographic-map-svg | T2 | real | 14927948 | unsupported | 75 | 50692096 | unsupported in V1: unsupported element or attribute (<tspan3993> at byte 14832988) |
| commons-corvuscoraxiucn-svg | T3 | real | 26496549 | limit_render | 913 | 564920320 | document exceeds a render safety limit |
| commons-dibujo-piu-svg | T3 | real | 87755815 | skipped | 0 | - | not read: 87755815 bytes exceeds --max-bytes 33554432 |
| commons-dukesofhohenbergarms-svg | T3 | real | 28075813 | unsupported | 200 | 96337920 | unsupported in V1: unsupported element or attribute |
| commons-france-osm-mapnik-svg | T2 | real | 19491319 | xml | 95 | 53657600 | malformed SVG: path data at byte 28855 |
| commons-gaussian-2d-svg | T2 | real | 16746052 | unsupported | 94 | 56279040 | unsupported in V1: unsupported element or attribute (<translate> at byte 6158) |
| commons-geographic-map-of-norway-sweden-finland-and-denmark-svg | T3 | real | 20101913 | xml | 142 | 82608128 | malformed SVG: viewport at byte 70 |
| commons-germany-districts-municipalities-location-map-2013-svg | T3 | real | 26486961 | xml | 121 | 70074368 | malformed SVG: path data at byte 474 |
| commons-ideogram-human-chromosome-20-svg | T3 | real | 21285516 | unsupported | 323 | 130580480 | unsupported in V1: unsupported element or attribute (<11px> at byte 20677771) |
| commons-koppen-geiger-map-a-present-svg | T3 | real | 79058833 | skipped | 0 | - | not read: 79058833 bytes exceeds --max-bytes 33554432 |
| commons-municipalities-of-italy-svg | T3 | real | 91915099 | skipped | 0 | - | not read: 91915099 bytes exceeds --max-bytes 33554432 |
| commons-spain-languages-de-svg | T3 | real | 90812529 | skipped | 0 | - | not read: 90812529 bytes exceeds --max-bytes 33554432 |
| natural-earth-world-map-blank | T1 | real | 241682 | ok | 45 | 20365312 | 128x128 frame, 0 disclosure(s) |
| natural-earth-world-map-countries | T1 | real | 260823 | ok | 58 | 29097984 | 128x128 frame, 0 disclosure(s) |
