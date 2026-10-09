#!/usr/bin/env python3
"""Render the pinned M0 corpus once to compact, independently generated PNG goldens."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import platform
import re
import subprocess
import sys
import tempfile
import time
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
GOLDEN = ROOT / "tests/golden"
ORACLE = ROOT / "tests/oracle"
EVIDENCE = ROOT / "artifacts/WO-M0-02/oracle-render.txt"


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(64 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def validate_lock() -> None:
    lock_path = GOLDEN / "manifest.lock"
    lock = json.loads(lock_path.read_text(encoding="utf-8"))
    metadata_path = ORACLE / "metadata.json"
    if (not metadata_path.is_file()
            or json.loads(metadata_path.read_text(encoding="utf-8")) != lock):
        raise ValueError("oracle metadata does not match the immutable golden lock")
    corpus = json.loads((ROOT / "bench/corpus/manifest.json").read_text(encoding="utf-8"))
    entries = lock["fixtures"]
    if len(entries) != 24 or len(corpus["fixtures"]) != 24:
        raise ValueError("oracle lock must contain all 24 pinned fixtures")
    by_id = {item["fixture_id"]: item for item in entries}
    if set(by_id) != {item["fixture_id"] for item in corpus["fixtures"]}:
        raise ValueError("oracle source fixture set differs from the pinned corpus")
    for source in corpus["fixtures"]:
        entry = by_id[source["fixture_id"]]
        source_path = ROOT / source["path"]
        png_path = ROOT / entry["png_path"]
        if sha256(source_path) != source["sha256"] or entry["source_sha256"] != source["sha256"]:
            raise ValueError(f"oracle source hash mismatch: {source['fixture_id']}")
        if not png_path.is_file() or sha256(png_path) != entry["png_sha256"]:
            raise ValueError(f"oracle PNG hash mismatch: {source['fixture_id']}")
        if entry["width"] != 256 or entry["height"] != 256:
            raise ValueError("oracle PNG dimensions are not fixed at 256x256")
        with png_path.open("rb") as stream:
            header = stream.read(24)
        if (len(header) != 24 or header[:8] != b"\x89PNG\r\n\x1a\n"
                or header[12:16] != b"IHDR"
                or int.from_bytes(header[16:20], "big") != 256
                or int.from_bytes(header[20:24], "big") != 256):
            raise ValueError(f"oracle PNG raster dimensions mismatch: {source['fixture_id']}")


def _verify_existing(lock_path: Path, renderer: str, version: str,
                     expected_version: str) -> bool:
    if not lock_path.exists():
        return False
    validate_lock()
    lock = json.loads(lock_path.read_text(encoding="utf-8"))
    if (lock["renderer"]["name"] != "librsvg rsvg-convert"
            or lock["renderer"]["version"] != expected_version
            or lock["renderer"]["version_output"] != version.strip()):
        raise ValueError("existing oracle renderer metadata differs; refusing regeneration")
    metadata_path = ORACLE / "metadata.json"
    if not metadata_path.is_file() or metadata_path.read_bytes() != lock_path.read_bytes():
        raise ValueError("oracle metadata copy does not match the immutable lock")
    corpus = json.loads((ROOT / "bench/corpus/manifest.json").read_text(encoding="utf-8"))
    expected = {item["fixture_id"]: item for item in lock["fixtures"]}
    cache = ROOT / ".cache/m0-02"
    cache.mkdir(parents=True, exist_ok=True)
    started = time.monotonic()
    with tempfile.TemporaryDirectory(prefix="oracle-render-", dir=cache) as temporary:
        entries = _render_fixtures(renderer, corpus["fixtures"], Path(temporary))
        elapsed = round(time.monotonic() - started, 6)
        for entry in entries:
            pinned = expected[entry["fixture_id"]]
            if (entry["source_sha256"] != pinned["source_sha256"]
                    or entry["png_sha256"] != pinned["png_sha256"]):
                raise ValueError(f"fresh renderer output differs from immutable baseline: {entry['fixture_id']}")
        _record_render(elapsed, expected_version, lock, entries, lock_path,
                       "re-rendered to temporary files; committed goldens unchanged")
    return True


def _render_fixtures(renderer: str, fixtures: list[dict[str, object]],
                     png_dir: Path) -> list[dict[str, object]]:
    entries = []
    for fixture in fixtures:
        source = ROOT / str(fixture["path"])
        destination = png_dir / f"{fixture['fixture_id']}.png"
        subprocess.run([renderer, "--format", "png", "--width", "256", "--height", "256",
                        "--output", str(destination), str(source)], check=True,
                       capture_output=True, text=True)
        entries.append({"fixture_id": fixture["fixture_id"], "source_path": fixture["path"],
                        "source_sha256": sha256(source),
                        "png_path": str(destination.relative_to(ROOT)),
                        "png_sha256": sha256(destination), "width": 256, "height": 256,
                        "license": "MPL-2.0 project-authored test data"})
    return entries


def _environment() -> dict[str, str]:
    profile = subprocess.run(["system_profiler", "SPHardwareDataType"], capture_output=True,
                             text=True, check=False).stdout
    def profile_value(label: str) -> str:
        match = re.search(rf"(?m)^\s*{re.escape(label)}:\s*(.+)$", profile)
        return match.group(1).strip() if match else "unavailable"
    return {"os": platform.platform(), "architecture": platform.machine(),
            "macos_version": platform.mac_ver()[0],
            "model_identifier": profile_value("Model Identifier"),
            "chip": profile_value("Chip"), "memory": profile_value("Memory")}


def _make_lock(expected_version: str, version: str, environment: dict[str, str],
               entries: list[dict[str, object]]) -> dict[str, object]:
    renderer = {"name": "librsvg rsvg-convert", "version": expected_version,
                "license": "LGPL-2.1-or-later (Homebrew package metadata)",
                "upstream": "https://gitlab.gnome.org/GNOME/librsvg",
                "documentation": ["https://gnome.pages.gitlab.gnome.org/librsvg/devel-docs/product.html",
                                  "https://gnome.pages.gitlab.gnome.org/librsvg/Rsvg-2.0/index.html"],
                "version_output": version.strip()}
    return {"schema_version": 1,
            "purpose": "immutable independent test oracle; not SVG conformance or Vexel performance",
            "renderer": renderer, "environment": environment,
            "raster": {"width": 256, "height": 256, "format": "PNG"},
            "fixtures": entries}


def _record_render(elapsed: float, expected_version: str, lock: dict[str, object],
                   entries: list[dict[str, object]], lock_path: Path,
                   mode: str) -> None:
    results_path = ROOT / "bench/results/WO-M0-02.json"
    results = json.loads(results_path.read_text(encoding="utf-8"))
    results["oracle_render"] = {"fixture_count": len(entries), "canvas": [256, 256],
                                "elapsed_seconds": elapsed,
                                "mode": mode,
                                "renderer_version": expected_version,
                                "renderer_license": "LGPL-2.1-or-later",
                                "lock_sha256": sha256(lock_path),
                                "png_total_bytes": sum((ROOT / str(e["png_path"])).stat().st_size
                                                        for e in entries),
                                "machine": lock["environment"]}
    results_path.write_text(json.dumps(results, indent=2) + "\n", encoding="utf-8")
    EVIDENCE.parent.mkdir(parents=True, exist_ok=True)
    EVIDENCE.write_text("mode: " + mode + "\n"
                        + "renderer: " + str(lock["renderer"]["version_output"]).replace("\n", "; ") + "\n"
                        + "license: LGPL-2.1-or-later (Homebrew package metadata)\n"
                        + "fixture_count: 24\ncanvas: 256x256\nelapsed_seconds: "
                        + f"{elapsed:.6f}\nlock_sha256: {sha256(lock_path)}\n", encoding="utf-8")


def render(renderer: str, expected_version: str) -> None:
    lock_path = GOLDEN / "manifest.lock"
    version = subprocess.run([renderer, "--version"], check=True, capture_output=True,
                             text=True).stdout
    if f"rsvg-convert version {expected_version}" not in version:
        raise ValueError(f"renderer version mismatch: expected {expected_version}")
    if _verify_existing(lock_path, renderer, version, expected_version):
        return
    metadata = json.loads((ROOT / "bench/corpus/manifest.json").read_text(encoding="utf-8"))
    png_dir = GOLDEN / "fixtures"
    png_dir.mkdir(parents=True, exist_ok=False)
    started = time.monotonic()
    entries = _render_fixtures(renderer, metadata["fixtures"], png_dir)
    lock = _make_lock(expected_version, version, _environment(), entries)
    GOLDEN.mkdir(parents=True, exist_ok=True)
    encoded = json.dumps(lock, indent=2) + "\n"
    lock_path.write_text(encoded, encoding="utf-8")
    ORACLE.mkdir(parents=True, exist_ok=False)
    (ORACLE / "metadata.json").write_text(encoded, encoding="utf-8")
    validate_lock()
    _record_render(round(time.monotonic() - started, 6), expected_version,
                   lock, entries, lock_path, "goldens generated and pinned")


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--render", action="store_true")
    parser.add_argument("--renderer", default="rsvg-convert")
    parser.add_argument("--expected-version", default="2.62.3")
    args = parser.parse_args(argv)
    try:
        if args.render:
            render(args.renderer, args.expected_version)
        else:
            validate_lock()
    except (OSError, ValueError, KeyError, json.JSONDecodeError, subprocess.CalledProcessError) as exc:
        print(f"render_oracle: {exc}", file=sys.stderr); return 2
    print("oracle lock valid: 24 fixtures, source and PNG SHA-256 verified")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
