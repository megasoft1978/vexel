# Real-corpus full-render diagnostics

Corpus manifest SHA-256: `9fbf9a2f384d70a41da72e99ac259d2c5d137ffd3c8a01ce4be4741c1c29bc30`.
Render binary: `build/mac-release/bin/svgworkbench` at `128x128`.
Size threshold: 33554432 bytes (larger files are recorded as `skipped`).
Files accounted for: 58.

Unlike `head-to-head.md`, this report measures a full-document render, not a synthetic single-element preview.

## Verdict histogram

| Verdict | Files |
|---|---:|
| `limit` | 26 |
| `ok` | 1 |
| `skipped` | 19 |
| `unsupported` | 8 |
| `xml` | 4 |

## Files that render

| ID | Bytes | Tier |
|---|---:|---|
| `generated-embedded_t1` | 912235 | T1 |

## Per-file results

| ID | Kind | Tier | Bytes | Verdict | Diagnostic | Offset | Excerpt |
|---|---|---|---:|---|---|---:|---|
| `generated-cad_t1` | generated | T1 | 997801 | `limit` | limit | 863987 | `...637</text><text x="570" y="72">Room 0638</text><text x="585" y="72">Room 0639</text><text x="600...` |
| `generated-cad_t2` | generated | T2 | 10592198 | `limit` | limit | 1527099 | `...><path d="M750 4488 a5 5 0 0 1 10 0"/><path d="M765 4488 h11 v7 h-11 z"/><rect x="780" y="4488" ...` |
| `generated-cad_t3` | generated | T3 | 109719583 | `skipped` | source exceeds --max-bytes (33554432) |  | `` |
| `generated-cnc_t1` | generated | T1 | 990772 | `limit` | limit | 691833 | `...25 .15"/><path d="M857 599l1.25 .15"/><path d="M858 606l1.25 .15"/><path d="M859 613l1.25 .15"/>...` |
| `generated-cnc_t2` | generated | T2 | 10162188 | `limit` | limit | 507893 | `...90 1965 l3 0 l0 3 l-3 0 Z"/><path d="M105 1965 l3 0 l0 3 l-3 0 Z"/><path d="M120 1965 l3 0 l0 3 ...` |
| `generated-cnc_t3` | generated | T3 | 102580014 | `skipped` | source exceeds --max-bytes (33554432) |  | `` |
| `generated-embedded_t1` | generated | T1 | 912235 | `ok` |  |  | `` |
| `generated-embedded_t2` | generated | T2 | 9371925 | `limit` | limit | 0 | `<?xml version="1.0" encoding="UTF-8"?>.<svg xmln...` |
| `generated-embedded_t3` | generated | T3 | 93970295 | `skipped` | source exceeds --max-bytes (33554432) |  | `` |
| `generated-giant_path_t1` | generated | T1 | 920236 | `limit` | limit | 603597 | `...30,610 L1531,647 L1532,684 L1533,721 L1534,758 L1535,795 L1536,832 L1537,869 L1538,906 L1539,943...` |
| `generated-giant_path_t2` | generated | T2 | 8340030 | `limit` | limit | 603597 | `...30,610 L1531,647 L1532,684 L1533,721 L1534,758 L1535,795 L1536,832 L1537,869 L1538,906 L1539,943...` |
| `generated-giant_path_t3` | generated | T3 | 91963466 | `skipped` | source exceeds --max-bytes (33554432) |  | `` |
| `generated-giant_path_t4` | generated | T4 | 827663466 | `skipped` | source exceeds --max-bytes (33554432) |  | `` |
| `generated-inkscape_t1` | generated | T1 | 979796 | `unsupported` | unsupported | 1214 | `...path-effect="#path-effect1" fill="none" stroke="url(#g)"><path d="M0 0h4v2h-4z"/><path d="M1 11h...` |
| `generated-inkscape_t2` | generated | T2 | 10062253 | `unsupported` | unsupported | 1214 | `...path-effect="#path-effect1" fill="none" stroke="url(#g)"><path d="M0 0h4v2h-4z"/><path d="M1 11h...` |
| `generated-inkscape_t3` | generated | T3 | 100883678 | `skipped` | source exceeds --max-bytes (33554432) |  | `` |
| `generated-map_t1` | generated | T1 | 1310972 | `unsupported` | unsupported | 915 | `...noise.</metadata>.<g id="regions" class="land"><path class="land" data-region="0" d="M12.92,6.60...` |
| `generated-map_t2` | generated | T2 | 14191241 | `unsupported` | unsupported | 915 | `...noise.</metadata>.<g id="regions" class="land"><path class="land" data-region="0" d="M12.60,6.60...` |
| `generated-map_t3` | generated | T3 | 150627424 | `skipped` | source exceeds --max-bytes (33554432) |  | `` |
| `generated-nested_t1` | generated | T1 | 978645 | `xml` | xml | 16843 | `..." transform="translate(.001 .001) rotate(.001)"><g id="nest-255" transform="translate(.001 .001)...` |
| `generated-nested_t2` | generated | T2 | 9430992 | `xml` | xml | 16843 | `..." transform="translate(.001 .001) rotate(.001)"><g id="nest-255" transform="translate(.001 .001)...` |
| `generated-nested_t3` | generated | T3 | 93956332 | `skipped` | source exceeds --max-bytes (33554432) |  | `` |
| `generated-science_t1` | generated | T1 | 1374397 | `limit` | limit | 0 | `<?xml version="1.0" encoding="UTF-8"?>.<svg xmln...` |
| `generated-science_t2` | generated | T2 | 14116675 | `limit` | limit | 903814 | `...1.3"/><circle cx="299.47" cy="285.17" r="1.3"/><circle cx="1274.69" cy="271.09" r="1.3"/><circle...` |
| `generated-science_t3` | generated | T3 | 117860153 | `skipped` | source exceeds --max-bytes (33554432) |  | `` |
| `generated-science_t4` | generated | T4 | 635202435 | `skipped` | source exceeds --max-bytes (33554432) |  | `` |
| `generated-text_t1` | generated | T1 | 887035 | `limit` | limit | 15500 | `...tion, and identifier</text><text x="685" y="35">Sample 000175: measurement, annotation, and iden...` |
| `generated-text_t2` | generated | T2 | 9024701 | `limit` | limit | 15500 | `...tion, and identifier</text><text x="685" y="35">Sample 000175: measurement, annotation, and iden...` |
| `generated-text_t3` | generated | T3 | 90488308 | `skipped` | source exceeds --max-bytes (33554432) |  | `` |
| `generated-use_t1` | generated | T1 | 986323 | `limit` | limit | 648183 | `...="#mark" x="321" y="773" width="6" height="6"/><use href="#mark" x="322" y="786" width="6" heigh...` |
| `generated-use_t2` | generated | T2 | 10036526 | `limit` | limit | 648183 | `...="#mark" x="321" y="773" width="6" height="6"/><use href="#mark" x="322" y="786" width="6" heigh...` |
| `generated-use_t3` | generated | T3 | 100542256 | `skipped` | source exceeds --max-bytes (33554432) |  | `` |
| `generated-use_t4` | generated | T4 | 1005597835 | `skipped` | source exceeds --max-bytes (33554432) |  | `` |
| `commons-1-42-polytope-7-cube-svg` | real | T3 | 29836045 | `limit` | limit | 16777216 | `...>..<line x1="437.078" y1="707.045" x2="540.969" y2="576.770"/>..<line x1="437.078" y1="707.045" ...` |
| `commons-2015-louisiana-gubernatorial-election-runoff-by-precinct-svg` | real | T2 | 16162430 | `limit` | limit | 738202 | `....2 -0.1,0.1 -0.2,0.4 -0.1,0.1 h -0.2 l -0.1,0.2 -0.2,0.6 v 0.2 l 0.1,0.2 v 0.3 l -0.1,0.1 -0.2,0...` |
| `commons-aegean-sea-anatolia-and-armenian-highlands-regions-large-topographic-basemap-svg` | real | T3 | 22666147 | `limit` | limit | 16777216 | `...2.21 L5450.69 2032.21 L5450.69 2032.21".           id="path129135" />.        <path.           d...` |
| `commons-amhara-topographic-map-svg` | real | T2 | 15593284 | `limit` | limit | 2506734 | `...9 0.0971-0.16061 0.2051-0.35704 0.36719-0.73633 0.71564 0.816 0.36897 2.3223-0.32617 2.9668 0.06...` |
| `commons-bulgaria-in-1915-geographic-map-en-svg` | real | T2 | 15165804 | `unsupported` | unsupported | 3805 | `...:#555555;stroke:none; opacity:0.2;}.</style>.  <clipPath.     id="IDmain">.    <rect.       x="0...` |
| `commons-cambodia-geographic-map-en-svg` | real | T3 | 20142590 | `limit` | limit | 16777216 | `...-0.01 -0.04,-0.02 -0.02,-0.01 h -0.01 l -0.05,-0.02 v 0" />.        <path.           id="path225...` |
| `commons-chuaattractormodified-svg` | real | T3 | 20763720 | `limit` | limit | 16777216 | `...0ff;stroke-linecap:square;stroke-width:0.5;"/>..   </g>..   <g id="line2d_80818">..    <path cli...` |
| `commons-coat-of-arms-of-the-kingdom-of-naples-1808-svg` | real | T3 | 25708531 | `limit` | limit | 16777216 | `...2.46875,5.34375 0.9883,3.5302 1.36894,8.48265 1.5625,12.6875 0.019,0.41342 -0.0149,0.35461 0,0.7...` |
| `commons-coat-of-arms-of-villa-alemana-svg` | real | T3 | 42628426 | `skipped` | source exceeds --max-bytes (33554432) |  | `` |
| `commons-columbiarivermap-svg` | real | T2 | 16687292 | `xml` | xml | 136 | `...rt Plug-In . SVG Version: 3.0.0 Build 77)  -->..<!DOCTYPE svg PUBLIC "-//W3C//DTD SVG 1.0//EN"  ...` |
| `commons-comunas-occitanas-d-alemanha-svg` | real | T3 | 43507208 | `skipped` | source exceeds --max-bytes (33554432) |  | `` |
| `commons-corsica-geographic-map-svg` | real | T2 | 14927948 | `unsupported` | unsupported | 14832988 | `...ign:end;writing-mode:lr-tb;text-anchor:end" id="tspan3993" sodipodi:role="line" y="179.84268" x=...` |
| `commons-corvuscoraxiucn-svg` | real | T3 | 26496549 | `limit` | limit | 16777216 | `...66 L1220.77,1528.52 L1220.68,1528.52 L1220.68,1528.37 L1220.58,1528.37 L1220.68,1527.93 L1220.68...` |
| `commons-dibujo-piu-svg` | real | T3 | 87755815 | `skipped` | source exceeds --max-bytes (33554432) |  | `` |
| `commons-dukesofhohenbergarms-svg` | real | T3 | 28075813 | `limit` | limit | 16777216 | `...6 -17.626,8.152 -24.833,12.545 -3.572,-8.096 -0.818,-15.308 0.114,-22.515 -10.744,5.028 -14.393,...` |
| `commons-france-osm-mapnik-svg` | real | T2 | 19491319 | `limit` | limit | 16777216 | `...004.004.008.004.004v.008l.004.008.004.004.004.008.004.008.004.004.008.008v.004l-.004.008.004.008...` |
| `commons-gaussian-2d-svg` | real | T2 | 16746052 | `xml` | xml | 56 | `...rsion="1.0" encoding="utf-8" standalone="no"?>..<!DOCTYPE svg PUBLIC "-//W3C//DTD SVG 1.1//EN".....` |
| `commons-geographic-map-of-norway-sweden-finland-and-denmark-svg` | real | T3 | 20101913 | `limit` | limit | 16777216 | `...2.56 -0.40 l-2.19 -0.90 l-1.76 -1.40 l-1.52 -1.70 l-1.58 -1.80 l-0.90 -2.20 l-1.08 -1.91 l-2.08 ...` |
| `commons-germany-districts-municipalities-location-map-2013-svg` | real | T3 | 26486961 | `limit` | limit | 16777216 | `....3-.1v-.4l-.2-.1-.2-1.5.1-.3.4-.1.6-.2.2-.1.2-.1.2-.2v-.1h.1.5l.4.2.1-.4.3-.2.1-.5-.3-.1-.1-.5v-...` |
| `commons-ideogram-human-chromosome-20-svg` | real | T3 | 21285516 | `limit` | limit | 16777216 | `...25189" />.      <stop.         offset="0.0820313".         style="stop-color:rgb(91.40625%,91.40...` |
| `commons-koppen-geiger-map-a-present-svg` | real | T3 | 79058833 | `skipped` | source exceeds --max-bytes (33554432) |  | `` |
| `commons-municipalities-of-italy-svg` | real | T3 | 91915099 | `skipped` | source exceeds --max-bytes (33554432) |  | `` |
| `commons-spain-languages-de-svg` | real | T3 | 90812529 | `skipped` | source exceeds --max-bytes (33554432) |  | `` |
| `natural-earth-world-map-blank` | real | T1 | 241682 | `unsupported` | unsupported | 828 | `... stroke-width="0.7"/>.  <g clip-path="url(#c)"><clipPath id="c"><path d="M207.2,2L500,2L792.8,2L...` |
| `natural-earth-world-map-countries` | real | T1 | 260823 | `unsupported` | unsupported | 828 | `... stroke-width="0.7"/>.  <g clip-path="url(#c)"><clipPath id="c"><path d="M207.2,2L500,2L792.8,2L...` |
