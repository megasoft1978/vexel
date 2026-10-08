# UI driver smoke files

`mixed-commands.jsonl` is the deterministic all-command replay used by
`WO-BOOT-07`. `verify_smoke.py` decodes each screenshot with the project PNG
profile validator and checks the semantic-tree and saved-state JSON outputs.
