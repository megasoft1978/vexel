# Real-corpus full-render diagnostics

Corpus manifest SHA-256: `9fbf9a2f384d70a41da72e99ac259d2c5d137ffd3c8a01ce4be4741c1c29bc30`.
Render binary: `build/mac-release/bin/svgworkbench` at `128x128`.
Size threshold: 33554432 bytes (larger files are recorded as `skipped`).
Files accounted for: 58; render-tested: 39; not render-tested: 19 (sources larger than --max-bytes (33554432) are recorded as skipped and were not rendered).

Unlike `head-to-head.md`, this report measures a full-document render, not a synthetic single-element preview.

## Verdict histogram

| Verdict | Files |
|---|---:|
| `limit` | 13 |
| `ok` | 3 |
| `skipped` | 19 |
| `unsupported` | 13 |
| `viewport` | 2 |
| `xml` | 8 |

## Files that render

| ID | Bytes | Tier |
|---|---:|---|
| `generated-cnc_t1` | 990772 | T1 |
| `generated-embedded_t1` | 912235 | T1 |
| `commons-2015-louisiana-gubernatorial-election-runoff-by-precinct-svg` | 16162430 | T2 |

## Per-file results

| ID | Kind | Tier | Bytes | Verdict | Diagnostic | Offset | Excerpt |
|---|---|---|---:|---|---|---:|---|
| `generated-cad_t1` | generated | T1 | 997801 | `limit` | limit | 0 | `<?xml version="1.0" encoding="UTF-8"?>.<svg xmln...` |
| `generated-cad_t2` | generated | T2 | 10592198 | `limit` | limit | 2680259 | `... 0 1 10 0"/><path d="M525 7860 h11 v7 h-11 z"/><rect x="540" y="7860" width="12" height="8"/><li...` |
| `generated-cad_t3` | generated | T3 | 109719583 | `skipped` |  |  | `not render-tested: source exceeds --max-bytes (33554432)` |
| `generated-cnc_t1` | generated | T1 | 990772 | `ok` |  |  | `` |
| `generated-cnc_t2` | generated | T2 | 10162188 | `limit` | limit | 2565705 | `...3 0 Z"/><path d="M525 9825 l3 0 l0 3 l-3 0 Z"/><path d="M540 9825 l3 0 l0 3 l-3 0 Z"/><path d="M...` |
| `generated-cnc_t3` | generated | T3 | 102580014 | `skipped` |  |  | `not render-tested: source exceeds --max-bytes (33554432)` |
| `generated-embedded_t1` | generated | T1 | 912235 | `ok` |  |  | `` |
| `generated-embedded_t2` | generated | T2 | 9371925 | `limit` | limit | 0 | `<?xml version="1.0" encoding="UTF-8"?>.<svg xmln...` |
| `generated-embedded_t3` | generated | T3 | 93970295 | `skipped` |  |  | `not render-tested: source exceeds --max-bytes (33554432)` |
| `generated-giant_path_t1` | generated | T1 | 920236 | `limit` | limit | 0 | `<?xml version="1.0" encoding="UTF-8"?>.<svg xmln...` |
| `generated-giant_path_t2` | generated | T2 | 8340030 | `limit` | limit | 0 | `<?xml version="1.0" encoding="UTF-8"?>.<svg xmln...` |
| `generated-giant_path_t3` | generated | T3 | 91963466 | `skipped` |  |  | `not render-tested: source exceeds --max-bytes (33554432)` |
| `generated-giant_path_t4` | generated | T4 | 827663466 | `skipped` |  |  | `not render-tested: source exceeds --max-bytes (33554432)` |
| `generated-inkscape_t1` | generated | T1 | 979796 | `unsupported` | unsupported | 1214 | `...path-effect="#path-effect1" fill="none" stroke="url(#g)"><path d="M0 0h4v2h-4z"/><path d="M1 11h...` |
| `generated-inkscape_t2` | generated | T2 | 10062253 | `unsupported` | unsupported | 1214 | `...path-effect="#path-effect1" fill="none" stroke="url(#g)"><path d="M0 0h4v2h-4z"/><path d="M1 11h...` |
| `generated-inkscape_t3` | generated | T3 | 100883678 | `skipped` |  |  | `not render-tested: source exceeds --max-bytes (33554432)` |
| `generated-map_t1` | generated | T1 | 1310972 | `unsupported` | unsupported | 915 | `...noise.</metadata>.<g id="regions" class="land"><path class="land" data-region="0" d="M12.92,6.60...` |
| `generated-map_t2` | generated | T2 | 14191241 | `unsupported` | unsupported | 915 | `...noise.</metadata>.<g id="regions" class="land"><path class="land" data-region="0" d="M12.60,6.60...` |
| `generated-map_t3` | generated | T3 | 150627424 | `skipped` |  |  | `not render-tested: source exceeds --max-bytes (33554432)` |
| `generated-nested_t1` | generated | T1 | 978645 | `xml` | xml | 16843 | `..." transform="translate(.001 .001) rotate(.001)"><g id="nest-255" transform="translate(.001 .001)...` |
| `generated-nested_t2` | generated | T2 | 9430992 | `xml` | xml | 16843 | `..." transform="translate(.001 .001) rotate(.001)"><g id="nest-255" transform="translate(.001 .001)...` |
| `generated-nested_t3` | generated | T3 | 93956332 | `skipped` |  |  | `not render-tested: source exceeds --max-bytes (33554432)` |
| `generated-science_t1` | generated | T1 | 1374397 | `limit` | limit | 0 | `<?xml version="1.0" encoding="UTF-8"?>.<svg xmln...` |
| `generated-science_t2` | generated | T2 | 14116675 | `limit` | limit | 2709482 | `...1.3"/><circle cx="405.52" cy="393.10" r="1.3"/><circle cx="309.73" cy="303.75" r="1.3"/><circle ...` |
| `generated-science_t3` | generated | T3 | 117860153 | `skipped` |  |  | `not render-tested: source exceeds --max-bytes (33554432)` |
| `generated-science_t4` | generated | T4 | 635202435 | `skipped` |  |  | `not render-tested: source exceeds --max-bytes (33554432)` |
| `generated-text_t1` | generated | T1 | 887035 | `limit` | limit | 235947 | `...ation, and identifier</text><text x="675" y="5">Sample 002825: measurement, annotation, and iden...` |
| `generated-text_t2` | generated | T2 | 9024701 | `limit` | limit | 235947 | `...ation, and identifier</text><text x="675" y="5">Sample 002825: measurement, annotation, and iden...` |
| `generated-text_t3` | generated | T3 | 90488308 | `skipped` |  |  | `not render-tested: source exceeds --max-bytes (33554432)` |
| `generated-use_t1` | generated | T1 | 986323 | `limit` | limit | 648183 | `...="#mark" x="321" y="773" width="6" height="6"/><use href="#mark" x="322" y="786" width="6" heigh...` |
| `generated-use_t2` | generated | T2 | 10036526 | `limit` | limit | 648183 | `...="#mark" x="321" y="773" width="6" height="6"/><use href="#mark" x="322" y="786" width="6" heigh...` |
| `generated-use_t3` | generated | T3 | 100542256 | `skipped` |  |  | `not render-tested: source exceeds --max-bytes (33554432)` |
| `generated-use_t4` | generated | T4 | 1005597835 | `skipped` |  |  | `not render-tested: source exceeds --max-bytes (33554432)` |
| `commons-1-42-polytope-7-cube-svg` | real | T3 | 29836045 | `xml` | xml | 15241908 | `...y1="425.803" x2="608.789" y2="440.122"/>..<line x1="579.054" y1="425.803" x2="629.366" y2="465.9...` |
| `commons-2015-louisiana-gubernatorial-election-runoff-by-precinct-svg` | real | T2 | 16162430 | `ok` |  |  | `` |
| `commons-aegean-sea-anatolia-and-armenian-highlands-regions-large-topographic-basemap-svg` | real | T3 | 22666147 | `viewport` | viewport | 70 | `...coding="UTF-8" standalone="no"?>.<svg.   width="7409".   height="2600".   version="1.0".   id="s...` |
| `commons-amhara-topographic-map-svg` | real | T2 | 15593284 | `unsupported` | unsupported | 14608543 | `...g>. <g transform="translate(-49.4 -30.176)">.  <path d="m365.29 432.38c-2.0945-0.60237-4.7362 0....` |
| `commons-bulgaria-in-1915-geographic-map-en-svg` | real | T2 | 15165804 | `unsupported` | unsupported | 3805 | `...:#555555;stroke:none; opacity:0.2;}.</style>.  <clipPath.     id="IDmain">.    <rect.       x="0...` |
| `commons-cambodia-geographic-map-en-svg` | real | T3 | 20142590 | `unsupported` | unsupported | 9490 | `...55555;stroke:none; opacity:0.2;}.]]></style>.  <clipPath.     id="IDmain">.    <rect.       id="...` |
| `commons-chuaattractormodified-svg` | real | T3 | 20763720 | `xml` | xml | 56 | `...rsion="1.0" encoding="utf-8" standalone="no"?>..<!DOCTYPE svg PUBLIC "-//W3C//DTD SVG 1.1//EN".....` |
| `commons-coat-of-arms-of-the-kingdom-of-naples-1808-svg` | real | T3 | 25708531 | `unsupported` | unsupported | 457167 | `...m="translate(-7.5575766,-121.5941)" />.        <use.           xlink:href="#g41720-5".          ...` |
| `commons-coat-of-arms-of-villa-alemana-svg` | real | T3 | 42628426 | `skipped` |  |  | `not render-tested: source exceeds --max-bytes (33554432)` |
| `commons-columbiarivermap-svg` | real | T2 | 16687292 | `xml` | xml | 136 | `...rt Plug-In . SVG Version: 3.0.0 Build 77)  -->..<!DOCTYPE svg PUBLIC "-//W3C//DTD SVG 1.0//EN"  ...` |
| `commons-comunas-occitanas-d-alemanha-svg` | real | T3 | 43507208 | `skipped` |  |  | `not render-tested: source exceeds --max-bytes (33554432)` |
| `commons-corsica-geographic-map-svg` | real | T2 | 14927948 | `unsupported` | unsupported | 14832988 | `...ign:end;writing-mode:lr-tb;text-anchor:end" id="tspan3993" sodipodi:role="line" y="179.84268" x=...` |
| `commons-corvuscoraxiucn-svg` | real | T3 | 26496549 | `unsupported` | unsupported | 56 | `...rsion='1.0' encoding='UTF-8' standalone='no'?>.<svg width="296.926mm" height="209.973mm" xmlns:c...` |
| `commons-dibujo-piu-svg` | real | T3 | 87755815 | `skipped` |  |  | `not render-tested: source exceeds --max-bytes (33554432)` |
| `commons-dukesofhohenbergarms-svg` | real | T3 | 28075813 | `unsupported` | unsupported | 26509048 | `...h355098".         style="clip-rule:evenodd;fill:#000000;fill-opacity:1;fill-rule:evenodd;stroke:...` |
| `commons-france-osm-mapnik-svg` | real | T2 | 19491319 | `xml` | xml | 39 | `<?xml version="1.0" encoding="UTF-8"?>.<!DOCTYPE svg PUBLIC '-//W3C//DTD SVG 1.1//EN' '...` |
| `commons-gaussian-2d-svg` | real | T2 | 16746052 | `xml` | xml | 56 | `...rsion="1.0" encoding="utf-8" standalone="no"?>..<!DOCTYPE svg PUBLIC "-//W3C//DTD SVG 1.1//EN".....` |
| `commons-geographic-map-of-norway-sweden-finland-and-denmark-svg` | real | T3 | 20101913 | `viewport` | viewport | 70 | `...coding="UTF-8" standalone="no"?>.<svg.   width="4600".   height="6056".   version="1.1".   id="s...` |
| `commons-germany-districts-municipalities-location-map-2013-svg` | real | T3 | 26486961 | `xml` | xml | 39 | `<?xml version="1.0" encoding="UTF-8"?>.<!DOCTYPE svg PUBLIC '-//W3C//DTD SVG 1.1//EN' '...` |
| `commons-ideogram-human-chromosome-20-svg` | real | T3 | 21285516 | `limit` | limit | 9614467 | `...-opacity:1;".         id="stop129367" />.      <stop.         offset="0.859375".         style="...` |
| `commons-koppen-geiger-map-a-present-svg` | real | T3 | 79058833 | `skipped` |  |  | `not render-tested: source exceeds --max-bytes (33554432)` |
| `commons-municipalities-of-italy-svg` | real | T3 | 91915099 | `skipped` |  |  | `not render-tested: source exceeds --max-bytes (33554432)` |
| `commons-spain-languages-de-svg` | real | T3 | 90812529 | `skipped` |  |  | `not render-tested: source exceeds --max-bytes (33554432)` |
| `natural-earth-world-map-blank` | real | T1 | 241682 | `unsupported` | unsupported | 828 | `... stroke-width="0.7"/>.  <g clip-path="url(#c)"><clipPath id="c"><path d="M207.2,2L500,2L792.8,2L...` |
| `natural-earth-world-map-countries` | real | T1 | 260823 | `unsupported` | unsupported | 828 | `... stroke-width="0.7"/>.  <g clip-path="url(#c)"><clipPath id="c"><path d="M207.2,2L500,2L792.8,2L...` |
