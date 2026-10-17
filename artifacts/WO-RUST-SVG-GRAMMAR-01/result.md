# WO-RUST-SVG-GRAMMAR-01 result

Status: PARTIAL. Fixed B and C. A, D, E blocked (stop per order). 

## Corpus
Tool: `build/cargo-target/aarch64-apple-darwin/release/vexel-corpus run --manifest ~/Desktop/Freelance/vexel-corpus-real/manifest.json --pins rust/tools/vexel-corpus/pins.json --out <dir> --only <id>` (build: `cargo build --release --offline --locked -p vexel-corpus`). External corpus read-only.

| id | before | after |
|---|---|---|
| generated-map_t1 | xml (invalid value @0) | ok |
| generated-nested_t1 | xml (XML @16843, DepthLimit) | limit_decode |
| commons-1-42-polytope-7-cube-svg | xml (XML @15241908) | limit_decode |
| commons-germany-...-2013-svg | xml (path data @474) | xml (unchanged, A blocked) |
| commons-france-osm-mapnik-svg | xml (path data @28855) | xml (unchanged, A blocked) |
| commons-aegean-...-basemap-svg | xml (viewport @70) | xml (unchanged, D blocked) |
| commons-columbiarivermap-svg | xml (XML @917) | xml (unchanged, E blocked) |

In-repo corpus: 24/24 ok. `pipeline_hashes`: 2 passed.

## Failing-first outputs (pre-fix)
- A: `M0 0l-.1.21.1.55z` -> `Err(Diagnostic { kind: Path, offset: 11 })`, expected `Ok([0.0, 0.0, -0.1, 0.21, 0.1, 0.55])`.
- B: 200 and 2000 `<path data-region>` and also `class`-only -> `InvalidArgument`.
- C: `Xml(DepthLimit)` vs `Limit(Depth)`; `Xml(NodeLimit)` vs `Limit(Nodes)`; `Xml(AttributeLimit)` vs `Limit(Attributes)`; `Xml(TextLimit)` vs `Limit(TextBytes)`.
- D: `Err(())` vs `Ok((7409, 2600))`.

## B (fixed): root cause
Not the `ignored.rs` visited index (order suspect). Count cliff hits any element count ~131+ (class-only too). `decode.rs` no-task tail ran `publish` and `finalize_animation` as two atomic steps; the second became a skipped boundary, marking Complete with an un-finalized scene (`decode_initialization.rs` require_finalized, via builder freeze). Fix: one atomic step for flush+publish+finalize (loop guarantees budget). Splitting into two atomics double-counted ignored attributes (400 for 200), so one step is required.
Tests: `vexel-svg-decode/tests/unknown_data_attributes.rs` (2, new).

## C (fixed)
`vexel-scene/src/lexical_document.rs` maps DepthLimit, NodeLimit, AttributeLimit, TextLimit, InputLimit to `Limit(Depth|Nodes|Attributes|TextBytes|SourceBytes)`. Limit values unchanged. No existing test pinned `Xml` for these five, so none updated. Tests: `vexel-scene/tests/xml_limit_kinds.rs` (5, new; node test uses 1,000,010 elements because the lexical default is MAX_NODES=1M, not 250,002).

## A (BLOCKED, not changed)
Accepting `.`-start implicit groups conflicts with frozen pins: `rust/fixtures/legacy/path/baseline-golden.txt` `CASE grammar-7` (`M0 0 L.2.3.4.5`) = `ERROR Path 10`, same site as the sample `M0 0l-.1.21.1.55z`. Also `number_separator.rs::integer_fraction_adjacency...` pins `M0 0 L1.2.3 4 5` as error, which the spec says is valid. AC2 and AC9 cannot both hold. Owner decision needed.
Archived failing test: `artifacts/WO-RUST-SVG-GRAMMAR-01/blocked-A-implicit_dot_repeat.rs.txt`.

## D (BLOCKED, reverted)
Removing the 4096 cap fixes aegean's root but breaks the frozen `vexel-svg-decode/tests/original_css.rs:95` (`height="5000"` -> Viewport). Changing it is a frozen-test edit not authorized by the order. Reverted. Raster limit check: `vexel-raster::MAX_AXIS = 4096` returns typed InvalidArgument (confirmed). With D applied, aegean moved to `number at byte 5673` (`<image x="0">`, embedded 4649x1274 PNG); not diagnosed further.
Archived test: `artifacts/WO-RUST-SVG-GRAMMAR-01/blocked-D-root_viewport_size.rs.txt`.

## E (BLOCKED, no change)
`Diagnostic`/`ErrorKind::Unsupported` carry no entity name (order blocker rule). Also frozen `vexel-svg-decode/tests/original_ignored.rs::original_doctype_entity_and_position_fail_as_xml` pins DOCTYPE entity refs as `Xml`.

## Verification
- `cargo +stable test -p vexel-svg-path|vexel-xml|vexel-svg-decode|vexel-svg-values|vexel-scene --locked --offline`: pass.
- `cargo +stable test -p vexel-app --test pipeline_hashes`: 2 passed.
- Workspace (`--no-fail-fast`): only failure `vexel-svg-scalar-index/tests/original_suites.rs:124` (2097152 vs 524288), reproduced on HEAD sources, pre-existing.
- `python3 scripts/check_rust_fmt.py`: pass.
- Frozen `cargo fmt --check` on 5 crates: fails only on pre-existing drift in untouched lines of vexel-svg-path, vexel-xml, vexel-scene (not files I changed).
- `scripts/wo_validate.py --ready`: valid. `git diff --check`: clean.
- Parity golden and `vexel-svg-path` sources: untouched.
