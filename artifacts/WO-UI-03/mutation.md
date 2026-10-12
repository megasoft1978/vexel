# WO-UI-03 AC-4: is the log bound real?

Mutation: in `src/app/viewer_editor_internal.h`, change

    #define EDITOR_LOG_CAP 64U

to

    #define EDITOR_LOG_CAP 256U

so the log keeps far more than the documented bound.

Observed: `./build/mac-debug/bin/viewer_workspace_test` exits **1** with
`log pane failed: 5`, which is the overflow assertion `log_count == 64`. Reverting
the mutation restores exit 0. The bound is therefore enforced and tested, not
merely documented.
