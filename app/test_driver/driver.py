#!/usr/bin/env python3
"""Replay bounded JSONL placeholder UI events and emit deterministic artifacts."""

from __future__ import annotations

import argparse
import json
import os
import struct
import sys
import zlib
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[2]
MAX_INPUT = 8 * 1024 * 1024
MAX_LINE = 65_536
MAX_COMMANDS = 10_000
MAX_PIXELS = 16_777_216
MAX_VIRTUAL_NS = 1_000_000_000_000


class DriverError(ValueError):
    """Invalid input or unsafe output path."""


def _pairs(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise DriverError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def _constant(value: str) -> None:
    raise DriverError(f"non-finite JSON value: {value}")


def _string(value: Any, field: str, limit: int, *, empty: bool = False) -> str:
    if not isinstance(value, str) or (not empty and not value):
        raise DriverError(f"{field} must be a string")
    try:
        encoded = value.encode("utf-8")
    except UnicodeEncodeError as exc:
        raise DriverError(f"{field} is not valid Unicode") from exc
    if len(encoded) > limit or "\x00" in value:
        raise DriverError(f"{field} exceeds its limit or contains NUL")
    return value


def _integer(value: Any, field: str, low: int, high: int) -> int:
    if type(value) is not int or not low <= value <= high:
        raise DriverError(f"{field} must be an integer from {low} to {high}")
    return value


def read_script(path: Path) -> list[dict[str, Any]]:
    try:
        with path.open("rb") as stream:
            data = stream.read(MAX_INPUT + 1)
    except OSError as exc:
        raise DriverError(f"cannot read script: {exc.strerror or 'I/O error'}") from exc
    if len(data) > MAX_INPUT:
        raise DriverError("script exceeds 8 MiB")
    try:
        data.decode("utf-8")
    except UnicodeDecodeError as exc:
        raise DriverError("script is not UTF-8") from exc
    lines = data.splitlines()
    if not lines or len(lines) > MAX_COMMANDS:
        raise DriverError("script must contain 1 to 10000 commands")
    commands: list[dict[str, Any]] = []
    for number, raw in enumerate(lines, 1):
        if len(raw) > MAX_LINE:
            raise DriverError(f"line {number} exceeds 65536 bytes")
        if not raw.strip():
            raise DriverError(f"line {number} is blank")
        try:
            command = json.loads(raw, object_pairs_hook=_pairs, parse_constant=_constant)
        except (json.JSONDecodeError, UnicodeDecodeError, DriverError) as exc:
            raise DriverError(f"line {number}: invalid JSON ({exc})") from exc
        if not isinstance(command, dict):
            raise DriverError(f"line {number}: command must be an object")
        commands.append(command)
    _validate_commands(commands)
    return commands


def _safe_script(raw: str) -> Path:
    candidate = Path(raw)
    if not candidate.is_absolute():
        candidate = ROOT / candidate
    resolved = candidate.resolve(strict=False)
    if not resolved.is_relative_to(ROOT):
        raise DriverError("script path must stay inside the repository")
    return resolved


def _shape(command: dict[str, Any], name: str, fields: set[str]) -> None:
    if command.get("command") != name or command.keys() != fields:
        raise DriverError(f"{name} has missing or unknown fields")


def _validate_commands(commands: list[dict[str, Any]]) -> None:
    if commands[0].get("command") != "open":
        raise DriverError("first command must be open")
    if commands[-1].get("command") != "quit":
        raise DriverError("last command must be quit")
    specs = {
        "open": {"command", "document_id"},
        "resize": {"command", "width", "height"},
        "pointer": {"command", "action", "x", "y"},
        "key": {"command", "action", "key"},
        "text_commit": {"command", "text"},
        "frame": {"command", "delta_ns"},
        "progress": {"command", "percent", "label"},
        "accessibility": {"command"},
        "screenshot": {"command"},
        "save": {"command"},
        "quit": {"command"},
    }
    width, height, virtual_ns = 320, 240, 0
    quit_seen = False
    for index, command in enumerate(commands):
        name = command.get("command")
        if not isinstance(name, str) or name not in specs:
            raise DriverError(f"command {index + 1}: unknown command")
        if quit_seen:
            raise DriverError("commands after quit are not allowed")
        _shape(command, name, specs[name])
        if index and name == "open":
            raise DriverError("open may appear only once and first")
        if name == "open":
            _string(command["document_id"], "document_id", 256)
        elif name == "resize":
            width = _integer(command["width"], "width", 1, 4096)
            height = _integer(command["height"], "height", 1, 4096)
            if width * height > MAX_PIXELS:
                raise DriverError("resize exceeds pixel limit")
        elif name == "pointer":
            if not isinstance(command["action"], str) or command["action"] not in {"move", "down", "up"}:
                raise DriverError("pointer action must be move, down, or up")
            x = _integer(command["x"], "x", 0, width - 1)
            y = _integer(command["y"], "y", 0, height - 1)
        elif name == "key":
            if not isinstance(command["action"], str) or command["action"] not in {"down", "up"}:
                raise DriverError("key action must be down or up")
            _string(command["key"], "key", 128)
        elif name == "text_commit":
            _string(command["text"], "text", 4096, empty=True)
        elif name == "frame":
            delta = _integer(command["delta_ns"], "delta_ns", 0, 1_000_000_000)
            virtual_ns += delta
            if virtual_ns > MAX_VIRTUAL_NS:
                raise DriverError("virtual time exceeds limit")
        elif name == "progress":
            _integer(command["percent"], "percent", 0, 100)
            _string(command["label"], "label", 256, empty=True)
        elif name == "quit":
            quit_seen = True
            if index != len(commands) - 1:
                raise DriverError("quit must be final")


def _safe_output(raw: str) -> Path:
    candidate = Path(raw)
    if not candidate.is_absolute():
        candidate = ROOT / candidate
    lexical = Path(os.path.abspath(candidate))
    cache = ROOT / ".cache"
    if not lexical.is_relative_to(cache) or lexical == cache:
        raise DriverError("output directory must be a new path under repository .cache")
    current = ROOT
    for part in lexical.relative_to(ROOT).parts:
        current = current / part
        if current.is_symlink():
            raise DriverError("output path contains a symlink")
    if lexical.exists():
        raise DriverError("output directory already exists")
    return lexical


def _chunk(kind: bytes, payload: bytes) -> bytes:
    return struct.pack(">I", len(payload)) + kind + payload + struct.pack(">I", zlib.crc32(kind + payload) & 0xFFFFFFFF)


def _png(width: int, height: int, seed: int, progress: int) -> bytes:
    accent = ((seed * 73 + 41) & 255, (seed * 151 + 89) & 255, (seed * 199 + 17) & 255)
    rows = bytearray()
    band = max(1, height // 12)
    for y in range(height):
        rows.append(0)
        active = progress > 0 and y < band and (y * 100 // band) < progress
        color = accent if y == 0 or active else (246, 247, 249)
        rows.extend(bytes((*color, 255)) * width)
    header = struct.pack(">2I5B", width, height, 8, 6, 0, 0, 0)
    return (b"\x89PNG\r\n\x1a\n" + _chunk(b"IHDR", header)
            + _chunk(b"IDAT", zlib.compress(bytes(rows), level=9)) + _chunk(b"IEND", b""))


def _json_bytes(value: Any) -> bytes:
    return (json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")) + "\n").encode("utf-8")


def replay(commands: list[dict[str, Any]], out: Path, seed: int) -> None:
    width, height, virtual_ns = 320, 240, 0
    document_id = commands[0]["document_id"]
    pointer: dict[str, Any] | None = None
    keys: set[str] = set()
    text_commits: list[str] = []
    progress = {"percent": 0, "label": ""}
    screenshot_number = accessibility_number = 0
    for index, command in enumerate(commands):
        name = command["command"]
        outputs: list[str] = []
        if name == "resize":
            width, height = command["width"], command["height"]
        elif name == "pointer":
            pointer = {"action": command["action"], "x": command["x"], "y": command["y"]}
        elif name == "key":
            if command["action"] == "down":
                keys.add(command["key"])
            else:
                keys.discard(command["key"])
        elif name == "text_commit":
            text_commits.append(command["text"])
        elif name == "frame":
            virtual_ns += command["delta_ns"]
        elif name == "progress":
            progress = {"percent": command["percent"], "label": command["label"]}
        elif name == "screenshot":
            filename = f"frame-{screenshot_number:04d}.png"
            (out / filename).write_bytes(_png(width, height, seed, progress["percent"]))
            screenshot_number += 1
            outputs.append(filename)
        elif name == "accessibility":
            filename = f"accessibility-{accessibility_number:04d}.json"
            tree = {"bounds": {"height": height, "width": width, "x": 0, "y": 0}, "children": [
                {"bounds": {"height": height, "width": width, "x": 0, "y": 0}, "id": "canvas",
                 "name": "Document canvas", "role": "canvas", "states": [], "value": document_id},
                {"bounds": {"height": 16, "width": width, "x": 0, "y": 0}, "id": "progress",
                 "name": progress["label"], "role": "status", "states": [], "value": progress["percent"]},
            ], "id": "root", "name": "Vexel placeholder", "role": "application",
                "states": [], "value": "", "virtual_time_ns": virtual_ns}
            (out / filename).write_bytes(_json_bytes(tree))
            accessibility_number += 1
            outputs.append(filename)
        elif name == "save":
            saved = {"document_id": document_id, "height": height, "keys_down": sorted(keys),
                     "last_text_commit": text_commits[-1] if text_commits else "", "pointer": pointer,
                     "progress": progress, "seed": seed, "virtual_time_ns": virtual_ns, "width": width}
            (out / "document.json").write_bytes(_json_bytes(saved))
            outputs.append("document.json")
        ack = {"command": name, "index": index, "outputs": outputs, "virtual_time_ns": virtual_ns}
        sys.stdout.buffer.write(_json_bytes(ack))


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--script", required=True)
    parser.add_argument("--output-dir", required=True)
    parser.add_argument("--seed", type=int, default=0)
    args = parser.parse_args(argv)
    if sys.platform != "darwin":
        print("error: UI test driver is supported on macOS only", file=sys.stderr)
        return 2
    if not 0 <= args.seed <= 0xFFFFFFFF:
        print("error: seed must be an unsigned 32-bit integer", file=sys.stderr)
        return 2
    try:
        script = _safe_script(args.script)
        commands = read_script(script)
        out = _safe_output(args.output_dir)
        out.parent.mkdir(parents=True, exist_ok=True)
        out.mkdir()
        replay(commands, out, args.seed)
    except (DriverError, OSError, ValueError) as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
