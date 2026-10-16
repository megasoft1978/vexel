import hashlib
import json
import pathlib
import sys

seed = json.loads(pathlib.Path("artifacts/WO-RUST-SVG-PARITY-HARNESS-01/source-seed.json").read_text())
failures = []
for item in seed["files"]:
    path = pathlib.Path(item["path"])
    data = path.read_bytes()
    actual = hashlib.sha256(data).hexdigest()
    valid = len(data) == item["bytes"] and actual == item["sha256"]
    print(f"{item['path']} bytes={len(data)} sha256={actual} {'OK' if valid else 'MISMATCH'}")
    if not valid:
        failures.append(item["path"])
print(f"verified={len(seed['files']) - len(failures)} mismatches={len(failures)}")
sys.exit(bool(failures))
