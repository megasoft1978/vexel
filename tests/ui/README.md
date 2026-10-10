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
