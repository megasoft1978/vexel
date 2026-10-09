#!/usr/bin/env python3
"""Validate the UI-driver smoke run's PNG, accessibility, and saved-state files."""

from __future__ import annotations

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
import golden_diff


def verify(raw: str) -> tuple[int, int, int]:
    output = Path(raw)
    if not output.is_absolute():
        output = ROOT / output
    output = output.resolve()
    if not output.is_relative_to(ROOT / ".cache") or not output.is_dir():
        raise ValueError("smoke output must be a directory under repository .cache")
    pngs = sorted(output.glob("frame-*.png"))
    trees = sorted(output.glob("accessibility-*.json"))
    saved = output / "document.json"
    if not pngs or not trees or not saved.is_file():
        raise ValueError("smoke output is missing PNG, accessibility, or saved-state artifacts")
    for png in pngs:
        width, height, _ = golden_diff.decode_png(png.read_bytes())
        if width * height > 16_777_216:
            raise ValueError(f"PNG exceeds pixel limit: {png.name}")
    for tree_path in trees:
        tree = json.loads(tree_path.read_text(encoding="utf-8"))
        if tree.get("role") != "application" or not isinstance(tree.get("children"), list):
            raise ValueError(f"invalid accessibility tree: {tree_path.name}")
    state = json.loads(saved.read_text(encoding="utf-8"))
    if not isinstance(state, dict) or not isinstance(state.get("document_id"), str):
        raise ValueError("saved document state is invalid")
    return len(pngs), len(trees), 1


def main(argv: list[str] | None = None) -> int:
    if len(argv if argv is not None else sys.argv[1:]) != 1:
        print("usage: verify_smoke.py OUTPUT_DIR", file=sys.stderr)
        return 2
    raw = (argv if argv is not None else sys.argv[1:])[0]
    try:
        counts = verify(raw)
    except (OSError, UnicodeError, json.JSONDecodeError, ValueError) as exc:
        print(f"invalid UI smoke output: {exc}", file=sys.stderr)
        return 1
    print(f"valid smoke artifacts: {counts[0]} PNG, {counts[1]} accessibility tree, {counts[2]} saved document")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
