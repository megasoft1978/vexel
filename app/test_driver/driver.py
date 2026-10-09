#!/usr/bin/env python3
"""Replay bounded JSONL placeholder UI events and emit deterministic artifacts."""

from __future__ import annotations

import argparse
import hashlib
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
        "navigate": {"command", "action", "x", "y", "value"},
        "index_progress": {"command", "generation", "percent", "phase", "label"},
        "cancel": {"command"},
        "accessibility": {"command"},
        "screenshot": {"command"},
        "save": {"command"},
        "quit": {"command"},
    }
    width, height, virtual_ns = 320, 240, 0
    quit_seen = False
    nav_generation = 0
    nav_phase = "idle"
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
        elif name == "navigate":
            action = command["action"]
            if not isinstance(action, str) or action not in {"fit", "wheel", "pinch", "pan"}:
                raise DriverError("navigation action must be fit, wheel, pinch, or pan")
            for field in ("x", "y", "value"):
                value = command[field]
                if type(value) not in (int, float) or abs(value) > 10000:
                    raise DriverError(f"navigation {field} must be finite and bounded")
                if isinstance(value, float) and not math.isfinite(value):
                    raise DriverError(f"navigation {field} must be finite and bounded")
            if action == "pinch" and abs(command["value"]) > 100:
                raise DriverError("pinch value exceeds the supported magnification range")
        elif name == "index_progress":
            generation = _integer(command["generation"], "generation", 0, 0xFFFFFFFFFFFFFFFF)
            _integer(command["percent"], "percent", 0, 100)
            phase = command["phase"]
            if generation < nav_generation or not isinstance(phase, str) or phase not in {
                "active", "complete", "canceled", "failed"
            }:
                raise DriverError("progress generation or phase is invalid")
            if generation == nav_generation and nav_phase in {"complete", "canceled", "failed"} and phase != nav_phase:
                raise DriverError("terminal progress generation cannot be reopened")
            if phase == "active" and command["percent"] >= 100:
                raise DriverError("active progress must be below 100 percent")
            nav_generation = generation
            nav_phase = phase
            _string(command["label"], "label", 95, empty=True)
        elif name == "cancel":
            pass
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


def _safe_repeat_output(raw: str) -> Path:
    """Reserve a new invocation directory under the requested cache root."""
    candidate = Path(raw)
    if not candidate.is_absolute():
        candidate = ROOT / candidate
    lexical = Path(os.path.abspath(candidate))
    cache = ROOT / ".cache"
    if not lexical.is_relative_to(cache) or lexical == cache:
        raise DriverError("repeat output root must stay under repository .cache")
    current = ROOT
    for part in lexical.relative_to(ROOT).parts:
        current = current / part
        if current.is_symlink():
            raise DriverError("repeat output path contains a symlink")
    lexical.mkdir(parents=True, exist_ok=True)
    for index in range(1, 10001):
        run = lexical / f"run-{index:04d}"
        try:
            run.mkdir()
            return run
        except FileExistsError:
            if run.is_symlink() or not run.is_dir():
                raise DriverError("repeat output run path is not a regular directory")
    raise DriverError("repeat output root reached its 10000-run limit")


def _hash_artifacts(directory: Path) -> dict[str, str]:
    return {path.name: hashlib.sha256(path.read_bytes()).hexdigest()
            for path in sorted(directory.iterdir()) if path.is_file()}


def repeat_replay(script: Path, out: Path, repeats: int, seed: int) -> str:
    started = time.monotonic()
    commands = read_script(script)
    event_order = [{"index": index, "command": item["command"]}
                   for index, item in enumerate(commands)]
    virtual_ns = 0
    frame_steps = []
    for index, command in enumerate(commands):
        if command["command"] == "frame":
            virtual_ns += command["delta_ns"]
            frame_steps.append({"index": index, "delta_ns": command["delta_ns"],
                                "virtual_time_ns": virtual_ns})
    baseline = None
    digests = []
    for iteration in range(1, repeats + 1):
        run = out / f"replay-{iteration:04d}"
        run.mkdir()
        replay(commands, run, seed, emit_acks=False)
        artifacts = _hash_artifacts(run)
        digest_input = _json_bytes({"artifacts": artifacts, "event_order": event_order,
                                    "frame_steps": frame_steps})
        digest = hashlib.sha256(digest_input).hexdigest()
        current = {"artifacts": artifacts, "event_order": event_order,
                   "frame_steps": frame_steps, "digest": digest}
        if baseline is not None and current != baseline:
            raise DriverError(f"replay determinism mismatch on iteration {iteration}")
        baseline = current
        digests.append(digest)
    elapsed = time.monotonic() - started
    summary = {"repeats": repeats, "identical": len(set(digests)) == 1,
               "digest": digests[0], "digests": digests,
               "event_order": event_order, "frame_steps": frame_steps,
               "elapsed_seconds": elapsed}
    (out / "result.json").write_bytes(_json_bytes(summary))
    sys.stdout.write(f"identical replay digests: {repeats}/{repeats} {digests[0]}\n")
    sys.stdout.write(f"stable event order: {len(event_order)} commands; virtual frame steps: {len(frame_steps)}\n")
    sys.stdout.write(f"repeat replay elapsed seconds: {elapsed:.6f}\n")
    return digests[0]


def validate_native_metal_replay_output(output: str) -> tuple[int, str, str]:
    lines = output.splitlines()
    prefix = "readback hashes identical: "
    order = "stable offscreen frame order: frame-0000..frame-0019"
    digest = lines[1][len(prefix):] if len(lines) > 1 and lines[1].startswith(prefix) else ""
    if (len(lines) != 3 or lines[0] != "native replay frames: 20" or len(digest) != 64 or
            any(character not in "0123456789abcdef" for character in digest) or lines[2] != order):
        raise DriverError("native Metal replay returned malformed frame evidence")
    return 20, digest, order


def native_metal_replay(out: Path, repeats: int, build_type: str = "release") -> float:
    if repeats != 20:
        raise DriverError("native Metal replay requires exactly 20 frames")
    if build_type not in ("debug", "release"):
        raise DriverError("native Metal replay build must be debug or release")
    binary = ROOT / "build" / f"mac-{build_type}" / "bin" / "svgworkbench"
    if not binary.is_file():
        raise DriverError("Release executable is missing; build preset mac-release first")
    started = time.monotonic()
    try:
        result = subprocess.run([str(binary), "--native-metal-replay-smoke"], cwd=ROOT,
                                capture_output=True, text=True, timeout=180, check=False)
    except (OSError, subprocess.TimeoutExpired) as exc:
        raise DriverError(f"native Metal replay could not complete: {exc}") from exc
    elapsed = time.monotonic() - started
    (out / "native-replay.txt").write_text(result.stdout, encoding="utf-8")
    (out / "native-replay.stderr.txt").write_text(result.stderr, encoding="utf-8")
    if result.returncode != 0:
        (out / "result.json").write_bytes(_json_bytes({
            "frames": 0, "elapsed_seconds": elapsed, "exit_code": result.returncode,
            "status": "failed", "diagnostic": result.stderr.strip()[:1000]}))
        raise DriverError(f"native Metal replay failed with exit {result.returncode}: {result.stderr.strip()[:500]}")
    frames, readback_hash, order = validate_native_metal_replay_output(result.stdout)
    summary = {"frames": frames, "readback_sha256": readback_hash,
               "frame_order": order, "elapsed_seconds": elapsed, "status": "passed"}
    (out / "result.json").write_bytes(_json_bytes(summary))
    sys.stdout.write("20 frames with identical readback hashes and stable offscreen frame order\n")
    sys.stdout.write(f"readback sha256: {summary['readback_sha256']}\nnative replay elapsed seconds: {elapsed:.6f}\n")
    return elapsed


def native_task_smoke(out: Path, cycles: int) -> None:
    if type(cycles) is not int or not 1 <= cycles <= 10000:
        raise DriverError("cycles must be an integer from 1 to 10000")
    binary = ROOT / "build" / "mac-debug" / "bin" / "svgworkbench"
    if not binary.is_file():
        raise DriverError("Debug executable is missing; build preset mac-debug first")
    started = time.monotonic()
    try:
        result = subprocess.run([str(binary), "--native-task-smoke", "--cycles", str(cycles), "--run"],
                                cwd=ROOT, capture_output=True, text=True, timeout=300, check=False)
    except (OSError, subprocess.TimeoutExpired) as exc:
        raise DriverError(f"native task smoke could not complete: {exc}") from exc
    elapsed = time.monotonic() - started
    (out / "native-task-smoke.txt").write_text(result.stdout, encoding="utf-8")
    (out / "native-task-smoke.stderr.txt").write_text(result.stderr, encoding="utf-8")
    if result.returncode != 0:
        raise DriverError(f"native task smoke failed with exit {result.returncode}: {result.stderr.strip()[:500]}")
    expected = [f"completed lifecycle cycles: {cycles}", "canceled queued work: pass",
                "canceled running work: pass", "stale publications: 0"]
    lines = result.stdout.splitlines()
    if lines[:4] != expected or len(lines) != 5 or not lines[4].startswith("maximum join seconds: "):
        raise DriverError("native task smoke returned malformed lifecycle evidence")
    join_seconds = float(lines[4].split(": ", 1)[1])
    if not 0.0 <= join_seconds < 2.0:
        raise DriverError("native task smoke exceeded the 2 second join limit")
    (out / "result.json").write_bytes(_json_bytes({"cycles": cycles, "elapsed_seconds": elapsed,
        "maximum_join_seconds": join_seconds, "canceled_queued": True,
        "canceled_running": True, "stale_publications": 0, "status": "passed"}))
    sys.stdout.write(result.stdout)


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


def _navigation_png(width: int, height: int, scale: float, pan_x: float, pan_y: float,
                    percent: int, phase: str, cancel_requested: bool) -> bytes:
    pixels = bytearray((246, 247, 249, 255) * (width * height))

    def rect(x: int, y: int, rect_width: int, rect_height: int,
             color: tuple[int, int, int, int]) -> None:
        left, top = max(0, x), max(0, y)
        right, bottom = min(width, x + rect_width), min(height, y + rect_height)
        for py in range(top, bottom):
            start = (py * width + left) * 4
            pixels[start:(py * width + right) * 4] = bytes(color) * (right - left)

    scene = ((42.0, 36.0, 112.0, 78.0, (218, 71, 68, 255)),
             (226.0, 48.0, 118.0, 84.0, (48, 111, 190, 255)),
             (134.0, 178.0, 96.0, 74.0, (46, 151, 103, 255)))
    for world_x, world_y, world_width, world_height, color in scene:
        left, top = round(pan_x + world_x * scale), round(pan_y + world_y * scale)
        rect(left, top, max(1, round(world_width * scale)), max(1, round(world_height * scale)), color)

    bar_height = min(height, 8)
    fill_width = width * percent // 100
    rect(0, 0, fill_width, bar_height, (34, 128, 224, 255))
    rect(fill_width, 0, width - fill_width, bar_height, (48, 54, 64, 255))
    if phase == "active":
        rect(8, 16, 96, 28, (44, 48, 56, 255) if not cancel_requested else (128, 132, 140, 255))
        for offset in range(12):
            for px, py in ((20 + offset, 24 + offset), (32 - offset, 24 + offset)):
                if 0 <= px < width and 0 <= py < height:
                    start = (py * width + px) * 4
                    pixels[start:start + 4] = b"\xff\xff\xff\xff"

    rows = bytearray()
    row_bytes = width * 4
    for y in range(height):
        rows.append(0)
        rows.extend(pixels[y * row_bytes:(y + 1) * row_bytes])
    header = struct.pack(">2I5B", width, height, 8, 6, 0, 0, 0)
    return (b"\x89PNG\r\n\x1a\n" + _chunk(b"IHDR", header)
            + _chunk(b"IDAT", zlib.compress(bytes(rows), level=9)) + _chunk(b"IEND", b""))


def _json_bytes(value: Any) -> bytes:
    return (json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")) + "\n").encode("utf-8")


def replay(commands: list[dict[str, Any]], out: Path, seed: int, *, emit_acks: bool = True,
           navigation_mode: bool = False) -> None:
    width, height, virtual_ns = 320, 240, 0
    document_id = commands[0]["document_id"]
    pointer: dict[str, Any] | None = None
    keys: set[str] = set()
    text_commits: list[str] = []
    progress = {"percent": 0, "label": ""}
    phase, generation = "idle", 0
    cancel_requested = False
    scale, pan_x, pan_y = 1.0, 0.0, 0.0
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
            phase = "active" if progress["percent"] < 100 else "complete"
        elif name == "navigate":
            action, x, y, value = command["action"], command["x"], command["y"], command["value"]
            if action == "fit":
                scale = max(0.05, min(64.0, min(width / 400.0, height / 300.0)))
                pan_x, pan_y = (width - 400.0 * scale) / 2.0, (height - 300.0 * scale) / 2.0
            elif action == "pan":
                pan_x += x
                pan_y += y
            else:
                factor = math.exp(-value * 0.001) if action == "wheel" else math.exp(value)
                next_scale = max(0.05, min(64.0, scale * factor))
                world_x, world_y = (x - pan_x) / scale, (y - pan_y) / scale
                pan_x, pan_y, scale = x - world_x * next_scale, y - world_y * next_scale, next_scale
        elif name == "index_progress":
            if generation != command["generation"]:
                cancel_requested = False
            generation, phase = command["generation"], command["phase"]
            progress = {"percent": command["percent"], "label": command["label"]}
            if phase == "complete":
                progress["percent"] = 100
        elif name == "cancel":
            if phase == "active":
                cancel_requested = True
        elif name == "screenshot":
            filename = f"frame-{screenshot_number:04d}.png"
            if navigation_mode:
                png = _png(width, height, seed, progress["percent"])
                marker = _chunk(b"tEXt", f"scale={scale:.6f};pan={pan_x:.3f},{pan_y:.3f}".encode())
                png = png[:-12] + marker + png[-12:]
                (out / filename).write_bytes(png)
                rendered_name = f"navigation-frame-{screenshot_number:04d}.png"
                rendered = _navigation_png(width, height, scale, pan_x, pan_y,
                                           progress["percent"], phase, cancel_requested)
                (out / rendered_name).write_bytes(rendered)
                outputs.append(rendered_name)
            else:
                (out / filename).write_bytes(_png(width, height, seed, progress["percent"]))
            screenshot_number += 1
            outputs.append(filename)
        elif name == "accessibility":
            filename = f"accessibility-{accessibility_number:04d}.json"
            tree = {"bounds": {"height": height, "width": width, "x": 0, "y": 0}, "children": [
                {"bounds": {"height": height, "width": width, "x": 0, "y": 0}, "id": "canvas",
                 "name": f"Zoom {scale * 100:.3f}%" if navigation_mode else "Document canvas", "role": "canvas", "states": [], "value": document_id},
                {"bounds": {"height": 20 if navigation_mode else 16, "width": width, "x": 0, "y": 0}, "id": "progress",
                 "name": progress["label"], "role": "progressbar" if navigation_mode else "status", "states": [], "value": progress["percent"]},
            ], "id": "root", "name": "Vexel placeholder", "role": "application",
                "states": [], "value": "", "virtual_time_ns": virtual_ns}
            if navigation_mode and phase == "active":
                tree["children"].append({"bounds": {"height": 28, "width": 96, "x": 0, "y": 20},
                    "enabled": not cancel_requested, "id": "cancel", "name": "Cancel indexing", "role": "button", "value": ""})
            if navigation_mode:
                tree["generation"] = generation
                tree["phase"] = phase
                tree["pan"] = {"x": round(pan_x, 3), "y": round(pan_y, 3)}
                tree["scale"] = round(scale, 6)
            (out / filename).write_bytes(_json_bytes(tree))
            if navigation_mode:
                (out / f"semantics-{accessibility_number:04d}.json").write_bytes(_json_bytes(tree))
            accessibility_number += 1
            outputs.append(filename)
        elif name == "save":
            saved = {"document_id": document_id, "height": height, "keys_down": sorted(keys),
                     "last_text_commit": text_commits[-1] if text_commits else "", "pointer": pointer,
                     "progress": progress, "seed": seed, "virtual_time_ns": virtual_ns, "width": width}
            (out / "document.json").write_bytes(_json_bytes(saved))
            outputs.append("document.json")
        ack = {"command": name, "index": index, "outputs": outputs, "virtual_time_ns": virtual_ns}
        if emit_acks:
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


def validate_metal_smoke_output(output: str) -> list[str]:
    expected = ["presented frames: 2", "byte-exact readback: pass", "resize/reset: pass"]
    if output.splitlines() != expected:
        raise DriverError("native Metal smoke returned missing or malformed presentation evidence")
    return expected


def native_metal_smoke(out: Path) -> float:
    binary = ROOT / "build" / "mac-release" / "bin" / "svgworkbench"
    if not binary.is_file():
        raise DriverError("Release executable is missing; build preset mac-release first")
    started = time.monotonic()
    try:
        result = subprocess.run([str(binary), "--native-metal-smoke"], cwd=ROOT,
                                capture_output=True, text=True, timeout=30, check=False)
    except (OSError, subprocess.TimeoutExpired) as exc:
        raise DriverError(f"native Metal smoke could not complete: {exc}") from exc
    elapsed = time.monotonic() - started
    (out / "native-metal-smoke.txt").write_text(result.stdout, encoding="utf-8")
    (out / "native-metal-smoke.stderr.txt").write_text(result.stderr, encoding="utf-8")
    summary = {"elapsed_seconds": elapsed, "exit_code": result.returncode, "frames": 0,
               "status": "failed"}
    if result.returncode != 0:
        summary["diagnostic"] = result.stderr.strip()[:1000]
        (out / "result.json").write_bytes(_json_bytes(summary))
        raise DriverError(f"native Metal smoke failed with exit {result.returncode}: {result.stderr.strip()[:500]}")
    lines = validate_metal_smoke_output(result.stdout)
    summary.update({"frames": 2, "readback": "byte-exact", "resize_reset": "passed", "status": "passed"})
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
    parser.add_argument("--navigation-replay", action="store_true")
    parser.add_argument("--native-metal-smoke", action="store_true")
    parser.add_argument("--repeat-replay", action="store_true")
    parser.add_argument("--native-metal-replay-smoke", action="store_true")
    parser.add_argument("--native-task-smoke", action="store_true")
    parser.add_argument("--cycles", type=int, default=100)
    parser.add_argument("--repeats", type=int, default=20)
    args = parser.parse_args(argv)
    if sys.platform != "darwin":
        print("error: UI test driver is supported on macOS only", file=sys.stderr)
        return 2
    if not 0 <= args.seed <= 0xFFFFFFFF:
        print("error: seed must be an unsigned 32-bit integer", file=sys.stderr)
        return 2
    try:
        modes = sum((args.native_metal_smoke, args.native_metal_replay_smoke, args.native_task_smoke,
                     args.native_platform_smoke, args.repeat_replay, args.navigation_replay))
        if modes > 1:
            raise DriverError("driver modes cannot be combined")
        if args.native_task_smoke:
            if args.script is not None:
                raise DriverError("--script cannot be combined with native task smoke")
            out = _safe_native_output(args.output_dir)
            native_task_smoke(out, args.cycles)
        elif args.native_metal_replay_smoke:
            if args.script is not None:
                raise DriverError("--script cannot be combined with native Metal replay")
            out = _safe_native_output(args.output_dir)
            native_metal_replay(out, args.repeats)
        elif args.repeat_replay:
            if args.script is None or not 1 <= args.repeats <= 100:
                raise DriverError("--repeat-replay requires --script and repeats from 1 to 100")
            script = _safe_script(args.script)
            out = _safe_repeat_output(args.output_dir)
            repeat_replay(script, out, args.repeats, args.seed)
        elif args.native_metal_smoke:
            if args.script is not None:
                raise DriverError("--script cannot be combined with --native-metal-smoke")
            out = _safe_native_output(args.output_dir)
            native_metal_smoke(out)
        elif args.native_platform_smoke:
            if args.script is not None:
                raise DriverError("--script cannot be combined with --native-platform-smoke")
            out = _safe_native_output(args.output_dir)
            native_smoke(out, args.cycles)
        elif args.navigation_replay:
            if args.script is None:
                raise DriverError("--navigation-replay requires --script")
            out = _safe_output(args.output_dir)
            script = _safe_script(args.script)
            commands = read_script(script)
            out.parent.mkdir(parents=True, exist_ok=True)
            out.mkdir()
            replay(commands, out, args.seed, navigation_mode=True)
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
