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
