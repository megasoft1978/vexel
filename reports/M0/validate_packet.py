#!/usr/bin/env python3
"""Validate source-linked M0 evidence without leaving the repository."""

from __future__ import annotations

import csv
import hashlib
import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[2]
PACKET = ROOT / "reports/M0"
REQUIRED_COUNTS = {
    "owner_provided_files_and_tasks", "recorded_interviews", "user_outcome_records",
    "synthetic_corpus_fixtures", "pinned_golden_fixtures", "stress_tiers",
    "stress_fixture_examples", "competitors_available", "competitors_unavailable",
    "compatible_vexel_render_available", "m0_03_trials", "requested_owner_examples",
}


def digest(path: Path) -> str:
    hasher = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            hasher.update(block)
    return hasher.hexdigest()


def check(condition: bool, label: str) -> None:
    if not condition:
        raise ValueError(label)
    print(f"PASS {label}")


def main() -> int:
    evidence = json.loads((PACKET / "evidence.json").read_text(encoding="utf-8"))
    report = (PACKET / "product-evidence.md").read_text(encoding="utf-8")
    check(evidence.get("schema_version") == 1, "schema version")
    counts = evidence.get("counts")
    check(isinstance(counts, dict) and REQUIRED_COUNTS <= counts.keys(), "required count fields")
    check(all(type(counts[key]) is int and counts[key] >= 0 for key in REQUIRED_COUNTS), "count values are non-negative integers")
    check(counts["requested_owner_examples"] >= 5, "at least five owner examples requested")
    request_section = report.split("## Owner evidence request", 1)
    check(len(request_section) == 2 and len([line for line in request_section[1].splitlines() if line.lstrip().startswith(tuple(f"{n}." for n in range(1, 6)))]) >= 5,
          "Markdown has a five-item owner evidence request")

    sources = evidence.get("sources")
    check(isinstance(sources, list) and bool(sources), "source records present")
    records: dict[str, dict[str, str]] = {}
    for record in sources:
        check(isinstance(record, dict) and set(record) == {"path", "sha256", "role"}, "source record shape")
        rel = Path(record["path"])
        check(not rel.is_absolute() and ".." not in rel.parts, f"source path is repository-relative: {rel}")
        path = (ROOT / rel).resolve()
        check(path.is_relative_to(ROOT), f"source resolves inside repository: {rel}")
        check(path.is_file(), f"source exists: {rel}")
        actual = digest(path)
        check(actual == record["sha256"], f"source SHA-256: {rel}")
        check(f"| `{rel.as_posix()}` | `{actual}` |" in report, f"Markdown source hash index: {rel}")
        records[rel.as_posix()] = record

    def source(name: str) -> Path:
        check(name in records, f"required source recorded: {name}")
        return ROOT / name

    manifest = json.loads(source("bench/corpus/manifest.json").read_text(encoding="utf-8"))
    manifest_digest = digest(source("bench/corpus/manifest.json"))
    locked_digest = source("bench/corpus/manifest.sha256").read_text(encoding="utf-8").split()[0]
    check(manifest_digest == locked_digest, "corpus manifest matches pinned digest")
    check(manifest["fixture_count"] == len(manifest["fixtures"]) == counts["synthetic_corpus_fixtures"], "synthetic corpus fixture count")
    stress_examples = sum("stress" in fixture.get("features", []) for fixture in manifest["fixtures"])
    check(stress_examples == counts["stress_fixture_examples"], "stress fixture example count")

    golden = json.loads(source("tests/golden/manifest.lock").read_text(encoding="utf-8"))
    check(len(golden["fixtures"]) == counts["pinned_golden_fixtures"], "pinned golden fixture count")
    m02 = json.loads(source("bench/results/WO-M0-02.json").read_text(encoding="utf-8"))
    check(len(m02["artifacts"]) == counts["stress_tiers"], "M0-02 generated stress tier count")
    check(evidence["measured_results"]["m0_02"]["oracle_render"]["fixture_count"] == len(golden["fixtures"]), "M0-02 oracle fixture count")

    m03 = json.loads(source("bench/results/WO-M0-03.json").read_text(encoding="utf-8"))
    with source("bench/results/WO-M0-03.csv").open(newline="", encoding="utf-8") as stream:
        rows = list(csv.DictReader(stream))
    check(len(rows) == len(m03["trials"]) == counts["m0_03_trials"], "M0-03 trial count matches JSON and CSV")
    check(len(m03["availability"]) == counts["competitors_available"] + counts["competitors_unavailable"], "competitor availability count")
    check(sum(bool(item["available"]) for item in m03["availability"]) == counts["competitors_available"], "available competitor count")
    check(sum(not bool(item["available"]) for item in m03["availability"]) == counts["competitors_unavailable"], "unavailable competitor count")
    check(int(bool(m03["vexel_version_probe"]["render_available"])) == counts["compatible_vexel_render_available"], "compatible Vexel render availability")
    check(counts["owner_provided_files_and_tasks"] == 0 and counts["recorded_interviews"] == 0 and counts["user_outcome_records"] == 0, "owner evidence counts explicitly zero")
    print("All source, hash, count, and request checks passed.")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError, KeyError, TypeError, json.JSONDecodeError) as error:
        print(f"FAIL {error}", file=sys.stderr)
        raise SystemExit(1)
