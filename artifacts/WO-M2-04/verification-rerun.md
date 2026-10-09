# WO-M2-04 host verification rerun

The executor's managed workspace runs failed in the existing benchmark sandbox
and native LaunchServices tests. The Release executable was already built in
the task worktree. Re-running the exact `./dev verify` command from the host
shell passed all 165 Python tests, native smoke, and the 20-frame Metal replay.
The full captured output is in `host-verify.txt`. The failures in
`verify.txt`, `verify-rerun.txt`, and `verify-final.txt` are preserved as
historical executor attempts; they are not reproduced by the host run.

The source-save-focused tests, byte/hash comparisons, atomic failure tests,
permission checks, and Debug/ASan/UBSan CTest 4/4 also pass. AC-6 is now green;
fresh independent review remains pending.
