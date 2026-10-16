# 03 — SVG parsing, large files, round-trip (Q5, Q8)

## Storage architecture

Use `mmap`/read-only mapping on platforms where available, but allow a streaming fallback. Keep original UTF-8 byte buffer immutable for the lifetime of a document; parser indexes token/source spans into it. Do not mutate mapped bytes. Build compact node/attribute arrays in arenas, not one malloc per XML item:

```c
typedef struct { uint32_t off, len; } StrSpan;       // bytes in source or intern table
typedef struct { uint32_t parent, first_child, next_sibling;
                 uint32_t name_id, attr_begin, attr_count;
                 uint32_t src_begin, src_end, flags; } XmlNode;
typedef struct { uint32_t name_id; StrSpan value; } XmlAttr;
```

Store nodes/attrs in chunked SoA or compact arrays; retain original spans and namespace-qualified names. Intern frequent element/attribute names, but avoid interning arbitrary long text/path strings. Parse path/style/transform/length values lazily and cache decoded data by node generation. Build scene nodes only for renderable/interactive content; keep unsupported subtree opaque. Add strict depth/entity/resource limits; disable external entities and remote-resource loads by default.

### Parser design choices

- Single pass XML tokenizer: quote-aware tag boundary scanning, UTF-8 validation, comments/CDATA/PI/DOCTYPE handling per explicit policy; record token/source spans, stack parent indices, namespace declarations. SVG files generally need namespace handling; namespace prefixes are arbitrary.
- A SIMD “structural index” stage can accelerate scanning for `<`, `>`, quotes, `&`, whitespace, but JSON simdjson speeds are **not transferable to XML**. XML nesting, quoted delimiters, entities and UTF encodings change the problem. Benchmark an ASCII fast path with scalar correctness fallback.
- Streaming parse and mmap are not mutually exclusive: map input, parse while building compact arrays, free decoded transient data; progressive scene compile can begin once a subtree is closed. Large root-level siblings can render before EOF if the format is not malformed.
- Avoid full generic DOM. pugixml documents a markup-heavy 2.1 MB sample using 2.1 MB source buffer + 8.3 MB document structures on 64-bit; it also offers a more compact mode with speed tradeoff. [pugixml manual](https://pugixml.org/docs/manual.html) Its parser comparison uses 1–20 MB files and minimum repeated parse time, not 500 MB SVG editor sessions. [benchmark](https://pugixml.org/benchmark.html)
- simdjson reports GB/s on JSON and explains allocation can dominate (their docs mention ~1.4 GB/s allocation behavior on some systems); use as motivation to reduce allocations, not an SVG/XML performance claim. [simdjson](https://github.com/simdjson/simdjson) · [performance notes](https://github.com/simdjson/simdjson/blob/master/doc/performance.md)
- Native XML libraries: Expat SAX, libxml2, pugixml, RapidXML, yxml differ in validation/DOM/encoding/support. If strict no-vendor means own parser, still use differential fuzzing against multiple established parsers. Do not claim a parse GB/s target until measured on actual SVG fixtures.

## CSS and SVG subset

Implement the highest-value static subset in explicit phases:

1. Presentation attributes + inline style declarations; property tables; inherited properties/defaults; `currentColor`, `inherit`, opacity, fill/stroke, stroke width/cap/join/miter, fill-rule, display/visibility, font basics.
2. `<style>` blocks with tokenizer/declaration parser; selectors: type, universal, class, ID, descendant/child, comma lists, supported pseudo-classes as documented. Specificity sort + source order. Unsupported selectors are preserved and flagged.
3. CSS variables/custom properties and `var()` resolution after cascade, with cycle detection; CSS units/font-relative units and viewport percentages resolved against context.
4. Transform lists, viewBox/preserveAspectRatio, nested `<svg>`, `symbol` and `use` references with cycle/depth guards; paint servers gradients and stop styles.
5. ClipPath, masks, patterns, markers. Then filters (blur, offset, color matrix, composite/blend) with bounded offscreen regions. External image links optional and explicit; no scripting/foreignObject in MVP.

For path data, one tokenizer must correctly handle sign-separated floats (`10-3`), exponents, omitted commas, repeated command parameters, smooth control points, arc flags and close-path. Parse relative coordinates while carrying current point/subpath origin. Use locale-independent float parse (C `strtod` is locale-sensitive and often slow); writing can use a shortest-round-trip algorithm such as Ryu. Eisel-Lemire-style fast parsing may be adapted, but no maintained canonical C implementation specifically for SVG was established here. Unknown: relative speed across target CPUs.

## 500 MB / 1 GB strategy

File size is a poor proxy. A file of long path numbers can have modest node count; base64 images can dominate bytes and decoded RAM; a million tiny elements can make indexes dominate. Track both encoded and expanded size.

- Document state: immutable mapped/original byte store; compact token arrays; scene-index pages allocated on demand; no per-element heap object. Report estimated bytes per XML node after prototype; do not predeclare a realistic number without profiling.
- Loading: parse in worker; publish completed subtrees; display progress and first viewport as soon as viewBox and visible scene bounds are available. When global styles or `<use>` dependencies arrive later, invalidate affected nodes.
- Spatial index: R-tree/BVH over conservative object bounds; separate layers/order index for painter’s order; special dependency graph for clip/mask/pattern/use. Build lazily and incrementally. Avoid index work for offscreen giant documents until needed.
- Rendering: visible tiles + LOD by zoom. At low zoom replace dense groups with cached tile/simplified geometry; preserve exact geometry at high zoom/selection. Screen-space simplification is render-only. Tile cache uses a memory budget and LRU; cancel stale pan/zoom jobs.
- Editing: document edits produce operations and local bounds invalidation; don't rebuild whole scene. A huge single path still requires full path parse for editing; tokenize path subranges or make parse cancellable. Render coarse preview while decode proceeds.
- Save: patch dirty spans when structural conditions permit; otherwise serialize changed subtrees and preserve untouched bytes. Atomic temp-write + rename; autosave operation log and recovery checkpoint.
- Very large untrusted inputs: configurable memory cap, maximum recursion/reference chain/filter extent, graceful “partial viewer” mode, no external entity expansion/network fetch.

## Round-trip, undo and recovery

Preserve source spans for untouched nodes; dirty flags identify exact start/end ranges. If edits alter one attribute, rewrite the start tag while preserving other attribute order/quotes where practical; if namespaces, inherited CSS or child order are touched, reserialize that subtree. Guarantee semantic preservation first; byte-identical output for untouched spans second. Track namespace bindings and original encodings/newlines. Build corpus assertions that unknown nodes, metadata, comments, IDs, styles, and editor namespaces survive open/save.

Undo/redo is an operation log, not whole-document copies: each operation stores stable node ID, before/after values or geometry, inverse, source spans and generation. Allocate undo records from chunked arenas; checkpoints bound replay. Autosave uses periodic compacted snapshots plus log; recovery scans valid records after crash. Keep save-as and source file backups explicit. Animation later attaches tracks to stable IDs and maps tracks to SMIL/CSS output; app-private namespace only for metadata not representable in standards. Round-trip private attributes unchanged.

## Public benchmark corpus and known sizes

| Corpus/source | What it stresses | Known published facts / URL |
|---|---|---|
| **W3C SVG 1.1 Full test suite** | Semantics, many small feature fixtures, renderer correctness | 445 tests (2011 release), archive links from [W3C overview](https://www.w3.org/Graphics/SVG/Test/Overview.html). Not a large-file stress set. |
| **resvg test suite** | Real-world SVG rendering coverage and edge cases | 1,338 test files are represented in Wikimedia’s 2021 comparison. [Corpus repo](https://github.com/linebender/resvg/tree/main/tests) · [MediaWiki overview](https://commons.wikimedia.org/wiki/User:JoKalliauer/SVG_test_suites). Exact current aggregate bytes were not established. |
| **Wikimedia SVG benchmarks / featured pictures** | Real authored SVG diversity and render correctness/speed | Public comparative suites and 2021 timing table: [MediaWiki SVG benchmarks](https://www.mediawiki.org/wiki/SVG_benchmarks). |
| **Natural Earth / QGIS world map** | GIS boundary paths, many vertices/groups; Natural Earth source data scales | Natural Earth publishes vector themes and a 219 MB quick-start kit, but not a standard ready-made 1 GB SVG. A Wikimedia Natural Earth world map SVG is 1.96 MB. [Downloads](https://www.naturalearthdata.com/downloads/) · [map file info](https://commons.wikimedia.org/wiki/File:NED_worldmap_110m.svg). Generate deterministic 10/100/500 MB SVGs from shapefiles and record recipe. |
| **OpenStreetMap exports** | Dense roads/labels, extensive path geometry, many objects; stress clipping and text | No stable public SVG export with validated fixed size found; OSM exports are often other formats and hosted vector tiles. Unknown specific fixed-size public test. Generate via QGIS or `osmium`/map style and record source snapshot, license and converter. |
| **Wikimedia Commons huge SVGs** | Hand-authored maps/diagrams and occasionally base64 images | Searchable category but file sizes vary and originals can change; pick and pin exact revision URL/hash. No authoritative large-file benchmark artifact named in this pass. |
| **Inkscape testfiles** | Inkscape features, editor metadata and regression cases | Project test tree [Inkscape testfiles](https://gitlab.com/inkscape/inkscape/-/tree/master/testfiles); obtain license/fixture review before redistribution. |
| **Generated stress suite (must build)** | 1M tiny rects; 100k text labels; 1M-segment single path; 100 MB path-data; 500 MB embedded base64; deep use/clip; gradient/filter flood | Deterministic generator + recipe should be checked in, outputs not necessarily. This is the only reliable way to create fixed 1 GB tiers. |

No verified one-gigabyte public SVG download with stable version and known provenance was located. Unknown. Do not use random giant downloads as a benchmark baseline. “Large SVGs” come from GIS/cartography, CAD conversion, plotting/diagram tools, scientific traces, generated maps and embedded image packaging; each stresses a different cost dimension. Size actual fixtures and check in a manifest: source bytes, element count, path commands, max depth, text chars, image count/pixels, filters and generator hash.
