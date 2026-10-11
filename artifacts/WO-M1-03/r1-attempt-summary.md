# WO-M1-03 revision 1 attempt

The first runner attempt produced a stable placeholder digest in 20/20 runs (`26e0099669a931b31d2afa0c35f2ed87b5da6d7ee9e326fe59eeaea965c183fc`). The runner's final command re-execution failed because the same fixed replay output directory already existed. The candidate then changed the replay output allocator to reserve unique `replay-NNNN` directories, and the focused repeat-replay test passed twice against the same root.

Native Metal replay and two existing native lifecycle tests failed before frame output with LaunchServices/Services XPC errors and exit -11. `./dev verify` reported 132 passed, 7 failed, 2 errors; ASan/UBSan CTest passed. No revision-1 acceptance or verifier pass was claimed. The exact failed command outputs are in `artifacts/runs/WO-M1-03/a3ab5096-b2c9-4c82-b9ec-fe3d72662432/`. Revision 2 requires rerunnable output roots and retries the native path.
