# WO-RUST-RECOVERY-SESSION-02 acceptance matrix

Status values below are intentionally **pending** until the integrated implementation and named verification commands have actually run. Map test names to the final integrated test output when it is available; do not infer a pass from source inspection.

| Criterion | Frozen acceptance | Evidence expected |
|---|---|---|
| AC-1 | A dirty saved `.vexel` session creates a bounded recovery capsule containing the complete scene and retained opaque payload. Invalid input, allocation failure, or failed publication leaves live session state unchanged. | Session capture tests: successful bounded capture and payload inclusion; each failure case asserts unchanged session state and capsule state as applicable. |
| AC-2 | Matching and conflicting capsules are discovered and decoded using the exact source path. Corrupt, unsupported, mismatched, and invalid capsules return actionable typed failures without partial adoption. | Discovery/decode tests for Matching and Conflict, corrupt/unsupported/mismatched/invalid cases, typed errors, and unchanged session on failure. |
| AC-3 | Explicit Restore / Discard Recovery / Cancel behavior is covered by fake-backend flow tests. Cancel is a no-op; conflict Restore cannot overwrite the changed source; Discard opens the normal source; failed opens preserve the current session. | `vexel-app` flow tests with fake backend, covering all three choices and conflict/failed-open preservation. |
| AC-4 | Restored scene, timeline, manifest, and asset bytes round-trip exactly; restored state is dirty, has no stale undo history, and preserves original source bytes. | Session and focused integration round-trip tests; exact byte comparisons, dirty/history assertions, original-source byte assertion. |
| AC-5 | Successful normal save and explicit discard apply capsule cleanup according to publication/durability outcomes, with tests for failure states. | Session save/discard cleanup tests for successful publication, pre-publication failure, and post-publication durability error semantics. |
| AC-6 | All new Rust source files are under 800 lines; no dependency, format change, GPL source, platform expansion, false automatic-capture claim, or weakened frozen oracle is introduced. | Changed-path/dependency/source line-count audit and `git diff --check`; review against the frozen contract. |
| AC-7 | Focused package/integration checks, workspace check/test, Python ledger/runner tests, ledger validation, `git diff --check`, and WO readiness validation pass. | Exact command output recorded in `result.md` or companion logs. Required commands are enumerated there. |

## Cryptographic provenance

The per-source key is SHA-256 over the canonicalized existing source path. Its implementation must follow the Secure Hash Standard, **NIST FIPS PUB 180-4**, *Secure Hash Standard (SHS)*, SHA-256 specification (August 2015), without a new dependency. Record implementation source location and cite the standard in the final result; do not claim conformance beyond the implementation and tests actually reviewed. Official provenance: [NIST FIPS 180-4](https://doi.org/10.6028/NIST.FIPS.180-4).

## Explicit deferred scope

This work order does not claim automatic or periodic/background snapshots, app-launch recovery scanning/presentation, missing-source recovery, retention/cleanup policy, broad UI/E2E coverage, or performance compliance. Those remain follow-up work. Existing `AppDocument::open_path` and MCP open behavior remain normal-open routes and do not imply a native recovery dialog.
