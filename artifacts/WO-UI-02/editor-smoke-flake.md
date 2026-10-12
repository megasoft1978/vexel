# Pre-existing `./dev editor-smoke` save defect observed during the WO-UI-02/03 sweep

The combined sweep run of `./dev editor-smoke` exited 1 while the same command had
just passed standalone. The diagnostic was:

```
saved SVG changed bytes outside the selected transform:
b'<svg xmlns=\'http://www.w3.org/2000/svg\' width=\'64\' height=\'64\'
viewBox=\'0 0 64 64\' transform="matrix(1 0 0 1 1.91194969 2)"></svg>\n'
```

This is not a fixture mismatch: the saved document lost its `<rect>` entirely and
the composed matrix was written onto the root `<svg>` element instead of the
selected shape. `assert_core_save` strips every `transform` attribute before
comparing, so the only way to reach this message is for the shape itself to vanish.

Observed rate in this session: three consecutive standalone runs exited 0, and one
sweep run failed, so it is intermittent rather than deterministic.

Scope: this is in the document-edit transform/serialisation path, which no UI order
in this sweep touched (the orders changed the chrome, the workspace model, the
inspector tabs and the log pane only). It is recorded here rather than hidden, and
it is tracked by `work/WO-BUG-01.md`, because a save that drops the selected shape
is the highest-severity class of defect in this project.

Reproduce: run `./dev editor-smoke` repeatedly; the canvas phase fails when the
saved file matches the bytes above.
