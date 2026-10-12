# WO-UI-01 pre-change failure (AC-4)

The focused test cannot exist before this work order, so the pre-change failure is
a compile-time diagnostic rather than a runtime one. Reproduce it by checking out
the base revision and building the new test against it:

```sh
git worktree add /tmp/ui01-base e31084f396d4d6ddabcfffb99f346d741f3a8349
cc -Isrc -Isrc/app -c tests/editor/viewer_workspace_test.c -o /tmp/w.o
# tests/editor/viewer_workspace_test.c:5:10: fatal error: 'viewer_workspace.h' file not found
```

The header, the workspace table and `viewer_editor_set_workspace` are all introduced
by this work order, so no runtime failure is possible at the base revision. The
verifier confirmed this diagnostic independently.
