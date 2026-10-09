# UI driver smoke files

`mixed-commands.jsonl` is the deterministic all-command replay used by
`WO-BOOT-07`. `verify_smoke.py` decodes each screenshot with the project PNG
profile validator and checks the semantic-tree and saved-state JSON outputs.

`navigation-replay.jsonl` covers fit, wheel and pinch zoom, pan, progress,
cancel request, screenshots, and semantic snapshots. Its new baselines live
only in `navigation-golden/`; `verify_navigation.py` validates every baseline
file against the SHA-256 manifest before byte comparison. The frozen lifecycle
scenario remains unchanged.
