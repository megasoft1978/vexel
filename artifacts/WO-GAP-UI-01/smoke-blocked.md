# AC-5 `./dev editor-smoke` could not be executed to completion here

`check-07.txt`: exit 1 at the first System Events call (`Can't get window 1 of process "svgworkbench"`).
Cause: the macOS session is locked (`CGSessionCopyCurrentDictionary()["CGSSessionScreenIsLocked"] == 1`),
so the app has no on-screen window for System Events to drive. The unmodified `HEAD` (changes stashed) fails
at the identical line, so this is environmental, not caused by this change.

`tests/editor/native_smoke_actions.sh` was rewritten for item 4 (pinned `VEXEL_TEST_WORKSPACE=draw`, Cmd-S and
copy; Control-PageUp from Draw asserts Inspect+Document tab; Control-PageDown asserts Draw+Object tab; Nodes,
Animate and Inspect visited with assertions; Cmd-S again and `cmp` against the copy). It passes `sh -n`
only; the live-window behaviour is unverified and must be re-run in an unlocked GUI session.
