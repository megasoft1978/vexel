# Continuation 10 independent review

Result: focused review PASS with a test-scope caveat.

`publication.rs` calls the paint resolver before the empty-edge `Unsupported` fallback and reports the paint value's source offset. That ordering matches frozen C `src/svg/paint/paint.c:553-691` and the zero-area gradient assertion in `tests/svg/shape/test_shape.c:613-626`. The production-path regression checks the exact error offset and confirms rollback leaves zero edges, shapes, paints and stops.

The injected malformed/limit cases do not parse a malformed gradient or enforce a real PaintStops limit. `ProductionFixture::resolve_cached` performs reference/template lookup and then returns the configured error. These cases prove resolver-error precedence and rollback ordering only. Candidate evidence correctly avoids claiming production parser/limit parity.

Decoder continuation 10 remains incomplete: the exact cache oracle is separately accepted in `WO-RUST-TILE-CACHE-ORACLE-01`; accepted paint/definitions composition and whole-render output/memory gates remain open.
