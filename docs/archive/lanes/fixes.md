# W5 fixes lane

## Built

- Changed the viewer's Open panel to an asynchronous sheet so AppKit's event
  loop can continue servicing viewer polling and shutdown. Reworked the native
  smoke automation with bounded startup, automation, and quit waits, phase
  diagnostics, process-group cleanup, and retained failure logs. Added a
  repeat harness that defaults to 20 runs and records each outcome.
- MCP `render_png` now decodes through the app's text and image media providers.
  MCP supports the `text` element template, bounded XML-fragment insertion,
  Save As to a new file inside granted directories, and closing a clean
  non-active document. Added broker feature coverage without requiring a UDS.
- Added a 64 KiB-bounded XML fragment edit API with single-element and
  post-insertion validation. Added text-node insertion/replacement support.
  New Save As destinations are atomically published without overwriting an
  existing file; tests cover collisions, write failures, and output bytes.
- Added a main-thread-only macOS save panel, unsaved-changes alert, and error
  alert API with injectable test hooks. The editor UI wiring remains for a
  later lane.

## Files changed

- `CMakeLists.txt`
- Document/save: `src/document/README.md`, `src/document/document_edit.h`,
  `src/document/document_edit_ops.c`, `src/document/source_save.c`,
  `src/platform/platform_atomic_new_file.h`,
  `src/platform/macos/platform_atomic_new_file.c`,
  `tests/document/README.md`, `tests/document/test_source_save.c`,
  `tests/edit/README.md`, `tests/edit/test_document_edit.c`
- MCP: `src/mcp/README.md`, `src/mcp/mcp_access.c`,
  `src/mcp/mcp_broker_docs.c`, `src/mcp/mcp_broker_edit.c`,
  `src/mcp/mcp_broker_internal.h`, `src/mcp/mcp_broker_render.c`,
  `src/mcp/mcp_tools.c`, `tests/mcp/README.md`,
  `tests/mcp/mcp_features_test.c`, `tests/mcp/test_e2e.py`
- Platform/viewer: `src/platform/README.md`,
  `src/platform/platform_window.h`, `src/platform/macos/README.md`,
  `src/platform/macos/platform_window.m`,
  `src/platform/macos/platform_dialog.h`,
  `src/platform/macos/platform_dialog.m`, `tests/platform/README.md`,
  `tests/platform/platform_dialog_test.m`, `tests/viewer/README.md`,
  `tests/viewer/native_smoke.sh`, `tests/viewer/native_smoke.py`,
  `tests/viewer/repeat_native_smoke.sh`
- Lane record: `docs/lanes/fixes.md`

## Verification

- `./dev build` — passed.
- `cmake --preset mac-release && cmake --build --preset mac-release` — passed.
- `ctest --test-dir build/mac-debug -R '^(mcp-features|document-edit|source-save|platform-dialog)$' --output-on-failure` — passed, 4/4.
- `ctest --test-dir build/mac-debug --output-on-failure` — 31 passed, 2 failed, 1 skipped. `native-viewer-smoke` and `index-replay` could not access LaunchServices/Metal; `mcp-stdio-e2e` was skipped because Unix-domain socket `bind(2)` is denied with `EPERM` in this environment.
- `./dev asan` — build passed; CTest 31 passed, 2 failed, 1 skipped for the same host limitations. No AddressSanitizer diagnostics were reported.
- `./dev ubsan` — build passed; CTest 31 passed, 2 failed, 1 skipped for the same host limitations. No UBSan diagnostics were reported.
- `./dev test-fast` — 243 passed, 8 failed, 2 errors, 1 skipped. Seven benchmark-harness tests were blocked by process-policy behavior in this environment; the native 100-cycle smoke hit LaunchServices errors, and two Metal driver tests could not access Metal. The test runner stopped before its bundled ledger step.
- `python3 -m unittest discover -s tests -t .` — 242 passed, 8 failed, 3 errors, 1 skipped. The failures cover the same seven benchmark-harness cases and native 100-cycle smoke. The errors are two Metal driver tests plus XML fuzz cleanup receiving `EPERM` when it tried to kill its supervised process group.
- `python3 scripts/ledger.py validate ledger.jsonl` — passed, 406 entries.
- `./dev viewer-smoke` — attempted once and failed before UI interaction: LaunchServices reported an invalid connection and the viewer had no Metal device/queue. The harness terminated the child processes and retained its failure log. The 20-run repeat harness is provided but remains unverified here.
- `git diff --check` — passed.

## Limits and unverified items

The repeat smoke run needs a normal interactive macOS session with a working
LaunchServices connection and Metal device. The raw MCP stdio test needs local
Unix-domain socket support. The full Python suite also exercises process
policies that this environment restricts, including process-group signaling.
The dialog API is implemented and tested but is not wired into the editor UI
in this lane. XML fragment input is limited to 64 KiB and exactly one direct
element.
