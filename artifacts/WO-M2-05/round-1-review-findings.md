# Fresh verifier round 1 findings and fixes

The exact-commit read-only review of `d18659542f7581c817441cc254d2751bc3660d2a` found:

- Attribute lookup matched local names without checking prefixes, and an explicit root default namespace was not validated. Attribute handling now rejects prefixes and accepts only the SVG namespace URI when present.
- The fuzzer's short random token generation could not reach configured command/edge limits. The sanitizer campaign now starts with a deterministic 327,730-byte case that reaches and verifies the command cap.
- Relative cubic/quadratic control points and endpoints were bounded after flattening. They are now checked before flattening, with diagnostic regressions.

These changes passed the full machine-check suite before fresh review round 2.
