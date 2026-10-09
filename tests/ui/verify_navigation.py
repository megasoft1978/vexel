#!/usr/bin/env python3
"""Verify deterministic navigation screenshots and accessible snapshots."""

from __future__ import annotations

import hashlib
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def verify_manifest(actual: Path, baseline: Path, name: str) -> int:
    manifest = json.loads((baseline / name).read_text(encoding="utf-8"))
    entries = manifest.get("files")
    if not isinstance(entries, dict) or not entries:
        raise ValueError("baseline manifest has no pinned files")
    for name, expected in sorted(entries.items()):
        if Path(name).name != name or not isinstance(expected, str) or len(expected) != 64:
            raise ValueError("baseline manifest entry is malformed")
        reference = (baseline / name).read_bytes()
        if hashlib.sha256(reference).hexdigest() != expected:
            raise ValueError(f"baseline hash mismatch: {name}")
        result = (actual / name).read_bytes()
        if result != reference:
            raise ValueError(f"replay differs from baseline: {name}")
    return len(entries)


def verify(actual: Path, baseline: Path) -> int:
    return (verify_manifest(actual, baseline, "manifest.json") +
            verify_manifest(actual, baseline, "manifest-rendered.json"))


def main() -> int:
    if len(sys.argv) != 5 or sys.argv[1] != "--actual" or sys.argv[3] != "--baseline":
        print("usage: verify_navigation.py --actual OUTPUT --baseline BASELINE", file=sys.stderr)
        return 2
    actual, baseline = (ROOT / sys.argv[2]).resolve(), (ROOT / sys.argv[4]).resolve()
    try:
        count = verify(actual, baseline)
    except (OSError, UnicodeError, json.JSONDecodeError, ValueError) as exc:
        print(f"navigation golden verification failed: {exc}", file=sys.stderr)
        return 1
    print(f"navigation golden replay matched: {count} files across metadata and rendered screens")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
