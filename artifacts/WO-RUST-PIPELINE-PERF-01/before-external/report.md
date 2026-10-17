# Rust corpus measurement

These are verdict counts and timings only, produced by one run on one host. They are not fidelity, conformance or performance claims. The corpus bytes are not in this repository; only manifest metadata, verdicts and measurements are recorded.

- tool: vexel-corpus, base HEAD `b2092b74da9c5b3ceb51494e46c292c9fac05d49` (working tree may be dirty)
- manifest: `~/Desktop/Freelance/vexel-corpus-real/manifest.json` sha256 `9fbf9a2f384d70a41da72e99ac259d2c5d137ffd3c8a01ce4be4741c1c29bc30`
- render size: 128x128, skip over 33554432 bytes, timeout 60 s, RSS limit 2048 MiB
- host: ProductName: macOS ProductVersion: 27.0.1 BuildVersion: 26A434; arm64

## Verdict histogram

| verdict | files |
|---|---|
| ok | 6 |
| unsupported | 5 |
| limit_decode | 6 |
| limit_render | 1 |
| xml | 9 |
| timeout | 12 |
| rss_exceeded | 0 |
| skipped | 19 |
| crash | 0 |
| guard_failed | 0 |

Total: 58 files, summed wall time 895151 ms.

## Slowest five (wall ms)

- `generated-cnc_t1` 60015
- `generated-embedded_t2` 60014
- `commons-corvuscoraxiucn-svg` 60013
- `generated-cnc_t2` 60013
- `generated-science_t2` 60013

## Largest peak RSS five (bytes)

- `generated-text_t2` 459718656
- `generated-text_t1` 392691712
- `generated-giant_path_t2` 339066880
- `commons-amhara-topographic-map-svg` 317440000
- `commons-1-42-polytope-7-cube-svg` 214007808

## Per file

| id | tier | kind | bytes | verdict | wall ms | peak RSS | detail |
|---|---|---|---|---|---|---|---|
| generated-cad_t1 | T1 | generated | 997801 | ok | 46910 | 209551360 | 128x128 frame, 0 disclosure(s) |
| generated-cad_t2 | T2 | generated | 10592198 | timeout | 60003 | - | killed after 60 s wall-clock timeout |
| generated-cad_t3 | T3 | generated | 109719583 | skipped | 0 | - | not read: 109719583 bytes exceeds --max-bytes 33554432 |
| generated-cnc_t1 | T1 | generated | 990772 | timeout | 60015 | - | killed after 60 s wall-clock timeout |
| generated-cnc_t2 | T2 | generated | 10162188 | timeout | 60013 | - | killed after 60 s wall-clock timeout |
| generated-cnc_t3 | T3 | generated | 102580014 | skipped | 0 | - | not read: 102580014 bytes exceeds --max-bytes 33554432 |
| generated-embedded_t1 | T1 | generated | 912235 | ok | 1150 | 24985600 | 128x128 frame, 0 disclosure(s) |
| generated-embedded_t2 | T2 | generated | 9371925 | timeout | 60014 | - | killed after 60 s wall-clock timeout |
| generated-embedded_t3 | T3 | generated | 93970295 | skipped | 0 | - | not read: 93970295 bytes exceeds --max-bytes 33554432 |
| generated-giant_path_t1 | T1 | generated | 920236 | ok | 56070 | 60129280 | 128x128 frame, 0 disclosure(s) |
| generated-giant_path_t2 | T2 | generated | 8340030 | limit_render | 353 | 339066880 | document exceeds a render safety limit |
| generated-giant_path_t3 | T3 | generated | 91963466 | skipped | 0 | - | not read: 91963466 bytes exceeds --max-bytes 33554432 |
| generated-giant_path_t4 | T4 | generated | 827663466 | skipped | 0 | - | not read: 827663466 bytes exceeds --max-bytes 33554432 |
| generated-inkscape_t1 | T1 | generated | 979796 | timeout | 60011 | - | killed after 60 s wall-clock timeout |
| generated-inkscape_t2 | T2 | generated | 10062253 | timeout | 60010 | - | killed after 60 s wall-clock timeout |
| generated-inkscape_t3 | T3 | generated | 100883678 | skipped | 0 | - | not read: 100883678 bytes exceeds --max-bytes 33554432 |
| generated-map_t1 | T1 | generated | 1310972 | xml | 2365 | 42336256 | malformed SVG: invalid value at byte 0 |
| generated-map_t2 | T2 | generated | 14191241 | timeout | 60003 | - | killed after 60 s wall-clock timeout |
| generated-map_t3 | T3 | generated | 150627424 | skipped | 0 | - | not read: 150627424 bytes exceeds --max-bytes 33554432 |
| generated-nested_t1 | T1 | generated | 978645 | xml | 12 | 10190848 | malformed SVG: XML at byte 16843 |
| generated-nested_t2 | T2 | generated | 9430992 | xml | 39 | 27131904 | malformed SVG: XML at byte 16843 |
| generated-nested_t3 | T3 | generated | 93956332 | skipped | 0 | - | not read: 93956332 bytes exceeds --max-bytes 33554432 |
| generated-science_t1 | T1 | generated | 1374397 | ok | 4797 | 79495168 | 128x128 frame, 0 disclosure(s) |
| generated-science_t2 | T2 | generated | 14116675 | timeout | 60013 | - | killed after 60 s wall-clock timeout |
| generated-science_t3 | T3 | generated | 117860153 | skipped | 0 | - | not read: 117860153 bytes exceeds --max-bytes 33554432 |
| generated-science_t4 | T4 | generated | 635202435 | skipped | 0 | - | not read: 635202435 bytes exceeds --max-bytes 33554432 |
| generated-text_t1 | T1 | generated | 887035 | limit_decode | 20783 | 392691712 | document exceeds a decode safety limit |
| generated-text_t2 | T2 | generated | 9024701 | limit_decode | 21548 | 459718656 | document exceeds a decode safety limit |
| generated-text_t3 | T3 | generated | 90488308 | skipped | 0 | - | not read: 90488308 bytes exceeds --max-bytes 33554432 |
| generated-use_t1 | T1 | generated | 986323 | limit_decode | 8075 | 62898176 | document exceeds a decode safety limit |
| generated-use_t2 | T2 | generated | 10036526 | limit_decode | 8199 | 180518912 | document exceeds a decode safety limit |
| generated-use_t3 | T3 | generated | 100542256 | skipped | 0 | - | not read: 100542256 bytes exceeds --max-bytes 33554432 |
| generated-use_t4 | T4 | generated | 1005597835 | skipped | 0 | - | not read: 1005597835 bytes exceeds --max-bytes 33554432 |
| commons-1-42-polytope-7-cube-svg | T3 | real | 29836045 | xml | 1499 | 214007808 | malformed SVG: XML at byte 15241908 |
| commons-2015-louisiana-gubernatorial-election-runoff-by-precinct-svg | T2 | real | 16162430 | timeout | 60010 | - | killed after 60 s wall-clock timeout |
| commons-aegean-sea-anatolia-and-armenian-highlands-regions-large-topographic-basemap-svg | T3 | real | 22666147 | xml | 135 | 71335936 | malformed SVG: viewport at byte 70 |
| commons-amhara-topographic-map-svg | T2 | real | 15593284 | unsupported | 708 | 317440000 | unsupported in V1: text (<text> at byte 14693546) |
| commons-bulgaria-in-1915-geographic-map-en-svg | T2 | real | 15165804 | timeout | 60007 | - | killed after 60 s wall-clock timeout |
| commons-cambodia-geographic-map-en-svg | T3 | real | 20142590 | timeout | 60006 | - | killed after 60 s wall-clock timeout |
| commons-chuaattractormodified-svg | T3 | real | 20763720 | limit_decode | 674 | 147980288 | document exceeds a decode safety limit |
| commons-coat-of-arms-of-the-kingdom-of-naples-1808-svg | T3 | real | 25708531 | unsupported | 203 | 82952192 | unsupported in V1: unsupported element or attribute (<use> at byte 457167) |
| commons-coat-of-arms-of-villa-alemana-svg | T3 | real | 42628426 | skipped | 0 | - | not read: 42628426 bytes exceeds --max-bytes 33554432 |
| commons-columbiarivermap-svg | T2 | real | 16687292 | xml | 54 | 41336832 | malformed SVG: XML at byte 917 |
| commons-comunas-occitanas-d-alemanha-svg | T3 | real | 43507208 | skipped | 0 | - | not read: 43507208 bytes exceeds --max-bytes 33554432 |
| commons-corsica-geographic-map-svg | T2 | real | 14927948 | unsupported | 84 | 51249152 | unsupported in V1: unsupported element or attribute (<tspan3993> at byte 14832988) |
| commons-corvuscoraxiucn-svg | T3 | real | 26496549 | timeout | 60013 | - | killed after 60 s wall-clock timeout |
| commons-dibujo-piu-svg | T3 | real | 87755815 | skipped | 0 | - | not read: 87755815 bytes exceeds --max-bytes 33554432 |
| commons-dukesofhohenbergarms-svg | T3 | real | 28075813 | unsupported | 278 | 94437376 | unsupported in V1: unsupported element or attribute |
| commons-france-osm-mapnik-svg | T2 | real | 19491319 | xml | 97 | 53346304 | malformed SVG: path data at byte 28855 |
| commons-gaussian-2d-svg | T2 | real | 16746052 | unsupported | 101 | 55181312 | unsupported in V1: unsupported element or attribute (<translate> at byte 6158) |
| commons-geographic-map-of-norway-sweden-finland-and-denmark-svg | T3 | real | 20101913 | xml | 144 | 82100224 | malformed SVG: viewport at byte 70 |
| commons-germany-districts-municipalities-location-map-2013-svg | T3 | real | 26486961 | xml | 123 | 69533696 | malformed SVG: path data at byte 474 |
| commons-ideogram-human-chromosome-20-svg | T3 | real | 21285516 | limit_decode | 347 | 123338752 | document exceeds a decode safety limit |
| commons-koppen-geiger-map-a-present-svg | T3 | real | 79058833 | skipped | 0 | - | not read: 79058833 bytes exceeds --max-bytes 33554432 |
| commons-municipalities-of-italy-svg | T3 | real | 91915099 | skipped | 0 | - | not read: 91915099 bytes exceeds --max-bytes 33554432 |
| commons-spain-languages-de-svg | T3 | real | 90812529 | skipped | 0 | - | not read: 90812529 bytes exceeds --max-bytes 33554432 |
| natural-earth-world-map-blank | T1 | real | 241682 | ok | 112 | 20054016 | 128x128 frame, 0 disclosure(s) |
| natural-earth-world-map-countries | T1 | real | 260823 | ok | 173 | 28180480 | 128x128 frame, 0 disclosure(s) |
