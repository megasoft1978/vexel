# W8 Test Hook

## Changes

- Added the macOS test-only `VEXEL_TEST_OPEN=<path>` startup hook. It validates
  the UTF-8 `.svg` path against `PLATFORM_PATH_CAP` and queues the existing
  `PLATFORM_EVENT_FILE_OPEN` event, without creating an `NSOpenPanel`.
- Changed `./dev viewer-smoke` to run the app's native injected-event smoke
  with the test-open path. It has a bounded timeout, diagnostics, and
  process-group cleanup. `./dev viewer-panel-smoke` preserves the real panel
  interaction as a separate best-effort check.
- Added a 120-second default process-group supervisor for editor smoke. On
  timeout or failure it terminates the shell, viewer, and UI automation
  children and retains a log with process details.
- Added runner regression tests and mapped them in `tests/test_map.json`.

## Verification

- Regression tests were run red before implementation: all three viewer
  runner tests failed against the old runner (one assertion and two signature
  errors).
- `python3 -m unittest tests.viewer.test_native_smoke tests.editor.test_smoke_supervisor`:
  6 tests passed.
- `sh -n dev tests/viewer/native_smoke.sh tests/viewer/panel_smoke.sh
  tests/editor/native_smoke.sh tests/editor/native_smoke_actions.sh` passed.
- `git diff --check` passed.
- `./dev t` and `./dev t --all` built Debug and passed their Python tests
  (69 tests on `--all`). CTest failed only `native-viewer-smoke` and
  `index-replay`; native viewer presentation reported unavailable Metal, and
  index replay reported no offscreen Metal device or queue.
- `mcp-stdio-e2e` was skipped because the sandbox denied Unix-domain socket
  bind with `EPERM`.

## Unverified

The full `VEXEL_TEST_OPEN` to visible-document flow could not complete in this
sandbox because native presentation has no Metal device and LaunchServices
reported connection errors. The real Open-panel test and editor UI automation
were not run. The Unix-domain socket MCP end-to-end test was skipped by the
sandbox policy.
