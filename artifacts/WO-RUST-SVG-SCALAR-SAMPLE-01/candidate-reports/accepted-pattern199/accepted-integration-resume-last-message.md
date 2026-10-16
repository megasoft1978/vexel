Revision 2 remains incomplete: protected `RasterJob` accounting counts old and staged caches against one 16 MiB limit, blocking valid near-cap rebuilds.

Implemented predictive aggregate admission, actual-capacity verification, and cleanup. All 24 tests pass, including unchanged prior 19. Offline check, test, WO validation, and owned formatting pass.

[Result](artifacts/WO-RUST-SVG-SCALAR-SAMPLE-01/result.md) · [Exact verification](artifacts/WO-RUST-SVG-SCALAR-SAMPLE-01/verification.json) · [Shared-contract blocker](artifacts/WO-RUST-SVG-SCALAR-SAMPLE-01/aggregate-contract-blocker.md)

Coordinator repair, root review/audit, and fresh-main checks remain required.