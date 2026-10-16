# Rust corpus measurement

These are verdict counts and timings only, produced by one run on one host. They are not fidelity, conformance or performance claims. The corpus bytes are not in this repository; only manifest metadata, verdicts and measurements are recorded.

- tool: vexel-corpus, base HEAD `842d6ce0373b6da3adf9799a6783da01d326efb5` (working tree may be dirty)
- manifest: `~/Desktop/Freelance/vexel-corpus-real/manifest.json` sha256 `9fbf9a2f384d70a41da72e99ac259d2c5d137ffd3c8a01ce4be4741c1c29bc30`
- render size: 128x128, skip over 33554432 bytes, timeout 60 s, RSS limit 2048 MiB
- host: ProductName: macOS ProductVersion: 27.0.1 BuildVersion: 26A434; arm64

## Verdict histogram

| verdict | files |
|---|---|
| ok | 1 |
| unsupported | 7 |
| limit | 28 |
| xml | 2 |
| timeout | 1 |
| rss_exceeded | 0 |
| skipped | 19 |
| crash | 0 |
| guard_failed | 0 |

Total: 58 files, summed wall time 124173 ms.

## Slowest five (wall ms)

- `generated-cnc_t1` 60005
- `generated-cad_t1` 28365
- `generated-giant_path_t1` 21727
- `generated-use_t1` 6767
- `generated-science_t1` 4097

## Largest peak RSS five (bytes)

- `generated-giant_path_t2` 339542016
- `generated-use_t1` 60964864
- `generated-cad_t1` 60768256
- `generated-giant_path_t1` 53346304
- `generated-science_t1` 51511296

## Per file

| id | tier | kind | bytes | verdict | wall ms | peak RSS | detail |
|---|---|---|---|---|---|---|---|
| generated-cad_t1 | T1 | generated | 997801 | unsupported | 28365 | 60768256 | unsupported in V1: text (<text> at byte 840302) |
| generated-cad_t2 | T2 | generated | 10592198 | limit | 12 | 18317312 | document exceeds a safety limit |
| generated-cad_t3 | T3 | generated | 109719583 | skipped | 0 | - | not read: 109719583 bytes exceeds --max-bytes 33554432 |
| generated-cnc_t1 | T1 | generated | 990772 | timeout | 60005 | - | killed after 60 s wall-clock timeout |
| generated-cnc_t2 | T2 | generated | 10162188 | limit | 12 | 17891328 | document exceeds a safety limit |
| generated-cnc_t3 | T3 | generated | 102580014 | skipped | 0 | - | not read: 102580014 bytes exceeds --max-bytes 33554432 |
| generated-embedded_t1 | T1 | generated | 912235 | unsupported | 25 | 15630336 | unsupported in V1: image (<image> at byte 894) |
| generated-embedded_t2 | T2 | generated | 9371925 | limit | 12 | 17104896 | document exceeds a safety limit |
| generated-embedded_t3 | T3 | generated | 93970295 | skipped | 0 | - | not read: 93970295 bytes exceeds --max-bytes 33554432 |
| generated-giant_path_t1 | T1 | generated | 920236 | ok | 21727 | 53346304 | 128x128 frame, 0 disclosure(s) |
| generated-giant_path_t2 | T2 | generated | 8340030 | limit | 354 | 339542016 | document exceeds a safety limit |
| generated-giant_path_t3 | T3 | generated | 91963466 | skipped | 0 | - | not read: 91963466 bytes exceeds --max-bytes 33554432 |
| generated-giant_path_t4 | T4 | generated | 827663466 | skipped | 0 | - | not read: 827663466 bytes exceeds --max-bytes 33554432 |
| generated-inkscape_t1 | T1 | generated | 979796 | unsupported | 25 | 19300352 | unsupported in V1: filter (<filter> at byte 623) |
| generated-inkscape_t2 | T2 | generated | 10062253 | limit | 12 | 17793024 | document exceeds a safety limit |
| generated-inkscape_t3 | T3 | generated | 100883678 | skipped | 0 | - | not read: 100883678 bytes exceeds --max-bytes 33554432 |
| generated-map_t1 | T1 | generated | 1310972 | xml | 2299 | 42041344 | malformed SVG: invalid value at byte 0 |
| generated-map_t2 | T2 | generated | 14191241 | limit | 12 | 21921792 | document exceeds a safety limit |
| generated-map_t3 | T3 | generated | 150627424 | skipped | 0 | - | not read: 150627424 bytes exceeds --max-bytes 33554432 |
| generated-nested_t1 | T1 | generated | 978645 | xml | 11 | 10092544 | malformed SVG: XML at byte 16843 |
| generated-nested_t2 | T2 | generated | 9430992 | limit | 12 | 17154048 | document exceeds a safety limit |
| generated-nested_t3 | T3 | generated | 93956332 | skipped | 0 | - | not read: 93956332 bytes exceeds --max-bytes 33554432 |
| generated-science_t1 | T1 | generated | 1374397 | unsupported | 4097 | 51511296 | unsupported in V1: text (<text> at byte 1374260) |
| generated-science_t2 | T2 | generated | 14116675 | limit | 12 | 21839872 | document exceeds a safety limit |
| generated-science_t3 | T3 | generated | 117860153 | skipped | 0 | - | not read: 117860153 bytes exceeds --max-bytes 33554432 |
| generated-science_t4 | T4 | generated | 635202435 | skipped | 0 | - | not read: 635202435 bytes exceeds --max-bytes 33554432 |
| generated-text_t1 | T1 | generated | 887035 | unsupported | 25 | 18661376 | unsupported in V1: text (<text> at byte 944) |
| generated-text_t2 | T2 | generated | 9024701 | limit | 12 | 16744448 | document exceeds a safety limit |
| generated-text_t3 | T3 | generated | 90488308 | skipped | 0 | - | not read: 90488308 bytes exceeds --max-bytes 33554432 |
| generated-use_t1 | T1 | generated | 986323 | limit | 6767 | 60964864 | document exceeds a safety limit |
| generated-use_t2 | T2 | generated | 10036526 | limit | 12 | 17760256 | document exceeds a safety limit |
| generated-use_t3 | T3 | generated | 100542256 | skipped | 0 | - | not read: 100542256 bytes exceeds --max-bytes 33554432 |
| generated-use_t4 | T4 | generated | 1005597835 | skipped | 0 | - | not read: 1005597835 bytes exceeds --max-bytes 33554432 |
| commons-1-42-polytope-7-cube-svg | T3 | real | 29836045 | limit | 12 | 37568512 | document exceeds a safety limit |
| commons-2015-louisiana-gubernatorial-election-runoff-by-precinct-svg | T2 | real | 16162430 | limit | 12 | 23887872 | document exceeds a safety limit |
| commons-aegean-sea-anatolia-and-armenian-highlands-regions-large-topographic-basemap-svg | T3 | real | 22666147 | limit | 12 | 30392320 | document exceeds a safety limit |
| commons-amhara-topographic-map-svg | T2 | real | 15593284 | limit | 12 | 23298048 | document exceeds a safety limit |
| commons-bulgaria-in-1915-geographic-map-en-svg | T2 | real | 15165804 | limit | 11 | 22904832 | document exceeds a safety limit |
| commons-cambodia-geographic-map-en-svg | T3 | real | 20142590 | limit | 12 | 27869184 | document exceeds a safety limit |
| commons-chuaattractormodified-svg | T3 | real | 20763720 | limit | 12 | 28491776 | document exceeds a safety limit |
| commons-coat-of-arms-of-the-kingdom-of-naples-1808-svg | T3 | real | 25708531 | limit | 12 | 33439744 | document exceeds a safety limit |
| commons-coat-of-arms-of-villa-alemana-svg | T3 | real | 42628426 | skipped | 0 | - | not read: 42628426 bytes exceeds --max-bytes 33554432 |
| commons-columbiarivermap-svg | T2 | real | 16687292 | limit | 12 | 24412160 | document exceeds a safety limit |
| commons-comunas-occitanas-d-alemanha-svg | T3 | real | 43507208 | skipped | 0 | - | not read: 43507208 bytes exceeds --max-bytes 33554432 |
| commons-corsica-geographic-map-svg | T2 | real | 14927948 | limit | 12 | 22659072 | document exceeds a safety limit |
| commons-corvuscoraxiucn-svg | T3 | real | 26496549 | limit | 12 | 34226176 | document exceeds a safety limit |
| commons-dibujo-piu-svg | T3 | real | 87755815 | skipped | 0 | - | not read: 87755815 bytes exceeds --max-bytes 33554432 |
| commons-dukesofhohenbergarms-svg | T3 | real | 28075813 | limit | 11 | 35799040 | document exceeds a safety limit |
| commons-france-osm-mapnik-svg | T2 | real | 19491319 | limit | 11 | 27213824 | document exceeds a safety limit |
| commons-gaussian-2d-svg | T2 | real | 16746052 | limit | 12 | 24477696 | document exceeds a safety limit |
| commons-geographic-map-of-norway-sweden-finland-and-denmark-svg | T3 | real | 20101913 | limit | 12 | 27820032 | document exceeds a safety limit |
| commons-germany-districts-municipalities-location-map-2013-svg | T3 | real | 26486961 | limit | 12 | 34209792 | document exceeds a safety limit |
| commons-ideogram-human-chromosome-20-svg | T3 | real | 21285516 | limit | 12 | 29016064 | document exceeds a safety limit |
| commons-koppen-geiger-map-a-present-svg | T3 | real | 79058833 | skipped | 0 | - | not read: 79058833 bytes exceeds --max-bytes 33554432 |
| commons-municipalities-of-italy-svg | T3 | real | 91915099 | skipped | 0 | - | not read: 91915099 bytes exceeds --max-bytes 33554432 |
| commons-spain-languages-de-svg | T3 | real | 90812529 | skipped | 0 | - | not read: 90812529 bytes exceeds --max-bytes 33554432 |
| natural-earth-world-map-blank | T1 | real | 241682 | unsupported | 81 | 17121280 | unsupported in V1: clip path |
| natural-earth-world-map-countries | T1 | real | 260823 | unsupported | 83 | 16416768 | unsupported in V1: text (<text> at byte 241676) |
## Comparison with the legacy C baselines

Sources (sha256 of the compared files):

- legacy 3/39 (tag legacy-c-final artifacts/WO-M4-10/render-diagnostics.json; copy baselines/WO-M4-10-render-diagnostics.json): sha256 `6f58c23965767aef6361c32cc1c56a8d8114d14fb6ac4ed7ddac6cf1627c4321`
- legacy 5/39 candidate (WO-CORPUS-04 render-diagnostics.json; copy baselines/WO-CORPUS-04-render-diagnostics.json): sha256 `3bffb27483b9c2f0dff067029267d3cc17c388395e6e04eeafb0b3f22b8bab27`
- current Rust run: sha256 `0192c0d25a9b7b854dda35c7eb661368cf78efcfdff526fdba8305c582e4e588`

| category | legacy 3/39 (tag legacy-c-final artifacts/WO-M4-10/render-diagnostics.json; copy baselines/WO-M4-10-render-diagnostics.json) | legacy 5/39 candidate (WO-CORPUS-04 render-diagnostics.json; copy baselines/WO-CORPUS-04-render-diagnostics.json) | current Rust run |
|---|---|---|---|
| ok | 3 | 5 | 1 |
| unsupported | 13 | 11 | 7 |
| limit | 13 | 15 | 28 |
| xml | 8 | 4 | 2 |
| timeout | 0 | 0 | 1 |
| skipped | 19 | 19 | 19 |
| viewport (legacy only; maps to `xml`) | 2 | 2 | 0 |
| path (legacy only; maps to `xml`) | 0 | 2 | 0 |
| **ok / attempted** | 3 / 39 | 5 / 39 | 1 / 39 |

Legacy `path` and `viewport` have no Rust verdict of that name. In the tables above they stay separate rows (never merged). For the per-file deltas below they are compared as `xml` (Rust reports malformed path data and viewport as `Malformed`, verdict `xml`); every such pair is printed with its raw legacy category.

### Per-file deltas vs legacy 3/39 (tag legacy-c-final artifacts/WO-M4-10/render-diagnostics.json; copy baselines/WO-M4-10-render-diagnostics.json)

New ok: 1

- `generated-giant_path_t1`: limit -> ok

Regressions: 3

- `generated-cnc_t1`: ok -> timeout
- `generated-embedded_t1`: ok -> unsupported
- `commons-2015-louisiana-gubernatorial-election-runoff-by-precinct-svg`: ok -> limit

Changed category: 22

- `generated-cad_t1`: limit -> unsupported
- `generated-inkscape_t2`: unsupported -> limit
- `generated-map_t1`: unsupported -> xml
- `generated-map_t2`: unsupported -> limit
- `generated-nested_t2`: xml -> limit
- `generated-science_t1`: limit -> unsupported
- `generated-text_t1`: limit -> unsupported
- `commons-1-42-polytope-7-cube-svg`: xml -> limit
- `commons-aegean-sea-anatolia-and-armenian-highlands-regions-large-topographic-basemap-svg`: viewport -> limit
- `commons-amhara-topographic-map-svg`: unsupported -> limit
- `commons-bulgaria-in-1915-geographic-map-en-svg`: unsupported -> limit
- `commons-cambodia-geographic-map-en-svg`: unsupported -> limit
- `commons-chuaattractormodified-svg`: xml -> limit
- `commons-coat-of-arms-of-the-kingdom-of-naples-1808-svg`: unsupported -> limit
- `commons-columbiarivermap-svg`: xml -> limit
- `commons-corsica-geographic-map-svg`: unsupported -> limit
- `commons-corvuscoraxiucn-svg`: unsupported -> limit
- `commons-dukesofhohenbergarms-svg`: unsupported -> limit
- `commons-france-osm-mapnik-svg`: xml -> limit
- `commons-gaussian-2d-svg`: xml -> limit
- `commons-geographic-map-of-norway-sweden-finland-and-denmark-svg`: viewport -> limit
- `commons-germany-districts-municipalities-location-map-2013-svg`: xml -> limit

Unchanged (including same-after-mapping): 32. Ids not in current report: 0.

### Per-file deltas vs legacy 5/39 candidate (WO-CORPUS-04 render-diagnostics.json; copy baselines/WO-CORPUS-04-render-diagnostics.json)

New ok: 1

- `generated-giant_path_t1`: limit -> ok

Regressions: 5

- `generated-cnc_t1`: ok -> timeout
- `generated-embedded_t1`: ok -> unsupported
- `generated-embedded_t2`: ok -> limit
- `generated-science_t1`: ok -> unsupported
- `commons-2015-louisiana-gubernatorial-election-runoff-by-precinct-svg`: ok -> limit

Changed category: 18

- `generated-cad_t1`: limit -> unsupported
- `generated-inkscape_t2`: unsupported -> limit
- `generated-map_t1`: limit -> xml
- `generated-nested_t2`: xml -> limit
- `generated-text_t1`: limit -> unsupported
- `commons-1-42-polytope-7-cube-svg`: xml -> limit
- `commons-aegean-sea-anatolia-and-armenian-highlands-regions-large-topographic-basemap-svg`: viewport -> limit
- `commons-amhara-topographic-map-svg`: unsupported -> limit
- `commons-bulgaria-in-1915-geographic-map-en-svg`: unsupported -> limit
- `commons-cambodia-geographic-map-en-svg`: unsupported -> limit
- `commons-coat-of-arms-of-the-kingdom-of-naples-1808-svg`: unsupported -> limit
- `commons-columbiarivermap-svg`: xml -> limit
- `commons-corsica-geographic-map-svg`: unsupported -> limit
- `commons-dukesofhohenbergarms-svg`: unsupported -> limit
- `commons-france-osm-mapnik-svg`: path -> limit
- `commons-gaussian-2d-svg`: unsupported -> limit
- `commons-geographic-map-of-norway-sweden-finland-and-denmark-svg`: viewport -> limit
- `commons-germany-districts-municipalities-location-map-2013-svg`: path -> limit

Unchanged (including same-after-mapping): 34. Ids not in current report: 0.


## Notes (coordinator, from independent review)

- `limit` is not split by cause in this report: 26 of the 28 sources exceed the 8 MiB decode cap and were rejected before any rendering; only `generated-giant_path_t2` and `generated-use_t1` are renderer limits. Read "limit 28 vs 13 legacy" accordingly.
- Baseline note: the committed WO-M4-10 diagnostics give 3 ok / 13 unsupported / 13 limit / 8 xml / 2 viewport; the 17/2/11/2/4 breakdown quoted by the order belongs to WO-CORPUS-01 and has no readable per-file source, so it is not in the comparison table.
