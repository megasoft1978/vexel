# vexel-svg-media

Owned SVG text/tspan and embedded PNG/JPEG algorithms for the complete
WO-RUST-SVG-MEDIA-01 catalog: strict data URI/base64 parsing and limits,
meet/slice/none placement, premultiplied bilinear sampling, entity/whitespace
and font/style/position parsing, contour-preserving outline conversion, and
bounded owned text layout caches.

`publish_text_request` and `publish_image_request` are the production routes.
Both REQUIRE a public `StylePublisher` implementing the real decoder common
style pipeline. There is no default implementation or neutral fallback. Owned
display preflight skips before callbacks/providers. Image hidden/collapse skips
its provider and geometry callbacks after traversal begin. Text hidden/collapse
still parses fonts, lays out, admits glyphs/document commands, checks positions
and word spacing, and advances cursor. Only its actual custom geometry is
skipped, per outlined run. Invalid visibility is ignored for no-outline text;
font/layout/position/word-spacing errors keep their original earlier priority.
The required begin/prepare/admitted-prepare/publish/finish callbacks receive original style/source
spans, effective end, geometry transform, independent host mapping, read-only
`CustomGeometry`, and the real SceneBuilder/DraftToken. `publish` returns only
`PublishedStyle` (shape styling and edge colors); it receives no admission token
and no success shape ID. The crate verifies callbacks did not append shapes,
retains the validated host map and image handle, assembles `ShapeInput` from its
original geometry, consumes its private admission, and finalizes the returned
shape. Callback failures share request rollback.
Common paint/stroke/opacity/clip/definition effects are explicit adapter duties,
with exact ordering and ownership in `src/publication.rs` and the work artifact
`style-publication-contract.md`. Fixture-only delegates exercise actual Scene
colors, stroke, clips and groups, without a fake native renderer.

Production shape admission now uses the accepted Scene lease after
`prepare_geometry` validates fill/currentColor and fill-rule. The crate reserves
one slot at the canonical element name offset, then calls the required
`prepare_admitted_geometry` for fill-opacity/stroke validation. Image admission
precedes media parsing/provider; text admission follows the original layout,
glyph/document budget, positions, spacing and visibility checks and precedes
outline geometry. The admission never enters `CustomGeometry` or a callback;
the crate consumes it with `append_admitted_shape`. Empty text geometry cancels
its lease. Generated glyph/image paths have no canonical
`<path d>` proof; host annotations remain independent of geometry transforms.
No `admit_shape_with_source` proof is fabricated for these media kinds.

Errors, cancellation, failed commit and unwinding panics roll back the complete
request including pending admissions. Text also restores state/cache on panic
before resuming unwinding. Panic-abort builds do not offer unwind recovery.
The geometry-only image fixture helper retains ordinary append for historical
late-failure assertions; it is excluded from production composition.

`render_text_request` and `decode_image_request` are geometry-only compatibility
fixture helpers; they do not implement common paint/style publication and MUST
NOT be used by production composition. They consume the accepted typed
requests and validate the canonical source, host `SourceMap`, ledger and scene
before provider work. Each operation uses the caller's real active DraftToken
with `begin_request`, `commit_request` and unconditional `rollback_request` on
any operation or commit failure. `append_image_plan` uses the same transaction.
Output shapes retain the host annotation independently of geometry transforms.
No caller rollback is needed to repair a failed request: the scene prefix and
outer token remain usable. Text failures additionally restore every TextState
field and remove new cache admissions while preserving earlier owned keys and
layouts. Vector capacity and allocation/work history may remain; no allocator
or RSS rollback claim is made.

Every successfully validated layout use, including cache hits, calls the accepted
`SceneBuilder::admit_text_commands` at the original run offset after glyph
admission checks. This is the original cumulative 4 Mi document budget. It is
separate from the 262144-command/256 KiB/32-entry cache limits. Accepted Scene153
keeps TextState unchanged and holds this document counter on SceneBuilder;
request rollback restores its checkpoint.

All eight historical defect tests now require correct request cleanup or
command-limit behavior. Additional tests interrupt every observed WorkControl
call boundary through request commit for canceled/work/arithmetic stops and
exercise late provider errors/malformed layouts. The assertion transitions are
recorded in artifacts/WO-RUST-SVG-MEDIA-01/assertion-transition.md. Earlier blocker
reports and logs remain historical evidence, superseded by the integration seed
and the current result/parity/coverage/verification reports.

Providers must be explicitly supplied; there are no production fixture fonts or
pixels. Fixture providers exist only in tests. Native CoreText/ImageIO, the full
mixed CLI/C renderer/selection suites, CoreText/librsvg comparison and final
application/runtime/release/UI proof remain later mandatory integration gates.
Actual global allocator OOM fault injection remains unexecuted; real capacity
admission failures and provider OOM cleanup are exercised with safe Rust.


Text family, position, XML entity, and whitespace scans charge WorkControl per
scanned scalar/byte; focused tests stop inside each scan and check exact
diagnostics. Malformed/incomplete UTF-8 is validated before slicing or copying.
Production outline conversion uses the authored run offset through
`text_path_geometry_at`; later cubic cancellation/limit and invalid-geometry
tests assert that offset. Fallible edge and subpath reserve failures retain the
authored offset. A fixture publisher also proves ordinary equal-sized append is
rejected, rolled back, and followed by same-draft retry.
