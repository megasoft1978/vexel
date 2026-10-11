# Initial runner attempt

The first isolated Codex execution created smoke output in the requested directory. The runner's deterministic replay then rejected the same command because the directory already existed. That uncovered a repeatability defect in the driver; native smoke now reserves a new numbered run directory and preserves prior output.

The initial sandboxed smoke attempt also recorded Launch Services / `com.apple.hiservices-xpcservice` errors and no scale event. A host-side run with a fresh output path passed 100 cycles. The initial full-suite subprocess policy tests returned sandbox exit 71 inside the nested executor; the serial host `./dev verify` rerun passed 136 tests. Exact initial failure artifacts are retained beside this note and are not represented as the final result.
