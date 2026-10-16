# Viewer2 lane

## Built

- Sharp visible-region rerendering runs on the viewer worker in 256 by 256
  backing-pixel tiles. Generation-checked results overlay the retained
  resampled preview. The cache is capped at 128 tiles and 32 MiB; the planner
  enforces a 200-million-unit work budget.
- AppKit Open, Reload, Finder open callbacks, `.svg` drag and drop, and a recent
  file menu with up to twelve paths. File size and modification time are polled
  every 250 ms for auto-reload.
- View controls include fit window, fit width, actual size, background cycle,
  checkerboard, pixel grid at high zoom, cursor document position, document
  tile size, and latest load/render timings. A no-argument launch prompts to
  open or drop a file.
- Load and sharp-render jobs use cancellable generation-tagged worker tasks.
  Error frames give a reason and reload hint. The headless `--render-frame`
  interface accepts bounded `--zoom` and `--region` arguments.
- The native smoke script is at `tests/viewer/native_smoke.sh`.

## Limits

- SVG source is capped at 1 GiB; rendered frame buffers at 128 MiB; viewports
  at 8192 pixels per axis; sharp tiles at 256 pixels per axis; sharp cache at
  32 MiB; sharp planning at 200 million estimated work units. Each sharp tile
  remains subject to the rasterizer's own limits.
- `--region` is expressed in decoded document-tile pixels and its rerendered
  target is capped at 4096 pixels per axis. `--zoom` is bounded to 0.001..1000.
- The viewer supports `.svg`, not `.svgz`; SVG text and editing tools are not
  implemented. Auto-reload detection uses size and mtime, so same-size edits
  with unchanged mtime are not detected.

## Verification and unverified behavior

- `./dev build` passed on Apple Silicon macOS with AppleClang 21.
- `cmake --preset mac-release && cmake --build --preset mac-release` passed.
- `ctest --test-dir build/mac-debug -R '^(viewer-navigation|viewer-composition|viewer-quality|viewer-frame)$' --output-on-failure`
  passed all four viewer tests. `python3 tests/test_viewer_frame.py` passed all
  twelve CLI acceptance tests, including missing, unreadable, and sparse
  over-limit files. The established inline PPM hashes remain unchanged.
- The focused viewer tests and all twelve CLI acceptance checks passed under
  both `build/mac-asan` and `build/mac-ubsan`. `./dev asan` and `./dev ubsan`
  built cleanly; their full CTest runs each passed 19/21 tests and failed only
  `native-viewer-smoke` and `index-replay` when Metal reported no device.
- Full Debug CTest passed 19/21 tests, with the same two Metal-dependent
  failures. Python discovery ran 218 tests and reported 8 failures and 2 errors:
  four process-policy subprocess checks returned exit 71, two process-capture
  assertions failed, another benchmark harness check observed zero trials
  instead of 30, and the native 100-cycle smoke failed because Metal was
  unavailable. The two Metal smoke/replay checks also errored without a device.
  The viewer CLI tests all passed in that discovery run.
- Native presentation and System Events interaction need an interactive
  desktop and Metal. CTest's `native-viewer-smoke` stopped before drawing its
  first frame; the System Events script was not run. Finder open, drag and drop,
  auto-reload, resize/minimize/restore, and display-scale changes remain
  unverified here.
