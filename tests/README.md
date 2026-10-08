# Python test harness

`scripts/test_runner.py` discovers `tests/test_*.py` with the standard-library
`unittest` loader, reports every test ID and result, and writes JSON conforming
to `docs/schemas/test-results.schema.json`. Statuses include pass, failure,
error, skip, expected failure, and unexpected success. All non-pass outcomes
are recorded; a skip or any other unsuccessful result fails the harness.
`./dev test-fast` stores its transient result at
`.cache/test-results/latest.json` and also validates `ledger.jsonl`.
The suite also validates the immutable `tests/golden/manifest.lock` and every
source/PNG SHA-256 offline; oracle tests never invoke `rsvg-convert`.

The JSON includes UTC `started_at` and `ended_at`, monotonic elapsed seconds,
status counts, and per-test IDs/statuses. Validation uses Python standard
library code only; no test framework dependency is installed.
