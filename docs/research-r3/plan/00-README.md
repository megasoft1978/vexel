# Project plan: large-SVG workbench

## Index

- [01 Product scope](01-product-scope.md)
- [02 Architecture and C API sketches](02-architecture.md)
- [03 Repository layout](03-repo-layout.md)
- [04 Coding standard and agent guidance](04-coding-standard.md)
- [05 Milestones M0–M14](05-roadmap-milestones.md)
- [06 Benchmark and quality](06-benchmark-and-quality.md)
- [07 Risk register](07-risk-register.md)
- [08 ADRs](08-decisions-adr.md)
- [09 First 30 days](09-first-30-days.md)
- [10 Owner questions](10-open-questions.md)

## One-page vision

Build a C17 desktop editor that opens large SVG documents progressively, gives responsive pan/zoom/selection and light editing, and saves without discarding unknown or Inkscape-specific XML. macOS is first. Linux and Windows use the same custom-drawn UI and normalized events, with small native shims for platform services. A CPU tiled renderer produces pixels presented through Metal first. Animation is a later feature in the same SVG format.

The first success is not “replace Inkscape.” It is a measured user workflow where a real map/scientific/production SVG becomes usable sooner, remains responsive during parse, can be lightly edited, and survives a round-trip intact. Donner is a close existing effort and must be benchmarked at every investment gate.

## Principles

1. **Readable:** straightforward C, small modules, names reveal ownership, no clever macro frameworks.
2. **Safe:** hostile SVG input, bounded work, checked arithmetic, fuzzing, sanitizers, immutable worker snapshots.
3. **Tidy:** stable APIs, source/provenance/license records, generated files marked, each module has an owner and tests.
4. **Fast by evidence:** baseline exact files and operations; optimize the measured bottleneck; preserve performance artifacts.
5. **Small by accounting:** record executable, bundle, dependency and cache bytes separately. C is not a size guarantee.
6. **Preserve source:** unknown SVG/XML survives untouched; diagnostics explain unsupported features; no silent drops.
7. **Same UI:** one paint/layout/accessibility model; platform code only implements window/input/IME/clipboard/dialog/shaping/menu/presentation interfaces.
8. **Incremental:** first visible content precedes full indexing; background work is cancellable and stale work cannot publish.
9. **Accessible from design:** semantic tree and keyboard/focus contracts exist before widget polish.
10. **Scope discipline:** static basic editor first, animation after M10 approval.

## Glossary

- **Source span:** byte range in the original mapped SVG.
- **Element record:** compact structural index entry; not a full decoded object.
- **Overlay:** edits applied over the unchanged source document.
- **Display list:** immutable render commands for a frame/generation.
- **Tile:** fixed pixel rectangle rendered and cached independently.
- **LOD:** level of detail used only for drawing; document/edit data remains complete.
- **TTFP:** time to first visible pixel.
- **MVP:** select, transform, inspect, basic style/path-node editing, undo, safe save.
- **Unknown preservation:** retaining unsupported XML and metadata on read/save.
- **System shim:** narrow OS-owned service adapter; no widget toolkit.
