"""Recoverable installation for the benchmark's JSON/CSV report pair."""

from __future__ import annotations

import json
import os
from pathlib import Path
import tempfile
from typing import Callable
import uuid


Validator = Callable[[Path, Path], None]


def _atomic_bytes(path: Path, data: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    fd, name = tempfile.mkstemp(prefix=f".{path.name}.", dir=path.parent)
    try:
        with os.fdopen(fd, "wb") as stream:
            stream.write(data)
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(name, path)
    except BaseException:
        Path(name).unlink(missing_ok=True)
        raise


def _temporary_bytes(path: Path, data: bytes) -> Path:
    path.parent.mkdir(parents=True, exist_ok=True)
    fd, name = tempfile.mkstemp(prefix=f".{path.name}.stage.", dir=path.parent)
    try:
        with os.fdopen(fd, "wb") as stream:
            stream.write(data)
            stream.flush()
            os.fsync(stream.fileno())
    except BaseException:
        Path(name).unlink(missing_ok=True)
        raise
    return Path(name)


def _journal_path(json_path: Path) -> Path:
    return json_path.with_name(f".{json_path.name}.install.json")


def _cleanup(journal_path: Path, journal: dict) -> None:
    for key in ("json_stage", "csv_stage", "json_backup", "csv_backup"):
        Path(journal[key]).unlink(missing_ok=True)
    journal_path.unlink(missing_ok=True)


def _validate_journal_paths(journal: dict, json_path: Path, csv_path: Path) -> None:
    for kind, target in (("json", json_path), ("csv", csv_path)):
        stage = Path(journal[f"{kind}_stage"])
        backup = Path(journal[f"{kind}_backup"])
        if (stage.parent != target.parent or not stage.name.startswith(f".{target.name}.stage.")
                or backup.parent != target.parent
                or not backup.name.startswith(f".{target.name}.")
                or not backup.name.endswith(".backup")
                or type(journal[f"had_{kind}"]) is not bool):
            raise ValueError("unsafe report install journal paths")


def recover_pair(json_path: Path, csv_path: Path, validate: Validator) -> bool:
    journal_path = _journal_path(json_path)
    if not journal_path.exists():
        return False
    journal = json.loads(journal_path.read_text(encoding="utf-8"))
    if journal.get("json_path") != str(json_path) or journal.get("csv_path") != str(csv_path):
        raise ValueError("report install journal does not match requested output pair")
    _validate_journal_paths(journal, json_path, csv_path)
    try:
        validate(json_path, csv_path)
    except Exception:
        for kind, target in (("json", json_path), ("csv", csv_path)):
            backup = Path(journal[f"{kind}_backup"])
            if journal[f"had_{kind}"]:
                if not backup.is_file():
                    raise ValueError(f"missing {kind} report rollback file")
                _atomic_bytes(target, backup.read_bytes())
            else:
                target.unlink(missing_ok=True)
    _cleanup(journal_path, journal)
    return True


def install_pair(json_path: Path, csv_path: Path, json_data: bytes, csv_data: bytes,
                 validate: Validator) -> None:
    if json_path.resolve() == csv_path.resolve():
        raise ValueError("JSON and CSV report paths must differ")
    recover_pair(json_path, csv_path, validate)
    token = uuid.uuid4().hex
    json_stage = None
    csv_stage = None
    json_backup = json_path.with_name(f".{json_path.name}.{token}.backup")
    csv_backup = csv_path.with_name(f".{csv_path.name}.{token}.backup")
    try:
        json_stage = _temporary_bytes(json_path, json_data)
        csv_stage = _temporary_bytes(csv_path, csv_data)
        had_json, had_csv = json_path.is_file(), csv_path.is_file()
        if had_json:
            _atomic_bytes(json_backup, json_path.read_bytes())
        if had_csv:
            _atomic_bytes(csv_backup, csv_path.read_bytes())
        validate(json_stage, csv_stage)
    except BaseException:
        for path in (json_stage, csv_stage, json_backup, csv_backup):
            if path is not None:
                path.unlink(missing_ok=True)
        raise
    journal = {"json_path": str(json_path), "csv_path": str(csv_path),
               "json_stage": str(json_stage), "csv_stage": str(csv_stage),
               "json_backup": str(json_backup), "csv_backup": str(csv_backup),
               "had_json": had_json, "had_csv": had_csv}
    journal_path = _journal_path(json_path)
    try:
        _atomic_bytes(journal_path, (json.dumps(journal, sort_keys=True) + "\n").encode())
    except BaseException:
        _cleanup(journal_path, journal)
        raise
    try:
        os.replace(csv_stage, csv_path)
        os.replace(json_stage, json_path)
        validate(json_path, csv_path)
    except BaseException:
        recover_pair(json_path, csv_path, validate)
        raise
    _cleanup(journal_path, journal)
