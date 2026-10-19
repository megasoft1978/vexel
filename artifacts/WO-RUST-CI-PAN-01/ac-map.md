# WO-RUST-CI-PAN-01 acceptance map

| Criterion | Evidence / status |
|---|---|
| AC-1: no test, workload, threshold, ignore rule, or oracle changes | The workflow diff changes only the existing workspace test command to add `-- --test-threads=1`. The pan test, workload, and frozen 15,657 ms threshold are unchanged. |
| AC-2: pan-present timing test passes alone and in the serialized workspace suite | The isolated Apple Silicon run passed at 12,022.2 ms against the unchanged 15,657 ms budget (per the work order evidence). The serialized workspace suite exited 0; the pan-present timing test passed in that run. Prior hosted failures remain `39259444573` at 23,530 ms and `39252868508` at 22,351 ms. |
| AC-3: serialized workspace, ledger/Python, readiness, formatting/diff checks pass | Local evidence: `cargo test --workspace --locked -- --test-threads=1` exited 0; the existing native-app verification artifact records Python unittest (51 tests), ledger validation (2,315 entries after coordinator acceptance bookkeeping), native-app readiness, and `git diff --check` passing. The local Rust CI policy check also exited 0 with `macOS-only CI policy: pass`. This does not record a CI-work-order-specific readiness result. |
| AC-4: Sol review is CLEAN and new hosted Actions run succeeds after commit/push | The existing native-app verification artifact records an independent Sol review as CLEAN. This is supporting local review evidence, not a claim that WO-RUST-CI-PAN-01 has been accepted. The exact post-commit SHA hosted Actions gate remains pending; no matching hosted run is claimed. |

The local test and policy-check evidence shows the serialized command and current workflow pass on this checkout. Hosted-runner behavior remains unverified until the workflow runs successfully for the exact post-commit SHA. This map does not claim work order acceptance.
