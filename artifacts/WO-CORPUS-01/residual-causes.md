# WO-CORPUS-01 residual causes

Source: `artifacts/WO-M4-10/render-diagnostics.json` regenerated with the order's corpus command (`mac-debug` binary). Excerpts are quoted verbatim from the report's own `excerpt` field; no corpus bytes are copied beyond that.

Result: `histogram.ok` stayed at 3 (before: 3); AC-3 is not met and the order's stop condition applies. No scope was widened. The files below are the next blockers for planner review.

## Files still `unsupported`, `xml` or `viewport` (item 6)

| ID | Before | Now | Offset | Excerpt (report) |
|---|---|---|---:|---|
| `generated-inkscape_t1` | `unsupported`@1214 | `unsupported` | 1214 | `...path-effect="#path-effect1" fill="none" stroke="url(#g)"><path d="M0 0h4v2h-4z"/><path d="M1 11h...` |
| `generated-inkscape_t2` | `unsupported`@1214 | `unsupported` | 1214 | `...path-effect="#path-effect1" fill="none" stroke="url(#g)"><path d="M0 0h4v2h-4z"/><path d="M1 11h...` |
| `generated-nested_t1` | `xml`@16843 | `xml` | 16843 | `..." transform="translate(.001 .001) rotate(.001)"><g id="nest-255" transform="translate(.001 .001)...` |
| `generated-nested_t2` | `xml`@16843 | `xml` | 16843 | `..." transform="translate(.001 .001) rotate(.001)"><g id="nest-255" transform="translate(.001 .001)...` |
| `commons-1-42-polytope-7-cube-svg` | `xml`@15241908 | `xml` | 15241908 | `...y1="425.803" x2="608.789" y2="440.122"/>..<line x1="579.054" y1="425.803" x2="629.366" y2="465.9...` |
| `commons-aegean-sea-anatolia-and-armenian-highlands-regions-large-topographic-basemap-svg` | `viewport`@70 | `viewport` | 70 | `...coding="UTF-8" standalone="no"?>.<svg.   width="7409".   height="2600".   version="1.0".   id="s...` |
| `commons-amhara-topographic-map-svg` | `unsupported`@14608543 | `unsupported` | 14608543 | `...g>. <g transform="translate(-49.4 -30.176)">.  <path d="m365.29 432.38c-2.0945-0.60237-4.7362 0....` |
| `commons-bulgaria-in-1915-geographic-map-en-svg` | `unsupported`@3805 | `unsupported` | 3805 | `...:#555555;stroke:none; opacity:0.2;}.</style>.  <clipPath.     id="IDmain">.    <rect.       x="0...` |
| `commons-cambodia-geographic-map-en-svg` | `unsupported`@9490 | `unsupported` | 9490 | `...55555;stroke:none; opacity:0.2;}.]]></style>.  <clipPath.     id="IDmain">.    <rect.       id="...` |
| `commons-coat-of-arms-of-the-kingdom-of-naples-1808-svg` | `unsupported`@457167 | `unsupported` | 457167 | `...m="translate(-7.5575766,-121.5941)" />.        <use.           xlink:href="#g41720-5".          ...` |
| `commons-columbiarivermap-svg` | `xml`@136 | `xml` | 917 | `.../www.w3.org/1999/xlink">..]>..<svg ... xmlns:x="&ns_extend;" xmlns:i="&ns_ai;" xmlns:graph="&ns_...` |
| `commons-corsica-geographic-map-svg` | `unsupported`@14832988 | `unsupported` | 14832988 | `...ign:end;writing-mode:lr-tb;text-anchor:end" id="tspan3993" sodipodi:role="line" y="179.84268" x=...` |
| `commons-dukesofhohenbergarms-svg` | `unsupported`@26509048 | `unsupported` | 26509048 | `...h355098".         style="clip-rule:evenodd;fill:#000000;fill-opacity:1;fill-rule:evenodd;stroke:...` |
| `commons-gaussian-2d-svg` | `xml`@56 | `unsupported` | 6158 | `...ext_1">..     <!-- ...3 -->..     <g transform="translate(174.5907 158.346516)scale(0.07 -0.07)"...` |
| `commons-geographic-map-of-norway-sweden-finland-and-denmark-svg` | `viewport`@70 | `viewport` | 70 | `...coding="UTF-8" standalone="no"?>.<svg.   width="4600".   height="6056".   version="1.1".   id="s...` |
| `natural-earth-world-map-blank` | `unsupported`@828 | `unsupported` | 828 | `... stroke-width="0.7"/>.  <g clip-path="url(#c)"><clipPath id="c"><path d="M207.2,2L500,2L792.8,2L...` |
| `natural-earth-world-map-countries` | `unsupported`@828 | `unsupported` | 828 | `... stroke-width="0.7"/>.  <g clip-path="url(#c)"><clipPath id="c"><path d="M207.2,2L500,2L792.8,2L...` |

## Formerly blocked files that moved to another verdict

These were blocked by one of the four constructs before; they now fail later, on a construct outside this order.

| ID | Before | Now | Offset | Excerpt (report) |
|---|---|---|---:|---|
| `generated-map_t1` | `unsupported`@915 | `limit` | 0 | `<?xml version="1.0" encoding="UTF-8"?>.<svg xmln...` |
| `generated-map_t2` | `unsupported`@915 | `limit` | 0 | `<?xml version="1.0" encoding="UTF-8"?>.<svg xmln...` |
| `commons-chuaattractormodified-svg` | `xml`@56 | `limit` | 13592242 | `...necap:square;stroke-width:0.5;"/>..   </g>..   <g id="line2d_65471">..    <path clip-path="url(#...` |
| `commons-corvuscoraxiucn-svg` | `unsupported`@56 | `limit` | 0 | `<?xml version='1.0' encoding='UTF-8' standalone=...` |
| `commons-france-osm-mapnik-svg` | `xml`@39 | `path` | 28855 | `... 314.9v22.57h-33.309v-25.254l.219-.023-.109-.426.145-.285.258.074-.012.152.25.137.148-.051.191.2...` |
| `commons-germany-districts-municipalities-location-map-2013-svg` | `xml`@39 | `path` | 455 | `... stroke-width="2">.   <path d="m1475.2 840.81-.4.6-.1.22v.21l-.1.21.1.55v.12l.6-.39 1.6-.75v.28l...` |

## AC-4 check

None of the `unsupported`/`xml`/`viewport` rows above is an unknown non-namespaced attribute, `<meta>`, `<!DOCTYPE>` or `writing-mode` failure:

- `commons-columbiarivermap-svg` (`xml` at 917): the excerpt shows an entity reference (`&ns_extend;`) to an entity declared in the DOCTYPE internal subset. The order requires this to keep failing as XML without expansion. The DOCTYPE itself, at byte 136 in the baseline, is now skipped.
- `commons-corsica-geographic-map-svg` (`unsupported` at 14832988, unchanged): the excerpt contains `writing-mode:lr-tb`, but it is not the cause. A scratch copy outside the repository with every `writing-mode:lr-tb;` removed still failed `unsupported` at the same construct (byte 14822576 in the copy). `writing-mode` in this file is now ignored and reported, and the blocker is the text content at that offset.
- `generated-inkscape_t1/_t2`: the excerpt shows `stroke="url(#g)"`, the stroke paint-server construct the order lists as a non-goal. The root cause was not isolated further.
- Remaining rows (`generated-nested_*`, `commons-1-42-polytope-7-cube-svg`, `clipPath`/`use`/`clip-rule`/transform rows, `viewport` rows, `natural-earth-*`): the excerpt does not show any of the four constructs at the offset; their precise causes were not diagnosed here (out of scope).
