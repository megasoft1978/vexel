# WO-RUST-SVG-COMPAT-01 result

Branch: worktree-agent-af2e37888bc65a68e. Base 115534eb.

Changed pins: none. The existing tests all pass unedited.

Corpus transitions (39 files, base -> after): 18 ok -> 19 ok, no base-ok regression.
- commons-bulgaria-in-1915-geographic-map-en-svg: unsupported -> ok
- commons-france-osm-mapnik-svg: unsupported -> limit_render
- commons-chuaattractormodified-svg: unsupported -> limit_decode
- commons-gaussian-2d-svg: unsupported -> limit_decode

Gap: symbol clipping is not implemented by the engine (no symbol clip exists in base). overflow hidden/default render unclipped.

Gap: the france blocker is now a render safety limit, not yet investigated.

Gap: reference-crate tests (svg_compat01.rs) were written after the fix, not red first.

Gap: cargo fmt ran after the test runs (formatting only).

Details: svg-compat.md. Sweep reports: sweep-base-report.json, sweep-after-report.json.
