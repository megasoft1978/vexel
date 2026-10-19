# Migration reconciliation candidate

Evidence-only candidate; **not migration closure**. `row-map.jsonl` accounts for every input row individually and retains source line numbers. Frozen input hashes and counts are recorded in `summary.json`.

## Disposition

- Rows mapped: **1610** (603 legacy baseline + 1,007 current additions).
- Accepted replacement evidence: **0 rows**. The frozen inventory itself reports zero accepted replacement rows; its result says no complete per-file parity map could be substantiated.
- pending: **609**.
- reference_historical: **308**.
- unresolved: **693**.
- Legacy C/Objective-C remains preserved and pending port. Four current `rust/fixtures/legacy/c-raster/*.c` copies are classified as reference data only; they are not runtime ports. Current shader/corpus/fixture assets retain their historical-data classification.
- Rust additions are `unresolved` against the baseline until exact source behavior/oracles are linked to an accepted work order, result, and Rust implementation path. This avoids treating source presence or similar names as parity.

## Follow-up groups

Group keys are path-prefix buckets for future scoped reconciliation orders, not ownership approvals. Each row remains individually listed in `row-map.jsonl`.

| Path group | Pending | Reference/historical | Unresolved |
|---|---:|---:|---:|
| `app/editor_chrome_render.c` | 1 | 0 | 0 |
| `app/editor_command_overlay_render.c` | 1 | 0 | 0 |
| `app/index_bench.c` | 1 | 0 | 0 |
| `app/index_replay.c` | 1 | 0 | 0 |
| `app/index_replay.h` | 1 | 0 | 0 |
| `app/main.c` | 1 | 0 | 0 |
| `app/spatial_cache_bench.c` | 1 | 0 | 0 |
| `app/test_driver` | 4 | 0 | 0 |
| `app/ui_demo.c` | 1 | 0 | 0 |
| `app/ui_demo.h` | 1 | 0 | 0 |
| `app/ui_demo_render.c` | 1 | 0 | 0 |
| `app/ui_focus_bench.c` | 1 | 0 | 0 |
| `app/vexel_mcp_main.c` | 1 | 0 | 0 |
| `bench/corpus` | 0 | 24 | 0 |
| `bench/generate` | 3 | 0 | 0 |
| `bench/harness` | 4 | 0 | 0 |
| `dev` | 1 | 0 | 0 |
| `packaging/__init__.py` | 1 | 0 | 0 |
| `packaging/vexel_packaging.py` | 1 | 0 | 0 |
| `reports/M0` | 1 | 0 | 0 |
| `reports/M3` | 2 | 0 | 0 |
| `reports/M4` | 2 | 0 | 0 |
| `resources/AppIcon.icns` | 0 | 1 | 0 |
| `rust/crates` | 0 | 8 | 673 |
| `rust/fixtures` | 1 | 275 | 2 |
| `rust/tools` | 0 | 0 | 18 |
| `scripts/bench_vs_inkscape.sh` | 1 | 0 | 0 |
| `scripts/check_ci.py` | 1 | 0 | 0 |
| `scripts/check_fuzz_workflow.py` | 1 | 0 | 0 |
| `scripts/check_rust_fmt.py` | 1 | 0 | 0 |
| `scripts/corpus_render_report.py` | 1 | 0 | 0 |
| `scripts/demo_run.sh` | 1 | 0 | 0 |
| `scripts/doctor.py` | 1 | 0 | 0 |
| `scripts/e2e_svg.py` | 1 | 0 | 0 |
| `scripts/fetch_corpus.py` | 1 | 0 | 0 |
| `scripts/fuzz_smoke.py` | 1 | 0 | 0 |
| `scripts/golden_diff.py` | 1 | 0 | 0 |
| `scripts/install_smoke.py` | 1 | 0 | 0 |
| `scripts/ledger.py` | 1 | 0 | 0 |
| `scripts/live_check.sh` | 1 | 0 | 0 |
| `scripts/make_icon.py` | 1 | 0 | 0 |
| `scripts/make_sbom.py` | 1 | 0 | 0 |
| `scripts/perf_gate.py` | 1 | 0 | 0 |
| `scripts/pixdiff.py` | 1 | 0 | 0 |
| `scripts/quick_test.py` | 1 | 0 | 0 |
| `scripts/sanitizer_probes.py` | 1 | 0 | 0 |
| `scripts/soak.py` | 1 | 0 | 0 |
| `scripts/test_runner.py` | 1 | 0 | 0 |
| `scripts/wo-run.sh` | 1 | 0 | 0 |
| `scripts/wo-verify.sh` | 1 | 0 | 0 |
| `scripts/wo_driver.py` | 1 | 0 | 0 |
| `scripts/wo_validate.py` | 1 | 0 | 0 |
| `scripts/xml_fuzz.py` | 1 | 0 | 0 |
| `scripts/xml_fuzz_campaign.py` | 1 | 0 | 0 |
| `src/app` | 109 | 0 | 0 |
| `src/base` | 2 | 0 | 0 |
| `src/document` | 16 | 0 | 0 |
| `src/geom` | 2 | 0 | 0 |
| `src/jobs` | 4 | 0 | 0 |
| `src/mcp` | 25 | 0 | 0 |
| `src/platform` | 50 | 0 | 0 |
| `src/project` | 25 | 0 | 0 |
| `src/raster` | 27 | 0 | 0 |
| `src/spatial` | 2 | 0 | 0 |
| `src/svg` | 61 | 0 | 0 |
| `src/ui` | 20 | 0 | 0 |
| `tests/__init__.py` | 1 | 0 | 0 |
| `tests/anim` | 9 | 0 | 0 |
| `tests/app` | 16 | 0 | 0 |
| `tests/base` | 1 | 0 | 0 |
| `tests/bench_raster_culling.py` | 1 | 0 | 0 |
| `tests/bench_shape_raster.py` | 1 | 0 | 0 |
| `tests/document` | 2 | 0 | 0 |
| `tests/e2e` | 9 | 0 | 0 |
| `tests/edit` | 3 | 0 | 0 |
| `tests/editor` | 27 | 0 | 0 |
| `tests/fuzz` | 2 | 0 | 0 |
| `tests/jobs` | 1 | 0 | 0 |
| `tests/mcp` | 12 | 0 | 0 |
| `tests/platform` | 5 | 0 | 0 |
| `tests/platform_media` | 3 | 0 | 0 |
| `tests/project` | 10 | 0 | 0 |
| `tests/raster` | 1 | 0 | 0 |
| `tests/realworld` | 3 | 0 | 0 |
| `tests/svg` | 39 | 0 | 0 |
| `tests/test_benchmark_harness.py` | 1 | 0 | 0 |
| `tests/test_boot05.py` | 1 | 0 | 0 |
| `tests/test_corpus_fetch.py` | 1 | 0 | 0 |
| `tests/test_corpus_render_report.py` | 1 | 0 | 0 |
| `tests/test_dev.py` | 1 | 0 | 0 |
| `tests/test_document_model.py` | 1 | 0 | 0 |
| `tests/test_golden_diff.py` | 1 | 0 | 0 |
| `tests/test_golden_oracle.py` | 1 | 0 | 0 |
| `tests/test_jobs.py` | 1 | 0 | 0 |
| `tests/test_ledger.py` | 1 | 0 | 0 |
| `tests/test_literal_lengths.py` | 1 | 0 | 0 |
| `tests/test_m3_packet.py` | 1 | 0 | 0 |
| `tests/test_perf_gate.py` | 1 | 0 | 0 |
| `tests/test_platform_driver.py` | 1 | 0 | 0 |
| `tests/test_platform_media.py` | 1 | 0 | 0 |
| `tests/test_quick_tools.py` | 1 | 0 | 0 |
| `tests/test_shape_render.py` | 1 | 0 | 0 |
| `tests/test_source_save.py` | 1 | 0 | 0 |
| `tests/test_spatial_cache.py` | 1 | 0 | 0 |
| `tests/test_stress_generator.py` | 1 | 0 | 0 |
| `tests/test_svg_index.py` | 1 | 0 | 0 |
| `tests/test_ui_chrome_golden.py` | 1 | 0 | 0 |
| `tests/test_ui_driver.py` | 1 | 0 | 0 |
| `tests/test_viewer_frame.py` | 1 | 0 | 0 |
| `tests/test_viewer_navigation.py` | 1 | 0 | 0 |
| `tests/test_w15_accessibility_localization.py` | 1 | 0 | 0 |
| `tests/test_wo_runner.py` | 1 | 0 | 0 |
| `tests/test_xml_fuzz_campaign.py` | 1 | 0 | 0 |
| `tests/test_xml_tokenizer.py` | 1 | 0 | 0 |
| `tests/ui` | 24 | 0 | 0 |
| `tests/viewer` | 7 | 0 | 0 |

## Evidence and limits

Per-row `accepted_replacement_evidence` is empty because the frozen inventory result explicitly reports zero substantiated per-file accepted replacement mappings. The exact path, original input row, existing inventory disposition, rationale, and required closure evidence are recorded for every row. The current Rust implementation additions need mapping to original C/Objective-C symbols and frozen assertions before any baseline row can move to accepted replacement. Python and shell tooling remain pending until accepted Rust equivalents or explicit historical dispositions are evidenced. No source, inventory input, ledger, or work order was changed; no tests, builds, benchmarks, or tools were run.
