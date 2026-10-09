# WO-M3-06 revision 2 amendment

The initial 58-file run completed 116 fresh processes but exposed a measurement blind spot: `svg_index_bench` computed `complete_index_ns` and `max_step_ns`, then returned before serializing them whenever no supported preview was found. This left per-file full-index and main-thread-step evidence missing for the very files the comparison must report honestly.

Revision 2 narrowly permits benchmark-only JSON output changes in `app/index_bench.c`. It records process outcome, index-completion state, exact index duration when complete, max-step duration, diagnostics, and RSS for both success and unsupported/malformed exits. No product renderer/index behavior changes. The first summary established 116 runs and zero crashes/limits but did not preserve per-file raw rows; those preliminary rows were replaced by the final complete-metrics campaign, which reruns from fresh processes after rebuilding Release.
