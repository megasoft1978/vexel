#!/usr/bin/env python3
"""Stream a manifest-verified corpus and inventory SVG path command letters."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
import xml.parsers.expat
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
JSON_OUT = ROOT / "reports/M4/path-command-inventory.json"
MD_OUT = ROOT / "reports/M4/path-command-inventory.md"
COMMANDS = "AaCcHhLlMmQqSsTtVvZz"
COMMAND_RE = re.compile(r"[" + re.escape(COMMANDS) + r"]")


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def parse_file(path: Path) -> Counter[str]:
    counts: Counter[str] = Counter()
    parser = xml.parsers.expat.ParserCreate(namespace_separator="}")

    def start_element(name: str, attrs: dict[str, str]) -> None:
        if name.rsplit("}", 1)[-1] == "path":
            data = attrs.get("d")
            if data:
                counts.update(COMMAND_RE.findall(data))

    parser.StartElementHandler = start_element
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            parser.Parse(block, False)
        parser.Parse(b"", True)
    return counts


def aggregate(corpus: Path) -> dict:
    corpus = corpus.resolve(strict=True)
    manifest = corpus / "manifest.json"
    manifest_digest = sha256_file(manifest)
    entries = json.loads(manifest.read_text(encoding="utf-8"))
    if not isinstance(entries, list):
        raise ValueError("manifest must be a JSON array")
    rows = []
    occurrences: Counter[str] = Counter({key: 0 for key in COMMANDS})
    file_counts: Counter[str] = Counter({key: 0 for key in COMMANDS})
    path_files = 0
    for entry in entries:
        relative = Path(entry["path"])
        source = (corpus / relative).resolve(strict=True)
        if not source.is_relative_to(corpus) or not source.is_file():
            raise ValueError(f"manifest path escapes corpus or is not a file: {relative}")
        actual_size = source.stat().st_size
        actual_hash = sha256_file(source)
        if actual_size != entry["bytes"] or actual_hash != entry["sha256"]:
            raise ValueError(f"manifest size/hash mismatch: {relative}")
        counts = parse_file(source)
        if counts:
            path_files += 1
            file_counts.update(counts.keys())
        occurrences.update(counts)
        rows.append({"path": entry["path"], "bytes": actual_size, "sha256": actual_hash,
                     "command_occurrences": {key: counts[key] for key in sorted(counts)}})
    return {"schema_version": 1, "corpus_root": str(corpus), "manifest_path": str(manifest),
            "manifest_sha256": manifest_digest, "files": len(entries), "path_bearing_files": path_files,
            "command_totals": {key: occurrences[key] for key in sorted(occurrences)},
            "command_file_counts": {key: file_counts[key] for key in sorted(file_counts)}, "entries": rows}


def markdown(report: dict) -> str:
    lines = ["# Real-corpus SVG path command inventory", "",
             f"Corpus: `{report['corpus_root']}` ({report['files']} files); manifest SHA-256 `{report['manifest_sha256']}`.",
             f"Path-bearing files: {report['path_bearing_files']}/{report['files']}. The tool verifies each manifest size and SHA-256 before parsing that file.",
             "", "Counts include command letters found inside `<path d>` attributes; file counts are distinct files.",
             "", "| Command | Occurrences | Files |", "|---|---:|---:|"]
    for command in sorted(report["command_totals"]):
        lines.append(f"| `{command}` | {report['command_totals'][command]} | {report['command_file_counts'][command]} |")
    return "\n".join(lines) + "\n"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--corpus-dir", type=Path, required=True)
    parser.add_argument("--check", action="store_true", help="verify generated JSON and Markdown match corpus")
    args = parser.parse_args()
    try:
        report = aggregate(args.corpus_dir)
        output_json = json.dumps(report, indent=2) + "\n"
        output_md = markdown(report)
        if args.check:
            if JSON_OUT.read_text(encoding="utf-8") != output_json or MD_OUT.read_text(encoding="utf-8") != output_md:
                raise ValueError("path-command inventory differs from current manifest-verified corpus")
            print(f"checked {report['files']} files; {report['path_bearing_files']} contain path data")
        else:
            JSON_OUT.write_text(output_json, encoding="utf-8")
            MD_OUT.write_text(output_md, encoding="utf-8")
            print(f"wrote {report['files']} files; {report['path_bearing_files']} contain path data")
    except (OSError, ValueError, KeyError, TypeError, json.JSONDecodeError, xml.parsers.expat.ExpatError) as exc:
        print(f"path-command inventory error: {exc}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
