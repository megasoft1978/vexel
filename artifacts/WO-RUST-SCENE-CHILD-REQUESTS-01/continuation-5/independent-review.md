**ACCEPT** — revision 2, continuation-5 closes both c4 proof gaps.

- The test submits the matching consumed child `DraftToken` and `RequestToken`, asserts rejection, then successfully opens and commits a descendant using the live parent.
- With a valid current child draft, shape and source mapping, the mapped test rejects the sealed pre-request tile and successfully mints from the post-request tile. Parent recovery remains demonstrated.
- Independently recomputed all candidate hashes and line counts: four production sources match c4 exactly; changed files are 406, 465, 142, 229 and 732 lines. Reversing only the c5 test additions in memory reproduces c4’s exact test hash.
- Peer audit is accurate: **375 peers, 371 unchanged, four allowed source differences**, no missing/protected mismatches, and only the permitted new Rust test. Frozen tests, manifests and protected inputs remain preserved; no prohibited scope changes were found.
- Supplied logs support **12 focused passes** and **233 package passes across 19 result blocks**, including **10 compile-fail doctests**, with zero failures. Fresh read-only scoped rustfmt and ready-validator checks passed.

Cargo results are supplied-log evidence; I did not rerun Cargo. No files were edited, workers spawned, or UI/E2E tests run. This accepts the Scene API candidate only; runtime rendering and full-app acceptance are not established.