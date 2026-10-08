#!/usr/bin/env python3
"""Validate and append the repository's transition-checked JSONL ledger."""

from __future__ import annotations

import argparse
import fcntl
import hashlib
import json
import math
import os
import re
import stat
import sys
import tempfile
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parent.parent
EVENTS = {
    "ready", "run_started", "run_finished", "verified", "accepted",
    "rejected", "merged", "blocked", "superseded",
}
REQUIRED = {
    "ts", "wo_id", "event", "revision", "base_sha", "head_sha", "run_id",
    "session_id", "model", "cli_version", "duration_s", "usage", "status",
    "artifacts", "note",
}
ALLOWED_NEXT = {
    None: {"ready"},
    "ready": {"run_started", "blocked"},
    "run_started": {"run_finished", "blocked"},
    "run_finished": {"run_started", "verified", "blocked"},
    "verified": {"accepted", "rejected", "blocked"},
    "rejected": {"run_started", "superseded", "blocked"},
    "accepted": {"merged", "superseded"},
    "blocked": {"run_started", "superseded"},
    "merged": set(),
    "superseded": set(),
}
SHA_RE = re.compile(r"^[0-9a-f]{7,40}$")
UTC_RE = re.compile(r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$")


class LedgerError(ValueError):
    """Raised when a ledger entry or transition violates the format."""


def _reject_json_constant(value: str) -> None:
    raise LedgerError(f"non-finite JSON constant {value}")


def _string(entry: dict[str, Any], key: str) -> str:
    value = entry[key]
    if not isinstance(value, str) or not value:
        raise LedgerError(f"{key} must be a non-empty string")
    return value


def validate_entry(entry: Any) -> None:
    if not isinstance(entry, dict):
        raise LedgerError("entry must be a JSON object")
    missing = REQUIRED - entry.keys()
    extra = entry.keys() - REQUIRED
    if missing or extra:
        raise LedgerError(f"entry keys mismatch; missing={sorted(missing)}, extra={sorted(extra)}")

    if not UTC_RE.fullmatch(_string(entry, "ts")):
        raise LedgerError("ts must be an RFC3339 UTC timestamp ending in Z")
    try:
        parsed = datetime.fromisoformat(entry["ts"].replace("Z", "+00:00"))
    except ValueError as exc:
        raise LedgerError("ts is not a valid timestamp") from exc
    if parsed.tzinfo != timezone.utc:
        raise LedgerError("ts must be UTC")

    for key in ("wo_id", "run_id", "session_id", "model", "cli_version", "status"):
        _string(entry, key)
    if not isinstance(entry["event"], str) or entry["event"] not in EVENTS:
        raise LedgerError("event is not supported")
    if type(entry["revision"]) is not int or entry["revision"] < 1:
        raise LedgerError("revision must be a positive integer")
    for key in ("base_sha", "head_sha"):
        if not SHA_RE.fullmatch(_string(entry, key)):
            raise LedgerError(f"{key} must be a 7-40 character lowercase git hash")

    duration = entry["duration_s"]
    if isinstance(duration, bool) or not isinstance(duration, (int, float)):
        raise LedgerError("duration_s must be a non-negative finite number")
    if duration < 0 or not math.isfinite(duration):
        raise LedgerError("duration_s must be a non-negative finite number")

    usage = entry["usage"]
    if not isinstance(usage, dict) or usage.keys() != {"input_tokens", "output_tokens"}:
        raise LedgerError("usage must contain only input_tokens and output_tokens")
    for value in usage.values():
        if value is not None and (type(value) is not int or value < 0):
            raise LedgerError("token counts must be non-negative integers or null")
    if not isinstance(entry["artifacts"], list) or not all(
        isinstance(item, str) for item in entry["artifacts"]
    ):
        raise LedgerError("artifacts must be an array of strings")
    if not isinstance(entry["note"], str) or len(entry["note"]) > 500:
        raise LedgerError("note must be a string of at most 500 characters")


def _decode_line(raw: bytes, line_number: int) -> dict[str, Any]:
    try:
        entry = json.loads(raw, parse_constant=_reject_json_constant)
    except (UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise LedgerError(f"line {line_number}: malformed JSON") from exc
    validate_entry(entry)
    return entry


def validate_bytes(data: bytes) -> list[dict[str, Any]]:
    if data and not data.endswith(b"\n"):
        raise LedgerError("ledger must end with a newline")
    entries: list[dict[str, Any]] = []
    run_ids: set[str] = set()
    latest: dict[tuple[str, int], str] = {}
    latest_entry: dict[tuple[str, int], dict[str, Any]] = {}
    for index, raw in enumerate(data.splitlines(), start=1):
        if not raw:
            raise LedgerError(f"line {index}: blank lines are not allowed")
        entry = _decode_line(raw, index)
        run_id = entry["run_id"]
        if run_id in run_ids:
            raise LedgerError(f"line {index}: duplicate run_id {run_id}")
        run_ids.add(run_id)
        key = (entry["wo_id"], entry["revision"])
        previous = latest.get(key)
        event = entry["event"]
        if event not in ALLOWED_NEXT[previous]:
            raise LedgerError(
                f"line {index}: invalid transition {previous!r} -> {event!r} for {key[0]}"
            )
        previous_entry = latest_entry.get(key)
        if event == "accepted" and previous_entry["status"] != "pass":
            raise LedgerError("accepted must follow a passing verified event")
        if event == "merged" and previous_entry["status"] != "accepted":
            raise LedgerError("merged must follow an accepted event")
        latest[key] = event
        latest_entry[key] = entry
        entries.append(entry)
    return entries


def validate_file(path: Path) -> int:
    try:
        entries = validate_bytes(path.read_bytes())
    except OSError as exc:
        raise LedgerError(f"cannot read {path}: {exc}") from exc
    return len(entries)


def _repo_path(raw: str) -> Path:
    path = Path(raw)
    resolved = (ROOT / path if not path.is_absolute() else path).resolve()
    if not resolved.is_relative_to(ROOT):
        raise LedgerError("ledger paths must stay inside the repository")
    return resolved


def append_entry(path: Path, entry: dict[str, Any]) -> None:
    validate_entry(entry)
    path = _repo_path(str(path))
    lock_key = hashlib.sha256(str(path).encode()).hexdigest()
    lock_path = ROOT / ".cache" / "ledger" / f"{lock_key}.lock"
    lock_path.parent.mkdir(parents=True, exist_ok=True)
    path.parent.mkdir(parents=True, exist_ok=True)
    encoded = json.dumps(entry, ensure_ascii=False, separators=(",", ":")).encode() + b"\n"

    with lock_path.open("a+b") as lock_file:
        fcntl.flock(lock_file.fileno(), fcntl.LOCK_EX)
        current = path.read_bytes() if path.exists() else b""
        entries = validate_bytes(current)
        if any(item["run_id"] == entry["run_id"] for item in entries):
            raise LedgerError(f"duplicate run_id {entry['run_id']}")
        proposed = validate_bytes(current + encoded)
        if len(proposed) != len(entries) + 1:
            raise LedgerError("append validation did not add exactly one entry")
        mode = stat.S_IMODE(path.stat().st_mode) if path.exists() else 0o644
        descriptor, temp_name = tempfile.mkstemp(prefix=f".{path.name}.", dir=path.parent)
        try:
            complete = current + encoded
            view = memoryview(complete)
            while view:
                written = os.write(descriptor, view)
                if written <= 0:
                    raise OSError("ledger update made no write progress")
                view = view[written:]
            os.fsync(descriptor)
            os.close(descriptor)
            descriptor = -1
            os.chmod(temp_name, mode)
            os.replace(temp_name, path)
        finally:
            if descriptor >= 0:
                os.close(descriptor)
            if os.path.exists(temp_name):
                os.unlink(temp_name)


def _load_entry(args: argparse.Namespace) -> dict[str, Any]:
    if args.entry_file:
        raw = _repo_path(args.entry_file).read_text(encoding="utf-8")
    else:
        raw = args.entry_json
    try:
        return json.loads(raw, parse_constant=_reject_json_constant)
    except json.JSONDecodeError as exc:
        raise LedgerError(f"malformed entry JSON: {exc}") from exc


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)
    validate = subparsers.add_parser("validate", help="validate a JSONL ledger")
    validate.add_argument("path")
    append = subparsers.add_parser("append", help="validate and append one transition")
    append.add_argument("path")
    group = append.add_mutually_exclusive_group(required=True)
    group.add_argument("--entry-json")
    group.add_argument("--entry-file")
    args = parser.parse_args(argv)

    try:
        if args.command == "validate":
            count = validate_file(_repo_path(args.path))
            print(f"valid ledger: {count} entries")
        else:
            append_entry(_repo_path(args.path), _load_entry(args))
            print("entry appended")
    except (LedgerError, OSError) as exc:
        print(f"ledger error: {exc}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
