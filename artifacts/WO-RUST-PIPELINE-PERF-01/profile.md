# Profile (macOS `sample`, release, 8 s)

Base: whole-scene revalidation per commit (see order). After the shape/edge/segment/geometry
fix, P8000 3592 -> 624 ms (geometry-source validation was the next quadratic term); inkscape_t1
then showed `validate::paint_view`, `SceneData::view` and `affine_valid` (per-commit paint
revalidation) which the paint/clip/group/image/etc. incremental change removed (19.6 s -> 0.18 s).
Remaining slow files: generated-giant_path_t1 (55 s ok): vexel-render `StrokePrims::join_hit_at`
and `segment_hit_at` (render, out of scope); embedded_t2 24 s ok (not profiled).
Hot spots 4 (charge chain) and 5 (scene_segments) are below the 10% threshold after the fix;
no compose/engine edit made.
