Implemented the candidate with explicit provisional state, guarded late animation installation, hidden-root completion, and usable media/planner/request/child ownership flows.

All three required commands passed:

- **173 Scene checks:** prior 156 preserved, plus 16 new tests and one compile-fail guard.
- **21 animation checks:** unchanged.
- Frozen work-order validation passed.

Baseline comparison found no unexpected changes. Maximum source/new-test length: 720 lines.

[Result, exact logs, and API coverage](artifacts/WO-RUST-SCENE-DECODE-INITIALIZATION-01/result.md)

Acceptance and seeding remain pending coordinator independent LunaHigh review, protected audit, and fresh-main checks.