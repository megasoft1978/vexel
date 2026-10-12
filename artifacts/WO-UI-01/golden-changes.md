# WO-UI-01 golden baseline change

Reason: the options band carries the four task-workspace tabs instead of the duplicated tool hint,
and the status bar is split into hint, report and readout regions. Every changed PNG has a
red-highlight diff produced by `./dev pixdiff` under `artifacts/WO-UI-01/`.

| fixture | before | after | diff image | change |
|---|---|---|---|---|
| `tests/editor/accessibility-localization-golden/accessibility-chrome-dark-200.png` | `74133af1bd973273` | `9f9ce8ae11cde8a3` | `diff-accessibility-chrome-dark-200.png` | 4532 of 1024000 pixels differ (0.443%), max delta 200, mean delta 118.6, bbox x25..1110 y764..788 |
| `tests/editor/accessibility-localization-golden/accessibility-chrome-light-200.png` | `928b7218e760c085` | `f02667988c48cd68` | `diff-accessibility-chrome-light-200.png` | 4532 of 1024000 pixels differ (0.443%), max delta 206, mean delta 122.2, bbox x25..1110 y764..788 |
| `tests/editor/accessibility-localization-golden/manifest.json` | `77e38a81d7334f12` | `a1a146208a0fe4fb` | `—` | checksum only (manifest) |
| `tests/editor/chrome-golden/editor-chrome-dark.png` | `a515b28335ca9180` | `c23b22cb37c0481a` | `diff-editor-chrome-dark.png` | 1395 of 1024000 pixels differ (0.136%), max delta 200, mean delta 86.5, bbox x1007..1195 y782..793 |
| `tests/editor/chrome-golden/editor-chrome-light.png` | `c97e3a2bc38b3df3` | `e9912300aa4311bf` | `diff-editor-chrome-light.png` | 1395 of 1024000 pixels differ (0.136%), max delta 206, mean delta 89.1, bbox x1007..1195 y782..793 |
| `tests/editor/chrome-golden/manifest.json` | `3efff40cf2e3069a` | `ec99d70973ed6995` | `—` | checksum only (manifest) |
| `tests/editor/snapping-golden/manifest.json` | `5439a1e804eb5e17` | `2bb8d79a8de67562` | `—` | checksum only (manifest) |
| `tests/editor/snapping-golden/snapping-dark.png` | `ac44cc2273f4b808` | `49ab4faa33877132` | `diff-snapping-dark.png` | 1204 of 1024000 pixels differ (0.118%), max delta 200, mean delta 92.5, bbox x1035..1215 y782..793 |
| `tests/editor/snapping-golden/snapping-light.png` | `633817909d9bee41` | `62d551e96641db0c` | `diff-snapping-light.png` | 1204 of 1024000 pixels differ (0.118%), max delta 206, mean delta 95.3, bbox x1035..1215 y782..793 |
| `tests/editor/viewer_chrome_test.c` | `53ef1cf35fa4c296` | `574bbdc113886bf4` | `—` | checksum only (manifest) |
| `tests/editor/viewer_workspace_test.c` | `e50363617552bfb7` | `f81b3f26900c2caf` | `—` | checksum only (manifest) |
| `tests/ui/command-overlay-golden/history-dark.png` | `2e6fce839bedae40` | `f9a25a1fadf0f8de` | `diff-history-dark.png` | 1850 of 1024000 pixels differ (0.181%), max delta 116, mean delta 52.1, bbox x936..1204 y782..793 |
| `tests/ui/command-overlay-golden/history-light.png` | `638522d29ccb3f96` | `66366dd6f4bacf4b` | `diff-history-light.png` | 1850 of 1024000 pixels differ (0.181%), max delta 120, mean delta 53.6, bbox x936..1204 y782..793 |
| `tests/ui/command-overlay-golden/manifest.json` | `3a99c2945f6bc083` | `7b492d19b2e075c3` | `—` | checksum only (manifest) |
| `tests/ui/command-overlay-golden/palette-dark.png` | `711b50aab413f475` | `f5c158b5718403f2` | `diff-palette-dark.png` | 1204 of 1024000 pixels differ (0.118%), max delta 116, mean delta 53.9, bbox x1035..1215 y782..793 |
| `tests/ui/command-overlay-golden/palette-light.png` | `4deb28ff993de792` | `f5f4f4d175ffe44e` | `diff-palette-light.png` | 1203 of 1024000 pixels differ (0.117%), max delta 120, mean delta 55.4, bbox x1035..1215 y782..793 |

This list is exhaustive: it is generated from `git status --porcelain -- tests/`, so every
changed fixture — including the four `manifest.json` files — is covered. `tests/editor/viewer_chrome_test.c`
and `tests/editor/viewer_workspace_test.c` changed as source, not fixtures.
