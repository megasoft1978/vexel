# Platform tests

`platform_dialog_test.m` installs a stub backend to test SVG filename
suggestions, selected-path bounds, all unsaved-changes choices, error alerts,
and rejection of off-main-thread calls. It does not open native panels.

`test_source_save.c` also covers the macOS atomic new-file adapter through the
document save target, including a no-replace collision and temporary cleanup.
