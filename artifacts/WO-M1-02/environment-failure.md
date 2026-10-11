# WO-M1-02 execution record

The first revision-1 attempts in the runner worktree failed before presentation: AppKit logged a LaunchServices notification failure and an invalid `com.apple.hiservices-xpcservice` connection, and the lifecycle harness exited 139 during teardown. Those initial failures are retained in `artifacts/runs/WO-M1-02/c2d6a10e-10b0-4764-90c7-9f09d2d4b121/`.

After a code correction, the successful serial rerun passed the native Metal smoke: two frames presented, byte-exact texture readback passed, and resize/reset passed (0.171 sec for the standalone smoke in the fresh verifier run). The focused suite passed 10 tests; `./dev verify` passed 139 tests with zero failures, errors, or skips. Debug/Release builds and ASan/UBSan CTest commands passed. Revision-2 final command evidence is in `artifacts/WO-M1-02/full-checks.txt` and `artifacts/runs/WO-M1-02/verify-6799f30d-5dd8-4285-88bd-ef9a3f2e7bee/`.
