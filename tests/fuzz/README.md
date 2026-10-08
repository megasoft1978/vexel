# CLI fuzz smoke

`scripts/fuzz_smoke.py` exercises only the current `svgworkbench` argv boundary.
It creates bounded printable ASCII and UTF-8 argument mutations, with a fixed
default seed of `12648430`; `--seed` overrides it and the exact seed and
iteration count are emitted in JSON. The runner targets 48 cases per requested
second and stops starting cases at the deadline, with 0.1 seconds of grace for
the last child process. At least one process is run. Duration is
3 seconds by default, must be greater than zero, and is capped at 60 seconds;
each child has a bounded timeout. The output records actual elapsed duration.

Exact valid `--version` and help output are accepted. The CLI's normal
usage-error exit is expected. A signal, timeout, sanitizer
diagnostic, or unexpected exit status retains the argv case and captured
stdout and stderr under `artifacts/WO-BOOT-05/fuzz-failures/`, then fails. The
transient summary defaults to `.cache/fuzz-smoke/latest.json`. The fuzzer does
not use the network or feed SVG data to an absent parser.

## XML tokenizer campaign

`./dev fuzz-xml --duration SEC --seed INTEGER` builds the Apple Silicon macOS
ASan/UBSan tokenizer target and runs a deterministic bounded campaign. Duration
must be 1–600 seconds; CI uses 30 seconds for pull requests/manual runs and 600
seconds on weekdays. The native process has no file, resource, or network access
and consumes at most 2 KiB of generated input per case. It supports
`--replay SEED ITERATION` and `--one-input-hex HEX ITERATION` for exact
deterministic reproduction, including iteration-tagged invariant diagnostics.

The controller owns campaign timing, process-group cleanup, reports, minimization,
and failure retention. It records build identity, compiler, seed, duration, and
iteration count. Successful inputs are discarded. Failure input, minimized input,
and raw diagnostics are retained below `artifacts/WO-M2-03/failures/`; transient
reports go below `.cache/WO-M2-03/`. The synthetic crash canary exercises capture
and minimization only and is not a tokenizer defect. No parser regression is
recorded until a real minimized input replays with the same normalized stack.
