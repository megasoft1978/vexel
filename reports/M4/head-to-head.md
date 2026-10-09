# Real-corpus head-to-head

Corpus: `/Users/megasoft78/Desktop/Freelance/vexel-corpus-real` (58 files), manifest SHA-256 `9fbf9a2f384d70a41da72e99ac259d2c5d137ffd3c8a01ce4be4741c1c29bc30`.
Baseline: Inkscape 1.4.4, SHA-256 `3bb506d86b65726e2efc4bedd725e09ed2d180caf661351e655c0c797edfc3ba`. Host: macOS 27.0, unavailable / arm64.

This measures open/index and first supported preview. It does not measure full-render parity. Inkscape rows use the supplied CLI baseline protocol, which differs by tier and is not an identical task.

## Per-file results

| ID | Tier | Bytes | Outcome | Vexel first preview (ms) | Full index (ms) | Max step (ms) | Vexel peak RSS (bytes) | Inkscape seconds | Inkscape peak RSS (bytes) | Trials |
|---|---:|---:|---|---:|---:|---:|---:|---:|---:|---|
| `generated-cad_t1` | T1 | 997801 | unsupported | — | 5.795 | 0.103 | 15892480 | 1.810 (PASS) | 461029376 | unsupported,unsupported,unsupported |
| `generated-cad_t2` | T2 | 10592198 | unsupported | — | 58.020 | 0.196 | 120979456 | 5.204 (PASS) | 3414999040 | unsupported,unsupported,unsupported |
| `generated-cad_t3` | T3 | 109719583 | unsupported | — | — | 0.182 | 270385152 | 86.352 (PASS) | 6987595776 | unsupported |
| `generated-cnc_t1` | T1 | 990772 | unsupported | — | 4.771 | 0.088 | 13352960 | 1.135 (PASS) | 459276288 | unsupported,unsupported,unsupported |
| `generated-cnc_t2` | T2 | 10162188 | unsupported | — | 47.078 | 0.175 | 93863936 | 5.099 (PASS) | 3554623488 | unsupported,unsupported,unsupported |
| `generated-cnc_t3` | T3 | 102580014 | unsupported | — | — | 0.172 | 296828928 | 120.422 (PASS) | 11081990144 | unsupported |
| `generated-embedded_t1` | T1 | 912235 | unsupported | — | 3.832 | 0.072 | 8060928 | 0.469 (PASS) | 201703424 | unsupported,unsupported,unsupported |
| `generated-embedded_t2` | T2 | 9371925 | unsupported | — | 38.223 | 0.166 | 48447488 | 2.207 (PASS) | 764624896 | unsupported,unsupported,unsupported |
| `generated-embedded_t3` | T3 | 93970295 | unsupported | — | — | 0.184 | 224428032 | 105.567 (PASS) | 7058604032 | unsupported |
| `generated-giant_path_t1` | T1 | 920236 | unsupported | — | 2.924 | 0.053 | 2998272 | 0.721 (PASS) | 171704320 | unsupported,unsupported,unsupported |
| `generated-giant_path_t2` | T2 | 8340030 | unsupported | — | 24.439 | 0.055 | 10420224 | 8.389 (PASS) | 429801472 | unsupported,unsupported,unsupported |
| `generated-giant_path_t3` | T3 | 91963466 | unsupported | — | 267.044 | 0.061 | 94044160 | 7.825 (PASS) | 6721667072 | unsupported |
| `generated-giant_path_t4` | T4 | 827663466 | unsupported | — | 2411.775 | 0.792 | 829734912 | 150.276 (PASS) | 13627867136 | unsupported |
| `generated-inkscape_t1` | T1 | 979796 | unsupported | — | 4.843 | 0.180 | 13631488 | 58.114 (PASS) | 585383936 | unsupported,unsupported,unsupported |
| `generated-inkscape_t2` | T2 | 10062253 | unsupported | — | 48.678 | 0.202 | 102612992 | 180.212 (FAIL / TIMEOUT) | 3929587712 | unsupported,unsupported,unsupported |
| `generated-inkscape_t3` | T3 | 100883678 | unsupported | — | — | 0.211 | 295141376 | 302.047 (FAIL / TIMEOUT) | — | unsupported |
| `generated-map_t1` | T1 | 1310972 | unsupported | — | 4.606 | 0.054 | 4587520 | 0.706 (PASS) | 179191808 | unsupported,unsupported,unsupported |
| `generated-map_t2` | T2 | 14191241 | unsupported | — | 45.464 | 0.059 | 29229056 | 1.200 (PASS) | 491094016 | unsupported,unsupported,unsupported |
| `generated-map_t3` | T3 | 150627424 | unsupported | — | 476.322 | 0.153 | 267485184 | 11.323 (PASS) | 3491594240 | unsupported |
| `generated-nested_t1` | T1 | 978645 | unsupported | — | — | 0.065 | 3178496 | 133.647 (PASS) | 372523008 | unsupported,unsupported,unsupported |
| `generated-nested_t2` | T2 | 9430992 | unsupported | — | — | 0.069 | 11649024 | 180.061 (FAIL / TIMEOUT) | 672284672 | unsupported,unsupported,unsupported |
| `generated-nested_t3` | T3 | 93956332 | unsupported | — | — | 0.073 | 96174080 | 300.085 (FAIL / TIMEOUT) | — | unsupported |
| `generated-science_t1` | T1 | 1374397 | unsupported | — | 5.369 | 0.094 | 9863168 | 0.802 (PASS) | 298762240 | unsupported,unsupported,unsupported |
| `generated-science_t2` | T2 | 14116675 | unsupported | — | 53.348 | 0.193 | 66912256 | 4.967 (PASS) | 1648328704 | unsupported,unsupported,unsupported |
| `generated-science_t3` | T3 | 117860153 | unsupported | — | — | 0.188 | 268582912 | 30.565 (PASS) | 8231960576 | unsupported |
| `generated-science_t4` | T4 | 635202435 | unsupported | — | — | 0.180 | 785924096 | 302.239 (FAIL / TIMEOUT) | — | unsupported |
| `generated-text_t1` | T1 | 887035 | unsupported | — | 4.282 | 0.086 | 10354688 | 2.000 (PASS) | 797786112 | unsupported,unsupported,unsupported |
| `generated-text_t2` | T2 | 9024701 | unsupported | — | 41.580 | 0.171 | 71221248 | 16.074 (PASS) | 6822363136 | unsupported,unsupported,unsupported |
| `generated-text_t3` | T3 | 90488308 | unsupported | — | — | 0.170 | 256737280 | 60.228 (PASS) | 14394998784 | unsupported |
| `generated-use_t1` | T1 | 986323 | unsupported | — | 6.168 | 0.091 | 16613376 | 1.043 (PASS) | 583909376 | unsupported,unsupported,unsupported |
| `generated-use_t2` | T2 | 10036526 | unsupported | — | 57.843 | 0.186 | 126959616 | 7.405 (PASS) | 4728373248 | unsupported,unsupported,unsupported |
| `generated-use_t3` | T3 | 100542256 | unsupported | — | — | 0.185 | 230998016 | 111.468 (PASS) | 10144759808 | unsupported |
| `generated-use_t4` | T4 | 1005597835 | unsupported | — | — | 0.188 | 1136050176 | 40.961 (FAIL / TIMEOUT) | 14546288640 | unsupported |
| `commons-1-42-polytope-7-cube-svg` | T3 | 29836045 | unsupported | — | — | 0.185 | 182108160 | 7.604 (PASS) | 4396826624 | unsupported |
| `commons-2015-louisiana-gubernatorial-election-runoff-by-precinct-svg` | T2 | 16162430 | unsupported | — | 48.516 | 0.059 | 25149440 | 1.106 (PASS) | 374915072 | unsupported,unsupported,unsupported |
| `commons-aegean-sea-anatolia-and-armenian-highlands-regions-large-topographic-basemap-svg` | T3 | 22666147 | unsupported | — | 74.364 | 0.171 | 44056576 | 1.740 (PASS) | 768491520 | unsupported |
| `commons-amhara-topographic-map-svg` | T2 | 15593284 | unsupported | — | 48.660 | 0.053 | 18481152 | 1.017 (PASS) | 297582592 | unsupported,unsupported,unsupported |
| `commons-bulgaria-in-1915-geographic-map-en-svg` | T2 | 15165804 | unsupported | — | 52.993 | 0.117 | 41172992 | 1.788 (PASS) | 610844672 | unsupported,unsupported,unsupported |
| `commons-cambodia-geographic-map-en-svg` | T3 | 20142590 | unsupported | — | 67.460 | 0.172 | 60063744 | 2.838 (PASS) | 931348480 | unsupported |
| `commons-chuaattractormodified-svg` | T3 | 20763720 | unsupported | — | — | 0.020 | 22806528 | 5.210 (PASS) | 2032254976 | unsupported |
| `commons-coat-of-arms-of-the-kingdom-of-naples-1808-svg` | T3 | 25708531 | unsupported | — | 86.664 | 0.092 | 47316992 | 1.686 (PASS) | 458997760 | unsupported |
| `commons-coat-of-arms-of-villa-alemana-svg` | T3 | 42628426 | unsupported | — | 130.181 | 0.088 | 57016320 | 5.606 (PASS) | 523485184 | unsupported |
| `commons-columbiarivermap-svg` | T2 | 16687292 | unsupported | — | — | 0.025 | 18726912 | 0.773 (PASS) | 361447424 | unsupported,unsupported,unsupported |
| `commons-comunas-occitanas-d-alemanha-svg` | T3 | 43507208 | unsupported | — | 126.855 | 0.051 | 55984128 | 3.748 (PASS) | 799064064 | unsupported |
| `commons-corsica-geographic-map-svg` | T2 | 14927948 | unsupported | — | 46.798 | 0.063 | 30015488 | 1.938 (PASS) | 516685824 | unsupported,unsupported,unsupported |
| `commons-corvuscoraxiucn-svg` | T3 | 26496549 | unsupported | — | 83.009 | 0.092 | 45891584 | 2.142 (PASS) | 696352768 | unsupported |
| `commons-dibujo-piu-svg` | T3 | 87755815 | unsupported | — | 291.861 | 0.157 | 174227456 | 13.365 (PASS) | 1827225600 | unsupported |
| `commons-dukesofhohenbergarms-svg` | T3 | 28075813 | unsupported | — | 91.665 | 0.198 | 53215232 | 2.876 (PASS) | 748142592 | unsupported |
| `commons-france-osm-mapnik-svg` | T2 | 19491319 | unsupported | — | — | 0.017 | 21528576 | 2.552 (PASS) | 521846784 | unsupported,unsupported,unsupported |
| `commons-gaussian-2d-svg` | T2 | 16746052 | unsupported | — | — | 0.011 | 18792448 | 1.556 (PASS) | 510377984 | unsupported,unsupported,unsupported |
| `commons-geographic-map-of-norway-sweden-finland-and-denmark-svg` | T3 | 20101913 | unsupported | — | 70.135 | 0.171 | 56213504 | 3.405 (PASS) | 1373749248 | unsupported |
| `commons-germany-districts-municipalities-location-map-2013-svg` | T3 | 26486961 | unsupported | — | — | 0.012 | 28524544 | 3.675 (PASS) | 765837312 | unsupported |
| `commons-ideogram-human-chromosome-20-svg` | T3 | 21285516 | unsupported | — | 85.030 | 0.181 | 103809024 | 1.971 (PASS) | 1239302144 | unsupported |
| `commons-koppen-geiger-map-a-present-svg` | T3 | 79058833 | unsupported | — | 244.713 | 0.162 | 146030592 | 6.839 (PASS) | 1859502080 | unsupported |
| `commons-municipalities-of-italy-svg` | T3 | 91915099 | unsupported | — | 270.152 | 0.051 | 100646912 | 5.145 (PASS) | 917422080 | unsupported |
| `commons-spain-languages-de-svg` | T3 | 90812529 | unsupported | — | 262.244 | 0.101 | 112082944 | 5.409 (PASS) | 1168162816 | unsupported |
| `natural-earth-world-map-blank` | T1 | 241682 | ok | 8.297 | 0.875 | 0.054 | 4767744 | 0.347 (PASS) | 150437888 | ok,ok,ok |
| `natural-earth-world-map-countries` | T1 | 260823 | ok | 8.406 | 0.965 | 0.056 | 4997120 | 0.409 (PASS) | 157794304 | ok,ok,ok |

## Tier summary

Speedup is Inkscape supplied probe seconds divided by Vexel first-preview seconds; RSS ratio is Inkscape peak RSS divided by Vexel peak RSS. Both geometric means include only successful Vexel outcomes and PASS Inkscape rows. Protocols differ, so these are directional workload comparisons.

| Tier | Files | OK | Unsupported | Limit | Crash/timeout | Comparable files | Geomean time ratio | Geomean RSS ratio |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| T1 | 12 | 2 | 10 | 0 | 0 | 2 | 45.110x | 31.565x (2 files) |
| T2 | 17 | 0 | 17 | 0 | 0 | 0 | — | — (0 files) |
| T3 | 26 | 0 | 26 | 0 | 0 | 0 | — | — (0 files) |
| T4 | 3 | 0 | 3 | 0 | 0 | 0 | — | — (0 files) |

## Wedge checks

- 100 MiB first visible preview under 2 seconds: **fail_no_preview**; measured 0 of 12 T3 files in the 80–120 MiB band; maximum None ns.
- Maximum main-thread index step under 100 ms: **pass**; maximum 210709 ns. Pre-index preview occurred in 0 eligible files.
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

Release binary SHA-256: `66a4fbaf06833fff50c3ea9471f6f3f0305c0a1368e957793a115214bca42c8f`. Fresh processes, 3 repetitions for T1/T2, 1 for T3/T4, each with a 120s timeout. External source files remain at the referenced corpus path; no corpus file is stored in this repository.
