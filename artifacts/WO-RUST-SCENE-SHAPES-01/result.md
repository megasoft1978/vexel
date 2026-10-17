# WO-RUST-SCENE-SHAPES-01 result

Branch: worktree-agent-aaad10676a18d42d6. Candidate; not merged.

Constant: `MAX_SHAPES = 262_144`, `MAX_USE_INSTANCES = MAX_SHAPES` (vexel-scene/src/types.rs). Why: 1.47x the 178,589-shape need of generated-use_t2. 1,048,576 rejected: vexel-xml caps one-element-per-shape documents at 1,000,000 nodes, and extrapolated RSS is about 2.9 GB.

Memory (release, macOS max RSS): flat/use/dash synthetic at 262,144 shapes peak 757-785 MB (refused at render); corpus use_t2 855 MB. Per-shape slope 2,731 B; Shape = 320 B; analytic 0.82 GB. Largest external sweep peak 1.36 GB.

Files: types.rs, dash_validate.rs, raster_job_views.rs, raster_job_scratch.rs, validation_tests.rs, vexel-svg-decode use_expansion.rs, two moved pins in vexel-svg-decode tests, two new test files. Full detail: shapes.md.

ACs: 1 pass, 2 pass, 3 pass (boundary tests above), 4 pass (nested refused, 12.9 s debug), 5 pass (122 us), 6 pass, 7 partial, 8 pass, 9 pass (workspace 1965 passed, 5 ignored; +10 tests), 10 pass, 11 pass, 12 pass.

Corpus before -> after (external, 128x128):

| file | before | after |
|---|---|---|
| generated-use_t2 | limit_decode | limit_render (decodes) |
| generated-science_t2 | limit_decode | limit_render (decodes) |
| generated-cad_t2 | limit_decode | limit_decode |
| generated-cnc_t2 | limit_decode | limit_decode |
| generated-inkscape_t2 | limit_decode | limit_decode |
| commons-cambodia-geographic-map-en-svg | limit_decode | limit_decode |

Totals: ok 12 to 12; limit_decode 12 to 10; limit_render 3 to 5; no ok-to-non-ok; no rss_exceeded or crash. In-repo 24/24 ok.

Pins moved (each with boundary test): validation_tests.rs:103; original_references.rs use-instance 65,536/65,537 to MAX_USE_INSTANCES; original_shapes.rs 70,000-path pin to exact MAX_SHAPES boundary.

Gaps:
- Render-stage kind for use_t2 and science_t2 not named (verdict tool collapses it). Candidates: 1,000,000 grid work caps, unconfirmed.
- Four decode-stage files: named limit not established.
- production_decode.rs:453 expansion cap not nested-tested.
- Base workspace count not re-run (static +10).
- Note: reconciliation of LIMITS-01 "Shapes" label: first refusal for use_t2 is the use-instance cap by count.
