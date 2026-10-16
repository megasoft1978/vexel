# vexel-svg-media

Owned SVG text/tspan and embedded PNG/JPEG algorithms for the complete
WO-RUST-SVG-MEDIA-01 catalog: strict data URI/base64 parsing and limits,
meet/slice/none placement, premultiplied bilinear sampling, entity/whitespace
and font/style/position parsing, contour-preserving outline conversion, and
bounded owned text layout caches.

`render_text_request` and `decode_image_request` consume the accepted typed
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
