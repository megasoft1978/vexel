# WO-UI-03 AC-5: logging never touches the document

The `check_log` case in `tests/editor/viewer_workspace_test.c` materialises the
document with `viewer_editor_materialize`, records an operation, a warning and an
error, forces 200 more entries so the ring wraps, opens the log pane, rebuilds the
semantic tree, materialises again and compares the two buffers byte for byte,
including their lengths. Any difference fails the test with code 10; the log also
never marks the document dirty because it is not part of `document_edit` history.
