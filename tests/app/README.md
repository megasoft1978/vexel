# App composition tests

`viewer_frame_test.c` exercises pure RGBA viewer composition, including fit,
cursor zoom, pan clamping, low-resolution sampling, transparency over a
checkerboard, visible sharp-tile overlays, the no-document prompt, error
frames with the interactive retry hint, and size bounds. It has no AppKit or
Metal dependency.
