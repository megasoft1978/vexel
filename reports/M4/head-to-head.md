# Real-corpus head-to-head

Corpus: `/Users/megasoft78/Desktop/Freelance/vexel-corpus-real` (58 files), manifest SHA-256 `9fbf9a2f384d70a41da72e99ac259d2c5d137ffd3c8a01ce4be4741c1c29bc30`.
Baseline: Inkscape 1.4.4, SHA-256 `3bb506d86b65726e2efc4bedd725e09ed2d180caf661351e655c0c797edfc3ba`. Host: macOS 27.0, Apple M5 Max / arm64.

This measures open/index and first supported preview. It does not measure full-render parity. Inkscape rows use the supplied CLI baseline protocol, which differs by tier and is not an identical task.

## Per-file results

| ID | Tier | Bytes | Outcome | Vexel first preview (ms) | Full index (ms) | Max step (ms) | Vexel peak RSS (bytes) | Inkscape seconds | Inkscape peak RSS (bytes) | Trials |
|---|---:|---:|---|---:|---:|---:|---:|---:|---:|---|
| `generated-cad_t1` | T1 | 997801 | unsupported | — | 5.602 | 0.099 | 15876096 | 1.810 (PASS) | 461029376 | unsupported,unsupported,unsupported |
| `generated-cad_t2` | T2 | 10592198 | unsupported | — | 56.089 | 0.192 | 120963072 | 5.204 (PASS) | 3414999040 | unsupported,unsupported,unsupported |
| `generated-cad_t3` | T3 | 109719583 | unsupported | — | — | 0.182 | 270368768 | 86.352 (PASS) | 6987595776 | unsupported |
| `generated-cnc_t1` | T1 | 990772 | unsupported | — | 4.617 | 0.084 | 13336576 | 1.135 (PASS) | 459276288 | unsupported,unsupported,unsupported |
| `generated-cnc_t2` | T2 | 10162188 | unsupported | — | 45.177 | 0.172 | 93847552 | 5.099 (PASS) | 3554623488 | unsupported,unsupported,unsupported |
| `generated-cnc_t3` | T3 | 102580014 | unsupported | — | — | 0.177 | 296812544 | 120.422 (PASS) | 11081990144 | unsupported |
| `generated-embedded_t1` | T1 | 912235 | unsupported | — | 3.792 | 0.068 | 8044544 | 0.469 (PASS) | 201703424 | unsupported,unsupported,unsupported |
| `generated-embedded_t2` | T2 | 9371925 | unsupported | — | 38.411 | 0.161 | 48431104 | 2.207 (PASS) | 764624896 | unsupported,unsupported,unsupported |
| `generated-embedded_t3` | T3 | 93970295 | unsupported | — | — | 0.187 | 224411648 | 105.567 (PASS) | 7058604032 | unsupported |
| `generated-giant_path_t1` | T1 | 920236 | unsupported | — | 2.830 | 0.045 | 2981888 | 0.721 (PASS) | 171704320 | unsupported,unsupported,unsupported |
| `generated-giant_path_t2` | T2 | 8340030 | unsupported | — | 24.190 | 0.053 | 10403840 | 8.389 (PASS) | 429801472 | unsupported,unsupported,unsupported |
| `generated-giant_path_t3` | T3 | 91963466 | unsupported | — | 276.789 | 0.055 | 94027776 | 7.825 (PASS) | 6721667072 | unsupported |
| `generated-giant_path_t4` | T4 | 827663466 | unsupported | — | 2486.861 | 0.193 | 829718528 | 150.276 (PASS) | 13627867136 | unsupported |
| `generated-inkscape_t1` | T1 | 979796 | unsupported | — | 4.578 | 0.172 | 13615104 | 58.114 (PASS) | 585383936 | unsupported,unsupported,unsupported |
| `generated-inkscape_t2` | T2 | 10062253 | unsupported | — | 47.102 | 0.193 | 102596608 | 180.212 (FAIL / TIMEOUT) | 3929587712 | unsupported,unsupported,unsupported |
| `generated-inkscape_t3` | T3 | 100883678 | unsupported | — | — | 0.183 | 295124992 | 302.047 (FAIL / TIMEOUT) | — | unsupported |
| `generated-map_t1` | T1 | 1310972 | unsupported | — | 4.283 | 0.058 | 4571136 | 0.706 (PASS) | 179191808 | unsupported,unsupported,unsupported |
| `generated-map_t2` | T2 | 14191241 | unsupported | — | 44.336 | 0.061 | 29212672 | 1.200 (PASS) | 491094016 | unsupported,unsupported,unsupported |
| `generated-map_t3` | T3 | 150627424 | unsupported | — | 465.701 | 0.167 | 267468800 | 11.323 (PASS) | 3491594240 | unsupported |
| `generated-nested_t1` | T1 | 978645 | unsupported | — | — | 0.075 | 3162112 | 133.647 (PASS) | 372523008 | unsupported,unsupported,unsupported |
| `generated-nested_t2` | T2 | 9430992 | unsupported | — | — | 0.070 | 11632640 | 180.061 (FAIL / TIMEOUT) | 672284672 | unsupported,unsupported,unsupported |
| `generated-nested_t3` | T3 | 93956332 | unsupported | — | — | 0.070 | 96157696 | 300.085 (FAIL / TIMEOUT) | — | unsupported |
| `generated-science_t1` | T1 | 1374397 | unsupported | — | 5.320 | 0.092 | 9846784 | 0.802 (PASS) | 298762240 | unsupported,unsupported,unsupported |
| `generated-science_t2` | T2 | 14116675 | unsupported | — | 53.040 | 0.189 | 66895872 | 4.967 (PASS) | 1648328704 | unsupported,unsupported,unsupported |
| `generated-science_t3` | T3 | 117860153 | unsupported | — | — | 0.182 | 268566528 | 30.565 (PASS) | 8231960576 | unsupported |
| `generated-science_t4` | T4 | 635202435 | unsupported | — | — | 0.181 | 785907712 | 302.239 (FAIL / TIMEOUT) | — | unsupported |
| `generated-text_t1` | T1 | 887035 | unsupported | — | 4.317 | 0.075 | 10338304 | 2.000 (PASS) | 797786112 | unsupported,unsupported,unsupported |
| `generated-text_t2` | T2 | 9024701 | unsupported | — | 41.511 | 0.170 | 71204864 | 16.074 (PASS) | 6822363136 | unsupported,unsupported,unsupported |
| `generated-text_t3` | T3 | 90488308 | unsupported | — | — | 0.213 | 256720896 | 60.228 (PASS) | 14394998784 | unsupported |
| `generated-use_t1` | T1 | 986323 | unsupported | — | 6.040 | 0.091 | 16596992 | 1.043 (PASS) | 583909376 | unsupported,unsupported,unsupported |
| `generated-use_t2` | T2 | 10036526 | unsupported | — | 58.320 | 0.193 | 126943232 | 7.405 (PASS) | 4728373248 | unsupported,unsupported,unsupported |
| `generated-use_t3` | T3 | 100542256 | unsupported | — | — | 0.201 | 230981632 | 111.468 (PASS) | 10144759808 | unsupported |
| `generated-use_t4` | T4 | 1005597835 | unsupported | — | — | 0.177 | 1136033792 | 40.961 (FAIL / TIMEOUT) | 14546288640 | unsupported |
| `commons-1-42-polytope-7-cube-svg` | T3 | 29836045 | unsupported | — | — | 0.163 | 182091776 | 7.604 (PASS) | 4396826624 | unsupported |
| `commons-2015-louisiana-gubernatorial-election-runoff-by-precinct-svg` | T2 | 16162430 | unsupported | — | 49.670 | 0.059 | 25133056 | 1.106 (PASS) | 374915072 | unsupported,unsupported,unsupported |
| `commons-aegean-sea-anatolia-and-armenian-highlands-regions-large-topographic-basemap-svg` | T3 | 22666147 | unsupported | — | 73.512 | 0.158 | 44040192 | 1.740 (PASS) | 768491520 | unsupported |
| `commons-amhara-topographic-map-svg` | T2 | 15593284 | unsupported | — | 47.432 | 0.061 | 18464768 | 1.017 (PASS) | 297582592 | unsupported,unsupported,unsupported |
| `commons-bulgaria-in-1915-geographic-map-en-svg` | T2 | 15165804 | unsupported | — | 52.611 | 0.070 | 41156608 | 1.788 (PASS) | 610844672 | unsupported,unsupported,unsupported |
| `commons-cambodia-geographic-map-en-svg` | T3 | 20142590 | unsupported | — | 71.083 | 0.164 | 60047360 | 2.838 (PASS) | 931348480 | unsupported |
| `commons-chuaattractormodified-svg` | T3 | 20763720 | unsupported | — | — | 0.018 | 22790144 | 5.210 (PASS) | 2032254976 | unsupported |
| `commons-coat-of-arms-of-the-kingdom-of-naples-1808-svg` | T3 | 25708531 | unsupported | — | 81.987 | 0.088 | 47300608 | 1.686 (PASS) | 458997760 | unsupported |
| `commons-coat-of-arms-of-villa-alemana-svg` | T3 | 42628426 | unsupported | — | 126.455 | 0.078 | 56999936 | 5.606 (PASS) | 523485184 | unsupported |
| `commons-columbiarivermap-svg` | T2 | 16687292 | unsupported | — | — | 0.014 | 18726912 | 0.773 (PASS) | 361447424 | unsupported,unsupported,unsupported |
| `commons-comunas-occitanas-d-alemanha-svg` | T3 | 43507208 | unsupported | — | 136.555 | 0.059 | 55967744 | 3.748 (PASS) | 799064064 | unsupported |
| `commons-corsica-geographic-map-svg` | T2 | 14927948 | unsupported | — | 48.129 | 0.063 | 29999104 | 1.938 (PASS) | 516685824 | unsupported,unsupported,unsupported |
| `commons-corvuscoraxiucn-svg` | T3 | 26496549 | unsupported | — | 83.708 | 0.063 | 45875200 | 2.142 (PASS) | 696352768 | unsupported |
| `commons-dibujo-piu-svg` | T3 | 87755815 | unsupported | — | 297.522 | 0.170 | 174211072 | 13.365 (PASS) | 1827225600 | unsupported |
| `commons-dukesofhohenbergarms-svg` | T3 | 28075813 | unsupported | — | 87.408 | 0.194 | 53198848 | 2.876 (PASS) | 748142592 | unsupported |
| `commons-france-osm-mapnik-svg` | T2 | 19491319 | unsupported | — | — | 0.015 | 21528576 | 2.552 (PASS) | 521846784 | unsupported,unsupported,unsupported |
| `commons-gaussian-2d-svg` | T2 | 16746052 | unsupported | — | — | 0.016 | 18792448 | 1.556 (PASS) | 510377984 | unsupported,unsupported,unsupported |
| `commons-geographic-map-of-norway-sweden-finland-and-denmark-svg` | T3 | 20101913 | unsupported | — | 67.101 | 0.179 | 56197120 | 3.405 (PASS) | 1373749248 | unsupported |
| `commons-germany-districts-municipalities-location-map-2013-svg` | T3 | 26486961 | unsupported | — | — | 0.016 | 28524544 | 3.675 (PASS) | 765837312 | unsupported |
| `commons-ideogram-human-chromosome-20-svg` | T3 | 21285516 | unsupported | — | 85.801 | 0.163 | 103792640 | 1.971 (PASS) | 1239302144 | unsupported |
| `commons-koppen-geiger-map-a-present-svg` | T3 | 79058833 | unsupported | — | 243.927 | 0.146 | 145997824 | 6.839 (PASS) | 1859502080 | unsupported |
| `commons-municipalities-of-italy-svg` | T3 | 91915099 | unsupported | — | 273.517 | 0.055 | 100630528 | 5.145 (PASS) | 917422080 | unsupported |
| `commons-spain-languages-de-svg` | T3 | 90812529 | unsupported | — | 277.057 | 0.081 | 112066560 | 5.409 (PASS) | 1168162816 | unsupported |
| `natural-earth-world-map-blank` | T1 | 241682 | ok | 8.617 | 0.855 | 0.052 | 4505600 | 0.347 (PASS) | 150437888 | ok,ok,ok |
| `natural-earth-world-map-countries` | T1 | 260823 | ok | 8.884 | 0.996 | 0.059 | 4669440 | 0.409 (PASS) | 157794304 | ok,ok,ok |

## Tier summary

Speedup is Inkscape supplied probe seconds divided by Vexel first-preview seconds; RSS ratio is Inkscape peak RSS divided by Vexel peak RSS. Both geometric means include only successful Vexel outcomes and PASS Inkscape rows. Protocols differ, so these are directional workload comparisons.

| Tier | Files | OK | Unsupported | Limit | Crash/timeout | Comparable files | Geomean time ratio | Geomean RSS ratio |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| T1 | 12 | 2 | 10 | 0 | 0 | 2 | 43.057x | 33.590x (2 files) |
| T2 | 17 | 0 | 17 | 0 | 0 | 0 | — | — (0 files) |
| T3 | 26 | 0 | 26 | 0 | 0 | 0 | — | — (0 files) |
| T4 | 3 | 0 | 3 | 0 | 0 | 0 | — | — (0 files) |

## Wedge checks

- 100 MiB first visible preview under 2 seconds: **fail_no_preview**; measured 0 of 12 T3 files in the 80–120 MiB band; maximum None ns.
- Maximum main-thread index step under 100 ms: **pass**; maximum 212750 ns. Pre-index preview occurred in 0 eligible files.
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

Release binary SHA-256: `f5662477f299143c36d3a6dfb8095d09b2547118ac85d00f5d59aa5743069634`. Fresh processes, 3 repetitions for T1/T2, 1 for T3/T4, each with a 120s timeout. External source files remain at the referenced corpus path; no corpus file is stored in this repository.
