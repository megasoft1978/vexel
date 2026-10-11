# Format compatibility and animation (Q7–Q8)

## Format facts and compatibility envelope

- SVG is XML and the editable document is more than its rendered pixels. SVG 1.1 Second Edition remains a W3C Recommendation; the newest SVG 2 snapshot found on 8 Oct 2026 is a Candidate Recommendation Snapshot dated 6 Oct 2026, so do not treat SVG 2 as a universally implemented stable baseline. [SVG 1.1](https://www.w3.org/TR/2011/REC-SVG11-20110816/REC-SVG11-20110816.pdf) · [SVG 2 status](https://www.w3.org/TR/SVG/)
- SVG user agents are expected to retain unknown foreign namespace nodes in DOM while ignoring them for rendering. That supports private metadata in principle, but only if an editor’s XML pipeline preserves it. [SVG 2 structure](https://www.w3.org/TR/SVG/struct.html)
- Inkscape SVG generally contains standard SVG plus Inkscape (`inkscape:`) and Sodipodi (`sodipodi:`) metadata. The `<sodipodi:namedview>` and `inkscape:label`, layer/groupmode, document units/view state, guides, grids, named views and tool state carry editor intent. The project describes Inkscape SVG as SVG plus private namespace data; “Plain SVG” is an export that strips editor-only fields. [Inkscape vs plain SVG](https://wiki.inkscape.org/wiki/Inkscape_SVG_vs._plain_SVG) · [private attributes](https://wiki.inkscape.org/wiki/Inkscape-specific_XML_attributes)
- Layers are SVG groups annotated with Inkscape attributes, not an independent SVG primitive. Preserve namespace URI and local name; prefixes may be changed by serializer without changing namespace semantics, but preserving original prefix/order is more friendly to diffs.
- Live Path Effects (LPEs) are editor-specific. Inkscape stores effect parameters and source path information in private metadata while a normal SVG output path remains renderable. A new editor must preserve the private data and understand common LPEs if it wants continued editability; flattening to an ordinary path keeps appearance but destroys editability. [LPE overview](https://wiki.inkscape.org/wiki/LivePathEffects)
- Preserve SVG filter graphs, `<use>` clones/reference links, markers, text and `<textPath>`, CSS, patterns and gradients, clips, masks, metadata, XMP, ICC references, external image references and embedded image payloads. Browser “renders similarly” is not enough for all editor-specific constructs.
- **Mesh gradients:** Inkscape supports mesh gradients in its own extension of SVG; SVG 2 changes/deferred mesh gradients and they are not a dependable cross-browser feature. Store and render them as first-class editor content, but offer an explicit raster/vector approximation for broad interchange. [SVG 2 changes](https://www.w3.org/2026/07/svg-snapshot/CRS-SVG2-20260722/changes.html) · [Inkscape mesh warning](https://wiki.inkscape.org/wiki/Mesh_Gradients)
- **Flowed text:** legacy Inkscape/Sodipodi flowed text may rely on private namespace constructs and is not safely interchangeable as ordinary SVG text. Preserve original XML; optionally convert on edit to standard `<text>`/`<tspan>` with a warning and reversible backup.
- **SVGZ:** support `.svgz` as gzip-compressed SVG bytes with expansion limits; preserve compression only when saving as SVGZ. Avoid decompression bombs and cap decoded image/filter surfaces.
- **Plain SVG vs Inkscape SVG:** offer both intentionally. Native mode preserves editor metadata. Plain export strips/compiles private metadata, but do not destructively replace the user’s source without an explicit “Save a Copy” style flow. Inkscape LPEs and named view hints can be lost in plain export.
- **Color:** SVG `color-profile` / ICC and modern CSS color syntax do not imply equal support across renderers. Define an explicit color engine and keep source values untouched; convert only in rendering/export. Inkscape 1.5 draft notes its own broad new color system, increasing the parity surface. [1.5 notes](https://wiki.inkscape.org/wiki/Release_notes/1.5)
- **Metadata:** preserve `<metadata>`, RDF/DC/XMP payloads, unknown attributes and processing instructions. Do not normalize or erase data the editor does not understand.

## Loss-minimizing round-trip architecture

“Byte-faithful” has two meanings. Strictly unchanged document sections can be emitted byte-for-byte; once XML is structurally edited, a general serializer may change whitespace, quote style, attribute order, prefixes or entity spelling. XML infoset preservation does not guarantee byte identity. Design for **lexical preservation of unchanged source spans** and **semantic preservation of every node**:

1. Keep original bytes (or mmap read-only) and record source span/token metadata for each element, attribute, text, comment and processing instruction.
2. Parse namespace URI + local name; retain prefix, attribute order, quote style, whitespace, comments and unknown node payloads in an ordered generic XML tree. Do not map unknown elements into lossy typed structs.
3. Maintain editor semantics and renderer caches as derived data keyed to XML node IDs/revision counters. The XML tree is authoritative; editing changes only touched nodes/attributes.
4. On save, splice untouched source spans verbatim where the edit model guarantees nesting and namespace context remain valid; serialize changed subtrees with stable formatting and preserve all unknown child nodes/attributes. If prefix declarations or ancestors changed, safely reserialize affected scope.
5. Keep an “original vs edited” diff and a recoverable backup. Emit warnings when changing feature semantics or flattening unsupported features.
6. Test opening and saving fixtures in both Inkscape and browsers; compare parsed XML semantics, visual raster output and exact bytes for untouched subtrees.

This is more robust than storing only a render scene graph. Inkscape itself has a generic XML representation tree separate from a typed object tree. [architecture](https://wiki.inkscape.org/wiki/Architectural_overview) · [XML subsystem](https://wiki.inkscape.org/wiki/XML_subsystem)

## Animation authoring design

### Output choices

| Representation | Advantages | Compatibility / downside | Recommendation |
|---|---|---|---|
| **SMIL SVG animation** (`<animate>`, `<animateTransform>`, `<animateMotion>`) | SVG-native, time-based, can animate many SVG attributes, embeds in the same SVG; broad current browser support according to MDN/Can I Use. [MDN](https://developer.mozilla.org/en-US/docs/Web/SVG/Guides/SVG_animation_with_SMIL) · [compat](https://caniuse.com/svg-smil) | Timing/event/sync complexity, browser interpolation differences and editor support gaps. Do not promise every SMIL feature. | Primary export for declarative SVG-compatible keyframes and transforms.
| **CSS animations/keyframes** | Standard `<style>` plus CSS; good for opacity, transforms and stylable properties; browsers can optimize. | Only animates CSS properties, syntax/cascade/media queries add complexity; cannot express all SVG attributes or SMIL timing. | Secondary export for CSS property tracks; preserve original stylesheets byte-for-byte unless user edits them. |
| **Web Animations API / JavaScript** | Powerful runtime control and interactivity. | Script execution disabled in some viewers; not a static declarative SVG-only result; security and portability risks. | Optional web publish/export with clear “script required” label, never default native save. |
| **Lottie / dotLottie** | Established runtime ecosystem for app motion, compact playback assets. | JSON is not SVG; only supported shapes/features translate and round-tripping loses SVG-specific features. | Export target for subset; never source of truth. |
| **Custom `anim:` namespace model** | Rich editable timeline metadata that a capable editor can preserve; stores timing, easing, channels, markers and editor UI state. | Stock Inkscape/browser may ignore animation if only custom metadata is present. A foreign namespace is not a standardized animation runtime. | Store private authoring metadata in `<metadata>` or namespaced elements, but compile supported tracks to standard SMIL/CSS at save. Keep both representations synchronized. |

**Browser support:** as checked 8 Oct 2026, MDN calls SVG `<animate>` well-established, and Can I Use reports broad modern Chrome, Edge, Firefox and Safari support, while Opera Mini/legacy browsers are exceptions. This is element-level support, not proof every animation attribute/interpolation feature behaves identically. [MDN `<animate>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Reference/Element/animate) · [Can I Use](https://caniuse.com/svg-smil)

**Inkscape round-trip today:** official project docs say Inkscape has no animation authoring support; an issue titled “Inkscape does not know how to deal with SVG `<Animated>`” documents failures on animated content. Generic XML preservation exists in architecture, but these sources do not establish that arbitrary SMIL or CSS animations survive Inkscape load/save intact. Therefore treat preservation as **unknown and test empirically**; do not promise round-trip compatibility through stock Inkscape. [Inkscape SVG overview](https://inkscape.org/en/develop/about-svg/) · [reported animated SVG issue](https://gitlab.com/inkscape/inbox/-/issues/10473) · [XML subsystem](https://wiki.inkscape.org/wiki/XML_subsystem)

### Recommended model and UI

- Timeline is **time-based**, with selectable frame-rate guides for preview/export, not frame-only storage. Store seconds or rational time units to avoid drift; show frame ticks as a view.
- Tracks are per object/property: transform components, opacity, fill/stroke color, supported geometry/path data, visibility, and optional marker events. Keep object IDs stable; display scene/layer hierarchy alongside timeline.
- Keyframes support hold, linear, cubic Bézier easing; expose motion path and transform origin. Start with transform, opacity, and colors; add path morphing only with compatible path topology and clear rejection/normalization behavior.
- Preview by evaluating animation into an immutable render snapshot; do not mutate the canonical XML on every playback tick. Scrubbing should update only affected render nodes/tiles.
- Use one animation model with compiler backends: SMIL first, CSS second. If a track cannot be represented losslessly, show a per-track export diagnostic and preserve the source metadata.
- Store canonical custom authoring info in a uniquely named namespace (e.g. `urn:product:animation:1`) attached to document metadata plus standard animation output. Do not use an unregistered prefix as if it were a standard namespace; namespace URI is the identity.
- Default static fallback is the frame at document time 0, with an explicit optional fallback time. Do not change the static fallback when playback is disabled.
- Animation editor should coexist with normal layers and object tools: select object in canvas → its animation tracks appear; timeline keyframes are undoable with ordinary document edits.

**Prior art lessons:** Glaxnimate demonstrates an editor that imports/exports animated SVG and Lottie; Sozi demonstrates a focused presentation animation tool built around SVG; Synfig uses a native animation model and imports SVG; Rive optimizes for interactive state-machine animation. None removes the need to specify exactly which features can be emitted as portable SVG. [Glaxnimate formats](https://docs.glaxnimate.org/en/formats.html) · [Sozi](https://github.com/sozi-projects/Sozi) · [Synfig SVG import](https://wiki.synfig.org/Doc%3ASvg2synfig) · [Rive editor](https://www.rive.app/editor)
