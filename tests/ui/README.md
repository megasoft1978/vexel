# UI driver smoke files

`mixed-commands.jsonl` is the deterministic all-command replay used by
`WO-BOOT-07`. `verify_smoke.py` decodes each screenshot with the project PNG
profile validator and checks the semantic-tree and saved-state JSON outputs.

`navigation-replay.jsonl` covers fit, wheel and pinch zoom, pan, progress,
cancel request, screenshots, and semantic snapshots. New rendered navigation
screens and semantics live only in `navigation-golden/`; the original flat
canvas baseline and manifest remain byte-for-byte unchanged. The verifier
checks both manifests and every SHA-256 before comparing output bytes. The
frozen lifecycle scenario remains unchanged.

`verify_style_goldens.py` checks 20 new deterministic scalar screenshots for
the editor chrome: dark/light at 1280×800 and 2560×1600 in empty, selection,
editing-field, error, and narrow-viewport states. Text in these screenshots
uses the scalar glyph painter so the PNG hashes do not depend on system font
revisions; `platform_text_ui_test` checks native CoreText output structurally.
Review the images before updating them with
`python3 tests/ui/verify_style_goldens.py --update`.

`verify_guides_goldens.py` checks the ruler and persistent-guide renderer at
1×/2× in light and dark themes. Its human-review PNGs live in
`guides-golden/`; ticks, guides, and numeric labels use deterministic scalar
rendering. Production numeric labels use CoreText. Update reviewed images with
`python3 tests/ui/verify_guides_goldens.py --update`.

The UIKit has separate deterministic component and icon sheets in
`uikit-golden/`. `ui_gallery` renders 24 widgets in seven control states at 1×
and 2× in both appearances; `ui_icon_gallery` renders all 53 authored paths at
24 pt and 48 pt. The human-review PNGs are in `docs/ui/`. Run the gallery
verifier through CTest as `ui-kit-gallery`, or run it directly with the two
renderer paths. Do not replace the older style or chrome fixtures with the
UIKit sheets.

`ui_kit_test` validates token contrast across light/dark and standard/high-
contrast profiles, icon path commands and bounds, keyboard/pointer state
machines, and paint bounds independent of viewport size. Its
`--contrast-report` mode generates `docs/ui/uikit-contrast-report.json`.
`ui_widget_fuzz` fuzzes number and hex-color parsing with a reproducible seed;
CTest runs 200,000 cases by default.
