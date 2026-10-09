#!/usr/bin/env python3
"""Run and validate M4 corpus coverage using the established progressive index harness."""

from __future__ import annotations

import argparse
import importlib.util
import json
import shutil
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
M3_SCRIPT = ROOT / "reports/M3/head_to_head.py"
RAW = ROOT / ".cache/M4/head-to-head-raw.json"
POINTER = ROOT / ".cache/M4/corpus-pointer.json"
REPORT = ROOT / "reports/M4/head-to-head.json"
MARKDOWN = ROOT / "reports/M4/head-to-head.md"


def load_harness():
    spec = importlib.util.spec_from_file_location("m3_head_to_head", M3_SCRIPT)
    if spec is None or spec.loader is None:
        raise RuntimeError("cannot load the frozen M3 progressive index harness")
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def render(harness, raw: dict) -> tuple[str, str]:
    report = harness.aggregate(raw)
    report["environment"]["binary_path"] = "build/mac-release/bin/svg_index_bench"
    size_paths = {
        "debug_svgworkbench": ROOT / "build/mac-debug/bin/svgworkbench",
        "release_svgworkbench": ROOT / "build/mac-release/bin/svgworkbench",
        "release_svg_index_bench": ROOT / "build/mac-release/bin/svg_index_bench",
        "debug_shape_render_cli": ROOT / "build/mac-debug/bin/shape_render_cli",
        "release_shape_render_cli": ROOT / "build/mac-release/bin/shape_render_cli",
        "debug_libvexel_shape": ROOT / "build/mac-debug/libvexel_shape.a",
        "release_libvexel_shape": ROOT / "build/mac-release/libvexel_shape.a",
    }
    report["binary_sizes_bytes"] = {
        name: path.stat().st_size for name, path in size_paths.items()
    }
    markdown = harness.render_markdown(report)
    report["results"] = [
        {
            **{key: row[key] for key in ("id", "path", "tier", "bytes", "sha256", "outcome", "trial_outcomes")},
            "metrics": row["metrics"],
            "inkscape": row["inkscape"],
        }
        for row in report["results"]
    ]
    return json.dumps(report, indent=2) + "\n", markdown


def run(harness) -> None:
    raw = harness.run_campaign()
    RAW.parent.mkdir(parents=True, exist_ok=True)
    RAW.write_text(json.dumps(raw, indent=2) + "\n", encoding="utf-8")
    shutil.copyfile(harness.POINTER, POINTER)
    report_json, report_md = render(harness, raw)
    REPORT.write_text(report_json, encoding="utf-8")
    MARKDOWN.write_text(report_md, encoding="utf-8")
    print(f"reported {len(raw['results'])} files / "
          f"{sum(len(row['trials']) for row in raw['results'])} fresh trials")


def check(harness) -> None:
    raw = json.loads(RAW.read_text(encoding="utf-8"))
    if json.loads(POINTER.read_text(encoding="utf-8")) != json.loads(harness.POINTER.read_text(encoding="utf-8")):
        raise ValueError("M4 corpus pointer differs from the verified harness pointer")
    expected_json, expected_md = render(harness, raw)
    if REPORT.read_text(encoding="utf-8") != expected_json or MARKDOWN.read_text(encoding="utf-8") != expected_md:
        raise ValueError("M4 report differs from retained trials or current corpus inputs")
    print(f"checked {len(raw['results'])} files / "
          f"{sum(len(row['trials']) for row in raw['results'])} fresh trials")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    modes = parser.add_mutually_exclusive_group(required=True)
    modes.add_argument("--run", action="store_true")
    modes.add_argument("--check", action="store_true")
    args = parser.parse_args()
    harness = load_harness()
    try:
        run(harness) if args.run else check(harness)
    except (OSError, ValueError, KeyError, json.JSONDecodeError, RuntimeError) as exc:
        print(f"M4 corpus benchmark error: {exc}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
