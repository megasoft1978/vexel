# Verification retries

The initial managed `./dev verify` runs reported sandbox-profile failures in
benchmark subprocess tests and an intermittent LaunchServices failure during
native smoke. The unmodified M2-04 base passed the full Python suite 165/165
in its worktree. A subsequent full run in the M2-05 worktree passed 168/168,
including native smoke and Metal replay. After the fill-group correction, the
final `./dev verify` passed 169/169, with five Debug CTest cases and native
smoke/replay passing. The early failures did not reproduce and are not a
remaining blocker. Their original command logs remain as `verify.txt` and
`verify-rerun.txt` for audit.
