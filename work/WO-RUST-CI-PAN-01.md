---
id: WO-RUST-CI-PAN-01
revision: 1
status: READY
milestone: FULL-RUST-PROJECT
owner: project-coordinator
executor: codex-gpt-6-luna
base_sha: 849371835fa4cb8c3e48ff154778a3ec5d7de196
spec_sha256: 313e9a05b7e39f9244441db1cb023f44c25557dbe440c8de2979f4466fd98dfe
parallel_safe: true
network: false
---
# Goal
Stop macOS CI's recurring false failure in the frozen pan-present timing gate by preventing parallel tests in the same harness from contending with the measured case. Keep the workload, assertion, and 15,657 ms threshold unchanged.

# Evidence and context
- GitHub Actions run `39259444573` at `849371835fa4cb8c3e48ff154778a3ec5d7de196` failed `vexel-app/tests/ui_canvas_pointer.rs::a_thousand_pan_events_present_within_the_frame_budget`: 23,530 ms versus 15,657 ms.
- Previous run `39252868508` failed the same test at 22,351 ms versus 15,657 ms.
- On the current Apple Silicon checkout, running the exact test by itself with one test thread passed at 12,022.2 ms. This supports test-harness contention as the cause; it does not prove hosted-runner behavior.
- The measurement shares `ui_canvas_pointer`'s harness with compose/render tests. Its wall-clock threshold is a frozen oracle and must not be raised, skipped, weakened, or replaced with a release build.

# Frozen change and acceptance
- Change only `.github/workflows/ci.yml`: run the existing workspace test command with Rust test harness concurrency set to one (`-- --test-threads=1`). Keep the macOS runner, toolchain, test suite, timeout and other steps unchanged.
- AC-1: no test, workload, timing threshold, ignore rule, or oracle changes.
- AC-2: the previously failing pan test passes with the unchanged limit when run alone and in the serialized workspace suite.
- AC-3: `cargo test --workspace --locked -- --test-threads=1`, ledger/Python checks, readiness, formatting/diff checks pass.
- AC-4: Sol review is CLEAN; after commit and push, the matching GitHub Actions run for the new commit succeeds.

# Allowed paths
- `.github/workflows/ci.yml`
- `work/WO-RUST-CI-PAN-01.md` and `ledger.jsonl` only by coordinator
- `artifacts/WO-RUST-CI-PAN-01/**`

# Protected paths
All application/test source, other workflows, toolchain and build configuration, manifests/locks, test oracles/thresholds, ADRs, unrelated work orders/evidence, OWNER-DIRECTIVES.md, external apps, and non-macOS platform code. No dependency, system package, browser/GUI, broad UI/E2E, threshold change, force push, or unrelated source edits.

# Verification
```text
VEXEL_PERF_PRINT=1 cargo test -p vexel-app --locked --test ui_canvas_pointer a_thousand_pan_events_present_within_the_frame_budget -- --exact --nocapture --test-threads=1
cargo test --workspace --locked -- --test-threads=1
python3 -m unittest tests.test_ledger tests.test_wo_runner
python3 scripts/ledger.py validate ledger.jsonl
python3 scripts/wo_validate.py --ready WO-RUST-CI-PAN-01
git diff --check
```

# Non-goals
Do not optimize renderer code without evidence of an isolated failure. Do not change benchmark implementation or threshold. No changes to application behavior, runner, release packaging, or other CI workflows.
