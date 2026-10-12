# WO-UI-01 golden baseline change

Every changed fixture is listed with its reason, its before/after SHA-256 and a red-highlight
side-by-side produced by `./dev pixdiff` under `artifacts/WO-UI-01/`.

Reason for all of them: the options band now carries the four task-workspace tabs instead of the
duplicated tool hint, and the status bar is split into hint, report and readout regions.

| fixture | before | after | diff image | change |
|---|---|---|---|---|
| `tests/editor/accessibility-localization-golden/accessibility-chrome-dark-200.png` | `1c953ac46d869f00` | `74133af1bd973273` | `diff-accessibility-chrome-dark-200.png` | 15930 of 1024000 pixels differ (1.556%), max delta 203, mean delta 107.7, bbox x16..1172 y96..788 |
| `tests/editor/accessibility-localization-golden/accessibility-chrome-light-200.png` | `06fb2ac3dcf7f116` | `928b7218e760c085` | `diff-accessibility-chrome-light-200.png` | 15930 of 1024000 pixels differ (1.556%), max delta 206, mean delta 107.9, bbox x16..1172 y96..788 |
| `tests/editor/chrome-golden/editor-chrome-dark.png` | `7437d32760258d85` | `a515b28335ca9180` | `diff-editor-chrome-dark.png` | 5697 of 1024000 pixels differ (0.556%), max delta 203, mean delta 86.6, bbox x8..1226 y48..793 |
| `tests/editor/chrome-golden/editor-chrome-light.png` | `39306343dcb04277` | `c97e3a2bc38b3df3` | `diff-editor-chrome-light.png` | 5697 of 1024000 pixels differ (0.556%), max delta 206, mean delta 87.3, bbox x8..1226 y48..793 |
| `tests/editor/snapping-golden/snapping-dark.png` | `b498c743d8c5a4c0` | `ac44cc2273f4b808` | `diff-snapping-dark.png` | 5557 of 1024000 pixels differ (0.543%), max delta 203, mean delta 86.8, bbox x8..1234 y48..793 |
| `tests/editor/snapping-golden/snapping-light.png` | `e38180699909be1e` | `633817909d9bee41` | `diff-snapping-light.png` | 5557 of 1024000 pixels differ (0.543%), max delta 206, mean delta 87.5, bbox x8..1234 y48..793 |
| `tests/ui/command-overlay-golden/history-dark.png` | `c8b520bf79df9682` | `2e6fce839bedae40` | `diff-history-dark.png` | 4197 of 1024000 pixels differ (0.410%), max delta 117, mean delta 49.4, bbox x8..1226 y48..793 |
| `tests/ui/command-overlay-golden/history-light.png` | `be07614707df3a48` | `638522d29ccb3f96` | `diff-history-light.png` | 4193 of 1024000 pixels differ (0.409%), max delta 120, mean delta 49.9, bbox x8..1226 y48..793 |
| `tests/ui/command-overlay-golden/palette-dark.png` | `8741605a0c636fb6` | `711b50aab413f475` | `diff-palette-dark.png` | 5556 of 1024000 pixels differ (0.543%), max delta 117, mean delta 50.6, bbox x8..1234 y48..793 |
| `tests/ui/command-overlay-golden/palette-light.png` | `827e5332475049fa` | `4deb28ff993de792` | `diff-palette-light.png` | 5556 of 1024000 pixels differ (0.543%), max delta 120, mean delta 50.8, bbox x8..1234 y48..793 |

All other fixtures are byte-identical. `tests/editor/viewer_chrome_test.c` was updated to
assert the new three-region structure (hint at the historical 12 pt inset, readout right-aligned at
1268) instead of the removed standalone `Ready · Saved` and `No selection` labels.
