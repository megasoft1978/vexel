---
id: WO-RUST-PORT-MAP-01
revision: 2
status: READY
milestone: FULL-RUST-PROJECT
owner: project-coordinator
executor: codex-gpt-6-luna
base_sha: 849371835fa4cb8c3e48ff154778a3ec5d7de196
spec_sha256: cdfe723838f34a2e21015f9bc3612f30d6e9e765ce9fa8954fe3297e9e8bdda2
parallel_safe: true
network: false
---
# Goal
Create a candidate semantic crosswalk from legacy first-party source and frozen tests to current Rust modules and accepted work-order evidence. The accepted inventory is accounting only; this task supplies evidence needed to plan actual port work. It does not accept replacements or close migration rows.

# Context and locks
- Preserve the C/Objective-C implementation, Python tools, fixtures and evidence as required by `OWNER-DIRECTIVES.md`.
- Use the immutable inputs from `artifacts/WO-RUST-MIGRATION-INVENTORY-01/` and the candidate row accounting in `artifacts/WO-RUST-MIGRATION-RECONCILE-01/`.
- Compare behavior and symbols in the assigned legacy/Rust areas; matching filenames or broad feature labels are insufficient.
- Cite legacy paths/symbols, Rust paths/symbols, accepted work-order IDs/results and frozen test/oracle evidence where found. Mark candidate-only, partial, missing, or uncertain mappings explicitly.
- Do not change source, manifests, tests, dependencies, original inventory, reconciliation evidence, or ledgers.
- Do not run tests, builds, benchmarks, GUI checks, or install tools. Use source inspection and existing accepted evidence only.

# Parallel slices and exclusive outputs
- `core`: legacy `src/base/**`, `src/document/**`, `src/geom/**`, `src/raster/**`, `src/spatial/**`, `src/svg/**`; `tests/base/**`, `tests/document/**`, `tests/edit/**`, `tests/fuzz/shape/**`, `tests/raster/**`, `tests/svg/**`, `tests/realworld/**`; and `tests/test_document_model.py`, `tests/test_literal_lengths.py`, `tests/test_shape_render.py`, `tests/test_source_save.py`, `tests/test_spatial_cache.py`, `tests/test_svg_index.py`, `tests/test_xml_tokenizer.py`. Output only `artifacts/WO-RUST-PORT-MAP-01/core.md`.
- `app-platform-ui`: top-level legacy `app/**` plus `src/app/**`, `src/ui/**`, `src/platform/**`; `tests/app/**`, `tests/editor/**`, `tests/ui/**`, `tests/platform/**`, `tests/platform_media/**`, `tests/e2e/**`, `tests/viewer/**`; and `tests/test_platform_driver.py`, `tests/test_platform_media.py`, `tests/test_ui_chrome_golden.py`, `tests/test_ui_driver.py`, `tests/test_viewer_frame.py`, `tests/test_viewer_navigation.py`, `tests/test_w15_accessibility_localization.py`. Output only `artifacts/WO-RUST-PORT-MAP-01/app-platform-ui.md`.
- `project-animation-mcp`: legacy `src/project/**`, `src/jobs/**`, `src/mcp/**`, `tests/project/**`, `tests/jobs/**`, `tests/anim/**`, `tests/mcp/**`, and `tests/test_jobs.py`. Output only `artifacts/WO-RUST-PORT-MAP-01/project-animation-mcp.md`.
- `tooling`: legacy `scripts/**`, `bench/**`, `packaging/**`, `reports/**`, `dev`, and all remaining Python/tooling tests not explicitly assigned above. This includes `tests/test_benchmark_harness.py`, `tests/test_boot05.py`, `tests/test_corpus_fetch.py`, `tests/test_corpus_render_report.py`, `tests/test_dev.py`, `tests/test_golden_diff.py`, `tests/test_golden_oracle.py`, `tests/test_m3_packet.py`, `tests/test_perf_gate.py`, `tests/test_quick_tools.py`, `tests/test_wo_runner.py`, and any other unassigned `tests/test_*.py`. Output only `artifacts/WO-RUST-PORT-MAP-01/tooling.md`.
- Reports classify each original baseline row exactly once. Other reports may link to an owning report but must not assign a conflicting classification. `tests/e2e/**` belongs only to app-platform-ui; `tests/platform_media/**`, including fixture generation, belongs only to app-platform-ui. If an owned row cannot be mapped, list it as unresolved rather than omitting it.
- Coordinator alone owns this work order and `ledger.jsonl`; no nested fan-out or cross-slice report edits.

# Acceptance
- AC-1: every assigned baseline source and test path appears in its slice report; no silent omissions.
- AC-1a: across all four reports, every legacy baseline source and frozen test path appears exactly once as a classification; cross-references are clearly labeled and do not duplicate ownership.
- AC-2: each path is classified as candidate replacement, partial candidate, no Rust candidate found, or unresolved; every candidate links concrete Rust symbols and existing acceptance evidence.
- AC-3: missing behavior is grouped into disjoint future work packages with real dependencies; no duplicate implementation is proposed where accepted Rust work exists.
- AC-4: candidate mappings do not change the prior `pending_port` or `unresolved` dispositions and make no parity/port-complete claim.
- AC-5: all preserved legacy sources, assets, Python tests and historical evidence remain untouched.

# Deferred verification
This is source/evidence mapping only. Product tests and parity campaigns are deferred to the consolidated final verification phase after all implementation tasks finish.
