## Comparison with the legacy C baselines

Sources (sha256 of the compared files):

- baseline: sha256 `0192c0d25a9b7b854dda35c7eb661368cf78efcfdff526fdba8305c582e4e588`
- current Rust run: sha256 `39235ad78b911cfc1d9dae24cc2c23a945ed7c1cd565e78d30c63be258b9cc89`

| category | baseline | current Rust run |
|---|---|---|
| ok | 1 | 2 |
| unsupported | 7 | 14 |
| limit | 28 | 0 |
| limit_decode | 0 | 4 |
| limit_render | 0 | 1 |
| xml | 2 | 10 |
| timeout | 1 | 8 |
| skipped | 19 | 19 |
| **ok / attempted** | 1 / 39 | 2 / 39 |

Legacy `path` and `viewport` have no Rust verdict of that name. In the tables above they stay separate rows (never merged). For the per-file deltas below they are compared as `xml` (Rust reports malformed path data and viewport as `Malformed`, verdict `xml`); every such pair is printed with its raw legacy category.

### Per-file deltas vs baseline

New ok: 1

- `commons-2015-louisiana-gubernatorial-election-runoff-by-precinct-svg`: limit -> ok

Regressions: none

Changed category: 27

- `generated-cad_t2`: limit -> timeout
- `generated-cnc_t2`: limit -> timeout
- `generated-embedded_t2`: limit -> unsupported
- `generated-giant_path_t2`: limit -> limit_render (same category after mapping)
- `generated-inkscape_t2`: limit -> unsupported
- `generated-map_t2`: limit -> timeout
- `generated-nested_t2`: limit -> xml
- `generated-science_t2`: limit -> timeout
- `generated-text_t2`: limit -> unsupported
- `generated-use_t1`: limit -> limit_decode (same category after mapping)
- `generated-use_t2`: limit -> limit_decode (same category after mapping)
- `commons-1-42-polytope-7-cube-svg`: limit -> xml
- `commons-aegean-sea-anatolia-and-armenian-highlands-regions-large-topographic-basemap-svg`: limit -> xml
- `commons-amhara-topographic-map-svg`: limit -> unsupported
- `commons-bulgaria-in-1915-geographic-map-en-svg`: limit -> timeout
- `commons-cambodia-geographic-map-en-svg`: limit -> timeout
- `commons-chuaattractormodified-svg`: limit -> limit_decode (same category after mapping)
- `commons-coat-of-arms-of-the-kingdom-of-naples-1808-svg`: limit -> unsupported
- `commons-columbiarivermap-svg`: limit -> xml
- `commons-corsica-geographic-map-svg`: limit -> unsupported
- `commons-corvuscoraxiucn-svg`: limit -> timeout
- `commons-dukesofhohenbergarms-svg`: limit -> xml
- `commons-france-osm-mapnik-svg`: limit -> xml
- `commons-gaussian-2d-svg`: limit -> unsupported
- `commons-geographic-map-of-norway-sweden-finland-and-denmark-svg`: limit -> xml
- `commons-germany-districts-municipalities-location-map-2013-svg`: limit -> xml
- `commons-ideogram-human-chromosome-20-svg`: limit -> limit_decode (same category after mapping)

Unchanged (including same-after-mapping): 35. Ids not in current report: 0.

