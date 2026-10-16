# Review test additions

Added only `tests/parse_limits.rs`; implementation and the coordinator-supplied `accepted-baseline/` were left unchanged. New cases cover all three limited parse APIs: defaults reject valid nonzero spans in sources above 16 MiB; a larger policy accepts and matches parsing of the same value; smaller source limits take precedence over invalid spans, list counts, malformed numbers, and unsupported units. Under-limit checks retain span, count, grammar, and absolute-offset errors. An exactly 16 MiB source is accepted, and each limited API with default policy matches its legacy entry point.

The existing 12 test functions/assertions remain intact. Verification passed with 15 unit tests and 4 integration tests (19 total). See `verification.txt` for the frozen command results. No implementation, manifests, dependencies, or files outside owned tests/artifacts changed.
