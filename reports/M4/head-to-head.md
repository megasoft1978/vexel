# Real-corpus head-to-head

Corpus: `/Users/megasoft78/Desktop/Freelance/vexel-corpus-real` (58 files), manifest SHA-256 `9fbf9a2f384d70a41da72e99ac259d2c5d137ffd3c8a01ce4be4741c1c29bc30`.
Baseline: Inkscape 1.4.4, SHA-256 `3bb506d86b65726e2efc4bedd725e09ed2d180caf661351e655c0c797edfc3ba`. Host: macOS 27.0, Apple M5 Max / arm64.

This measures open/index and first supported preview. It does not measure full-render parity. Inkscape rows use the supplied CLI baseline protocol, which differs by tier and is not an identical task.

## Per-file results

| ID | Tier | Bytes | Outcome | Vexel first preview (ms) | Full index (ms) | Max step (ms) | Vexel peak RSS (bytes) | Inkscape seconds | Inkscape peak RSS (bytes) | Trials |
|---|---:|---:|---|---:|---:|---:|---:|---:|---:|---|
| `generated-cad_t1` | T1 | 997801 | unsupported | — | 6.234 | 0.110 | 15106048 | 1.810 (PASS) | 461029376 | unsupported,unsupported,unsupported |
| `generated-cad_t2` | T2 | 10592198 | unsupported | — | 63.079 | 0.218 | 120979456 | 5.204 (PASS) | 3414999040 | unsupported,unsupported,unsupported |
| `generated-cad_t3` | T3 | 109719583 | unsupported | — | — | 0.300 | 270385152 | 86.352 (PASS) | 6987595776 | unsupported |
| `generated-cnc_t1` | T1 | 990772 | unsupported | — | 5.599 | 0.100 | 12582912 | 1.135 (PASS) | 459276288 | unsupported,unsupported,unsupported |
| `generated-cnc_t2` | T2 | 10162188 | unsupported | — | 50.699 | 0.217 | 93847552 | 5.099 (PASS) | 3554623488 | unsupported,unsupported,unsupported |
| `generated-cnc_t3` | T3 | 102580014 | unsupported | — | — | 0.188 | 296828928 | 120.422 (PASS) | 11081990144 | unsupported |
| `generated-embedded_t1` | T1 | 912235 | unsupported | — | 4.517 | 0.076 | 8044544 | 0.469 (PASS) | 201703424 | unsupported,unsupported,unsupported |
| `generated-embedded_t2` | T2 | 9371925 | unsupported | — | 42.050 | 0.188 | 50020352 | 2.207 (PASS) | 764624896 | unsupported,unsupported,unsupported |
| `generated-embedded_t3` | T3 | 93970295 | unsupported | — | — | 0.233 | 226000896 | 105.567 (PASS) | 7058604032 | unsupported |
| `generated-giant_path_t1` | T1 | 920236 | unsupported | — | 3.321 | 0.061 | 2981888 | 0.721 (PASS) | 171704320 | unsupported,unsupported,unsupported |
| `generated-giant_path_t2` | T2 | 8340030 | unsupported | — | 27.015 | 0.062 | 10403840 | 8.389 (PASS) | 429801472 | unsupported,unsupported,unsupported |
| `generated-giant_path_t3` | T3 | 91963466 | unsupported | — | 286.913 | 0.078 | 94027776 | 7.825 (PASS) | 6721667072 | unsupported |
| `generated-giant_path_t4` | T4 | 827663466 | unsupported | — | 2698.387 | 0.176 | 829734912 | 150.276 (PASS) | 13627867136 | unsupported |
| `generated-inkscape_t1` | T1 | 979796 | unsupported | — | 5.562 | 0.228 | 13615104 | 58.114 (PASS) | 585383936 | unsupported,unsupported,unsupported |
| `generated-inkscape_t2` | T2 | 10062253 | unsupported | — | 51.634 | 0.203 | 102596608 | 180.212 (FAIL / TIMEOUT) | 3929587712 | unsupported,unsupported,unsupported |
| `generated-inkscape_t3` | T3 | 100883678 | unsupported | — | — | 0.193 | 295157760 | 302.047 (FAIL / TIMEOUT) | — | unsupported |
| `generated-map_t1` | T1 | 1310972 | unsupported | — | 5.114 | 0.064 | 4571136 | 0.706 (PASS) | 179191808 | unsupported,unsupported,unsupported |
| `generated-map_t2` | T2 | 14191241 | unsupported | — | 50.658 | 0.071 | 29212672 | 1.200 (PASS) | 491094016 | unsupported,unsupported,unsupported |
| `generated-map_t3` | T3 | 150627424 | unsupported | — | 492.788 | 0.163 | 267468800 | 11.323 (PASS) | 3491594240 | unsupported |
| `generated-nested_t1` | T1 | 978645 | unsupported | — | — | 0.069 | 3194880 | 133.647 (PASS) | 372523008 | unsupported,unsupported,unsupported |
| `generated-nested_t2` | T2 | 9430992 | unsupported | — | — | 0.083 | 11649024 | 180.061 (FAIL / TIMEOUT) | 672284672 | unsupported,unsupported,unsupported |
| `generated-nested_t3` | T3 | 93956332 | unsupported | — | — | 0.072 | 96157696 | 300.085 (FAIL / TIMEOUT) | — | unsupported |
| `generated-science_t1` | T1 | 1374397 | unsupported | — | 6.165 | 0.099 | 9879552 | 0.802 (PASS) | 298762240 | unsupported,unsupported,unsupported |
| `generated-science_t2` | T2 | 14116675 | unsupported | — | 57.454 | 0.203 | 66895872 | 4.967 (PASS) | 1648328704 | unsupported,unsupported,unsupported |
| `generated-science_t3` | T3 | 117860153 | unsupported | — | — | 13.735 | 270155776 | 30.565 (PASS) | 8231960576 | unsupported |
| `generated-science_t4` | T4 | 635202435 | unsupported | — | — | 0.228 | 785924096 | 302.239 (FAIL / TIMEOUT) | — | unsupported |
| `generated-text_t1` | T1 | 887035 | unsupported | — | 5.233 | 0.080 | 10338304 | 2.000 (PASS) | 797786112 | unsupported,unsupported,unsupported |
| `generated-text_t2` | T2 | 9024701 | unsupported | — | 47.227 | 0.196 | 72794112 | 16.074 (PASS) | 6822363136 | unsupported,unsupported,unsupported |
| `generated-text_t3` | T3 | 90488308 | unsupported | — | — | 0.172 | 256720896 | 60.228 (PASS) | 14394998784 | unsupported |
| `generated-use_t1` | T1 | 986323 | unsupported | — | 6.288 | 0.094 | 15826944 | 1.043 (PASS) | 583909376 | unsupported,unsupported,unsupported |
| `generated-use_t2` | T2 | 10036526 | unsupported | — | 66.020 | 0.210 | 128532480 | 7.405 (PASS) | 4728373248 | unsupported,unsupported,unsupported |
| `generated-use_t3` | T3 | 100542256 | unsupported | — | — | 18.516 | 232587264 | 111.468 (PASS) | 10144759808 | unsupported |
| `generated-use_t4` | T4 | 1005597835 | unsupported | — | — | 17.220 | 1136066560 | 40.961 (FAIL / TIMEOUT) | 14546288640 | unsupported |
| `commons-1-42-polytope-7-cube-svg` | T3 | 29836045 | unsupported | — | — | 0.204 | 182091776 | 7.604 (PASS) | 4396826624 | unsupported |
| `commons-2015-louisiana-gubernatorial-election-runoff-by-precinct-svg` | T2 | 16162430 | unsupported | — | 56.275 | 0.103 | 25165824 | 1.106 (PASS) | 374915072 | unsupported,unsupported,unsupported |
| `commons-aegean-sea-anatolia-and-armenian-highlands-regions-large-topographic-basemap-svg` | T3 | 22666147 | unsupported | — | 78.548 | 0.181 | 45613056 | 1.740 (PASS) | 768491520 | unsupported |
| `commons-amhara-topographic-map-svg` | T2 | 15593284 | unsupported | — | 51.608 | 0.072 | 18481152 | 1.017 (PASS) | 297582592 | unsupported,unsupported,unsupported |
| `commons-bulgaria-in-1915-geographic-map-en-svg` | T2 | 15165804 | unsupported | — | 58.203 | 0.087 | 41156608 | 1.788 (PASS) | 610844672 | unsupported,unsupported,unsupported |
| `commons-cambodia-geographic-map-en-svg` | T3 | 20142590 | unsupported | — | 77.243 | 0.199 | 61636608 | 2.838 (PASS) | 931348480 | unsupported |
| `commons-chuaattractormodified-svg` | T3 | 20763720 | unsupported | — | — | 0.019 | 22806528 | 5.210 (PASS) | 2032254976 | unsupported |
| `commons-coat-of-arms-of-the-kingdom-of-naples-1808-svg` | T3 | 25708531 | unsupported | — | 93.887 | 0.102 | 47300608 | 1.686 (PASS) | 458997760 | unsupported |
| `commons-coat-of-arms-of-villa-alemana-svg` | T3 | 42628426 | unsupported | — | 143.863 | 0.083 | 56999936 | 5.606 (PASS) | 523485184 | unsupported |
| `commons-columbiarivermap-svg` | T2 | 16687292 | unsupported | — | — | 0.057 | 18743296 | 0.773 (PASS) | 361447424 | unsupported,unsupported,unsupported |
| `commons-comunas-occitanas-d-alemanha-svg` | T3 | 43507208 | unsupported | — | 138.464 | 0.083 | 55984128 | 3.748 (PASS) | 799064064 | unsupported |
| `commons-corsica-geographic-map-svg` | T2 | 14927948 | unsupported | — | 50.487 | 0.072 | 30031872 | 1.938 (PASS) | 516685824 | unsupported,unsupported,unsupported |
| `commons-corvuscoraxiucn-svg` | T3 | 26496549 | unsupported | — | 85.807 | 0.140 | 45891584 | 2.142 (PASS) | 696352768 | unsupported |
| `commons-dibujo-piu-svg` | T3 | 87755815 | unsupported | — | 307.507 | 0.189 | 174243840 | 13.365 (PASS) | 1827225600 | unsupported |
| `commons-dukesofhohenbergarms-svg` | T3 | 28075813 | unsupported | — | 95.726 | 0.212 | 53198848 | 2.876 (PASS) | 748142592 | unsupported |
| `commons-france-osm-mapnik-svg` | T2 | 19491319 | unsupported | — | — | 0.014 | 21528576 | 2.552 (PASS) | 521846784 | unsupported,unsupported,unsupported |
| `commons-gaussian-2d-svg` | T2 | 16746052 | unsupported | — | — | 0.017 | 18792448 | 1.556 (PASS) | 510377984 | unsupported,unsupported,unsupported |
| `commons-geographic-map-of-norway-sweden-finland-and-denmark-svg` | T3 | 20101913 | unsupported | — | 72.880 | 0.179 | 57786368 | 3.405 (PASS) | 1373749248 | unsupported |
| `commons-germany-districts-municipalities-location-map-2013-svg` | T3 | 26486961 | unsupported | — | — | 0.036 | 28524544 | 3.675 (PASS) | 765837312 | unsupported |
| `commons-ideogram-human-chromosome-20-svg` | T3 | 21285516 | unsupported | — | 90.278 | 0.276 | 103792640 | 1.971 (PASS) | 1239302144 | unsupported |
| `commons-koppen-geiger-map-a-present-svg` | T3 | 79058833 | unsupported | — | 267.544 | 0.233 | 146030592 | 6.839 (PASS) | 1859502080 | unsupported |
| `commons-municipalities-of-italy-svg` | T3 | 91915099 | unsupported | — | 302.053 | 0.084 | 100630528 | 5.145 (PASS) | 917422080 | unsupported |
| `commons-spain-languages-de-svg` | T3 | 90812529 | unsupported | — | 290.108 | 0.105 | 112082944 | 5.409 (PASS) | 1168162816 | unsupported |
| `natural-earth-world-map-blank` | T1 | 241682 | ok | 9.688 | 0.999 | 0.054 | 4653056 | 0.347 (PASS) | 150437888 | ok,ok,ok |
| `natural-earth-world-map-countries` | T1 | 260823 | ok | 9.525 | 1.106 | 0.063 | 4734976 | 0.409 (PASS) | 157794304 | ok,ok,ok |

## Tier summary

Speedup is Inkscape supplied probe seconds divided by Vexel first-preview seconds; RSS ratio is Inkscape peak RSS divided by Vexel peak RSS. Both geometric means include only successful Vexel outcomes and PASS Inkscape rows. Protocols differ, so these are directional workload comparisons.

| Tier | Files | OK | Unsupported | Limit | Crash/timeout | Comparable files | Geomean time ratio | Geomean RSS ratio |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| T1 | 12 | 2 | 10 | 0 | 0 | 2 | 39.217x | 32.824x (2 files) |
| T2 | 17 | 0 | 17 | 0 | 0 | 0 | — | — (0 files) |
| T3 | 26 | 0 | 26 | 0 | 0 | 0 | — | — (0 files) |
| T4 | 3 | 0 | 3 | 0 | 0 | 0 | — | — (0 files) |

## Wedge checks

- 100 MiB first visible preview under 2 seconds: **fail_no_preview**; measured 0 of 12 T3 files in the 80–120 MiB band; maximum None ns.
- Maximum main-thread index step under 100 ms: **pass**; maximum 18515542 ns. Pre-index preview occurred in 0 eligible files.
- 100 MiB-band RSS allowance (3× source bytes + 128 MiB): **pass** across 12 eligible files; measured peak RSS appears in the JSON and per-file table.
- Inkscape RSS can exceed Vexel's indexed input footprint substantially; this comparison is not a memory parity claim.

## Failures and limits

- `generated-cad_t1`: unsupported (status=6 diagnostic=0@0 exit=1; status=6 diagnostic=0@0 exit=1; status=6 diagnostic=0@0 exit=1).
- `generated-cad_t2`: unsupported (status=6 diagnostic=0@0 exit=1; status=6 diagnostic=0@0 exit=1; status=6 diagnostic=0@0 exit=1).
- `generated-cad_t3`: unsupported (status=4 diagnostic=14@16785356 exit=1).
- `generated-cnc_t1`: unsupported (status=6 diagnostic=0@0 exit=1; status=6 diagnostic=0@0 exit=1; status=6 diagnostic=0@0 exit=1).
- `generated-cnc_t2`: unsupported (status=6 diagnostic=0@0 exit=1; status=6 diagnostic=0@0 exit=1; status=6 diagnostic=0@0 exit=1).
- `generated-cnc_t3`: unsupported (status=4 diagnostic=14@28094636 exit=1).
- `generated-embedded_t1`: unsupported (status=6 diagnostic=0@0 exit=1; status=6 diagnostic=0@0 exit=1; status=6 diagnostic=0@0 exit=1).
- `generated-embedded_t2`: unsupported (status=6 diagnostic=0@0 exit=1; status=6 diagnostic=0@0 exit=1; status=6 diagnostic=0@0 exit=1).
- `generated-embedded_t3`: unsupported (status=4 diagnostic=12@33839300 exit=1).
- `generated-giant_path_t1`: unsupported (status=6 diagnostic=0@0 exit=1; status=6 diagnostic=0@0 exit=1; status=6 diagnostic=0@0 exit=1).
- `generated-giant_path_t2`: unsupported (status=6 diagnostic=0@0 exit=1; status=6 diagnostic=0@0 exit=1; status=6 diagnostic=0@0 exit=1).
- `generated-giant_path_t3`: unsupported (status=6 diagnostic=0@0 exit=1).
- `generated-giant_path_t4`: unsupported (status=6 diagnostic=0@0 exit=1).
- `generated-inkscape_t1`: unsupported (status=6 diagnostic=0@0 exit=1; status=6 diagnostic=0@0 exit=1; status=6 diagnostic=0@0 exit=1).
- `generated-inkscape_t2`: unsupported (status=6 diagnostic=0@0 exit=1; status=6 diagnostic=0@0 exit=1; status=6 diagnostic=0@0 exit=1).
- `generated-inkscape_t3`: unsupported (status=4 diagnostic=14@19710664 exit=1).
- `generated-map_t1`: unsupported (status=6 diagnostic=0@0 exit=1; status=6 diagnostic=0@0 exit=1; status=6 diagnostic=0@0 exit=1).
- `generated-map_t2`: unsupported (status=6 diagnostic=0@0 exit=1; status=6 diagnostic=0@0 exit=1; status=6 diagnostic=0@0 exit=1).
- `generated-map_t3`: unsupported (status=6 diagnostic=0@0 exit=1).
- `generated-nested_t1`: unsupported (status=4 diagnostic=10@16843 exit=1; status=4 diagnostic=10@16843 exit=1; status=4 diagnostic=10@16843 exit=1).
- `generated-nested_t2`: unsupported (status=4 diagnostic=10@16843 exit=1; status=4 diagnostic=10@16843 exit=1; status=4 diagnostic=10@16843 exit=1).
- `generated-nested_t3`: unsupported (status=4 diagnostic=10@16843 exit=1).
- `generated-science_t1`: unsupported (status=6 diagnostic=0@0 exit=1; status=6 diagnostic=0@0 exit=1; status=6 diagnostic=0@0 exit=1).
- `generated-science_t2`: unsupported (status=6 diagnostic=0@0 exit=1; status=6 diagnostic=0@0 exit=1; status=6 diagnostic=0@0 exit=1).
- `generated-science_t3`: unsupported (status=4 diagnostic=12@13776363 exit=1).
- `generated-science_t4`: unsupported (status=4 diagnostic=12@13776298 exit=1).
- `generated-text_t1`: unsupported (status=6 diagnostic=0@0 exit=1; status=6 diagnostic=0@0 exit=1; status=6 diagnostic=0@0 exit=1).
- `generated-text_t2`: unsupported (status=6 diagnostic=0@0 exit=1; status=6 diagnostic=0@0 exit=1; status=6 diagnostic=0@0 exit=1).
- `generated-text_t3`: unsupported (status=4 diagnostic=14@23260205 exit=1).
- `generated-use_t1`: unsupported (status=6 diagnostic=0@0 exit=1; status=6 diagnostic=0@0 exit=1; status=6 diagnostic=0@0 exit=1).
- `generated-use_t2`: unsupported (status=6 diagnostic=0@0 exit=1; status=6 diagnostic=0@0 exit=1; status=6 diagnostic=0@0 exit=1).
- `generated-use_t3`: unsupported (status=4 diagnostic=12@11239856 exit=1).
- `generated-use_t4`: unsupported (status=4 diagnostic=12@11239856 exit=1).
- `commons-1-42-polytope-7-cube-svg`: unsupported (status=4 diagnostic=14@14206027 exit=1).
- `commons-2015-louisiana-gubernatorial-election-runoff-by-precinct-svg`: unsupported (status=6 diagnostic=0@0 exit=1; status=6 diagnostic=0@0 exit=1; status=6 diagnostic=0@0 exit=1).
- `commons-aegean-sea-anatolia-and-armenian-highlands-regions-large-topographic-basemap-svg`: unsupported (status=6 diagnostic=0@0 exit=1).
- `commons-amhara-topographic-map-svg`: unsupported (status=6 diagnostic=0@0 exit=1; status=6 diagnostic=0@0 exit=1; status=6 diagnostic=0@0 exit=1).
- `commons-bulgaria-in-1915-geographic-map-en-svg`: unsupported (status=6 diagnostic=0@0 exit=1; status=6 diagnostic=0@0 exit=1; status=6 diagnostic=0@0 exit=1).
- `commons-cambodia-geographic-map-en-svg`: unsupported (status=6 diagnostic=0@0 exit=1).
- `commons-chuaattractormodified-svg`: unsupported (status=4 diagnostic=15@56 exit=1).
- `commons-coat-of-arms-of-the-kingdom-of-naples-1808-svg`: unsupported (status=6 diagnostic=0@0 exit=1).
- `commons-coat-of-arms-of-villa-alemana-svg`: unsupported (status=6 diagnostic=0@0 exit=1).
- `commons-columbiarivermap-svg`: unsupported (status=4 diagnostic=15@136 exit=1; status=4 diagnostic=15@136 exit=1; status=4 diagnostic=15@136 exit=1).
- `commons-comunas-occitanas-d-alemanha-svg`: unsupported (status=6 diagnostic=0@0 exit=1).
- `commons-corsica-geographic-map-svg`: unsupported (status=6 diagnostic=0@0 exit=1; status=6 diagnostic=0@0 exit=1; status=6 diagnostic=0@0 exit=1).
- `commons-corvuscoraxiucn-svg`: unsupported (status=6 diagnostic=0@0 exit=1).
- `commons-dibujo-piu-svg`: unsupported (status=6 diagnostic=0@0 exit=1).
- `commons-dukesofhohenbergarms-svg`: unsupported (status=6 diagnostic=0@0 exit=1).
- `commons-france-osm-mapnik-svg`: unsupported (status=4 diagnostic=15@39 exit=1; status=4 diagnostic=15@39 exit=1; status=4 diagnostic=15@39 exit=1).
- `commons-gaussian-2d-svg`: unsupported (status=4 diagnostic=15@56 exit=1; status=4 diagnostic=15@56 exit=1; status=4 diagnostic=15@56 exit=1).
- `commons-geographic-map-of-norway-sweden-finland-and-denmark-svg`: unsupported (status=6 diagnostic=0@0 exit=1).
- `commons-germany-districts-municipalities-location-map-2013-svg`: unsupported (status=4 diagnostic=15@39 exit=1).
- `commons-ideogram-human-chromosome-20-svg`: unsupported (status=6 diagnostic=0@0 exit=1).
- `commons-koppen-geiger-map-a-present-svg`: unsupported (status=6 diagnostic=0@0 exit=1).
- `commons-municipalities-of-italy-svg`: unsupported (status=6 diagnostic=0@0 exit=1).
- `commons-spain-languages-de-svg`: unsupported (status=6 diagnostic=0@0 exit=1).

## Reproducibility

Release binary SHA-256: `0b22776da01bd1241b0170a45f5b79597bbf57d8edb858cf8ed29f0799e9ade5`. Fresh processes, 3 repetitions for T1/T2, 1 for T3/T4, each with a 120s timeout. External source files remain at the referenced corpus path; no corpus file is stored in this repository.
