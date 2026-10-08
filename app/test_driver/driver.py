#!/usr/bin/env python3
"""Replay bounded JSONL placeholder UI events and emit deterministic artifacts."""

from __future__ import annotations

import argparse
import json
import math
import os
import struct
import subprocess
import sys
import time
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


def _safe_native_output(raw: str) -> Path:
    """Reserve a fresh numbered run below a stable native-smoke output root."""
    candidate = Path(raw)
    if not candidate.is_absolute():
        candidate = ROOT / candidate
    lexical = Path(os.path.abspath(candidate))
    cache = ROOT / ".cache"
    if not lexical.is_relative_to(cache) or lexical == cache:
        raise DriverError("native output root must stay under repository .cache")
    current = ROOT
    for part in lexical.relative_to(ROOT).parts:
        current = current / part
        if current.is_symlink():
            raise DriverError("native output path contains a symlink")
    if lexical.exists() and not lexical.is_dir():
        raise DriverError("native output root must be a directory")
    lexical.mkdir(parents=True, exist_ok=True)
    for index in range(1, 10001):
        run = lexical / f"run-{index:04d}"
        try:
            run.mkdir()
            return run
        except FileExistsError:
            if run.is_symlink() or not run.is_dir():
                raise DriverError("native output run path is not a regular directory")
    raise DriverError("native output root reached its 10000-run limit")


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


def validate_native_smoke_output(output: str, cycles: int) -> list[str]:
    """Reject incomplete or malformed native lifecycle evidence."""
    expected = ["native smoke events: open resize scale key pointer close",
                f"completed cycles: {cycles}"]
    lines = output.splitlines()
    if lines != expected:
        raise DriverError("native smoke returned missing or malformed event evidence")
    return lines


def validate_normalized_events(events: list[dict[str, Any]], expected: list[str]) -> None:
    """Check the documented event payload before a replay is accepted."""
    if [event.get("type") for event in events] != expected:
        raise DriverError("native replay has missing, extra, or reordered events")
    for event in events:
        kind = event["type"]
        if kind in {"resize", "scale"}:
            if (type(event.get("pixel_width")) is not int or type(event.get("pixel_height")) is not int or
                    event["pixel_width"] <= 0 or event["pixel_height"] <= 0 or
                    type(event.get("backing_scale")) not in (int, float) or
                    not math.isfinite(event["backing_scale"]) or event["backing_scale"] <= 0):
                raise DriverError(f"native {kind} event has malformed backing dimensions")
        elif kind.startswith("key_"):
            if (type(event.get("key_code")) is not int or not 0 <= event["key_code"] <= 65535 or
                    type(event.get("modifiers")) is not int or not 0 <= event["modifiers"] <= 0xFFFFFFFF or
                    not isinstance(event.get("text", ""), str) or len(event.get("text", "").encode("utf-8")) > 255):
                raise DriverError("native key event has malformed payload")
        elif kind.startswith("pointer_"):
            if (type(event.get("x")) is not int or type(event.get("y")) is not int or
                    event["x"] < 0 or event["y"] < 0 or
                    (kind != "pointer_move" and (type(event.get("button")) is not int or
                                                   not 0 <= event["button"] <= 31))):
                raise DriverError("native pointer event has malformed payload")


def native_command_arguments(commands: list[dict[str, Any]]) -> list[str]:
    """Translate the validated JSONL actions into native executable arguments."""
    result: list[str] = []
    for command in commands:
        name = command["command"]
        if name == "open":
            result.extend(["--open", command["document_id"]])
        elif name == "resize":
            result.extend(["--resize", str(command["width"]), str(command["height"])])
        elif name == "key":
            if command["key"] != "a":
                raise DriverError("native smoke currently supports the key 'a'")
            result.extend(["--key", command["action"], command["key"]])
        elif name == "pointer":
            result.extend(["--pointer", command["action"], str(command["x"]), str(command["y"])])
        elif name == "quit":
            result.append("--quit")
        else:
            raise DriverError(f"native smoke does not support the {name} command")
    return result


def native_smoke(out: Path, cycles: int) -> float:
    if type(cycles) is not int or not 1 <= cycles <= 10000:
        raise DriverError("cycles must be an integer from 1 to 10000")
    binary = ROOT / "build" / "mac-release" / "bin" / "svgworkbench"
    if not binary.is_file():
        raise DriverError("Release executable is missing; build preset mac-release first")
    scenario_path = ROOT / "tests" / "ui" / "native-lifecycle.jsonl"
    scenario = read_script(scenario_path)
    native_arguments = native_command_arguments(scenario)
    (out / "scenario.jsonl").write_bytes(scenario_path.read_bytes())
    started = time.monotonic()
    try:
        result = subprocess.run([str(binary), "--native-platform-smoke", "--cycles", str(cycles),
                                 *native_arguments],
                                cwd=ROOT, capture_output=True, text=True, timeout=300, check=False)
    except (OSError, subprocess.TimeoutExpired) as exc:
        raise DriverError(f"native smoke could not complete: {exc}") from exc
    elapsed = time.monotonic() - started
    (out / "native-smoke.txt").write_text(result.stdout, encoding="utf-8")
    (out / "native-smoke.stderr.txt").write_text(result.stderr, encoding="utf-8")
    summary = {"cycles": cycles, "elapsed_seconds": elapsed, "exit_code": result.returncode,
               "events": [], "status": "failed"}
    if result.returncode != 0:
        summary["diagnostic"] = result.stderr.strip()[:1000]
        (out / "result.json").write_bytes(_json_bytes(summary))
        raise DriverError(f"native smoke failed with exit {result.returncode}: {result.stderr.strip()[:500]}")
    lines = validate_native_smoke_output(result.stdout, cycles)
    summary.update({"events": lines[0].split(": ", 1)[1].split(), "status": "passed"})
    (out / "result.json").write_bytes(_json_bytes(summary))
    sys.stdout.write("\n".join(lines) + "\n")
    sys.stdout.write(f"native smoke elapsed seconds: {elapsed:.6f}\n")
    return elapsed


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--script")
    parser.add_argument("--output-dir", required=True)
    parser.add_argument("--seed", type=int, default=0)
    parser.add_argument("--native-platform-smoke", action="store_true")
    parser.add_argument("--cycles", type=int, default=100)
    args = parser.parse_args(argv)
    if sys.platform != "darwin":
        print("error: UI test driver is supported on macOS only", file=sys.stderr)
        return 2
    if not 0 <= args.seed <= 0xFFFFFFFF:
        print("error: seed must be an unsigned 32-bit integer", file=sys.stderr)
        return 2
    try:
        if args.native_platform_smoke:
            if args.script is not None:
                raise DriverError("--script cannot be combined with --native-platform-smoke")
            out = _safe_native_output(args.output_dir)
            native_smoke(out, args.cycles)
        else:
            if args.script is None:
                raise DriverError("--script is required unless --native-platform-smoke is used")
            out = _safe_output(args.output_dir)
            script = _safe_script(args.script)
            commands = read_script(script)
            out.parent.mkdir(parents=True, exist_ok=True)
            out.mkdir()
            replay(commands, out, args.seed)
    except (DriverError, OSError, ValueError) as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
