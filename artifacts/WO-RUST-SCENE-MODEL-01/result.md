# WO-RUST-SCENE-MODEL-01: owned scene model

Complete named scene DATA MODEL gate. All 19 scene tables and 7 insertion-serial
tables are privately owned, with checked immutable views of source, animation,
children and sparse paints/dashes. Source, source mapping, transforms, path
metadata, strokes, paints/stops, shapes, viewport, images, clips, patterns,
masks, filters/primitives, groups, ignored summaries and text/media records are
implemented. Source and accepted animation share one Arc<Vec<u8>>. SceneGraph is
Send+Sync Rust-only immutable data; no renderer/decoder/native provider stub.

Builder supports every table, source-coherent shape/path/color transactions,
checked geometric capacity growth, explicit retained/transient memory policy,
actual nested capacities, scene-local marks/rollback, stale/cross-scope handles,
sealed child links and complete freeze validation. Group ranges are depth-first,
ordered, disjoint among siblings and contained in ancestors. Bounds, source spans,
media stride/premultiplication, parents, filter dependencies, sparse references,
path metadata, finite coordinates, viewport and reference admission caps check
before publication. Controlled copy/validation loops poll inside traversal.

## API adaptations from the accepted design proposal

- Handles include graph, scene, slot and monotonically assigned insertion serial;
  raw per-edge shape slots stay u32. ElementId is a source-token index that the
  future decoder validates against its token owner. Decoder PatternTicket and
  pattern continuation request are part of the next decoder/kernel contract gate.
- ShapeInput requires exact per-edge colors and a real owned PathDocument. Custom
  geometry can have empty authored path metadata; no artificial color or parser
  output is manufactured by the model. All reserves precede related publication.
- BuildMark is opaque, scene-local and stores every table length plus image-pixel
  count; rollback invalidates prior marks for that scene. Independent child scenes,
  global ignored notices, work charges and allocation history are not rewound.
  The decoder deduplicates ignored token identities before aggregate recording.
- SceneView is a complete borrowed ViewTables with source/animation/ignored and
  checked sealed-child lookup. No lifetime is tied to a temporary view wrapper.
- MemoryStats reports actual vector storage, not allocator overhead, Arc control
  blocks or RSS. Source capacity appears once through animation.storage_bytes().
  History records builder reserves/payload admission, not earlier provider/parser
  allocator calls. Native reserve/Vec bulk moves/drop remain indivisible; no UI
  latency or achieved performance claim follows from these checks.
- Limits are clamped to reference bounds; explicit smaller policies are supported.
  No smaller default byte-policy envelope is introduced. Text request/layout
  validation is data-only; native CoreText/ImageIO implementations remain required.
- Rust Result expresses success; error names preserve legacy spellings and expose
  separate Canceled and detailed limit categories. XmlError data stays typed.

## Review and proof

Independent actual subscription Luna High Codex exec review, with two reused
session rechecks, ended ACCEPT. Initial source-accounting concern was false:
accepted OwnedAnimation.storage_bytes already counts source capacity. The exact
regression proves single inclusion and rejects spare-source capacity beyond the
budget. Move/simple-command edge-range validation and opacity sibling-order
validation were repaired, with focused regressions. Initial findings and all
recheck results/usage/terminal process evidence are preserved; no checks waived.

Fresh named main checks pass: offline local-path lock regeneration; vexel-scene
check; all 27 tests (6unit +21integration); frozen work-order hash; ledger; diff
whitespace; exact protected files/603 legacy sources/157 frozen renderer oracle files/HEAD/index audit. Candidate
source hashes match final review. All source/test files are below800lines. Cargo
changes register only this local crate and existing local dependencies; external
18-package pin graph remains unchanged. No broad UI/E2E or workspace-wide suite.

This adds27 recorded focused test entries to460, totaling487 across28 accepted
main packages. These are named entries, not a claim of487distinct input fixtures,
full SVG/render parity or complete product QA. Full decoder/shared kernel contracts,
rendering, native window/UI/media, actual application/animation/live MCP composition,
large-file editing/save performance, full Rust tooling migration and final quality,
packaged release and recorded website/README demos all remain required.
