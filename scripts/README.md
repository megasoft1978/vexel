# Ledger utility

`ledger.py` validates the machine-readable `ledger.jsonl` history and appends a
single transition. The schema is `docs/schemas/ledger-entry.schema.json`;
transition rules live in `ledger.py` and reject invalid event order or repeated
`run_id` values.

```sh
python3 scripts/ledger.py validate ledger.jsonl
python3 scripts/ledger.py append ledger.jsonl --entry-file .cache/next-entry.json
```

Appends serialize through an advisory `fcntl` lock in ignored `.cache/ledger/`.
Each update is written and fsynced to a temporary file in the ledger directory,
then atomically replaces the log while the lock is held. Invalid input is
rejected before the ledger is changed. The utility uses only Python's standard
library and accepts ledger paths inside this repository.
