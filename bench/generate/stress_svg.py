#!/usr/bin/env python3
"""Generate deterministic, bounded SVG stress inputs without buffering documents."""

from __future__ import annotations

import argparse
import base64
import hashlib
import json
import os
import platform
import random
import re
import subprocess
import sys
import tempfile
import time
import zlib
from pathlib import Path
from typing import BinaryIO, Iterator

CHUNK = 64 * 1024
MAX_BYTES = 1_000_000_000
MAX_OBJECTS = 1_000_000
MAX_PATH_COMMANDS = 5_000_000
MAX_DEPTH = 128
MAX_IMAGE_BYTES = 16_000_000
FAMILIES = ("object-heavy", "path-heavy", "nested-depth", "embedded-image")
PNG_SIGNATURE = b"\x89PNG\r\n\x1a\n"
PNG_PIXEL = zlib.compress(b"\x00\x20\x60\xa0\xff")


def _validate(family: str, seed: int, size: int, objects: int, commands: int,
              depth: int, image_bytes: int) -> None:
    if family not in FAMILIES and family != "mixed":
        raise ValueError("unknown family")
    if type(seed) is not int or seed < 0 or seed > 0xFFFFFFFFFFFFFFFF:
        raise ValueError("seed must be an unsigned 64-bit integer")
    if type(size) is not int or size < 128 or size > MAX_BYTES:
        raise ValueError(f"size must be between 128 and {MAX_BYTES}")
    if type(objects) is not int or objects < 0 or objects > MAX_OBJECTS:
        raise ValueError(f"objects must be between 0 and {MAX_OBJECTS}")
    if type(commands) is not int or commands < 0 or commands > MAX_PATH_COMMANDS:
        raise ValueError(f"commands must be between 0 and {MAX_PATH_COMMANDS}")
    if type(depth) is not int or depth < 0 or depth > MAX_DEPTH:
        raise ValueError(f"depth must be between 0 and {MAX_DEPTH}")
    if type(image_bytes) is not int or image_bytes < 0 or image_bytes > MAX_IMAGE_BYTES:
        raise ValueError(f"image payload must be between 0 and {MAX_IMAGE_BYTES}")


def _write_all(stream: BinaryIO, data: bytes, digest: "hashlib._Hash") -> None:
    stream.write(data)
    digest.update(data)


def _file_sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(CHUNK), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _png_chunk(kind: bytes, data: bytes) -> bytes:
    body = kind + data
    return len(data).to_bytes(4, "big") + body + (zlib.crc32(body) & 0xFFFFFFFF).to_bytes(4, "big")


def _image_payload(size: int) -> Iterator[bytes]:
    ihdr = _png_chunk(b"IHDR", b"\x00\x00\x00\x01\x00\x00\x00\x01\x08\x06\x00\x00\x00")
    idat = _png_chunk(b"IDAT", PNG_PIXEL)
    base = len(PNG_SIGNATURE) + len(ihdr) + len(idat) + 12
    if size < base:
        raise ValueError(f"valid embedded PNG payload must be at least {base} bytes")
    yield PNG_SIGNATURE
    yield ihdr
    yield idat
    remaining = size - base
    while remaining:
        # PNG ancillary tEXt chunks carry deterministic padding without changing pixels.
        chunk_size = min(remaining, 64 * 1024 + 12)
        if 0 < remaining - chunk_size < 12:
            chunk_size -= 12 - (remaining - chunk_size)
        data_size = chunk_size - 12
        yield _png_chunk(b"tEXt", b"vexel\x00" + b"x" * (data_size - 6))
        remaining -= chunk_size
    yield _png_chunk(b"IEND", b"")


def _b64_chunks(chunks: Iterator[bytes]) -> Iterator[bytes]:
    carry = b""
    for chunk in chunks:
        data = carry + chunk
        usable = len(data) - len(data) % 3
        if usable:
            yield base64.b64encode(data[:usable])
        carry = data[usable:]
    if carry:
        yield base64.b64encode(carry)


def _semantic_chunks(family: str, seed: int, objects: int, commands: int, depth: int,
                     image_bytes: int) -> tuple[Iterator[bytes], dict[str, int]]:
    rng = random.Random(seed)
    nested = depth if family in ("nested-depth", "mixed") else 0
    object_count = objects if family == "object-heavy" else (max(1, objects // 3) if family == "mixed" else 0)
    command_count = commands if family == "path-heavy" else (max(1, commands // 3) if family == "mixed" else 0)
    payload_count = image_bytes if family == "embedded-image" else (max(0, image_bytes // 2) if family == "mixed" else 0)
    counts = {"objects": 0, "path_commands": 0, "nested_groups": nested,
              "images": 0, "image_payload_bytes": 0}
    counts.update(objects=object_count, path_commands=command_count,
                  images=int(payload_count > 0), image_payload_bytes=payload_count)
    def chunks() -> Iterator[bytes]:
        yield b'<svg xmlns="http://www.w3.org/2000/svg" width="256" height="256">\n'
        if nested:
            yield b"<g>" * nested
        if family in ("object-heavy", "mixed"):
            for _ in range(object_count):
                x, y = rng.randrange(256), rng.randrange(256)
                yield f'<rect x="{x}" y="{y}" width="1" height="1"/>\n'.encode()
        if family in ("path-heavy", "mixed"):
            if command_count:
                yield b'<path d="M0 0 '
                for _ in range(command_count):
                    yield f'L{rng.randrange(256)} {rng.randrange(256)} '.encode()
                yield b'"/>\n'
        if family in ("embedded-image", "mixed"):
            if payload_count:
                yield b'<image width="1" height="1" href="data:image/png;base64,'
                yield from _b64_chunks(_image_payload(payload_count))
                yield b'"/>\n'
    return chunks(), counts


def _expected_output(family: str, seed: int, objects: int, commands: int,
                     depth: int, image_bytes: int, size: int) -> tuple[int, str]:
    chunks, _ = _semantic_chunks(family, seed, objects, commands, depth, image_bytes)
    total = 0
    for chunk in chunks:
        total += len(chunk)
        if total > size:
            raise ValueError("semantic content exceeds requested output size")
    suffix = b"</g>\n" * (depth if family in ("nested-depth", "mixed") else 0) + b"</svg>\n"
    padding = size - total - len(suffix)
    if padding < 7:
        raise ValueError("semantic content exceeds requested output size")
    digest = hashlib.sha256()
    chunks, _ = _semantic_chunks(family, seed, objects, commands, depth, image_bytes)
    for chunk in chunks:
        digest.update(chunk)
    digest.update(b"<!--")
    remaining = padding - 7
    filler = b"x" * min(CHUNK, remaining)
    while remaining:
        block = filler if remaining >= len(filler) else filler[:remaining]
        digest.update(block)
        remaining -= len(block)
    digest.update(b"-->")
    digest.update(suffix)
    return total, digest.hexdigest()


def _install_atomically(path: Path, family: str, seed: int, objects: int,
                        commands: int, depth: int, image_bytes: int,
                        size: int, semantic_size: int, expected_digest: str) -> str:
    fd, temporary = tempfile.mkstemp(prefix=".stress-svg-", dir=path.parent)
    digest = hashlib.sha256()
    written = 0
    try:
        with os.fdopen(fd, "wb") as stream:
            marker, closer = b"<!--", b"-->"
            chunks, _ = _semantic_chunks(family, seed, objects, commands, depth, image_bytes)
            for chunk in chunks:
                _write_all(stream, chunk, digest); written += len(chunk)
            suffix = b"</g>\n" * (depth if family in ("nested-depth", "mixed") else 0) + b"</svg>\n"
            padding = size - semantic_size - len(suffix) - 7
            pad_body = padding
            _write_all(stream, marker, digest); written += len(marker)
            filler = b"x" * min(CHUNK, pad_body)
            remaining = pad_body
            while remaining:
                block = filler if remaining >= len(filler) else filler[:remaining]
                _write_all(stream, block, digest); written += len(block); remaining -= len(block)
            _write_all(stream, closer, digest); written += len(closer)
            _write_all(stream, suffix, digest); written += len(suffix)
            stream.flush(); os.fsync(stream.fileno())
        if written != size or digest.hexdigest() != expected_digest:
            raise ValueError("generated bytes failed their deterministic size/hash check")
        # Linking makes installation atomic and refuses a destination race.
        os.link(temporary, path)
        os.unlink(temporary)
    except BaseException:
        try:
            os.close(fd)
        except OSError:
            pass
        try:
            os.unlink(temporary)
        except FileNotFoundError:
            pass
        raise
    return digest.hexdigest()


def generate(path: Path, *, family: str, seed: int, size: int,
             objects: int = 100, commands: int = 100, depth: int = 4,
             image_bytes: int = 1024) -> dict[str, object]:
    """Create atomically or verify an identical existing deterministic result."""
    _validate(family, seed, size, objects, commands, depth, image_bytes)
    path = Path(path)
    if ".." in path.parts or path.name in ("", ".", ".."):
        raise ValueError("destination path is invalid")
    path.parent.mkdir(parents=True, exist_ok=True)
    expected_semantic_size, expected_digest = _expected_output(
        family, seed, objects, commands, depth, image_bytes, size)
    _, counts = _semantic_chunks(family, seed, objects, commands, depth, image_bytes)
    started = time.monotonic()
    try:
        report_path = str(path.relative_to(Path(__file__).resolve().parents[2]))
    except ValueError:
        report_path = str(path)
    if path.is_symlink():
        raise ValueError("destination must not be a symbolic link")
    if path.exists():
        if (not path.is_file() or path.stat().st_size != size
                or _file_sha256(path) != expected_digest):
            raise ValueError("existing destination differs; refusing overwrite")
        return {"path": report_path, "family": family, "seed": seed, "bytes": size,
                "sha256": expected_digest, "counts": counts,
                "elapsed_seconds": round(time.monotonic() - started, 6)}
    digest = _install_atomically(path, family, seed, objects, commands, depth,
                                 image_bytes, size, expected_semantic_size, expected_digest)
    return {"path": report_path, "family": family, "seed": seed, "bytes": size,
            "sha256": digest, "counts": counts,
            "elapsed_seconds": round(time.monotonic() - started, 6)}


def _inside_cache(path: Path) -> Path:
    root = Path(__file__).resolve().parents[2]
    target = path if path.is_absolute() else root / path
    cache = (root / ".cache").resolve()
    resolved_parent = target.parent.resolve()
    if resolved_parent != cache and cache not in resolved_parent.parents:
        raise ValueError("CLI output must be inside repo-local .cache")
    return target


def verify_report(path: Path) -> None:
    report = json.loads(path.read_text(encoding="utf-8"))
    verified = 0
    for item in report["artifacts"]:
        artifact = Path(item["path"])
        if not artifact.is_absolute():
            artifact = Path(__file__).resolve().parents[2] / artifact
        if not artifact.is_file() or artifact.stat().st_size != item["bytes"]:
            raise ValueError(f"artifact size mismatch: {artifact}")
        digest = hashlib.sha256()
        with artifact.open("rb") as stream:
            for chunk in iter(lambda: stream.read(CHUNK), b""):
                digest.update(chunk)
        if digest.hexdigest() != item["sha256"]:
            raise ValueError(f"artifact hash mismatch: {artifact}")
        verified += 1
    print(f"verified {verified} generated artifacts by size and SHA-256")


def matrix(output_dir: Path, report_path: Path, seed: int) -> None:
    specs = (("object-heavy", 10_000_000, 10_000, 0, 0, 0, "object-10mb.svg"),
             ("path-heavy", 100_000_000, 0, 100_000, 0, 0, "path-100mb.svg"),
             ("mixed", MAX_BYTES, 100_000, 1_000_000, 32, 512_000, "mixed-1gb.svg"))
    artifacts = []
    for family, size, objects, commands, depth, image_bytes, name in specs:
        artifacts.append(generate(output_dir / name, family=family, seed=seed, size=size,
                                  objects=objects, commands=commands, depth=depth,
                                  image_bytes=image_bytes))
    report_path.parent.mkdir(parents=True, exist_ok=True)
    profile = subprocess.run(["system_profiler", "SPHardwareDataType"], capture_output=True,
                             text=True, check=False).stdout
    def profile_value(label: str) -> str:
        match = re.search(rf"(?m)^\s*{re.escape(label)}:\s*(.+)$", profile)
        return match.group(1).strip() if match else "unavailable"
    machine = {"platform": platform.platform(), "architecture": platform.machine(),
               "python": platform.python_version(), "model_identifier": profile_value("Model Identifier"),
               "chip": profile_value("Chip"), "memory": profile_value("Memory"),
               "macos_version": platform.mac_ver()[0]}
    report: dict[str, object] = {"schema_version": 1, "seed": seed, "machine": machine,
                                "artifacts": artifacts}
    if report_path.is_file():
        try:
            previous = json.loads(report_path.read_text(encoding="utf-8"))
            previous_artifacts = previous.get("artifacts")
            if (previous.get("seed") != seed or not isinstance(previous_artifacts, list)
                    or [(Path(item.get("path", "")).name, item.get("bytes"), item.get("sha256"))
                        for item in previous_artifacts]
                    != [(Path(item.get("path", "")).name, item.get("bytes"), item.get("sha256"))
                        for item in artifacts]):
                raise ValueError("existing benchmark report does not match generated artifacts")
            report["artifacts"] = [
                {**current, "elapsed_seconds": previous_item.get("elapsed_seconds",
                                                                  current["elapsed_seconds"])}
                for current, previous_item in zip(artifacts, previous_artifacts)]
            report["machine"] = previous.get("machine", machine)
            if isinstance(previous.get("oracle_render"), dict):
                report["oracle_render"] = previous["oracle_render"]
        except (OSError, json.JSONDecodeError):
            raise ValueError("existing benchmark report is malformed")
    report_bytes = (json.dumps(report, indent=2) + "\n").encode("utf-8")
    fd, temporary = tempfile.mkstemp(prefix=".stress-report-", dir=report_path.parent)
    try:
        with os.fdopen(fd, "wb") as stream:
            stream.write(report_bytes)
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(temporary, report_path)
    except BaseException:
        try:
            os.close(fd)
        except OSError:
            pass
        try:
            os.unlink(temporary)
        except FileNotFoundError:
            pass
        raise
    print(json.dumps({"artifacts": artifacts}, indent=2))


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--matrix", action="store_true")
    parser.add_argument("--seed", type=int, default=0)
    parser.add_argument("--output-dir", type=Path, default=Path(".cache/generated"))
    parser.add_argument("--report", type=Path, default=Path("bench/results/stress.json"))
    parser.add_argument("--verify-report", type=Path)
    args = parser.parse_args(argv)
    try:
        if args.verify_report:
            verify_report(args.verify_report)
        elif args.matrix:
            output = _inside_cache(args.output_dir)
            report = args.report if args.report.is_absolute() else Path(__file__).resolve().parents[2] / args.report
            matrix(output, report, args.seed)
        else:
            parser.error("select --matrix or --verify-report")
    except (OSError, ValueError, KeyError, json.JSONDecodeError) as exc:
        print(f"stress_svg: {exc}", file=sys.stderr); return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
