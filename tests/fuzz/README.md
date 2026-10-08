# CLI fuzz smoke

`scripts/fuzz_smoke.py` exercises only the current `svgworkbench` argv boundary.
It creates bounded printable ASCII and UTF-8 argument mutations, with a fixed
default seed of `12648430`; `--seed` overrides it and the exact seed and
iteration count are emitted in JSON. At least one process is run. Duration is
3 seconds by default, must be greater than zero, and is capped at 60 seconds;
each child has a bounded timeout. The output records actual elapsed duration.

The CLI's normal usage-error exit is expected. A signal, timeout, sanitizer
diagnostic, or unexpected exit status retains the argv case and captured
stdout and stderr under `artifacts/WO-BOOT-05/fuzz-failures/`, then fails. The
transient summary defaults to `.cache/fuzz-smoke/latest.json`. The fuzzer does
not use the network or feed SVG data to an absent parser.
