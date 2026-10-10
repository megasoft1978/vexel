# Platform tests

`platform_dialog_test.m` installs a stub backend to test SVG filename
suggestions, selected-path bounds, all unsaved-changes choices, error alerts,
PNG scale/selection options, and rejection of off-main-thread calls. It does not
open native panels. `platform_png_test.c` encodes a bounded RGBA8 image and
verifies the PNG signature, dimensions, and decoded pixel bytes through the
existing ImageIO media provider. `viewer_export_test.c` injects the PNG save
panel response, exports both the full document and a selected element at 2×,
then decodes the files to verify dimensions and pixels.

`test_source_save.c` also covers the macOS atomic new-file adapter through the
document save target, including a no-replace collision and temporary cleanup.
