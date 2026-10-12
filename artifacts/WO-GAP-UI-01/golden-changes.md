# WO-GAP-UI-01 golden baseline change

Reason: the status bar's right readout now also shows the document size (source byte length with a unit,
for example `215 B`, `3.4 KB`, `1.2 MB`), as `work/WO-UI-01.md` item 4 requires. Nothing else in the
chrome changed. Fixtures were regenerated with each verifier's `--update` after the new readout was
reviewed. Before = blob at `HEAD` (`286ab3a`), after = the file in this commit. Side-by-side images:
before left, after right, same scale; stats are from `scripts/pixdiff.py`.

| fixture | before | after | side-by-side | change |
|---|---|---|---|---|
| `tests/editor/accessibility-localization-golden/accessibility-chrome-dark-200.png` | `c57e89f23d8dad80` | `db41c6f7f91f8b63` | `side-by-side-accessibility-chrome-dark-200.png` | 5713 of 1024000 pixels differ (0.558%), max delta 200, mean delta 119.3, bbox x25..1110 y764..788 |
| `tests/editor/accessibility-localization-golden/accessibility-chrome-light-200.png` | `16d5cbfbb72adad4` | `5c083b1acfa93a12` | `side-by-side-accessibility-chrome-light-200.png` | 5713 of 1024000 pixels differ (0.558%), max delta 206, mean delta 122.9, bbox x25..1110 y764..788 |
| `tests/editor/accessibility-localization-golden/manifest.json` | `0dce3de893fdab88` | `5ad40d6de8365966` | `—` | checksum only (manifest) |
| `tests/editor/chrome-golden/editor-chrome-dark.png` | `c59b33f4ab4b5682` | `1c9b2eed9092c103` | `side-by-side-editor-chrome-dark.png` | 1645 of 1024000 pixels differ (0.161%), max delta 200, mean delta 87.3, bbox x950..1195 y782..793 |
| `tests/editor/chrome-golden/editor-chrome-light.png` | `bbab25a6905a5930` | `c4a5423239e625ec` | `side-by-side-editor-chrome-light.png` | 1645 of 1024000 pixels differ (0.161%), max delta 206, mean delta 89.8, bbox x950..1195 y782..793 |
| `tests/editor/chrome-golden/manifest.json` | `8b045eee7c46da8a` | `437ec71521b76101` | `—` | checksum only (manifest) |
| `tests/editor/snapping-golden/manifest.json` | `3f7d010e2c3e5e38` | `43d4de9a91d9ba80` | `—` | checksum only (manifest) |
| `tests/editor/snapping-golden/snapping-dark.png` | `fa6f4376cfbc9dad` | `ddd6c7a0a06237ff` | `side-by-side-snapping-dark.png` | 1191 of 1024000 pixels differ (0.116%), max delta 200, mean delta 90.2, bbox x1034..1215 y782..792 |
| `tests/editor/snapping-golden/snapping-light.png` | `658e9885c4eb9afc` | `564f5163fff54e9a` | `side-by-side-snapping-light.png` | 1191 of 1024000 pixels differ (0.116%), max delta 206, mean delta 92.9, bbox x1034..1215 y782..792 |
| `tests/ui/command-overlay-golden/history-dark.png` | `1e0341ae8a8b0831` | `28daa4e73b6152df` | `side-by-side-history-dark.png` | 1812 of 1024000 pixels differ (0.177%), max delta 116, mean delta 52.0, bbox x937..1204 y782..793 |
| `tests/ui/command-overlay-golden/history-light.png` | `54e50807ef019f01` | `35cbabfbb6bd701b` | `side-by-side-history-light.png` | 1812 of 1024000 pixels differ (0.177%), max delta 120, mean delta 53.4, bbox x937..1204 y782..793 |
| `tests/ui/command-overlay-golden/manifest.json` | `cbfbfbbe825c173a` | `a013d065605df5af` | `—` | checksum only (manifest) |
| `tests/ui/command-overlay-golden/palette-dark.png` | `5eda92f20a405b91` | `133153d45006c482` | `side-by-side-palette-dark.png` | 1189 of 1024000 pixels differ (0.116%), max delta 116, mean delta 52.2, bbox x1034..1215 y782..792 |
| `tests/ui/command-overlay-golden/palette-light.png` | `b39c7fc1f653d321` | `c515bd74e6142528` | `side-by-side-palette-light.png` | 1189 of 1024000 pixels differ (0.116%), max delta 120, mean delta 53.6, bbox x1034..1215 y782..792 |

The list is generated from `git status --porcelain` over the four golden directories, so every changed
fixture, including the four `manifest.json` files, is covered. No fixture under `tests/golden/` or
`tests/oracle/` changed. Source files changed alongside (not fixtures): `tests/editor/viewer_chrome_test.c`
(expected readout label now includes the size) and `tests/editor/viewer_workspace_test.c`.
