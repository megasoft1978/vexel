# WO-M0-02 handoff

M0-02 is not merged. Corrected candidate: branch `wo/WO-M0-02`, commit
`1a45004c96873b643f35427ba2766e88ba020273`.

The latest fresh read-only review found three blocking items: semantic SVG
content was buffered, embedded image data was invalid, and AC-6 evidence came
from a different worktree. The candidate now streams semantic chunks, emits a
valid PNG data URI, validates PNG chunks/pixels in tests, and records the full
verification output from the matching candidate worktree. Evidence and exact
findings are in `review-findings.txt`.

After correction, local acceptance passed: 10 generator tests, 42 oracle/runner
tests, 83 total `./dev verify` tests with zero failures/errors/skips, exact
10,000,000 / 100,000,000 / 1,000,000,000-byte generation, report hash
verification, oracle-lock verification, ledger/work-order validation, diff
check, ASan and UBSan CTest (1/1 each), and a standalone embedded PNG render.
The 1GB mixed input SHA-256 is
`895780c50a7e3b6f05e1ad34b68ca6d22684699e6fa1b7425a7f1baaa6caa9e7`.

The frozen work order allows two verifier rounds; both have been used. The
corrected candidate has not had independent review, runner acceptance, or
merge. Next step requires owner/planner direction to authorize a fresh review
of the exact candidate commit or revise the process cap. If review passes,
finish runner acceptance and merge M0-02, then proceed to M0-03 and stop at the
M0-04 owner gate as planned.
