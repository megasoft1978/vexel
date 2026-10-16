# WO-RUST-SVG-ANIMATION-01 candidate

Implemented the safe, standard-library SMIL parser/evaluator with borrowed source bytes and owned bounded track data. The candidate covers the named C parse/evaluate API, original source spans, supported track grammar, timing and syncbase behavior, numeric/color/transform/path/motion values, diagnostics, and the 15 authored fixtures including both motion fixtures. No dependencies, manifests, peers, legacy files, or shared records were changed.

Review fixes: malformed target base values now emit one `InvalidTrack` at the animation element-name offset. Duplicate local `mpath` IDs now reject even when the first matching path lacks `d`, yielding `UnsupportedPath` at the animation element-name offset. Both focused regressions assert exact diagnostic kind/offset and that the unsupported track is omitted.

Additional tests cover direct absolute `end`, stored additive/accumulate flags, repeat iteration, and the legacy evaluator's numeric samples for additive base value and repeat accumulation. The tests assert current overlay samples and track flags independently; they add no renderer composition behavior. Existing bounds, source-span, sync-cycle, unsupported timing, interpolation, and motion checks remain in place.

The render-frame and render-animation assertions remain pending the later renderer/CLI port. This is an uncommitted candidate for independent recheck; it does not claim renderer or app integration.
