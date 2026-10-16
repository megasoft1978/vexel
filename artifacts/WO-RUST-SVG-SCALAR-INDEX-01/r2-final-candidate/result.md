# R2 legacy API review follow-up

The public Vec compatibility namespace/root reexports were removed from normal builds. Historical Vec-backed entry points, their legacy-only helpers/constants and raw Vec reserve helper are now compiled only under `cfg(test)`. The public allocation-free `fill_color(SceneView, ShapeId)` retains the required `raster_fill_scanline_cache_color` production behavior. Shared allocation-free geometry, heapsort, AVL and budget helpers remain compiled for the admitted owning-job implementation. All original unit implementations and assertions are preserved; integration suites already use owning-job APIs. README and parity-map describe the restricted surface.

Fresh focused check/test use the candidate-local offline Cargo registry recorded in prior R2 evidence. The default registry fails resolving unrelated `objc2-image-io` before compile. All 37 focused tests pass. Test builds report historical helper dead-code/unused-import warnings because the old full public surface is no longer exported. Work order validation and scoped formatting pass. No manifest, lock, peer source or main checkout source was changed.

Candidate only: independent review and fresh main checks remain required. Full renderer/native provider/app/UI/release/demo gates remain open.
